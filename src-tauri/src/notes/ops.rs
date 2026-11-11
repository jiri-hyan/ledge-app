//! Zápisy zadání do poznámky (I/O nad čistými funkcemi z `brief` / `sections` / `frontmatter`):
//! stav (`status:`), další metadata (`cost`, `session`…), odpověď na otázku, otázky, záznam do logu a
//! stav bodu. Každý zápis: cesta ověřená přes `check_note_path`, zámek cesty, čtení → změna → kontrola,
//! že soubor mezitím nezměnil sync (otisk) → atomický zápis. Mění se jen dotčené místo, `updated` se
//! obnoví. Vrací čerstvý `NoteDetail` (UI / MCP z něj berou nový otisk).

use super::edit::{NoteDetail, body_view, detail, hash, read_bytes, resolve};
use super::*;

type Pairs<'a> = &'a [(&'a str, Option<&'a str>)];

/// Klíče frontmatteru zadání, které smí `meta` měnit.
const META_KEYS: [&str; 5] = ["status", "session", "cost", "budget", "model"];

/// Čtení → `f(front, tělo)` (vrací nové tělo, nebo `None` = tělo beze změny) → zápis.
fn modify(
    root: &Path,
    path: &Path,
    now: &DateTime<FixedOffset>,
    f: impl FnOnce(&mut Vec<String>, &str) -> Result<Option<String>, String>,
) -> Result<NoteDetail, String> {
    let (real, real_root) = resolve(root, path)?;
    let lock = path_lock(&real);
    let _g = lock.lock();
    let bytes = read_bytes(&real)?;
    let text = String::from_utf8_lossy(&bytes);
    let (front, body) = split(&text);
    let mut front = front.unwrap_or_default();
    let (lead, rest) = body_view(body);
    let new_rest = f(&mut front, rest)?;
    set(&mut front, "updated", &iso(now));
    let out = render(&front, &format!("{lead}{}", new_rest.as_deref().unwrap_or(rest)));
    if hash(&read_bytes(&real)?) != hash(&bytes) {
        return Err(tr("Poznámka se mezitím změnila na disku — načti ji znovu a zkus to znovu.", "The note changed on disk meanwhile — reload it and retry.").into());
    }
    write_atomic(&real, out.as_bytes())?;
    Ok(detail(path, &real, &real_root, out.as_bytes()))
}

/// Hodnota `status:` z textu UI / MCP; prázdná = odebrat klíč.
pub fn parse_status(s: &str) -> Result<Option<&'static str>, String> {
    let t = s.trim().to_ascii_lowercase();
    if t.is_empty() {
        return Ok(None);
    }
    STATUSES.iter().find(|x| **x == t).map(|x| Some(*x)).ok_or_else(|| trf!("Neznámý stav „{s}“ ({}).", "Unknown status “{s}” ({}).", STATUSES.join(", ")))
}

/// Nastaví `status:` (prázdný = odebere).
pub fn set_status(root: &Path, path: &Path, status: &str, now: &DateTime<FixedOffset>) -> Result<NoteDetail, String> {
    let st = parse_status(status)?;
    meta(root, path, &[("status", st)], now)
}

/// Nastaví / odebere `status`, `session`, `cost`, `budget`, `model` (jiný klíč = chyba). Hodnoty musí být
/// už platné (stav, model ze seznamu; `cost`/`budget` číslo ≥ 0; `session` id) — jinak chyba.
pub fn meta(root: &Path, path: &Path, pairs: Pairs, now: &DateTime<FixedOffset>) -> Result<NoteDetail, String> {
    for (k, v) in pairs {
        if !META_KEYS.contains(k) {
            return Err(trf!("Klíč „{k}“ tu nejde měnit.", "The key “{k}” can't be changed here."));
        }
        let ok = match (*k, v) {
            (_, None) => true,
            ("status", Some(s)) => STATUSES.contains(s),
            ("model", Some(m)) => MODELS.contains(m),
            ("cost" | "budget", Some(n)) => n.parse::<f64>().is_ok_and(|x| x.is_finite() && x >= 0.0),
            ("session", Some(s)) => !s.is_empty() && s.len() <= 64 && s.chars().all(|c| c.is_ascii_alphanumeric() || c == '-'),
            _ => false,
        };
        if !ok {
            return Err(trf!("Neplatná hodnota klíče „{k}“.", "Invalid value for “{k}”."));
        }
    }
    modify(root, path, now, |front, _| {
        set_meta(front, pairs);
        Ok(None)
    })
}

