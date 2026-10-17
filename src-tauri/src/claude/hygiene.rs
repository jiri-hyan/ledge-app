//! Hlídač nedokončené práce session (`claude.enforceCommit`): které soubory session sama upravila
//! (`Track`, otisky `Fp`), baseline repa při prvním eventu, kontrola gitem (`git_hygiene`) a
//! rozhodnutí o blokaci `Stop`. Git vždy s termínem (`Budgets`) a nikdy pod zámkem stavu.
//! Závisí na `paths.rs` (cesty, porcelain) a `gitinfo` (spuštění gitu).

use super::paths::{is_dirty, path_key, porcelain_paths, repo_relative};
use super::{Claude, Inner, SessionState};
use crate::config::EnforceCommit;
use crate::gitinfo;
use serde_json::{Value, json};
use std::path::Path;
use std::time::{Duration, Instant, UNIX_EPOCH};

/// Git při Stop/SessionEnd smí trvat nejvýš tohle (hook Claude Code má timeout 5 s).
const HYGIENE_BUDGET: Duration = Duration::from_millis(2000);
/// Baseline (kořen repa + HEAD) při prvním eventu session.
const BASELINE_BUDGET: Duration = Duration::from_millis(2000);
/// Kolikrát zkusit baseline, když git selže / nedoběhne / cwd není v gitu.
pub(super) const BASELINE_ATTEMPTS: u8 = 3;
/// Strop sledovaných souborů na session (nejstarší vypadávají).
const MAX_FILES: usize = 500;

/// Termíny gitu hlídače. Produkce = konstanty výše; testy je prodlouží (`Claude::patient`), aby
/// nezávisely na vytížení stroje.
#[derive(Debug, Clone, Copy)]
pub(super) struct Budgets {
    pub(super) baseline: Duration,
    pub(super) hygiene: Duration,
}

impl Default for Budgets {
    fn default() -> Self {
        Self { baseline: BASELINE_BUDGET, hygiene: HYGIENE_BUDGET }
    }
}

/// Otisk souboru (délka + mtime) — levný, stačí poznat „někdo na soubor sáhl“.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Fp {
    pub len: u64,
    pub mtime_ns: u128,
}

pub fn fingerprint(path: &str) -> Option<Fp> {
    let m = std::fs::metadata(path).ok()?;
    let mtime_ns = m.modified().ok()?.duration_since(UNIX_EPOCH).ok()?.as_nanos();
    Some(Fp { len: m.len(), mtime_ns })
}

#[derive(Debug, Clone, PartialEq)]
pub(super) struct TrackedFile {
    key: String,
    /// absolutní cesta s `/`
    abs: String,
    /// otisk hned po poslední úpravě touto session
    fp: Option<Fp>,
    /// `false` = soubor byl při kontrole čistý (zacommitovaný); zůstává jen jako pathspec pro commity
    active: bool,
}

/// Interní stav hlídače pro session — mimo `Session`, aby se neklonoval do snapshotu.
#[derive(Debug, Default)]
pub(super) struct Track {
    /// nejstarší první, max. `MAX_FILES`
    pub(super) files: Vec<TrackedFile>,
    /// kořen repa z `rev-parse --show-toplevel`; `None` = zatím nezjištěno / mimo git → hlídač neplatí
    pub(super) repo: Option<String>,
    /// HEAD při prvním eventu (`None` = repo bez commitů)
    pub(super) baseline: Option<String>,
    pub(super) baseline_ok: bool,
    pub(super) baseline_attempts: u8,
    pub(super) baseline_busy: bool,
}

impl Track {
    pub(super) fn record(&mut self, abs: String, fp: Option<Fp>) {
        let key = path_key(&abs);
        self.files.retain(|f| f.key != key);
        self.files.push(TrackedFile { key, abs, fp, active: true });
        if self.files.len() > MAX_FILES {
            let extra = self.files.len() - MAX_FILES;
            self.files.drain(..extra);
        }
    }
}

