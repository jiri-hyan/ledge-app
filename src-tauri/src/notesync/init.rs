//! První synchronizace a slučování: stav kořene (`root_state`), založení repa (`Syncer::fresh`) a
//! napojení existujících poznámek na neprázdný remote (`Syncer::clone_merge`: clone vedle, sloučit
//! podle `merge_plan`, push, prohodit složky, záloha `.pred-sync-…`). Závisí na `git.rs`, `plan.rs`.

use super::git::{commit_all, git, local_fail, net_fail, setup_repo};
use super::{Cmp, Ctx, Outcome, RootState, Syncer, commit_message, is_tmp, local_only, merge_plan};
use chrono::{DateTime, FixedOffset};
use std::path::{Path, PathBuf};

/// Všechny soubory pod `dir` (relativně, `/`), bez `.git` a symlinků.
fn walk(dir: &Path) -> Vec<String> {
    fn go(base: &Path, dir: &Path, out: &mut Vec<String>) {
        let Ok(rd) = std::fs::read_dir(dir) else { return };
        for e in rd.flatten() {
            let Ok(t) = e.file_type() else { continue };
            let p = e.path();
            if t.is_symlink() {
                continue;
            }
            if t.is_dir() {
                if dir == base && e.file_name() == ".git" {
                    continue;
                }
                go(base, &p, out);
            } else if t.is_file()
                && let Ok(rel) = p.strip_prefix(base)
            {
                out.push(rel.to_string_lossy().replace('\\', "/"));
            }
        }
    }
    let mut out = Vec::new();
    go(dir, dir, &mut out);
    out.sort();
    out
}

pub(super) fn rel_path(base: &Path, rel: &str) -> PathBuf {
    rel.split('/').fold(base.to_path_buf(), |p, c| p.join(c))
}

/// Má kořen něco, co se má synchronizovat (ne jen lokální stav)?
fn has_notes(root: &Path) -> bool {
    walk(root).iter().any(|r| !local_only(r) && !is_tmp(r))
}

pub(super) fn root_state(root: &Path, remote: &str) -> RootState {
    if root.join(".git").exists() {
        return RootState::Repo;
    }
    if !root.is_dir() {
        return RootState::Empty;
    }
    if let Ok(top) = git(root, remote, &["rev-parse", "--show-toplevel"], false) {
        return RootState::InsideOtherRepo(top.trim().to_string());
    }
    if has_notes(root) { RootState::LocalFiles } else { RootState::Empty }
}

/// Otisk souboru (velikost + mtime) — druhé kolo slučování bere jen to, co se od prvního změnilo.
type Prints = std::collections::HashMap<String, (u64, Option<std::time::SystemTime>)>;

fn print_of(p: &Path) -> (u64, Option<std::time::SystemTime>) {
    std::fs::metadata(p).map(|m| (m.len(), m.modified().ok())).unwrap_or((0, None))
}

/// Zkopíruje lokální soubory `src` do `dst` podle [`merge_plan`]; soubory se stejným otiskem jako
/// v `skip` přeskočí. Vrací otisky všech souborů `src` (z doby před kopírováním).
fn merge_into(src: &Path, dst: &Path, pc: &str, skip: &Prints) -> Result<Prints, String> {
    let all = walk(src);
    let prints: Prints = all.iter().map(|rel| (rel.clone(), print_of(&rel_path(src, rel)))).collect();
    let files: Vec<(String, Cmp)> = all
        .into_iter()
        .filter(|rel| skip.get(rel) != prints.get(rel))
        .map(|rel| {
            let d = rel_path(dst, &rel);
            let c = match std::fs::read(&d) {
                Err(_) => Cmp::Missing,
                Ok(theirs) => {
                    if std::fs::read(rel_path(src, &rel)).is_ok_and(|ours| ours == theirs) { Cmp::Same } else { Cmp::Different }
                }
            };
            (rel, c)
        })
        .collect();
    let plan = merge_plan(&files, pc, |p| rel_path(dst, p).exists());
    for c in &plan {
        let from = rel_path(src, &c.from);
        let to = rel_path(dst, &c.to);
        if let Some(parent) = to.parent() {
            std::fs::create_dir_all(parent).map_err(|e| trf!("Složka {}: {e}", "Folder {}: {e}", parent.display()))?;
        }
        let res = if c.rewrite.is_empty() {
            std::fs::copy(&from, &to).map(|_| ())
        } else {
            std::fs::read(&from).and_then(|b| match String::from_utf8(b) {
                Ok(mut text) => {
                    for (a, b) in &c.rewrite {
                        text = text.replace(a.as_str(), b.as_str());
                    }
                    std::fs::write(&to, text)
                }
                Err(e) => std::fs::write(&to, e.into_bytes()),
            })
        };
        res.map_err(|e| format!("Kopie {} → {}: {e}", from.display(), to.display()))?;
    }
    Ok(prints)
}

fn stamp(now: &DateTime<FixedOffset>) -> String {
    now.format("%Y%m%d-%H%M%S").to_string()
}

