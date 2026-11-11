//! Zadání v poznámce — body se stavem (`- [ ]` otevřený, `- [~] text — důkaz: …` navržený, `- [x]`
//! hotový), snímky navázané na bod (odsazený řádek `  ![popisek](./a.png)` pod bodem) a odkazy
//! `@cesta` / `@cesta:řádek` v textu bodu. Čisté funkce nad tělem (čísla řádků od 0 jako `md::tasks`,
//! tj. v `edit::body_view(...).1`); zápis mění jen jeden řádek, zbytek zůstává bajtově stejný.
//! Odkazy se dorovnají proti složce projektu až v I/O vrstvě (`resolve_refs`).

use super::md::{TaskState, bracket_link, is_fence, list_item, relative_image, task_state};
use super::md_list::indent_width;
use crate::i18n::tr;
use serde::Serialize;
use std::collections::HashSet;
use std::path::{Path, PathBuf};

/// Snímek: `src` jak stojí v Markdownu, `path` absolutní, `caption` = alt text.
#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ItemImage {
    pub src: String,
    pub path: String,
    pub caption: String,
}

/// Odkaz na soubor projektu. `path` (absolutní) a `exists` doplní `resolve_refs`; `path` je `null`,
/// když odkaz mířil mimo složku projektu (`..`, absolutní cesta) nebo projekt nemá složku.
#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Ref {
    pub raw: String,
    pub rel: String,
    pub line: Option<u32>,
    pub path: Option<String>,
    pub exists: bool,
}

/// Bod zadání (jakýkoli úkol `- [ ]` / `- [~]` / `- [x]`, i číslovaný).
#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Item {
    /// řádek těla od 0 (jako `md::tasks`)
    pub line: usize,
    pub state: TaskState,
    /// text bez značky a bez důkazu
    pub text: String,
    pub evidence: Option<String>,
    pub images: Vec<ItemImage>,
    pub refs: Vec<Ref>,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Scan {
    pub items: Vec<Item>,
    /// relativní snímky, které nepatří k žádnému bodu
    pub loose: Vec<ItemImage>,
}

/// Jednořádková podoba textu (zalomení → mezera) — vše, co jde do jednoho řádku Markdownu.
pub fn one_line(s: &str) -> String {
    s.split(['\n', '\r']).map(str::trim).filter(|p| !p.is_empty()).collect::<Vec<_>>().join(" ")
}

/// Celý řádek je obrázek `![alt](src)` → (alt, src).
pub(super) fn image_only(line: &str) -> Option<(&str, &str)> {
    let t = line.trim();
    let s = t.strip_prefix('!')?;
    let (alt, src, len) = bracket_link(s)?;
    (len == s.len()).then_some((alt, src.trim()))
}

/// Cesta ke snímku / souboru z relativního odkazu (`./a.png`) vedle `base`.
fn join_rel(base: &Path, rel: &str) -> PathBuf {
    let mut p = base.to_path_buf();
    for part in rel.split(['/', '\\']).filter(|s| !s.is_empty() && *s != ".") {
        p.push(part);
    }
    p
}

fn image_of(base: &Path, alt: &str, src: &str) -> ItemImage {
    ItemImage { src: src.to_string(), path: join_rel(base, src).to_string_lossy().into_owned(), caption: alt.trim().to_string() }
}

/// Text bodu rozdělený na text a důkaz (` — důkaz: …`, též `–`/`-` a `dukaz`/`evidence`).
pub fn split_evidence(text: &str) -> (&str, Option<&str>) {
    let mut best: Option<(usize, usize)> = None;
    for dash in [" — ", " – ", " - "] {
        for (i, _) in text.match_indices(dash) {
            let rest = &text[i + dash.len()..];
            for kw in ["důkaz", "dukaz", "evidence"] {
                if rest.get(..kw.len()).is_some_and(|h| h.to_lowercase() == kw)
                    && rest[kw.len()..].starts_with(':')
                    && best.is_none_or(|(b, _)| i < b)
                {
                    best = Some((i, i + dash.len() + kw.len() + 1));
                }
            }
        }
    }
    match best {
        Some((i, ev)) => (text[..i].trim_end(), Some(text[ev..].trim()).filter(|e| !e.is_empty())),
        None => (text.trim_end(), None),
    }
}