impl Claude {
    pub(super) fn store_baseline(&self, sid: &str, res: Option<(String, Option<String>)>) {
        let mut g = self.inner.lock();
        let Some(tr) = g.track.get_mut(sid) else { return };
        tr.baseline_busy = false;
        if let (false, Some((repo, head))) = (tr.baseline_ok, res) {
            tr.baseline_ok = true;
            tr.repo = Some(repo);
            tr.baseline = head;
        }
    }

    /// Přepočítá nedokončenou práci session (git mimo zámek). U `Stop` vrátí tělo blokace, pokud
    /// má režim `block` a výsledek je čerstvý — git, který nedoběhl, nikdy neblokuje.
    pub(super) fn run_check(&self, sid: &str, is_stop: bool, mode: EnforceCommit, stop_hook_active: bool) -> Option<String> {
        let snap = {
            let g = self.inner.lock();
            g.track.get(sid).and_then(|t| Some((t.repo.clone()?, t.baseline.clone(), t.files.clone())))
        };
        let mut reply = None;
        let mut g = self.inner.lock();
        if let Some((repo, base, files)) = snap {
            drop(g);
            let h = git_hygiene(&repo, base.as_deref(), &files, self.budgets.hygiene);
            g = self.inner.lock();
            let Inner { sessions, track, .. } = &mut *g;
            if let (Some(sess), Some(tr)) = (sessions.get_mut(sid), track.get_mut(sid)) {
                for (key, fp) in &h.clean {
                    // jen pokud se mezitím nezměnil (nová úprava během kontroly)
                    if let Some(f) = tr.files.iter_mut().find(|f| &f.key == key && &f.fp == fp) {
                        f.active = false;
                    }
                }
                if let Some(u) = &h.uncommitted {
                    sess.uncommitted = u.clone();
                }
                if let Some(n) = h.unpushed {
                    sess.unpushed = n;
                }
                if is_stop
                    && h.uncommitted.is_some()
                    && h.unpushed.is_some()
                    && let Some(reason) = block_reason(mode, stop_hook_active, &sess.uncommitted, sess.unpushed)
                {
                    // Claude místo konce pokračuje
                    sess.state = SessionState::Working;
                    reply = Some(block_body(&reason));
                }
            }
        }
        // ukončená a čistá → pryč (šéf už nemá koho honit)
        if g.sessions.get(sid).is_some_and(|x| x.ended && !x.unfinished()) {
            g.sessions.remove(sid);
            g.track.remove(sid);
        }
        drop(g);
        (self.on_change)();
        reply
    }

    /// Levný recheck ukončených nedokončených session (šéf mohl commitnout ručně). Volá se z
    /// periodického refreshe gitu; blokuje volající vlákno (git), proto jen z pozadí.
    pub fn recheck_ended(&self) {
        let mode = *self.enforce.lock();
        let ids: Vec<String> = self.inner.lock().sessions.values().filter(|s| s.ended).map(|s| s.id.clone()).collect();
        for id in ids {
            if mode == EnforceCommit::Off {
                let mut g = self.inner.lock();
                g.sessions.remove(&id);
                g.track.remove(&id);
                drop(g);
                (self.on_change)();
            } else {
                self.run_check(&id, false, mode, false);
            }
        }
    }
}

/// Soubor, který nástroj session upravil (`PostToolUse`). Bash se nesleduje — poctivě to nejde.
pub fn edited_path(tool: &str, input: &Value) -> Option<String> {
    let key = match tool {
        "Edit" | "Write" | "MultiEdit" => "file_path",
        "NotebookEdit" => "notebook_path",
        _ => return None,
    };
    let p = input.get(key).and_then(Value::as_str)?.trim();
    (!p.is_empty()).then(|| p.to_string())
}

/// Verdikt pro jeden sledovaný soubor při kontrole.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileVerdict {
    /// čistý v gitu → zacommitováno, odepsat ze sledovaných
    Clean,
    /// neuložený a otisk sedí na poslední úpravu session → její nezacommitovaná práce
    Own,
    /// neuložený, ale od její úpravy ho měnil někdo jiný (šéf, jiná session) → nepřipisovat
    Foreign,
}

