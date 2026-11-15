//! Zápis termínu bodu z UI (`note_task_snooze`): mění jen token `⏰` na jednom řádku (+ `updated`),
//! s ochranou proti přepsání (otisk souboru jako `edit::toggle_task`). Gramatika je v `taskline.rs`.

use super::edit::{SaveOutcome, body_view, detail, hash, read_bytes, resolve, utf8_text};
use super::*;

/// Nastaví termín bodu na řádku `line` (řádek těla od 0, jako `md::tasks`): `due` = `YYYY-MM-DD` (celodenní),
/// `YYYY-MM-DD HH:MM`, prázdné = termín odebrat. Jiný obsah na disku než `expected_hash` → `Changed`.
pub fn snooze(root: &Path, path: &Path, line: usize, due: &str, expected_hash: &str, now: &DateTime<FixedOffset>) -> Result<SaveOutcome, String> {
    let due = taskline::parse_due_arg(due)?;
    let (real, real_root) = resolve(root, path)?;
    let lock = path_lock(&real);
    let _g = lock.lock();
    let bytes = read_bytes(&real)?;
    if hash(&bytes) != expected_hash {
        return Ok(SaveOutcome::Changed);
    }
    let (front, body) = split(utf8_text(&bytes)?);
    let mut front = front.unwrap_or_default();
    let (lead, rest) = body_view(body);
    let new_rest = taskline::edit_line(rest, line, |t| taskline::set_due(t, due))?;
    set(&mut front, "updated", &iso(now));
    let out = render(&front, &format!("{lead}{new_rest}"));
    write_atomic(&real, out.as_bytes())?;
    Ok(SaveOutcome::Saved { note: Box::new(detail(path, &real, &real_root, out.as_bytes())) })
}

#[cfg(test)]
mod tests {
    use super::super::testkit::tmp_root;
    use super::*;

    fn now() -> DateTime<FixedOffset> {
        parse_time("2026-11-14T08:00:00+01:00").unwrap()
    }

    #[test]
    fn snooze_changes_only_the_token_and_checks_the_hash() {
        let root = tmp_root();
        let p = root.path().join("2026-11-14-080000-a.md");
        let text = "---\ncreated: 2026-11-14T07:00:00+01:00\n---\n# n\r\n- [ ] doktor ⏰ 2026-11-14 09:00 !\r\n- [ ] druhý\r\n";
        std::fs::write(&p, text).unwrap();
        let h = hash(text.as_bytes());
        let SaveOutcome::Saved { .. } = snooze(root.path(), &p, 1, "2026-11-15 10:30", &h, &now()).unwrap() else { panic!("uloženo") };
        let after = std::fs::read_to_string(&p).unwrap();
        assert!(after.contains("# n\r\n- [ ] doktor ⏰ 2026-11-15 10:30 !\r\n- [ ] druhý\r\n"), "{after:?}");
        assert!(after.contains("updated: 2026-11-14T08:00:00+01:00"), "{after:?}");
        // stará čtečka (jiný editor změnil soubor) → nic se nepřepíše
        assert_eq!(snooze(root.path(), &p, 1, "2026-11-20", &h, &now()).unwrap(), SaveOutcome::Changed);
        assert_eq!(std::fs::read_to_string(&p).unwrap(), after);
        // odebrání termínu, špatný řádek, špatný formát
        let h2 = hash(after.as_bytes());
        snooze(root.path(), &p, 1, "", &h2, &now()).unwrap();
        assert!(std::fs::read_to_string(&p).unwrap().contains("- [ ] doktor !\r\n"));
        let h3 = hash(std::fs::read(&p).unwrap().as_slice());
        assert!(snooze(root.path(), &p, 0, "2026-11-20", &h3, &now()).is_err());
        assert!(snooze(root.path(), &p, 1, "zítra", &h3, &now()).is_err());
    }
}