/// Odkazy `@rel` / `@rel:řádek` v textu. `@` musí stát na začátku slova (e-mail `a@b.cz` ani `@jmeno`
/// bez `/` či `.` se neberou). Duplicity (stejné `raw`) se vynechají.
pub fn parse_refs(text: &str) -> Vec<Ref> {
    let mut out: Vec<Ref> = Vec::new();
    let mut prev: Option<char> = None;
    for (i, c) in text.char_indices() {
        let start = c == '@' && prev.is_none_or(|p| p.is_whitespace() || matches!(p, '(' | '[' | '"' | '\''));
        prev = Some(c);
        if !start {
            continue;
        }
        let tail = &text[i + 1..];
        let end = tail.find(|ch: char| ch.is_whitespace() || matches!(ch, '`' | '"' | '\'' | '<' | '>' | ')' | ']' | ',' | ';' | '|')).unwrap_or(tail.len());
        let tok = tail[..end].trim_end_matches(['.', ':', '!', '?']);
        let (rel, line) = match tok.rsplit_once(':') {
            Some((r, n)) if !n.is_empty() && n.bytes().all(|b| b.is_ascii_digit()) => (r, n.parse().ok()),
            _ => (tok, None),
        };
        if rel.is_empty() || !rel.contains(['/', '\\', '.']) {
            continue;
        }
        let raw = format!("@{tok}");
        if !out.iter().any(|r| r.raw == raw) {
            out.push(Ref { raw, rel: rel.to_string(), line, path: None, exists: false });
        }
    }
    out
}

/// Relativní cesta smí zůstat uvnitř složky: žádná absolutní, disk (`:`), `..`, řídicí znaky.
pub fn rel_inside(rel: &str) -> bool {
    !rel.is_empty()
        && !rel.contains(':')
        && !rel.starts_with(['/', '\\'])
        && !rel.split(['/', '\\']).any(|p| p == "..")
        && !rel.chars().any(char::is_control)
}

/// Doplní `path` a `exists` (jen když `dir` je složka projektu; existující cíl nesmí symlinkem ven).
pub fn resolve_refs(items: &mut [Item], dir: Option<&Path>) {
    let canon_dir = dir.and_then(|d| std::fs::canonicalize(d).ok());
    for r in items.iter_mut().flat_map(|i| i.refs.iter_mut()) {
        let (Some(dir), true) = (dir, rel_inside(&r.rel)) else { continue };
        let full = join_rel(dir, &r.rel);
        let exists = full.exists();
        let inside = !exists || matches!((std::fs::canonicalize(&full), &canon_dir), (Ok(c), Some(base)) if c.starts_with(base));
        if !inside {
            continue;
        }
        r.path = Some(full.to_string_lossy().into_owned());
        r.exists = exists;
    }
}

/// Body, jejich snímky a nenavázané snímky. Bloky kódu se přeskočí. Obrázek patří k bodu, když je na
/// řádku sám, odsazený víc než bod a následuje hned za bodem (nebo za jeho dalším snímkem).
pub fn scan_body(body: &str, base: &Path) -> Scan {
    let lines: Vec<&str> = body.split('\n').map(|l| l.trim_end_matches('\r')).collect();
    let mut sc = Scan::default();
    let mut attached: HashSet<usize> = HashSet::new();
    let mut code = false;
    for (i, line) in lines.iter().enumerate() {
        if is_fence(line) {
            code = !code;
            continue;
        }
        if code || attached.contains(&i) {
            continue;
        }
        let item = list_item(line).and_then(|(_, _, rest)| task_state(rest));
        if let Some((state, after)) = item {
            let w = indent_width(line);
            let mut images = Vec::new();
            for (j, l) in lines.iter().enumerate().skip(i + 1) {
                match image_only(l) {
                    Some((alt, src)) if indent_width(l) > w && relative_image(src) => {
                        images.push(image_of(base, alt, src));
                        attached.insert(j);
                    }
                    _ => break,
                }
            }
            let (text, evidence) = split_evidence(after);
            sc.items.push(Item { line: i, state, text: text.to_string(), evidence: evidence.map(str::to_string), images, refs: parse_refs(text) });
        }
        sc.loose.extend(inline_images(line, base));
    }
    sc
}