/// `recorded` = otisk po poslední úpravě session, `current` = otisk teď.
pub fn classify(dirty: bool, recorded: Option<Fp>, current: Option<Fp>) -> FileVerdict {
    match (dirty, recorded) {
        (false, _) => FileVerdict::Clean,
        (true, Some(r)) if current == Some(r) => FileVerdict::Own,
        _ => FileVerdict::Foreign,
    }
}

/// Rozhodnutí o blokaci `Stop`. Nikdy při `stop_hook_active` (Claude už jednou pokračoval kvůli
/// Stop hooku → další blokace by byla smyčka). Text nechává Claudovi cestu ven, když mu uživatel
/// commit / push zakázal nebo čeká na jeho odpověď — postrčit, ne donutit proti pokynu.
pub fn block_reason(mode: EnforceCommit, stop_hook_active: bool, uncommitted: &[String], unpushed: u32) -> Option<String> {
    if mode != EnforceCommit::Block || stop_hook_active {
        return None;
    }
    if !uncommitted.is_empty() {
        let shown = uncommitted.iter().take(10).cloned().collect::<Vec<_>>().join(", ");
        let more = if uncommitted.len() > 10 { format!(" (+{} dalších)", uncommitted.len() - 10) } else { String::new() };
        return Some(format!(
            "Máš nezacommitované své změny: {shown}{more}. Commitni je (a pushni, má-li větev upstream). \
             Pokud ti uživatel řekl necommitovat, nebo čekáš na jeho odpověď, jednou větou to napiš a skonči."
        ));
    }
    (unpushed > 0).then(|| {
        format!(
            "Máš nepushnuté své commity ({unpushed}). Pushni je. \
             Pokud ti uživatel řekl nepushovat, nebo čekáš na jeho odpověď, jednou větou to napiš a skonči."
        )
    })
}

/// Tělo odpovědi http hooku Stop — Claude Code ho zpracuje jako JSON výstup command hooku.
pub fn block_body(reason: &str) -> String {
    json!({ "decision": "block", "reason": reason }).to_string()
}

/// Kořen repa a HEAD pro `cwd`. `None` = selhání / mimo git (zkusí se znovu);
/// repo bez commitů → `Some((kořen, None))`.
pub(super) fn git_baseline(cwd: &str, budget: Duration) -> Option<(String, Option<String>)> {
    let dir = Path::new(cwd);
    if cwd.is_empty() || !dir.is_dir() {
        return None;
    }
    let deadline = Instant::now() + budget;
    if let Some(out) = gitinfo::run(dir, &["rev-parse", "--show-toplevel", "HEAD"], deadline) {
        let mut l = out.lines().map(str::trim).filter(|l| !l.is_empty());
        if let (Some(root), Some(head)) = (l.next(), l.next()) {
            return Some((root.to_string(), Some(head.to_string())));
        }
    }
    // HEAD neexistuje (repo bez commitů)
    let root = gitinfo::run(dir, &["rev-parse", "--show-toplevel"], deadline)?;
    let root = root.trim();
    (!root.is_empty()).then(|| (root.to_string(), None))
}

/// Výsledek kontroly; `None` u položky = ten dotaz nedoběhl → nic netvrdit.
#[derive(Debug, Default)]
struct Hygiene {
    uncommitted: Option<Vec<String>>,
    unpushed: Option<u32>,
    /// zacommitované soubory k odepsání: (klíč, otisk, se kterým se počítalo)
    clean: Vec<(String, Option<Fp>)>,
}

