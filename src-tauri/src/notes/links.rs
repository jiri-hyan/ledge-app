//! Vazby mezi poznámkami a štítky v textu (Lístky 2.0): `[[název poznámky]]` (i `[[název|popisek]]`,
//! jako Obsidian — poznámky zůstávají obyčejné `.md`) a `#štítek`. Čisté funkce: rozpoznání na pozici
//! (`wiki_at`, `tag_at` — používá je `md::to_html`, který vše escapuje), štítky v těle (`inline_tags`,
//! bloky kódu a `kód` se přeskakují), dohledání cíle (`resolve`: titulek bez ohledu na
//! velikost písmen a diakritiku, pak jméno souboru) a našeptávač (`suggest`).

use super::meta::{norm_tag, tag_char};
use super::search::fold_text;
use super::{NoteView, store};
use serde::Serialize;
use std::path::Path;

/// Nejdelší název ve `[[…]]` (delší text už není odkaz, ale omyl).
const MAX_WIKI: usize = 200;

/// `[[cíl]]` / `[[cíl|popisek]]` na začátku `s`: (cíl, popisek, délka v bajtech). Uvnitř nesmí být
/// `[`, `]` ani konec řádku; prázdný cíl není odkaz.
pub fn wiki_at(s: &str) -> Option<(&str, &str, usize)> {
    let rest = s.strip_prefix("[[")?;
    // hledá se jen v okně `MAX_WIKI` (řádek plný `[[` bez `]]` nesmí být kvadratický)
    let mut lim = rest.len().min(MAX_WIKI + 2);
    while !rest.is_char_boundary(lim) {
        lim -= 1;
    }
    let end = rest[..lim].find("]]")?;
    let inner = &rest[..end];
    if inner.is_empty() || inner.len() > MAX_WIKI || inner.contains(['[', ']', '\n', '\r']) {
        return None;
    }
    let (target, label) = match inner.split_once('|') {
        Some((t, l)) if !l.trim().is_empty() => (t.trim(), l.trim()),
        Some((t, _)) => (t.trim(), t.trim()),
        None => (inner.trim(), inner.trim()),
    };
    (!target.is_empty()).then_some((target, label, end + 4))
}

/// `#štítek` na bajtu `i` řádku `s` (před ním začátek nebo mezera / závorka; za `#` aspoň jeden znak
/// štítku, ne jen číslice — `#1` není štítek, `# Nadpis` taky ne): (štítek, délka v bajtech včetně `#`).
pub fn tag_at(s: &str, i: usize) -> Option<(String, usize)> {
    if s.as_bytes().get(i) != Some(&b'#') {
        return None;
    }
    let before_ok = s[..i].chars().next_back().is_none_or(|c| c.is_whitespace() || matches!(c, '(' | ',' | ';'));
    if !before_ok {
        return None;
    }
    let rest = &s[i + 1..];
    let len = rest.char_indices().find(|(_, c)| !tag_char(*c)).map_or(rest.len(), |(k, _)| k);
    // koncové `/` nepatří do štítku (`#a/` na konci věty)
    let raw = rest[..len].trim_end_matches('/');
    norm_tag(raw).map(|t| (t, raw.len() + 1))
}

/// Řádky těla mimo bloky kódu, s vyříznutým inline `kódem`.
fn text_lines(body: &str) -> impl Iterator<Item = String> + '_ {
    let mut fence = false;
    body.lines().filter_map(move |l| {
        let t = l.trim_start();
        if t.starts_with("```") || t.starts_with("~~~") {
            fence = !fence;
            return None;
        }
        if fence {
            return None;
        }
        let mut out = String::with_capacity(l.len());
        let mut code = false;
        for c in l.chars() {
            if c == '`' {
                code = !code;
                out.push(' ');
            } else {
                out.push(if code { ' ' } else { c });
            }
        }
        Some(out)
    })
}

/// `#štítky` v těle (bez kódu), v pořadí výskytu, bez duplicit.
pub fn inline_tags(body: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for line in text_lines(body) {
        let mut i = 0;
        while let Some(k) = line[i..].find('#') {
            let at = i + k;
            match tag_at(&line, at) {
                Some((t, len)) => {
                    if !out.contains(&t) {
                        out.push(t);
                    }
                    i = at + len;
                }
                None => i = at + 1,
            }
        }
    }
    out
}

/// Jméno souboru bez časové předpony `YYYY-MM-DD-HHMMSS-` (ta je u poznámek z Ledge).
fn stem_name(path: &str) -> String {
    let stem = Path::new(path).file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
    let b = stem.as_bytes();
    let stamped = b.len() > 18 && b[17] == b'-' && b[..17].iter().enumerate().all(|(i, c)| if matches!(i, 4 | 7 | 10) { *c == b'-' } else { c.is_ascii_digit() });
    if stamped { stem[18..].to_string() } else { stem }
}

/// Poznámka pro `[[name]]`: titulek (bez velikosti písmen a diakritiky), pak jméno souboru (i `slug`
/// bez časové předpony). Víc shod → nehotová před hotovou, pak nejnovější (`all` je nejnovější první).
pub fn resolve<'a>(all: &'a [NoteView], name: &str) -> Option<&'a NoteView> {
    let want = fold_text(name.trim());
    if want.is_empty() {
        return None;
    }
    let slug = store::slug(name);
    let by_title = |n: &&NoteView| fold_text(n.title.trim()) == want;
    let by_file = |n: &&NoteView| {
        let s = stem_name(&n.path);
        fold_text(&s) == want || s == slug
    };
    let pick = |f: &dyn Fn(&&NoteView) -> bool| all.iter().filter(|n| !n.done).find(f).or_else(|| all.iter().find(f));
    pick(&by_title).or_else(|| pick(&by_file))
}