/// Odpověď šéfa k otázce `index` (prázdná odebere).
pub fn answer_question(root: &Path, path: &Path, index: usize, text: &str, now: &DateTime<FixedOffset>) -> Result<NoteDetail, String> {
    modify(root, path, now, |_, body| answer(body, index, text).map(Some))
}

/// Přidá otázky do `## Otázky`.
pub fn ask(root: &Path, path: &Path, qs: &[String], now: &DateTime<FixedOffset>) -> Result<NoteDetail, String> {
    modify(root, path, now, |_, body| add_questions(body, qs).map(Some))
}

/// Připíše do `## Log` řádek s aktuálním časem.
pub fn log(root: &Path, path: &Path, text: &str, now: &DateTime<FixedOffset>) -> Result<NoteDetail, String> {
    modify(root, path, now, |_, body| log_append(body, &iso(now), text).map(Some))
}

/// Nastaví stav bodu (viz `brief::set_item`; `line` jen nápověda, bod se hledá podle `text`).
pub fn task_set(
    root: &Path,
    path: &Path,
    line: Option<usize>,
    text: Option<&str>,
    state: md::TaskState,
    evidence: Option<&str>,
    now: &DateTime<FixedOffset>,
) -> Result<NoteDetail, String> {
    modify(root, path, now, |_, body| set_item(body, line, text, state, evidence).map(Some))
}

#[cfg(test)]
mod tests {
    use super::*;
    use md::TaskState;

    fn at(s: &str) -> DateTime<FixedOffset> {
        DateTime::parse_from_rfc3339(s).unwrap()
    }

    /// Poznámka v dočasném kořeni; vrací (kořen, cesta, text).
    fn note(text: &str) -> (tempfile::TempDir, PathBuf) {
        let root = tempfile::tempdir().unwrap();
        let p = root.path().join("2026-11-11-100000-zadani.md");
        std::fs::write(&p, text).unwrap();
        (root, p)
    }

    const NOTE: &str = "---\ncreated: 2026-11-11T10:00:00+01:00\nx-vlastni: ahoj\ndone: false\n---\n\n# Cíl\n- [ ] první\n  ![snímek](./a.png)\n- [ ] druhý\n";

    #[test]
    fn status_and_meta_roundtrip_keep_unknown_keys_and_body() {
        let (root, p) = note(NOTE);
        let now = at("2026-11-11T12:00:00+01:00");
        let d = set_status(root.path(), &p, "Running", &now).unwrap();
        assert_eq!(d.view.status.as_deref(), Some("running"));
        meta(root.path(), &p, &[("cost", Some("1.25")), ("budget", Some("5")), ("model", Some("opus")), ("session", Some("0b9e-1"))], &now).unwrap();
        let d = super::edit::read(root.path(), &p).unwrap();
        assert_eq!((d.view.cost, d.view.budget, d.view.model.as_deref(), d.view.session.as_deref()), (Some(1.25), Some(5.0), Some("opus"), Some("0b9e-1")));
        // tělo a neznámé klíče beze změny, jen přibyly klíče a `updated`
        let text = std::fs::read_to_string(&p).unwrap();
        assert!(text.contains("x-vlastni: ahoj\ndone: false\nstatus: running\n"));
        assert!(text.ends_with("\n\n# Cíl\n- [ ] první\n  ![snímek](./a.png)\n- [ ] druhý\n"));
        // odebrání
        set_status(root.path(), &p, "", &now).unwrap();
        meta(root.path(), &p, &[("cost", None), ("budget", None), ("model", None), ("session", None)], &now).unwrap();
        let text = std::fs::read_to_string(&p).unwrap();
        assert_eq!(text.replace("updated: 2026-11-11T12:00:00+01:00\n", ""), NOTE);
    }

    #[test]
    fn meta_refuses_bad_keys_and_values() {
        let (root, p) = note(NOTE);
        let now = at("2026-11-11T12:00:00+01:00");
        for bad in [("done", "true"), ("created", "x"), ("status", "hotovo"), ("model", "gpt"), ("cost", "-1"), ("cost", "nan"), ("budget", "abc"), ("session", "a b"), ("session", "")] {
            assert!(meta(root.path(), &p, &[(bad.0, Some(bad.1))], &now).is_err(), "{bad:?}");
        }
        assert!(set_status(root.path(), &p, "hotovo", &now).is_err());
        assert_eq!(std::fs::read_to_string(&p).unwrap(), NOTE, "špatný vstup nic nezapsal");
    }