/// Nezacommitované soubory session a počet jejích nepushnutých commitů.
///
/// Nepushnuto = `git rev-list --count <baseline>..HEAD --not --remotes -- <soubory session>`
/// (pathspecy přes stdin): commity přibyté od startu session, které nejsou na ŽÁDNÉM remote-tracking
/// refu a sahají na soubory session — ne šéfovy commity jiných souborů ani commity stažené pullem.
/// Počítá se JEN když větev má upstream (`@{u}`): bez remote/upstreamu by push byl nesplnitelný
/// a hlídač by session honil donekonečna → 0.
fn git_hygiene(repo: &str, baseline: Option<&str>, files: &[TrackedFile], budget: Duration) -> Hygiene {
    let deadline = Instant::now() + budget;
    let dir = Path::new(repo);
    let mut h = Hygiene::default();
    let rel: Vec<(&TrackedFile, String)> = files.iter().filter_map(|f| Some((f, repo_relative(repo, &f.abs)?))).collect();
    let active: Vec<&(&TrackedFile, String)> = rel.iter().filter(|(f, _)| f.active).collect();
    if active.is_empty() {
        h.uncommitted = Some(vec![]);
    } else if let Some(out) = gitinfo::run(
        dir,
        &["--no-optional-locks", "-c", "core.quotePath=false", "status", "--porcelain=v1", "--untracked-files=normal"],
        deadline,
    ) {
        let dirty: Vec<String> = porcelain_paths(&out).iter().map(|d| path_key(d)).filter(|d| !d.is_empty()).collect();
        let mut unc = vec![];
        for (f, r) in active {
            match classify(is_dirty(r, &dirty), f.fp, fingerprint(&f.abs)) {
                FileVerdict::Clean => h.clean.push((f.key.clone(), f.fp)),
                FileVerdict::Own => unc.push(r.clone()),
                FileVerdict::Foreign => {}
            }
        }
        h.uncommitted = Some(unc);
    }
    // nepushnuto — selhání tady nezahodí spočtené `uncommitted`
    if rel.is_empty() || gitinfo::run(dir, &["rev-parse", "--abbrev-ref", "--symbolic-full-name", "@{u}"], deadline).is_none() {
        h.unpushed = Some(0);
        return h;
    }
    let range = baseline.map(|b| format!("{b}..HEAD")).unwrap_or_else(|| "HEAD".into());
    let mut input = format!("{range}\n--\n");
    for (_, r) in &rel {
        input.push_str(r);
        input.push('\n');
    }
    h.unpushed = gitinfo::run_with_input(dir, &["--literal-pathspecs", "rev-list", "--count", "--stdin", "--not", "--remotes"], Some(&input), deadline)
        .and_then(|o| o.trim().parse().ok());
    h
}

#[cfg(test)]
mod tests {
    use super::super::testkit::{ev, git, sess, temp_repo, temp_repo_upstream, write_tool};
    use super::*;
    use std::sync::Arc;

    #[test]
    fn edited_path_table() {
        let cases: &[(&str, Value, Option<&str>)] = &[
            ("Edit", json!({"file_path":"C:/r/a.rs","old_string":"x"}), Some("C:/r/a.rs")),
            ("Write", json!({"file_path":"C:\\r\\b.rs"}), Some("C:\\r\\b.rs")),
            ("MultiEdit", json!({"file_path":"/r/c.rs","edits":[]}), Some("/r/c.rs")),
            ("NotebookEdit", json!({"notebook_path":"C:/r/n.ipynb"}), Some("C:/r/n.ipynb")),
            // NotebookEdit bere jen notebook_path
            ("NotebookEdit", json!({"file_path":"C:/r/n.ipynb"}), None),
            // Bash ani čtení se nesledují
            ("Bash", json!({"command":"echo > x.txt","file_path":"C:/r/x.txt"}), None),
            ("Read", json!({"file_path":"C:/r/a.rs"}), None),
            ("Edit", json!({}), None),
            ("Edit", json!({"file_path":"   "}), None),
            ("Edit", json!({"file_path":42}), None),
            ("Edit", Value::Null, None),
            ("", json!({"file_path":"C:/r/a.rs"}), None),
        ];
        for (tool, input, want) in cases {
            assert_eq!(edited_path(tool, input).as_deref(), *want, "{tool} {input}");
        }
    }

    #[test]
    fn classify_table() {
        use FileVerdict::*;
        let a = Some(Fp { len: 10, mtime_ns: 1 });
        let b = Some(Fp { len: 10, mtime_ns: 2 });
        let c = Some(Fp { len: 11, mtime_ns: 1 });
        let cases: &[(bool, Option<Fp>, Option<Fp>, FileVerdict)] = &[
            // čistý → odepsat, ať je otisk jakýkoli
            (false, a, a, Clean),
            (false, a, b, Clean),
            (false, None, None, Clean),
            // neuložený a beze změny od úpravy session → její
            (true, a, a, Own),
            // někdo na něj od té doby sáhl (mtime / délka) → ne její
            (true, a, b, Foreign),
            (true, a, c, Foreign),
            // smazaný po její úpravě / otisk nezjištěn → nepřipisovat
            (true, a, None, Foreign),
            (true, None, None, Foreign),
            (true, None, a, Foreign),
        ];
        for (dirty, rec, cur, want) in cases {
            assert_eq!(classify(*dirty, *rec, *cur), *want, "{dirty} {rec:?} {cur:?}");
        }
    }