/// Návrh pro našeptávač `[[…`: titulek a cesta.
#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct LinkHit {
    pub title: String,
    pub path: String,
}

/// Našeptávač: titulky obsahující `q` (bez diakritiky), začátek titulku první, nehotové před hotovými;
/// prázdné `q` = nejnovější. Bez prázdných titulků a duplicit.
pub fn suggest(all: &[NoteView], q: &str, limit: usize) -> Vec<LinkHit> {
    let want = fold_text(q.trim());
    let mut hits: Vec<(u8, &NoteView)> = all
        .iter()
        .filter(|n| !n.title.trim().is_empty() && !n.title.contains(['[', ']', '|']))
        .filter_map(|n| {
            let t = fold_text(&n.title);
            let rank = if want.is_empty() || t.starts_with(&want) { 0 } else if t.contains(&want) { 1 } else { return None };
            Some((rank * 2 + u8::from(n.done), n))
        })
        .collect();
    hits.sort_by_key(|(r, _)| *r);
    let mut out: Vec<LinkHit> = Vec::new();
    for (_, n) in hits {
        if out.len() >= limit {
            break;
        }
        if !out.iter().any(|o| fold_text(&o.title) == fold_text(&n.title)) {
            out.push(LinkHit { title: n.title.trim().to_string(), path: n.path.clone() });
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::super::DEFAULT_KIND;
    use super::*;

    /// (vstup, očekávaná vazba (cíl, popisek, délka))
    type WikiCase<'a> = (&'a str, Option<(&'a str, &'a str, usize)>);
    /// (řádek, pozice `#`, očekávaný štítek (štítek, délka))
    type TagCase<'a> = (&'a str, usize, Option<(&'a str, usize)>);

    #[test]
    fn wiki_at_table() {
        let cases: &[WikiCase] = &[
            ("[[Nákup]] zbytek", Some(("Nákup", "Nákup", 10))),
            ("[[ cíl | popisek ]]", Some(("cíl", "popisek", 20))),
            ("[[cíl|]]", Some(("cíl", "cíl", 9))),
            ("[[]]", None),
            ("[[ ]]", None),
            ("[[|x]]", None),
            ("[[a]b]]", None),
            ("[[neuzavřený", None),
            ("[a](b)", None),
            ("[[a\nb]]", None),
        ];
        for (s, want) in cases {
            assert_eq!(wiki_at(s), *want, "{s}");
        }
    }

    #[test]
    fn tag_at_table() {
        let cases: &[TagCase] = &[
            ("#qa", 0, Some(("qa", 3))),
            ("viz #Práce.", 4, Some(("práce", 7))),
            ("(#a/b)", 1, Some(("a/b", 4))),
            ("#a/ konec", 0, Some(("a", 2))),
            ("# Nadpis", 0, None),
            ("#123", 0, None),
            ("issue#5", 5, None),
            ("https://x.cz/#kotva", 13, None),
            ("x #", 2, None),
        ];
        for (s, i, want) in cases {
            assert_eq!(tag_at(s, *i), want.map(|(t, l)| (t.to_string(), l)), "{s} @{i}");
        }
    }

    #[test]
    fn inline_tags_and_links_skip_code() {
        let body = "# Nadpis\nÚkol #qa a #Web, znovu #qa\n`#kod` [[Nákup]] a [[nakup|jinak]]\n```\n#ne [[Ne]]\n```\n- [ ] #tým [[Druhá poznámka]]";
        assert_eq!(inline_tags(body), ["qa", "web", "tým"]);
        assert!(inline_tags("").is_empty() && inline_tags("[[#ne]] a#ne").is_empty());
    }

    fn nv(path: &str, title: &str, done: bool) -> NoteView {
        NoteView {
            path: path.into(),
            project: None,
            created: String::new(),
            created_ms: 0,
            remind: None,
            remind_ms: None,
            done,
            title: title.into(),
            heading: None,
            images: 0,
            kind: DEFAULT_KIND.into(),
            updated: None,
            tasks_done: 0,
            tasks_total: 0,
            color: None,
            tags: Vec::new(),
        }
    }

    #[test]
    fn resolve_table() {
        let all = [
            nv(r"C:\n\_obecne\2026-10-20-100000-nakup.md", "Nákup", true),
            nv(r"C:\n\_obecne\2026-10-19-100000-nakup.md", "Nákup", false),
            nv(r"C:\n\_obecne\Seznam filmů.md", "Filmy na víkend", false),
            nv(r"C:\n\app\2026-10-18-090000-release-checklist.md", "Release — checklist", false),
        ];
        let cases: &[(&str, Option<usize>)] = &[
            ("nákup", Some(1)),
            ("NAKUP", Some(1)),
            ("Filmy na víkend", Some(2)),
            ("seznam filmu", Some(2)),
            ("release checklist", Some(3)),
            ("neexistuje", None),
            ("  ", None),
        ];
        for (name, want) in cases {
            assert_eq!(resolve(&all, name).map(|n| n.path.as_str()), want.map(|i| all[i].path.as_str()), "{name}");
        }
    }

    #[test]
    fn suggest_table() {
        let all = [nv("a", "Nákup", false), nv("b", "Velký nákup", false), nv("c", "Nákup", true), nv("d", "", false), nv("e", "Hotový nákupní seznam", true), nv("f", "x [y]", false)];
        let titles = |q: &str, n: usize| suggest(&all, q, n).into_iter().map(|h| h.title).collect::<Vec<_>>();
        assert_eq!(titles("nak", 10), ["Nákup", "Velký nákup", "Hotový nákupní seznam"]);
        assert_eq!(titles("", 2), ["Nákup", "Velký nákup"]);
        assert_eq!(titles("zzz", 10), Vec::<String>::new());
    }
}
