//! Propojení se složkou — provedení kroků plánu (I/O). Každý zápis je atomický (`write_atomic`), každý
//! přepis se předtím ověří proti hashi z plánu (soubor se mezitím změnil → krok se přeskočí, vyřeší se
//! příště) a všechny cesty jdou přes `safe_path` (nic mimo vybranou složku / kořen poznámek).

use super::folder_sync_plan::{Entry, Op, Side, Step, join_rel, split_rel, stem_of};
use super::folder_sync_scan::{is_link, mtime_us, read_checked, safe_path};
use super::{MAX_IMAGE, lifecycle, path_lock, split, write_atomic};
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Konflikt k zobrazení: co a kde se uložilo.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConflictInfo {
    /// název poznámky
    pub name: String,
    /// kdy (ms od epochy)
    pub at_ms: i64,
    /// absolutní cesta kopie v Ledge (poznámka k otevření)
    pub ledge: Option<String>,
    /// absolutní cesta kopie ve složce
    pub folder: Option<String>,
    /// která verze zůstala na původním místě: `folder` | `ledge`
    pub kept: String,
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Report {
    pub imported: usize,
    pub exported: usize,
    pub updated_ledge: usize,
    pub updated_folder: usize,
    pub deleted_ledge: usize,
    pub deleted_folder: usize,
    pub conflicts: Vec<ConflictInfo>,
    /// kroky přeskočené, protože se soubor mezitím změnil
    pub skipped: usize,
    pub errors: Vec<String>,
    /// zapsalo se do kořene poznámek (přenačíst)
    pub ledge_touched: bool,
}

pub struct Env<'a> {
    /// kanonický kořen poznámek Ledge
    pub root: &'a Path,
    /// kanonická vybraná složka
    pub base: &'a Path,
    pub now_ms: i64,
}

enum Done {
    Yes,
    /// soubor se mezitím změnil / zmizel
    Skip,
}

/// Provede kroky a vrátí nové záznamy (otisky souborů zapsaných teď se doplní ze souborů).
pub fn apply(env: &Env, steps: Vec<Step>, rep: &mut Report) -> Vec<Entry> {
    let mut out = Vec::new();
    for st in steps {
        let keep_only = st.op == Op::Keep;
        match exec(env, &st.op, rep) {
            Ok(Done::Yes) => out.extend(st.keep.into_iter().map(|e| if keep_only { e } else { with_sigs(env, e) })),
            Ok(Done::Skip) => {
                rep.skipped += 1;
                out.extend(st.fallback);
            }
            Err(msg) => {
                if rep.errors.len() < 5 {
                    rep.errors.push(msg);
                }
                out.extend(st.fallback);
            }
        }
    }
    out
}

fn with_sigs(env: &Env, mut e: Entry) -> Entry {
    let stat = |base: &Path, rel: &str| safe_path(base, rel, false).ok().and_then(|p| std::fs::symlink_metadata(p).ok());
    if let Some(m) = stat(env.root, &e.ledge) {
        (e.lm, e.ls) = (mtime_us(&m), m.len());
    }
    if let Some(m) = stat(env.base, &e.ext) {
        (e.em, e.es) = (mtime_us(&m), m.len());
    }
    e
}

fn exec(env: &Env, op: &Op, rep: &mut Report) -> Result<Done, String> {
    match op {
        Op::Keep => Ok(Done::Yes),
        Op::Import { ext, ledge, ext_hash, over } => {
            let Some(bytes) = read_checked(&safe_path(env.base, ext, false)?, ext_hash)? else { return Ok(Done::Skip) };
            if !write_checked(env.root, ledge, &bytes, over.as_deref())? {
                return Ok(Done::Skip);
            }
            rep.ledge_touched = true;
            if over.is_some() { rep.updated_ledge += 1 } else { rep.imported += 1 }
            Ok(Done::Yes)
        }
        Op::Export { ledge, ext, ledge_hash, over } => {
            let Some(bytes) = read_checked(&safe_path(env.root, ledge, false)?, ledge_hash)? else { return Ok(Done::Skip) };
            if !write_checked(env.base, ext, &bytes, over.as_deref())? {
                return Ok(Done::Skip);
            }
            copy_images(env, ledge, ext, &bytes);
            if over.is_some() { rep.updated_folder += 1 } else { rep.exported += 1 }
            Ok(Done::Yes)
        }
        Op::Conflict { ledge, ext, ledge_hash, ext_hash, winner, copy_ledge, copy_ext } => conflict(env, ledge, ext, (ledge_hash, ext_hash), *winner, (copy_ledge, copy_ext), rep),
        Op::DeleteLedge { ledge, hash } => {
            let path = safe_path(env.root, ledge, false)?;
            if read_checked(&path, hash)?.is_none() {
                return Ok(Done::Skip);
            }
            lifecycle::archive(env.root, &path)?;
            rep.deleted_ledge += 1;
            rep.ledge_touched = true;
            Ok(Done::Yes)
        }
        Op::DeleteExt { ext, hash } => {
            let path = safe_path(env.base, ext, false)?;
            if read_checked(&path, hash)?.is_none() {
                return Ok(Done::Skip);
            }
            let (dir, file) = split_rel(ext);
            let mut n = 1;
            let target = loop {
                let name = if n == 1 { file.to_string() } else { format!("{} ({n}).md", stem_of(file)) };
                let t = safe_path(env.base, &join_rel(".trash", &join_rel(dir, &name)), true)?;
                if !t.exists() {
                    break t;
                }
                n += 1;
            };
            std::fs::rename(&path, &target).map_err(|e| crate::errors::io(&trf!("{} nejde přesunout do .trash", "Can't move {} to .trash", path.display()), &e))?;
            rep.deleted_folder += 1;
            Ok(Done::Yes)
        }
    }
}

