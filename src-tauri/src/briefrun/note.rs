//! Tenká vrstva nad poznámkou zadání pro běh: čtení metadat (`project`, `status`, `session`, `cost`, `budget`,
//! `model`) a otevřených otázek; zápisy jdou přes `notes::ops` (zámek, otisk, atomický zápis), otázky přes
//! `notes::sections::questions`. Vlastní parsování ani zápis tu nejsou.

use crate::i18n::tr;
use crate::notes::{self, edit::body_view, get, split};
use chrono::{DateTime, FixedOffset};
use std::path::{Path, PathBuf};

type Pairs<'a> = &'a [(&'a str, Option<&'a str>)];

const MAX_BYTES: u64 = 1 << 20;

/// Co běh potřebuje z poznámky.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Info {
    /// kanonická cesta (klíč běžících zadání)
    pub real: PathBuf,
    pub project: Option<String>,
    pub status: Option<String>,
    pub session: Option<String>,
    pub cost: f64,
    pub budget: Option<f64>,
    pub model: Option<String>,
    pub open_questions: bool,
}

fn read(real: &Path) -> Result<Vec<u8>, String> {
    let too_big = std::fs::metadata(real).map(|m| m.len() > MAX_BYTES).unwrap_or(false);
    if too_big {
        return Err(tr("Poznámka je větší než 1 MB.", "The note is larger than 1 MB.").into());
    }
    std::fs::read(real).map_err(|e| crate::errors::io(tr("Poznámku nejde přečíst", "Can't read the note"), &e))
}

/// Metadata z textu poznámky (čisté). `folder` = jméno složky jako záloha projektu (viz `parse_note`).
pub fn info_from(text: &str, real: &Path, folder: Option<&str>) -> Info {
    let view = notes::parse_note(real, text, folder, 0);
    let (front, body) = split(text);
    let front = front.unwrap_or_default();
    let num = |k: &str| get(&front, k).and_then(|v| v.trim().parse::<f64>().ok()).filter(|x| x.is_finite() && *x >= 0.0);
    let txt = |k: &str| get(&front, k).map(|v| v.trim().to_string()).filter(|v| !v.is_empty());
    Info {
        real: real.to_path_buf(),
        project: view.project,
        status: txt("status"),
        session: txt("session"),
        cost: num("cost").unwrap_or(0.0),
        budget: num("budget"),
        model: txt("model"),
        open_questions: open_questions(body_view(body).1),
    }
}

/// Načte poznámku (cesta ověřená `check_note_path`).
pub fn load(root: &Path, path: &Path) -> Result<Info, String> {
    let real = notes::check_note_path(root, path)?;
    let real_root = root.canonicalize().map_err(|e| crate::errors::io(tr("Složku poznámek nejde otevřít", "Can't open the notes folder"), &e))?;
    let parent = real.parent().unwrap_or(&real_root);
    let folder = (parent != real_root).then(|| parent.file_name().map(|n| n.to_string_lossy().into_owned())).flatten();
    let text = String::from_utf8_lossy(&read(&real)?).into_owned();
    Ok(info_from(&text, &real, folder.as_deref()))
}

/// Nastaví / odebere klíče frontmatteru — jádro poznámek (zámek, otisk, atomický zápis, `updated`).
pub fn meta(root: &Path, path: &Path, pairs: Pairs, now: &DateTime<FixedOffset>) -> Result<(), String> {
    notes::ops::meta(root, path, pairs, now).map(|_| ())
}

/// Připíše do `## Log` řádek s časem.
pub fn log(root: &Path, path: &Path, text: &str, now: &DateTime<FixedOffset>) -> Result<(), String> {
    notes::ops::log(root, path, text, now).map(|_| ())
}

/// V `## Otázky` je otázka bez odpovědi?
pub fn open_questions(body: &str) -> bool {
    notes::sections::questions(body).iter().any(|q| q.answer.as_deref().is_none_or(str::is_empty))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn open_questions_table() {
        let cases: &[(&str, bool)] = &[
            ("bez sekce", false),
            ("## Otázky\n- ❓ Kterou větev?\n", true),
            ("## Otázky\n- ❓ Kterou větev?\n  - ↳ main\n", false),
            ("## Otázky\n- ❓ A?\n  - ↳ ano\n- ❓ B?\n", true),
            ("## Otázky\n- ❓ A?\n- ❓ B?\n  - ↳ ne\n", true),
            ("## Otázky\n- ❓ A?\n  - ↳ \n", true),
            ("## Otázky\n- ❓ A?\n## Log\n- x\n", true),
            ("## Log\n- ❓ jen v logu\n", false),
            ("## Questions\r\n- ❓ A?\r\n  - ↳ ok\r\n", false),
        ];
        for (b, want) in cases {
            assert_eq!(open_questions(b), *want, "{b:?}");
        }
    }

    #[test]
    fn info_from_frontmatter() {
        let text = "---\nproject: app\nstatus: waiting\nsession: 0f8fad5b-d9cb-469f-a165-70867728950e\ncost: 0.42\nbudget: 3\nmodel: opus\n---\n## Otázky\n- ❓ A?\n";
        let i = info_from(text, Path::new(r"C:\n\app\z.md"), None);
        assert_eq!(i.project.as_deref(), Some("app"));
        assert_eq!(i.status.as_deref(), Some("waiting"));
        assert_eq!(i.session.as_deref(), Some("0f8fad5b-d9cb-469f-a165-70867728950e"));
        assert_eq!((i.cost, i.budget, i.model.as_deref()), (0.42, Some(3.0), Some("opus")));
        assert!(i.open_questions);
        let i = info_from("---\ncost: -1\nbudget: x\n---\ntext", Path::new(r"C:\n\app-v2\z.md"), Some("app-v2"));
        assert_eq!((i.cost, i.budget, i.project.as_deref()), (0.0, None, Some("app-v2")));
    }

    #[test]
    fn meta_and_log_write_only_their_place() {
        let dir = std::env::temp_dir().join(format!("ledge-briefnote-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("app")).unwrap();
        let p = dir.join("app").join("z.md");
        std::fs::write(&p, "---\nproject: app\nkind: zadani\n---\nCíl\n- [ ] bod\n").unwrap();
        let now = notes::now_local();
        meta(&dir, &p, &[("status", Some("running")), ("cost", Some("0.5"))], &now).unwrap();
        log(&dir, &p, "Běh zastaven.", &now).unwrap();
        meta(&dir, &p, &[("cost", None)], &now).unwrap();
        let out = std::fs::read_to_string(&p).unwrap();
        assert!(out.starts_with("---\nproject: app\nkind: zadani\nstatus: running\n"), "{out}");
        assert!(!out.contains("cost:"));
        assert!(out.contains("updated: "));
        assert!(out.contains("Cíl\n- [ ] bod\n\n## Log\n\n- "), "{out}");
        assert!(out.trim_end().ends_with("Běh zastaven."));
        let i = load(&dir, &p).unwrap();
        assert_eq!(i.status.as_deref(), Some("running"));
        assert!(meta(&dir, &dir.join("mimo.txt"), &[], &now).is_err(), "jen poznámky .md v kořeni");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