    #[test]
    fn track_record_dedupes_and_caps() {
        let mut t = Track::default();
        for i in 0..(MAX_FILES + 20) {
            t.record(format!("C:/r/f{i}.rs"), None);
        }
        assert_eq!(t.files.len(), MAX_FILES);
        assert_eq!(t.files[0].abs, "C:/r/f20.rs", "nejstarší vypadly");
        // znovu upravený soubor se přesune na konec a znovu aktivuje
        t.files[0].active = false;
        let fp = Some(Fp { len: 1, mtime_ns: 1 });
        t.record("C:/r/f20.rs".into(), fp);
        assert_eq!(t.files.len(), MAX_FILES);
        let last = t.files.last().unwrap();
        assert!(last.active && last.fp == fp);
    }

    #[test]
    fn block_reason_table() {
        use EnforceCommit::*;
        let unc = vec!["a.rs".to_string(), "b.rs".to_string()];
        let own = "Máš nezacommitované své změny: a.rs, b.rs. Commitni je (a pushni, má-li větev upstream). \
                   Pokud ti uživatel řekl necommitovat, nebo čekáš na jeho odpověď, jednou větou to napiš a skonči.";
        let push2 = "Máš nepushnuté své commity (2). Pushni je. \
                     Pokud ti uživatel řekl nepushovat, nebo čekáš na jeho odpověď, jednou větou to napiš a skonči.";
        // režim, stop_hook_active, nezacommitované, nepushnuté, očekávaný důvod
        type Case<'a> = (EnforceCommit, bool, &'a [String], u32, Option<&'a str>);
        let cases: &[Case] = &[
            (Block, false, &unc, 0, Some(own)),
            (Block, false, &unc, 3, Some(own)),
            (Block, false, &[], 2, Some(push2)),
            (Block, false, &[], 0, None),
            // ochrana před smyčkou
            (Block, true, &unc, 2, None),
            (Notify, false, &unc, 2, None),
            (Off, false, &unc, 2, None),
        ];
        for (mode, active, u, n, want) in cases {
            assert_eq!(block_reason(*mode, *active, u, *n).as_deref(), *want, "{mode:?} {active} {n}");
        }
        let many: Vec<String> = (0..12).map(|i| format!("f{i}")).collect();
        assert!(block_reason(Block, false, &many, 0).unwrap().contains("f9 (+2 dalších). Commitni"));
        let b: Value = serde_json::from_str(&block_body("x")).unwrap();
        assert_eq!(b, json!({"decision":"block","reason":"x"}));
    }

