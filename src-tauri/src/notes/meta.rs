//! Barva a štítky poznámky (Lístky 2.0): klíče frontmatteru `color:` a `tags:` — čitelné i mimo Ledge
//! (Obsidian, GitHub), synchronizují se s poznámkou. Štítky poznámky = `tags:` ze frontmatteru (inline
//! `[a, b]`, blokový seznam `- a`, nebo `a, b`) ∪ `#štítek` v těle (`links::inline_tags`). Ledge píše jen
//! inline tvar `tags: [a, b]`; blokový seznam při zápisu nahradí celý (žádné osiřelé `  - x`).
//! Čisté funkce (`color_of`, `tags_of`, `with_meta`) + jeden zápis s otiskem (`set_meta`) — ostatní klíče,
//! pořadí i tělo zůstávají beze změny.

use super::edit::{SaveOutcome, detail, hash, read_bytes, resolve};
use super::{iso, path_lock, render, set, split, unset, write_atomic};
use chrono::{DateTime, FixedOffset};
use serde::Deserialize;
use std::path::Path;

/// Paleta barev lístku (id do frontmatteru). Prázdné / chybějící = výchozí vzhled.
pub const COLORS: [&str; 8] = ["yellow", "orange", "red", "pink", "purple", "blue", "teal", "green"];
/// Nejvýš tolik štítků na poznámku (víc nedává smysl a hlídá to délku frontmatteru).
pub const MAX_TAGS: usize = 12;
const MAX_TAG_CHARS: usize = 40;

/// Barva: id z `COLORS` nebo `#rrggbb` (ručně psaná poznámka).
pub fn valid_color(c: &str) -> bool {
    COLORS.contains(&c) || (c.len() == 7 && c.starts_with('#') && c[1..].chars().all(|h| h.is_ascii_hexdigit()))
}

/// Znak, který smí být ve štítku (písmena vč. diakritiky, číslice, `-`, `_`, `/`).
pub fn tag_char(c: char) -> bool {
    c.is_alphanumeric() || matches!(c, '-' | '_' | '/')
}

/// Štítek v kanonické podobě (bez `#`, malými písmeny), `None` = neplatný (prázdný, jen číslice, cizí znaky, dlouhý).
pub fn norm_tag(raw: &str) -> Option<String> {
    let t = raw.trim().trim_start_matches('#').trim().to_lowercase();
    let n = t.chars().count();
    (n > 0 && n <= MAX_TAG_CHARS && t.chars().all(tag_char) && !t.chars().all(|c| c.is_ascii_digit())).then_some(t)
}

fn push_tag(out: &mut Vec<String>, raw: &str) {
    if let Some(t) = norm_tag(raw.trim().trim_matches(['"', '\'']))
        && !out.contains(&t)
        && out.len() < MAX_TAGS * 2
    {
        out.push(t);
    }
}

fn is_key(l: &str, key: &str) -> bool {
    !l.starts_with([' ', '\t']) && l.split_once(':').is_some_and(|(k, _)| k.trim() == key)
}

/// Štítky ze frontmatteru: `tags: [a, b]`, `tags: a, b`, nebo `tags:` + odsazené `- a`.
pub fn front_tags(front: &[String]) -> Vec<String> {
    let mut out = Vec::new();
    let Some(i) = front.iter().position(|l| is_key(l, "tags")) else { return out };
    let v = front[i].split_once(':').map_or("", |(_, v)| v).trim();
    if v.is_empty() {
        for l in front[i + 1..].iter().take_while(|l| l.starts_with([' ', '\t'])) {
            if let Some(item) = l.trim().strip_prefix('-') {
                push_tag(&mut out, item);
            }
        }
    } else {
        for item in v.trim_start_matches('[').trim_end_matches(']').split(',') {
            push_tag(&mut out, item);
        }
    }
    out
}

/// Barva ze frontmatteru (neplatná = žádná).
pub fn color_of(front: &[String]) -> Option<String> {
    super::get(front, "color").map(|c| c.trim().to_lowercase()).filter(|c| valid_color(c))
}

/// Štítky poznámky: frontmatter první, pak `#štítek` z těla; bez duplicit.
pub fn tags_of(front: &[String], body: &str) -> Vec<String> {
    let mut out = front_tags(front);
    for t in super::links::inline_tags(body) {
        if !out.contains(&t) {
            out.push(t);
        }
    }
    out
}

/// Odebere klíč i s jeho odsazeným pokračováním (blokový seznam). Vrací index, kde klíč byl.
fn remove_block(front: &mut Vec<String>, key: &str) -> Option<usize> {
    let i = front.iter().position(|l| is_key(l, key))?;
    let mut end = i + 1;
    while end < front.len() && front[end].starts_with([' ', '\t']) {
        end += 1;
    }
    front.drain(i..end);
    Some(i)
}