/// Zapíše `bytes` do `base/rel`. `over` = hash, který musí mít přepisovaný soubor (`None` = soubor nesmí
/// existovat). `false` = podmínka nesedí (mezitím se změnil / vznikl). Zápis pod zámkem cesty — stejným
/// jako ukládá editor Ledge.
fn write_checked(base: &Path, rel: &str, bytes: &[u8], over: Option<&str>) -> Result<bool, String> {
    let dst = safe_path(base, rel, true)?;
    let lock = path_lock(&dst);
    let _g = lock.lock();
    match over {
        Some(h) => {
            if read_checked(&dst, h)?.is_none() {
                return Ok(false);
            }
        }
        None if dst.exists() => return Ok(false),
        None => {}
    }
    write_atomic(&dst, bytes)?;
    Ok(true)
}

/// Obě strany se změnily: nejdřív se uloží kopie druhé verze (nic se neztratí), teprve pak se přepíše
/// soubor poraženého.
fn conflict(env: &Env, ledge: &str, ext: &str, hashes: (&str, &str), winner: Side, copies: (&Option<String>, &Option<String>), rep: &mut Report) -> Result<Done, String> {
    let (Some(l), Some(x)) = (read_checked(&safe_path(env.root, ledge, false)?, hashes.0)?, read_checked(&safe_path(env.base, ext, false)?, hashes.1)?) else {
        return Ok(Done::Skip);
    };
    let (win, lose) = if winner == Side::Ext { (&x, &l) } else { (&l, &x) };
    if let Some(c) = copies.0 && !write_checked(env.root, c, lose, None)? {
        return Ok(Done::Skip);
    }
    if let Some(c) = copies.1 {
        if !write_checked(env.base, c, lose, None)? {
            return Ok(Done::Skip);
        }
        if winner == Side::Ext {
            copy_images(env, ledge, c, lose);
        }
    }
    let (base, rel, expect) = if winner == Side::Ext { (env.root, ledge, hashes.0) } else { (env.base, ext, hashes.1) };
    if !write_checked(base, rel, win, Some(expect))? {
        return Ok(Done::Skip);
    }
    rep.ledge_touched = true;
    let abs = |b: &Path, r: &Option<String>| r.as_deref().and_then(|r| safe_path(b, r, false).ok()).map(|p| super::strip_verbatim(&p).to_string_lossy().into_owned());
    rep.conflicts.push(ConflictInfo {
        name: stem_of(split_rel(ledge).1).to_string(),
        at_ms: env.now_ms,
        ledge: abs(env.root, copies.0),
        folder: abs(env.base, copies.1),
        kept: if winner == Side::Ext { "folder" } else { "ledge" }.into(),
    });
    Ok(Done::Yes)
}

/// Obrázky `![](./x.png)` exportované poznámky se zkopírují vedle ní (jen PNG ve stejné složce, co ještě
/// ve složce nejsou) — ať v Obsidianu nejsou rozbité odkazy. Chyby se ignorují (poznámka je důležitější).
fn copy_images(env: &Env, ledge: &str, ext: &str, note: &[u8]) {
    let text = String::from_utf8_lossy(note);
    let (src_dir, dst_dir) = (split_rel(ledge).0, split_rel(ext).0);
    for name in lifecycle::refs_of(split(&text).1).into_iter().filter(|n| !n.contains('/') && n.to_ascii_lowercase().ends_with(".png")).take(40) {
        let (Ok(src), Ok(dst)) = (safe_path(env.root, &join_rel(src_dir, &name), false), safe_path(env.base, &join_rel(dst_dir, &name), true)) else { continue };
        let ok = std::fs::symlink_metadata(&src).is_ok_and(|m| m.is_file() && !is_link(&src, &m) && m.len() <= MAX_IMAGE as u64);
        if ok && !dst.exists() && let Ok(bytes) = std::fs::read(&src) {
            let _ = write_atomic(&dst, &bytes);
        }
    }
}

/// Abs. cesta pro zobrazení (bez `\\?\`).
pub fn display(p: &Path) -> String {
    super::strip_verbatim(p).to_string_lossy().into_owned()
}