/// Relativní obrázky `![alt](src)` kdekoli v řádku.
fn inline_images(line: &str, base: &Path) -> Vec<ItemImage> {
    let mut out = Vec::new();
    let mut rest = line;
    while let Some(i) = rest.find("![") {
        let s = &rest[i + 1..];
        match bracket_link(s) {
            Some((alt, src, len)) => {
                if relative_image(src.trim()) {
                    out.push(image_of(base, alt, src.trim()));
                }
                rest = &s[len..];
            }
            None => rest = &rest[i + 2..],
        }
    }
    out
}

/// Který bod: podle `text` (`line` je jen nápověda — posunutý řádek se dohledá). Bez `text` rozhoduje
/// jen `line`. Víc shod bez sedící nápovědy = chyba, nic nenalezeno = chyba.
pub fn find_item(items: &[Item], line: Option<usize>, text: Option<&str>) -> Result<usize, String> {
    let text = text.map(str::trim).filter(|t| !t.is_empty());
    let Some(text) = text else {
        let l = line.ok_or_else(|| tr("Chybí text nebo řádek bodu.", "Missing the item's text or line."))?;
        return items.iter().position(|i| i.line == l).ok_or_else(|| trf!("Na řádku {l} není bod.", "There's no item on line {l}."));
    };
    let (want, _) = split_evidence(text);
    let hits: Vec<usize> = (0..items.len()).filter(|&k| items[k].text.trim() == want.trim()).collect();
    if let Some(&k) = hits.iter().find(|&&k| Some(items[k].line) == line) {
        return Ok(k);
    }
    match hits.as_slice() {
        [k] => Ok(*k),
        [] => Err(trf!("Bod „{text}“ v poznámce není (zkontroluj text přes notes_read).", "No item “{text}” in the note (check the text with notes_read).")),
        _ => Err(trf!("Bodů „{text}“ je v poznámce víc — upřesni řádek.", "Several items “{text}” in the note — give the line.")),
    }
}