impl Syncer {
    pub(super) fn fresh(&mut self, ctx: &Ctx) -> Result<(), Outcome> {
        std::fs::create_dir_all(ctx.root).map_err(|e| Outcome::Chyba(trf!("Složka {}: {e}", "Folder {}: {e}", ctx.root.display())))?;
        git(ctx.root, ctx.remote, &["init", "-q", "-b", "main"], false).map_err(|f| local_fail("git init", f))?;
        setup_repo(ctx.root, ctx.remote, ctx.pc)?;
        commit_all(ctx.root, ctx.remote, &commit_message(ctx.pc, &ctx.now))?;
        self.notes.push(format!("sync: {} založen jako repo pro {}", ctx.root.display(), ctx.remote));
        Ok(())
    }

    /// Případ b): clone do sourozenecké složky, sloučit lokální, commit + push, prohodit. Stará
    /// složka zůstane jako `<jméno>.pred-sync-<čas>` (nemaže se). Vrací cestu zálohy.
    pub(super) fn clone_merge(&mut self, ctx: &Ctx) -> Result<PathBuf, Outcome> {
        let root = ctx.root;
        let parent = root.parent().ok_or_else(|| Outcome::Chyba(crate::i18n::tr("Kořen poznámek nemá nadřazenou složku.", "The notes folder has no parent folder.").into()))?;
        let name = root.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_else(|| "notes".into());
        std::fs::create_dir_all(parent).map_err(|e| Outcome::Chyba(trf!("Složka {}: {e}", "Folder {}: {e}", parent.display())))?;
        // zbytky dřívějšího nedokončeného pokusu (náš vlastní dočasný klon — data jsou v kořeni / na remote)
        let clone_prefix = format!("{name}.sync-clone-");
        if let Ok(rd) = std::fs::read_dir(parent) {
            for e in rd.flatten().filter(|e| e.file_name().to_string_lossy().starts_with(&clone_prefix)) {
                let _ = std::fs::remove_dir_all(e.path());
            }
        }
        let tmp = parent.join(format!("{clone_prefix}{}", stamp(&ctx.now)));
        let tmp_s = tmp.to_string_lossy().into_owned();
        if let Err(f) = git(parent, ctx.remote, &["clone", "-q", "--origin", "origin", "--", ctx.remote, &tmp_s], true) {
            let _ = std::fs::remove_dir_all(&tmp);
            return Err(net_fail(f));
        }
        let fail = |o: Outcome| {
            let _ = std::fs::remove_dir_all(&tmp);
            o
        };
        setup_repo(&tmp, ctx.remote, ctx.pc).map_err(fail)?;
        let first = merge_into(root, &tmp, ctx.pc, &Prints::new()).map_err(|e| fail(Outcome::Chyba(e)))?;
        commit_all(&tmp, ctx.remote, &commit_message(ctx.pc, &ctx.now)).map_err(fail)?;
        let branch = git(&tmp, ctx.remote, &["symbolic-ref", "--short", "HEAD"], false).map(|b| b.trim().to_string()).unwrap_or_else(|_| "main".into());
        // push hned: i kdyby prohození selhalo, sloučená data už jsou na remote (další pokus je idempotentní)
        if let Err(f) = git(&tmp, ctx.remote, &["push", "-q", "-u", "origin", &branch], true) {
            let o = net_fail(f);
            self.notes.push(format!("sync: push sloučeného klonu selhal ({o:?}), dožene ho další cyklus"));
        }
        let backup = parent.join(format!("{name}.pred-sync-{}", stamp(&ctx.now)));
        if root.exists() {
            std::fs::rename(root, &backup).map_err(|e| fail(Outcome::Chyba(trf!("Nejde odsunout {} (otevřený soubor?): {e}", "Can't move {} aside (an open file?): {e}", root.display()))))?;
        }
        // watcher mohl mezi dvěma přejmenováními založit prázdný kořen znovu → smazat jen prázdný
        let swap = std::fs::rename(&tmp, root).or_else(|_| std::fs::remove_dir(root).and_then(|_| std::fs::rename(&tmp, root)));
        if let Err(e) = swap {
            if backup.exists() {
                let _ = std::fs::remove_dir(root);
                let _ = std::fs::rename(&backup, root);
            }
            return Err(fail(Outcome::Chyba(trf!("Nejde prohodit složky poznámek: {e}", "Can't swap the notes folders: {e}"))));
        }
        // co mezitím přibylo ve staré složce (zápis během klonování) — stejné se přeskočí
        if backup.exists() {
            match merge_into(&backup, root, ctx.pc, &first) {
                Ok(_) => {}
                Err(e) => self.notes.push(format!("sync: dosloučení z {} selhalo: {e}", backup.display())),
            }
            self.notes.push(format!("sync: poznámky napojené na {}, původní složka ponechána jako {}", ctx.remote, backup.display()));
        }
        Ok(backup)
    }
}
