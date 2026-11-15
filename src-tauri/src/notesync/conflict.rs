//! Konflikt téže poznámky na dvou PC: sync se kvůli ní nezastaví. Po neúspěšném rebase se sloučí `merge`
//! a každý konfliktní soubor se rozhodne zvlášť ([`decide`]): na původním místě zůstane verze z remote
//! (tu mají všichni ostatní), lokální verze se uloží vedle jako `název (konflikt Ledge <datum>).md`
//! (vzor „Propojit se složkou“, `folder_sync_plan::conflict_rel`). Smazání vs. změna = vyhrává změna.
//! Nic se neztratí; cokoli neočekávaného = `merge --abort` a dosavadní `Outcome::Konflikt`.
//! Závisí na `git.rs`.

use super::git::{git, nul_list};
use crate::notes::folder_sync_plan::{conflict_rel, split_rel};
use std::collections::{BTreeMap, HashSet};
use std::path::Path;

/// Jak se vyřeší jeden konfliktní soubor.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Resolution {
    /// obě strany změnily / přidaly: remote zůstane, lokální verze do kopie
    RemoteWinsCopyLocal,
    /// lokálně smazáno, remote změnil: zůstane remote (lokální verze neexistuje)
    TakeRemote,
    /// lokálně změněno, remote smazal: zůstane lokální verze
    KeepLocal,
}

/// Z fází v indexu (2 = lokální, 3 = remote) → řešení. `None` = nic k řešení (neočekávané).
pub(super) fn decide(local: bool, remote: bool) -> Option<Resolution> {
    match (local, remote) {
        (true, true) => Some(Resolution::RemoteWinsCopyLocal),
        (false, true) => Some(Resolution::TakeRemote),
        (true, false) => Some(Resolution::KeepLocal),
        (false, false) => None,
    }
}

/// Výstup `git ls-files -u -z` (`mode sha fáze\tcesta`) → cesta → (má fázi 2, má fázi 3).
pub(super) fn parse_unmerged(out: &str) -> BTreeMap<String, (bool, bool)> {
    let mut m: BTreeMap<String, (bool, bool)> = BTreeMap::new();
    for rec in nul_list(out) {
        let Some((meta, path)) = rec.split_once('\t') else { continue };
        let e = m.entry(path.to_string()).or_default();
        match meta.split_whitespace().nth(2) {
            Some("2") => e.0 = true,
            Some("3") => e.1 = true,
            _ => {}
        }
    }
    m
}

/// Jméno kopie vedle `rel` (přípona zůstane; `conflict_rel` samo vrací `.md`).
pub(super) fn copy_rel(taken: &mut HashSet<String>, rel: &str, today: &str) -> String {
    let r = conflict_rel(taken, rel, today);
    match split_rel(rel).1.rsplit_once('.') {
        Some((_, ext)) if !ext.eq_ignore_ascii_case("md") && r.ends_with(".md") => format!("{}.{ext}", &r[..r.len() - 3]),
        _ => r,
    }
}

/// Po neúspěšném rebase (už přerušeném): `merge` + rozhodnutí po souborech + merge commit.
/// `Ok(kopie)` = vytvořené kopie (`původní → kopie`); `Err` = nic se nezměnilo (merge přerušen).
pub(super) fn merge_keep_both(root: &Path, remote: &str, upstream: &str, msg: &str, today: &str) -> Result<Vec<String>, String> {
    let merged = git(root, remote, &["merge", "-q", "--no-edit", "--no-verify", upstream], false);
    let abort = |why: String| {
        let _ = git(root, remote, &["merge", "--abort"], false);
        Err(why)
    };
    let unmerged = match git(root, remote, &["ls-files", "-u", "-z"], false) {
        Ok(o) => parse_unmerged(&o),
        Err(_) => return abort("ls-files".into()),
    };
    if unmerged.is_empty() {
        return if merged.is_ok() { Ok(Vec::new()) } else { abort("merge bez konfliktních souborů".into()) };
    }
    let mut taken: HashSet<String> = git(root, remote, &["ls-files", "-z"], false).map(|s| nul_list(&s)).unwrap_or_default().into_iter().map(|p| p.to_lowercase()).collect();
    let mut copies = Vec::new();
    for (path, (local, remote_has)) in &unmerged {
        let step = |args: &[&str]| git(root, remote, args, false).map(|_| ()).map_err(|_| format!("git {}", args.join(" ")));
        let res: Result<(), String> = (|| {
            match decide(*local, *remote_has) {
                Some(Resolution::RemoteWinsCopyLocal) => {
                    let copy = copy_rel(&mut taken, path, today);
                    step(&["checkout", "--ours", "--", path])?;
                    let (from, to) = (root.join(path), root.join(&copy));
                    std::fs::copy(&from, &to).map_err(|e| format!("kopie {copy}: {e}"))?;
                    step(&["checkout", "--theirs", "--", path])?;
                    step(&["add", "--", path, &copy])?;
                    copies.push(format!("{path} → {copy}"));
                }
                Some(Resolution::TakeRemote) => {
                    step(&["checkout", "--theirs", "--", path])?;
                    step(&["add", "--", path])?;
                }
                Some(Resolution::KeepLocal) => step(&["add", "--", path])?,
                None => return Err(format!("{path}: neznámý konflikt")),
            }
            Ok(())
        })();
        if let Err(e) = res {
            // vzniklé kopie jsou neindexované soubory — merge --abort je nechá; smaž je, ať se příště nezdvojí
            for c in copies.iter().filter_map(|c| c.split_once(" → ").map(|x| x.1)) {
                let _ = std::fs::remove_file(root.join(c));
            }
            return abort(e);
        }
    }
    if git(root, remote, &["commit", "-q", "--no-verify", "-m", msg], false).is_err() {
        for c in copies.iter().filter_map(|c| c.split_once(" → ").map(|x| x.1)) {
            let _ = std::fs::remove_file(root.join(c));
        }
        return abort("git commit".into());
    }
    Ok(copies)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decide_table() {
        assert_eq!(decide(true, true), Some(Resolution::RemoteWinsCopyLocal));
        assert_eq!(decide(false, true), Some(Resolution::TakeRemote));
        assert_eq!(decide(true, false), Some(Resolution::KeepLocal));
        assert_eq!(decide(false, false), None);
    }

    #[test]
    fn parse_unmerged_stages() {
        let out = "100644 aaa 1\ta b.md\x00100644 bbb 2\ta b.md\x00100644 ccc 3\ta b.md\x00100644 ddd 2\tx.md\x00100644 eee 3\ty/z.png\x00";
        let m = parse_unmerged(out);
        assert_eq!(m["a b.md"], (true, true));
        assert_eq!(m["x.md"], (true, false));
        assert_eq!(m["y/z.png"], (false, true));
    }

    #[test]
    fn copy_rel_keeps_extension_and_is_unique() {
        let mut taken: HashSet<String> = HashSet::new();
        assert_eq!(copy_rel(&mut taken, "a/n.md", "2026-11-13"), "a/n (konflikt Ledge 2026-11-13).md");
        assert_eq!(copy_rel(&mut taken, "a/n.md", "2026-11-13"), "a/n (konflikt Ledge 2026-11-13) (2).md");
        assert_eq!(copy_rel(&mut taken, "a/n-1.png", "2026-11-13"), "a/n-1 (konflikt Ledge 2026-11-13).png");
    }
}