/// Nastaví stav bodu (a důkaz) — mění jen jeho řádek. `evidence`: `Some(text)` nastaví, `Some("")`
/// odebere, `None` nechá (u návratu na otevřený odebere zastaralý důkaz). Vrací nové tělo.
pub fn set_item(body: &str, line: Option<usize>, text: Option<&str>, state: TaskState, evidence: Option<&str>) -> Result<String, String> {
    let sc = scan_body(body, Path::new(""));
    let it = &sc.items[find_item(&sc.items, line, text)?];
    let mut lines: Vec<String> = body.split('\n').map(str::to_string).collect();
    let raw = &lines[it.line];
    let l = raw.trim_end_matches('\r');
    let cr = &raw[l.len()..];
    let (_, _, rest) = list_item(l).ok_or_else(|| tr("Řádek už není bod.", "The line is no longer an item."))?;
    let at = l.len() - rest.len();
    let new_ev: Option<String> = match evidence.map(one_line) {
        Some(e) if e.is_empty() => None,
        Some(e) => Some(e),
        None if state == TaskState::Open => None,
        None => it.evidence.clone(),
    };
    let after = &rest[3..];
    let lead = &after[..after.len() - after.trim_start().len()];
    let tail = if new_ev == it.evidence {
        after.to_string()
    } else {
        match &new_ev {
            Some(e) => format!("{lead}{} — důkaz: {e}", it.text),
            None => format!("{lead}{}", it.text),
        }
    };
    lines[it.line] = format!("{}[{}]{tail}{cr}", &l[..at], state.mark());
    Ok(lines.join("\n"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use TaskState::*;

    fn base() -> &'static Path {
        Path::new("C:\\n")
    }

    #[test]
    fn scan_states_evidence_images_and_loose() {
        let body = "# Cíl\n\n- [ ] Opravit header @src/Header.tsx:42\n  ![rozbitý header](./a-1.png)\n  ![druhý](./a-2.png)\n- [~] Dokončit login — důkaz: commit abc123\n1. [x] Hotovo — důkaz: test `login_ok`\n![volný](./b.png)\n```\n- [ ] v kódu\n![x](./c.png)\n```\n";
        let sc = scan_body(body, base());
        let got: Vec<_> = sc.items.iter().map(|i| (i.line, i.state, i.text.as_str(), i.evidence.as_deref(), i.images.len())).collect();
        assert_eq!(
            got,
            vec![
                (2, Open, "Opravit header @src/Header.tsx:42", None, 2),
                (5, Proposed, "Dokončit login", Some("commit abc123"), 0),
                (6, Done, "Hotovo", Some("test `login_ok`"), 0),
            ]
        );
        assert_eq!(sc.items[0].images[0], ItemImage { src: "./a-1.png".into(), path: "C:\\n\\a-1.png".into(), caption: "rozbitý header".into() });
        assert_eq!(sc.items[0].refs, vec![Ref { raw: "@src/Header.tsx:42".into(), rel: "src/Header.tsx".into(), line: Some(42), path: None, exists: false }]);
        assert_eq!(sc.loose.iter().map(|i| i.src.as_str()).collect::<Vec<_>>(), vec!["./b.png"]);
    }

    #[test]
    fn image_attaches_only_when_indented_and_adjacent() {
        // neodsazený obrázek, vzdálený obrázek a externí URL k bodu nepatří
        let b = "- [ ] a\n![x](./1.png)\n- [ ] b\n\n  ![y](./2.png)\n- [ ] c\n  ![z](https://example.com/z.png)\n";
        let sc = scan_body(b, base());
        assert!(sc.items.iter().all(|i| i.images.is_empty()));
        assert_eq!(sc.loose.len(), 2);
    }

    #[test]
    fn evidence_table() {
        let cases: &[(&str, &str, Option<&str>)] = &[
            ("text", "text", None),
            ("text — důkaz: commit a1", "text", Some("commit a1")),
            ("text - dukaz: test x", "text", Some("test x")),
            ("text – Evidence: PR 5", "text", Some("PR 5")),
            ("text — důkaz:", "text", None),
            ("jen — pomlčka", "jen — pomlčka", None),
            ("a — důkaz: x — důkaz: y", "a", Some("x — důkaz: y")),
        ];
        for (t, text, ev) in cases {
            assert_eq!(split_evidence(t), (*text, *ev), "{t}");
        }
    }

    #[test]
    fn refs_table() {
        let r = |t: &str| parse_refs(t).into_iter().map(|r| (r.rel, r.line)).collect::<Vec<_>>();
        assert_eq!(r("viz @src/a.ts."), vec![("src/a.ts".into(), None)]);
        assert_eq!(r("(@src/x.ts:42) a @README.md"), vec![("src/x.ts".into(), Some(42)), ("README.md".into(), None)]);
        assert_eq!(r("@a/b.rs:7: konec"), vec![("a/b.rs".into(), Some(7))]);
        assert_eq!(r("@a/b.rs @a/b.rs"), vec![("a/b.rs".into(), None)]);
        assert!(r("napiš uzivatel@example.com").is_empty());
        assert!(r("@uzivatel řekl").is_empty());
        assert!(r("@ samotné").is_empty());
        assert_eq!(r("@../tajne.txt"), vec![("../tajne.txt".into(), None)]);
    }

    #[test]
    fn rel_inside_table() {
        for ok in ["a.rs", "src/a/b.ts", "src\\a.ts", "./x.md", "a b/c.md"] {
            assert!(rel_inside(ok), "{ok}");
        }
        for bad in ["", "../x", "a/../../b", "..\\x", "/etc/passwd", "\\\\srv\\x", "C:\\Windows\\x", "C:x", "a\u{0}b"] {
            assert!(!rel_inside(bad), "{bad:?}");
        }
    }

    #[test]
    fn resolve_refs_inside_project_only() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("src")).unwrap();
        std::fs::write(dir.path().join("src/a.ts"), "x").unwrap();
        let mut items = scan_body("- [ ] @src/a.ts:3 @src/zmizel.ts @../ven.txt @/etc/x.md", base()).items;
        resolve_refs(&mut items, Some(dir.path()));
        let r = &items[0].refs;
        assert!(r[0].exists && r[0].path.as_deref().unwrap().ends_with("a.ts"));
        assert!(!r[1].exists && r[1].path.is_some());
        assert!(r[2].path.is_none() && !r[2].exists);
        assert!(r[3].path.is_none());
        let mut none = scan_body("- [ ] @src/a.ts", base()).items;
        resolve_refs(&mut none, None);
        assert!(none[0].refs[0].path.is_none());
    }

    #[test]
    fn set_item_table_changes_only_that_line() {
        let body = "# Zadání\n\n- [ ] první\n  ![s](./a.png)\n- [ ] druhý\n- [~] třetí — důkaz: commit 1\n";
        let ok = |l: Option<usize>, t: Option<&str>, s, e: Option<&str>| set_item(body, l, t, s, e).unwrap();
        // navrhnout s důkazem (zalomení se sloučí)
        assert_eq!(ok(Some(4), Some("druhý"), Proposed, Some("test `a`\nb")), body.replace("- [ ] druhý", "- [~] druhý — důkaz: test `a` b"));
        // posunutý řádek se dohledá podle textu
        assert_eq!(ok(Some(0), Some("druhý"), Done, None), body.replace("- [ ] druhý", "- [x] druhý"));
        // jen podle řádku
        assert_eq!(ok(Some(2), None, Done, None), body.replace("- [ ] první", "- [x] první"));
        // potvrzení zachová důkaz, zamítnutí ho odebere
        assert_eq!(ok(None, Some("třetí"), Done, None), body.replace("[~] třetí", "[x] třetí"));
        assert_eq!(ok(None, Some("třetí"), Open, None), body.replace("- [~] třetí — důkaz: commit 1", "- [ ] třetí"));
        assert_eq!(ok(None, Some("třetí"), Proposed, Some("commit 2")), body.replace("commit 1", "commit 2"));
        assert_eq!(ok(None, Some("třetí — důkaz: commit 1"), Proposed, Some("")), body.replace(" — důkaz: commit 1", ""));
        // chyby: nic / nejednoznačné / žádný text ani řádek / řádek bez bodu
        assert!(set_item(body, None, Some("nic"), Done, None).is_err());
        assert!(set_item(&format!("{body}- [ ] druhý\n"), None, Some("druhý"), Done, None).is_err());
        assert!(set_item(body, None, None, Done, None).is_err());
        assert!(set_item(body, Some(0), None, Done, None).is_err());
        // nejednoznačné, ale řádek nápovědy sedí
        let dup = format!("{body}- [ ] druhý\n");
        assert!(set_item(&dup, Some(6), Some("druhý"), Done, None).unwrap().ends_with("- [x] druhý\n"));
    }

    #[test]
    fn set_item_keeps_crlf_and_numbering() {
        let body = "1. [ ] a\r\n2. [ ] b\r\n";
        assert_eq!(set_item(body, None, Some("b"), Proposed, Some("x")).unwrap(), "1. [ ] a\r\n2. [~] b — důkaz: x\r\n");
    }
}