/// Změna barvy / štítků (chybějící pole = beze změny; prázdná barva / prázdný seznam = klíč pryč).
#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct MetaPatch {
    pub color: Option<String>,
    pub tags: Option<Vec<String>>,
}

/// Řádky frontmatteru po změně barvy / štítků. Neplatná barva nebo štítek = chyba (nic se nezapíše).
pub fn apply_patch(front: &mut Vec<String>, p: &MetaPatch) -> Result<(), String> {
    if let Some(c) = &p.color {
        let c = c.trim().to_lowercase();
        if c.is_empty() {
            unset(front, "color");
        } else if valid_color(&c) {
            set(front, "color", &if c.starts_with('#') { format!("\"{c}\"") } else { c });
        } else {
            return Err(trf!("Neznámá barva „{c}“.", "Unknown color “{c}”."));
        }
    }
    if let Some(tags) = &p.tags {
        let mut clean: Vec<String> = Vec::new();
        for t in tags {
            let n = norm_tag(t).ok_or_else(|| trf!("Štítek „{t}“ smí mít jen písmena, číslice, - _ / (nejvýš 40 znaků).", "Tag “{t}” may only contain letters, digits, - _ / (at most 40 characters)."))?;
            if !clean.contains(&n) {
                clean.push(n);
            }
        }
        if clean.len() > MAX_TAGS {
            return Err(trf!("Nejvýš {MAX_TAGS} štítků na poznámku.", "At most {MAX_TAGS} tags per note."));
        }
        let at = remove_block(front, "tags");
        if !clean.is_empty() {
            let line = format!("tags: [{}]", clean.join(", "));
            match at {
                Some(i) => front.insert(i, line),
                None => front.push(line),
            }
        }
    }
    Ok(())
}

/// Celý text poznámky po změně barvy / štítků (`updated` se nastaví jen při skutečné změně).
pub fn with_meta(text: &str, p: &MetaPatch, now: &DateTime<FixedOffset>) -> Result<String, String> {
    let (front, body) = split(text);
    let had = front.is_some();
    let mut front = front.unwrap_or_default();
    let before = front.clone();
    apply_patch(&mut front, p)?;
    if front == before {
        return Ok(text.to_string());
    }
    if had || !front.is_empty() {
        set(&mut front, "updated", &iso(now));
    }
    Ok(render(&front, body))
}