    #[test]
    fn question_answer_log_and_task_flow() {
        let (root, p) = note(NOTE);
        let now = at("2026-11-11T12:00:00+01:00");
        let d = ask(root.path(), &p, &["Který branch?".into(), "Smím měnit schéma?".into()], &now).unwrap();
        assert_eq!(d.questions.len(), 2);
        let d = answer_question(root.path(), &p, 1, "ne", &now).unwrap();
        assert_eq!((d.questions[0].answer.as_deref(), d.questions[1].answer.as_deref()), (None, Some("ne")));
        let d = log(root.path(), &p, "začal jsem", &now).unwrap();
        assert!(d.body.ends_with("- 2026-11-11T12:00:00+01:00 začal jsem"), "{}", d.body);
        let d = task_set(root.path(), &p, Some(0), Some("druhý"), TaskState::Proposed, Some("commit abc"), &now).unwrap();
        let it = d.items.iter().find(|i| i.text == "druhý").unwrap();
        assert_eq!((it.state, it.evidence.as_deref()), (TaskState::Proposed, Some("commit abc")));
        assert_eq!(d.items[0].images[0].caption, "snímek");
        assert_eq!((d.view.tasks_done, d.view.tasks_total), (0, 2));
        // chyba nic nezapíše
        let before = std::fs::read(&p).unwrap();
        assert!(task_set(root.path(), &p, None, Some("nic"), TaskState::Done, None, &now).is_err());
        assert!(answer_question(root.path(), &p, 9, "x", &now).is_err());
        assert_eq!(std::fs::read(&p).unwrap(), before);
    }

    #[test]
    fn refuses_paths_outside_root() {
        let (root, _) = note(NOTE);
        let other = tempfile::tempdir().unwrap();
        let outside = other.path().join("x.md");
        std::fs::write(&outside, NOTE).unwrap();
        assert!(set_status(root.path(), &outside, "ready", &at("2026-11-11T12:00:00+01:00")).is_err());
        assert_eq!(std::fs::read_to_string(&outside).unwrap(), NOTE);
    }

    #[test]
    fn save_places_anchored_images_after_existing_attachments_and_keeps_old_shape() {
        use super::super::testkit::{b64, png};
        let (root, p) = note(NOTE);
        let now = at("2026-11-11T12:00:00+01:00");
        let d = super::edit::read(root.path(), &p).unwrap();
        let img = b64(&png(3));
        let body = d.body.clone();
        let req = |images: serde_json::Value| -> super::edit::SaveNote {
            serde_json::from_value(serde_json::json!({ "path": p.to_string_lossy(), "expectedHash": d.hash, "body": body, "images": images })).unwrap()
        };
        // starý tvar (holý řetězec) = na konec
        let out = super::edit::save(root.path(), &req(serde_json::json!([img.clone()])), &now).unwrap();
        let super::edit::SaveOutcome::Saved { note: n1 } = out else { panic!("změna na disku") };
        assert!(n1.body.ends_with("- [ ] druhý\n\n![](./2026-11-11-100000-zadani-1.png)"), "{}", n1.body);
        // nový tvar: pod bod `první` (řádek 1), za jeho stávající snímek; bod `druhý` (řádek 3) bez příloh
        let (_r2, p2) = note(NOTE);
        let d2 = super::edit::read(_r2.path(), &p2).unwrap();
        let req2: super::edit::SaveNote = serde_json::from_value(serde_json::json!({
            "path": p2.to_string_lossy(), "expectedHash": d2.hash, "body": d2.body,
            "images": [{ "png": img.clone(), "caption": "Druhý snímek", "anchorLine": 1 }, { "png": img, "caption": "K druhému", "anchorLine": 3 }]
        }))
        .unwrap();
        let super::edit::SaveOutcome::Saved { note } = super::edit::save(_r2.path(), &req2, &now).unwrap() else { panic!("změna na disku") };
        assert!(
            note.body.ends_with("- [ ] první\n  ![snímek](./a.png)\n  ![Druhý snímek](./2026-11-11-100000-zadani-1.png)\n- [ ] druhý\n  ![K druhému](./2026-11-11-100000-zadani-2.png)"),
            "{}",
            note.body
        );
        assert_eq!(note.items[0].images.iter().map(|i| i.caption.as_str()).collect::<Vec<_>>(), vec!["snímek", "Druhý snímek"]);
        assert_eq!(note.items[1].images[0].caption, "K druhému");
        assert!(note.loose_images.is_empty());
    }
}