    #[test]
    fn hygiene_counts_only_own_files_and_commits() {
        let (repo, _remote) = temp_repo_upstream();
        let dir = repo.path();
        let cwd = dir.to_string_lossy().to_string();
        let c = Claude::new(Arc::new(|| {}), Duration::from_secs(1)).inline().patient();
        c.handle_event(&ev("UserPromptSubmit", "a", &cwd, json!({"prompt":"uprav"})));
        // šéfova ruční rozdělaná práce — nepočítá se
        std::fs::write(dir.join("sefova.txt"), "moje").unwrap();
        std::fs::write(dir.join("base.txt"), "šéf změnil").unwrap();
        // session zapíše soubor; Bash se nesleduje
        std::fs::create_dir_all(dir.join("nova dir")).unwrap();
        write_tool(&c, "a", &cwd, &dir.join("nova dir").join("a b.rs"), "x");
        std::fs::write(dir.join("bash.txt"), "x").unwrap();
        c.handle_event(&ev("PostToolUse", "a", &cwd, json!({"tool_name":"Bash","tool_input":{"command":"echo x > bash.txt"}})));
        // sdílený soubor: session ho upraví, pak na něj sáhne šéf → už není „její“
        write_tool(&c, "a", &cwd, &dir.join("spolecny.txt"), "session");
        std::fs::write(dir.join("spolecny.txt"), "šéf to přepsal jinak").unwrap();
        let reply = c.handle_event(&ev("Stop", "a", &cwd, json!({"stop_hook_active": false})));
        assert_eq!(reply, None, "notify neblokuje");
        let s = sess(&c, "a");
        assert_eq!(s.uncommitted, vec!["nova dir/a b.rs"]);
        assert_eq!(s.unpushed, 0);

        // šéf commitne svou práci → session to nepřipisuje
        git(dir, &["add", "sefova.txt"]);
        git(dir, &["commit", "-q", "-m", "sef"]);
        c.handle_event(&ev("Stop", "a", &cwd, json!({})));
        assert_eq!(sess(&c, "a").unpushed, 0);

        // session commitne → nezacommitováno 0, nepushnuto 1; soubor je odepsaný ze sledovaných
        git(dir, &["add", "nova dir"]);
        git(dir, &["commit", "-q", "-m", "session"]);
        c.handle_event(&ev("Stop", "a", &cwd, json!({})));
        let s = sess(&c, "a");
        assert!(s.uncommitted.is_empty());
        assert_eq!(s.unpushed, 1);
        {
            let g = c.inner.lock();
            let f = g.track["a"].files.iter().find(|f| f.abs.ends_with("a b.rs")).unwrap();
            assert!(!f.active, "zacommitovaný soubor se odepíše");
        }
        // další tah bez úprav: commit pořád nepushnutý (pathspec drží i odepsané soubory)
        c.handle_event(&ev("Stop", "a", &cwd, json!({})));
        assert_eq!(sess(&c, "a").unpushed, 1);

        // ukončená nedokončená session zůstává; resume pod stejným id ji „oživí“
        c.handle_event(&ev("SessionEnd", "a", &cwd, json!({})));
        assert!(sess(&c, "a").ended && sess(&c, "a").unfinished());
        c.handle_event(&ev("UserPromptSubmit", "a", &cwd, json!({})));
        assert!(!sess(&c, "a").ended);
        c.handle_event(&ev("SessionEnd", "a", &cwd, json!({})));
        assert!(sess(&c, "a").ended);

        // šéf pushne ručně → recheck ukončené session ji odebere
        c.recheck_ended();
        assert!(sess(&c, "a").ended, "bez pushe zůstává");
        git(dir, &["push", "-q", "origin", "HEAD:main"]);
        c.recheck_ended();
        assert!(c.sessions().is_empty());
        assert!(c.inner.lock().track.is_empty());
    }

    #[test]
    fn no_upstream_means_nothing_to_push() {
        // repo bez remote
        let repo = temp_repo();
        let cwd = repo.path().to_string_lossy().to_string();
        let c = Claude::new(Arc::new(|| {}), Duration::from_secs(1)).inline().patient();
        c.set_enforce(EnforceCommit::Block);
        write_tool(&c, "a", &cwd, &repo.path().join("a.txt"), "x");
        git(repo.path(), &["add", "."]);
        git(repo.path(), &["commit", "-q", "-m", "a"]);
        assert_eq!(c.handle_event(&ev("Stop", "a", &cwd, json!({}))), None, "nesplnitelný push se nevyžaduje");
        assert_eq!(sess(&c, "a").unpushed, 0);

        // remote existuje, ale nová větev nemá upstream
        let (repo, _remote) = temp_repo_upstream();
        let cwd = repo.path().to_string_lossy().to_string();
        git(repo.path(), &["checkout", "-q", "-b", "nova"]);
        write_tool(&c, "b", &cwd, &repo.path().join("b.txt"), "x");
        git(repo.path(), &["add", "."]);
        git(repo.path(), &["commit", "-q", "-m", "b"]);
        assert_eq!(c.handle_event(&ev("Stop", "b", &cwd, json!({}))), None);
        assert_eq!(sess(&c, "b").unpushed, 0);
    }