/// Zápis barvy / štítků s otiskem (jako `edit::save`): jiný obsah na disku = `Changed`, nic se nezapíše.
pub fn set_meta(root: &Path, path: &Path, expected_hash: &str, p: &MetaPatch, now: &DateTime<FixedOffset>) -> Result<SaveOutcome, String> {
    let (real, real_root) = resolve(root, path)?;
    let lock = path_lock(&real);
    let _g = lock.lock();
    let bytes = read_bytes(&real)?;
    if hash(&bytes) != expected_hash {
        return Ok(SaveOutcome::Changed);
    }
    let text = String::from_utf8_lossy(&bytes);
    let out = with_meta(&text, p, now)?;
    if out != text {
        write_atomic(&real, out.as_bytes())?;
    }
    Ok(SaveOutcome::Saved { note: Box::new(detail(path, &real, &real_root, out.as_bytes())) })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn now() -> DateTime<FixedOffset> {
        DateTime::parse_from_rfc3339("2026-10-23T10:00:00+02:00").unwrap()
    }

    fn lines(s: &[&str]) -> Vec<String> {
        s.iter().map(|l| l.to_string()).collect()
    }

    #[test]
    fn tags_from_front_table() {
        let cases: &[(&[&str], &[&str])] = &[
            (&["tags: [prace, Nákup]"], &["prace", "nákup"]),
            (&["tags: prace, #qa"], &["prace", "qa"]),
            (&["tags:", "  - a", "  - 'b c'", "- mimo", "x: 1"], &["a"]),
            (&["tags: [\"x\", 'y', x]"], &["x", "y"]),
            (&["tags: [123, ok/sub, a_b-c]"], &["ok/sub", "a_b-c"]),
            (&["tags: []"], &[]),
            (&["project: a"], &[]),
            (&["  tags: [vnoreny]"], &[]),
        ];
        for (front, want) in cases {
            assert_eq!(front_tags(&lines(front)), want.iter().map(|s| s.to_string()).collect::<Vec<_>>(), "{front:?}");
        }
    }

    #[test]
    fn color_validation() {
        for (c, ok) in [("yellow", true), ("green", true), ("#a1B2c3", true), ("#abc", false), ("cerna", false), ("", false), ("#gggggg", false)] {
            assert_eq!(valid_color(c), ok, "{c}");
        }
        assert_eq!(color_of(&lines(&["color: Teal"])).as_deref(), Some("teal"));
        assert_eq!(color_of(&lines(&["color: \"#aabbcc\""])).as_deref(), Some("#aabbcc"));
        assert_eq!(color_of(&lines(&["color: neon"])), None);
    }

    #[test]
    fn norm_tag_table() {
        for (raw, want) in [("#Práce", Some("práce")), ("  qa ", Some("qa")), ("123", None), ("2026q4", Some("2026q4")), ("a b", None), ("", None), ("#", None), ("x<y", None)] {
            assert_eq!(norm_tag(raw).as_deref(), want, "{raw}");
        }
        assert!(norm_tag(&"a".repeat(41)).is_none());
    }

    #[test]
    fn with_meta_keeps_unknown_keys_order_and_body() {
        let src = "---\nproject: a\nautor: claude\ntags:\n  - stary\n  - b\ndone: false\nzz: 1\n---\n\nTělo #inline\n";
        let p = MetaPatch { color: Some("blue".into()), tags: Some(vec!["Nový".into(), "b".into(), "#b".into()]) };
        let out = with_meta(src, &p, &now()).unwrap();
        assert_eq!(out, "---\nproject: a\nautor: claude\ntags: [nový, b]\ndone: false\nzz: 1\ncolor: blue\nupdated: 2026-10-23T10:00:00+02:00\n---\n\nTělo #inline\n");
        let (f, body) = split(&out);
        let f = f.unwrap();
        assert_eq!(color_of(&f).as_deref(), Some("blue"));
        assert_eq!(tags_of(&f, body), ["nový", "b", "inline"]);
        // odebrání: prázdná barva i seznam = klíč pryč, zbytek beze změny
        let back = with_meta(&out, &MetaPatch { color: Some(String::new()), tags: Some(vec![]) }, &now()).unwrap();
        assert_eq!(back, "---\nproject: a\nautor: claude\ndone: false\nzz: 1\nupdated: 2026-10-23T10:00:00+02:00\n---\n\nTělo #inline\n");
        // beze změny = text bajtově stejný (ani `updated`)
        assert_eq!(with_meta(src, &MetaPatch::default(), &now()).unwrap(), src);
        // bez frontmatteru → vznikne
        assert_eq!(with_meta("text\n", &MetaPatch { color: Some("red".into()), tags: None }, &now()).unwrap(), "---\ncolor: red\nupdated: 2026-10-23T10:00:00+02:00\n---\ntext\n");
    }

    #[test]
    fn with_meta_rejects_bad_values() {
        assert!(with_meta("x", &MetaPatch { color: Some("neon".into()), tags: None }, &now()).is_err());
        assert!(with_meta("x", &MetaPatch { color: None, tags: Some(vec!["a b".into()]) }, &now()).is_err());
        let many: Vec<String> = (0..13).map(|i| format!("t{i}")).collect();
        assert!(with_meta("x", &MetaPatch { color: None, tags: Some(many) }, &now()).is_err());
        // hex barva jde do uvozovek (YAML komentář `#`)
        let out = with_meta("x", &MetaPatch { color: Some("#AABBCC".into()), tags: None }, &now()).unwrap();
        assert!(out.contains("color: \"#aabbcc\"\n"), "{out}");
    }

    #[test]
    fn set_meta_checks_hash() {
        let root = super::super::testkit::tmp_root();
        let dir = root.path().join("_obecne");
        std::fs::create_dir_all(&dir).unwrap();
        let p = dir.join("a.md");
        std::fs::write(&p, "---\nkind: napad\n---\n\nAhoj\n").unwrap();
        let h = hash(&std::fs::read(&p).unwrap());
        let patch = MetaPatch { color: Some("green".into()), tags: Some(vec!["qa".into()]) };
        assert_eq!(set_meta(root.path(), &p, "jiny", &patch, &now()).unwrap(), SaveOutcome::Changed);
        match set_meta(root.path(), &p, &h, &patch, &now()).unwrap() {
            SaveOutcome::Saved { note } => {
                assert_eq!(note.view.color.as_deref(), Some("green"));
                assert_eq!(note.view.tags, ["qa"]);
                assert_eq!(note.body, "Ahoj");
            }
            SaveOutcome::Changed => panic!("mělo se uložit"),
        }
        let disk = std::fs::read_to_string(&p).unwrap();
        assert!(disk.starts_with("---\nkind: napad\ncolor: green\ntags: [qa]\nupdated: "), "{disk}");
    }
}