    /// Výsledek nezávisí na rychlosti stroje: test má dlouhé termíny gitu (`patient`), produkce
    /// pořád konstanty.
    #[test]
    fn baseline_retries_then_gives_up() {
        let prod = Claude::new(Arc::new(|| {}), Duration::from_secs(1)).budgets;
        assert_eq!((prod.baseline, prod.hygiene), (BASELINE_BUDGET, HYGIENE_BUDGET));
        let plain = tempfile::tempdir().unwrap();
        let cwd = plain.path().to_string_lossy().to_string();
        let c = Claude::new(Arc::new(|| {}), Duration::from_secs(1)).inline().patient();
        c.handle_event(&ev("UserPromptSubmit", "a", &cwd, json!({})));
        assert!(!c.inner.lock().track["a"].baseline_ok);
        // mezitím se z cwd stane repo → další event baseline dožene
        git(plain.path(), &["init", "-q"]);
        c.handle_event(&ev("UserPromptSubmit", "a", &cwd, json!({})));
        {
            let g = c.inner.lock();
            let t = &g.track["a"];
            assert!(t.baseline_ok && t.repo.is_some() && t.baseline.is_none(), "repo bez commitů");
            assert_eq!(t.baseline_attempts, 2);
        }
        // mimo git: max. 3 pokusy
        let other = tempfile::tempdir().unwrap();
        let cwd = other.path().to_string_lossy().to_string();
        for _ in 0..6 {
            c.handle_event(&ev("PostToolUse", "x", &cwd, json!({"tool_name":"Read"})));
        }
        assert_eq!(c.inner.lock().track["x"].baseline_attempts, BASELINE_ATTEMPTS);
    }

    #[test]
    fn hygiene_outside_git_and_off() {
        let plain = tempfile::tempdir().unwrap();
        let cwd = plain.path().to_string_lossy().to_string();
        let c = Claude::new(Arc::new(|| {}), Duration::from_secs(1)).inline().patient();
        c.set_enforce(EnforceCommit::Block);
        write_tool(&c, "x", &cwd, &plain.path().join("a.txt"), "x");
        assert_eq!(c.handle_event(&ev("Stop", "x", &cwd, json!({}))), None);
        assert!(!sess(&c, "x").unfinished());
        c.handle_event(&ev("SessionEnd", "x", &cwd, json!({})));
        assert!(c.sessions().is_empty());

        // off → žádný git, nic neblokuje
        let repo = temp_repo();
        let cwd = repo.path().to_string_lossy().to_string();
        let c = Claude::new(Arc::new(|| {}), Duration::from_secs(1)).inline().patient();
        c.set_enforce(EnforceCommit::Off);
        write_tool(&c, "y", &cwd, &repo.path().join("a.txt"), "x");
        assert_eq!(c.handle_event(&ev("Stop", "y", &cwd, json!({}))), None);
        assert!(!sess(&c, "y").unfinished());
        assert!(!c.inner.lock().track["y"].baseline_ok, "off nevolá git");
    }

    #[test]
    fn notify_computes_in_background() {
        let repo = temp_repo();
        let cwd = repo.path().to_string_lossy().to_string();
        let c = Claude::new(Arc::new(|| {}), Duration::from_secs(1)).patient();
        c.handle_event(&ev("UserPromptSubmit", "a", &cwd, json!({})));
        // počkej na baseline z pozadí, ať se úprava nepočítá do HEAD před ní (není podstatné, jen determinismus)
        let t0 = std::time::Instant::now();
        while !c.inner.lock().track["a"].baseline_ok && t0.elapsed() < Duration::from_secs(30) {
            std::thread::sleep(Duration::from_millis(10));
        }
        write_tool(&c, "a", &cwd, &repo.path().join("a.txt"), "x");
        assert_eq!(c.handle_event(&ev("Stop", "a", &cwd, json!({}))), None);
        while sess(&c, "a").uncommitted.is_empty() && t0.elapsed() < Duration::from_secs(30) {
            std::thread::sleep(Duration::from_millis(10));
        }
        assert_eq!(sess(&c, "a").uncommitted, vec!["a.txt"]);
    }
}
