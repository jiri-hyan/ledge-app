//! Hledání v těle poznámek (Ctrl+K) a skládání textu bez diakritiky: `fold_text` (malá písmena bez
//! diakritiky, znak za znak), `Folded` (složený text + mapa bajtů zpět do originálu), `snippet`
//! (úryvek kolem shody), `matches` (všechna slova musí sedět) a `Index` (poznámky v paměti, přenačtené při
//! změně `rev`). Čisté funkce + malý index; I/O jen `Index::build` (čte `.md` pod kořenem).

use super::{NoteView, split, store, edit};
use serde::Serialize;
use std::path::Path;

/// Malá písmena bez diakritiky, přesně jeden znak za znak (délky v znacích zůstanou — proto se dají
/// hledat pozice v originálu).
pub fn fold_text(s: &str) -> String {
    s.chars().map(|c| store::fold(c.to_lowercase().next().unwrap_or(c))).collect()
}

/// Složený text + pro každý bajt složeného textu odpovídající bajt originálu (`map.len() == text.len() + 1`).
pub struct Folded {
    pub text: String,
    map: Vec<usize>,
}

impl Folded {
    pub fn new(orig: &str) -> Self {
        let mut text = String::with_capacity(orig.len());
        let mut map = Vec::with_capacity(orig.len() + 1);
        for (at, c) in orig.char_indices() {
            let f = store::fold(c.to_lowercase().next().unwrap_or(c));
            text.push(f);
            map.extend(std::iter::repeat_n(at, f.len_utf8()));
        }
        map.push(orig.len());
        Self { text, map }
    }

    /// Bajtový rozsah ve složeném textu → rozsah v originálu.
    pub fn orig(&self, from: usize, to: usize) -> (usize, usize) {
        (self.map[from], self.map[to])
    }
}

/// Slova dotazu (složená); prázdný dotaz = žádná.
pub fn words(query: &str) -> Vec<String> {
    fold_text(query).split_whitespace().map(str::to_string).collect()
}

/// Všechna slova se vyskytují v `hay` (už složeném).
pub fn matches(hay: &str, words: &[String]) -> bool {
    !words.is_empty() && words.iter().all(|w| hay.contains(w.as_str()))
}

/// Úryvek z originálu kolem prvního výskytu prvního slova: ~`before` znaků před a ~`after` za shodou,
/// bílé znaky sloučené, `…` na uříznutých koncích. `None`, když slovo v textu není.
pub fn snippet(orig: &str, words: &[String], before: usize, after: usize) -> Option<String> {
    let f = Folded::new(orig);
    let w = words.first()?;
    let at = f.text.find(w.as_str())?;
    let (from, to) = f.orig(at, at + w.len());
    let pre: Vec<usize> = orig[..from].char_indices().map(|(i, _)| i).collect();
    let start = if pre.len() > before { pre[pre.len() - before] } else { 0 };
    let end = orig[to..].char_indices().nth(after).map_or(orig.len(), |(i, _)| to + i);
    let mut out: String = orig[start..end].split_whitespace().collect::<Vec<_>>().join(" ");
    if start > 0 {
        out.insert(0, '…');
    }
    if end < orig.len() {
        out.push('…');
    }
    Some(out)
}

/// Strop těla v indexu (delší poznámka se prohledá jen do tohoto místa).
const BODY_MAX: usize = 64 * 1024;

/// Tělo poznámky bez frontmatteru (bez obrázků a bez ověřování otisku — jen text k hledání). Cesta musí
/// ležet v kořeni (`check_note_path`), nečitelný / příliš velký soubor = prázdné tělo.
fn read_body(root: &Path, path: &Path) -> String {
    let Ok(real) = store::check_note_path(root, path) else { return String::new() };
    let Ok(meta) = std::fs::metadata(&real) else { return String::new() };
    if meta.len() > super::MAX_NOTE_BYTES {
        return String::new();
    }
    let Ok(bytes) = std::fs::read(&real) else { return String::new() };
    let text = String::from_utf8_lossy(&bytes);
    let body = edit::body_view(split(&text).1).1;
    body.trim_end().chars().take(BODY_MAX).collect()
}

/// Jedna poznámka v indexu.
struct Doc {
    view: NoteView,
    body: String,
    /// složený titulek + tělo (co se hledá)
    hay: String,
}

/// Zásah hledání: poznámka + úryvek z těla (prázdný, když sedí jen titulek).
#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Hit {
    #[serde(flatten)]
    pub view: NoteView,
    pub snippet: String,
}

/// Poznámky v paměti pro hledání. Staví se líně při první hledání po změně `rev` (stejně jako seznam
/// poznámek); archivované se nehledají.
#[derive(Default)]
pub struct Index {
    rev: u64,
    docs: Vec<Doc>,
}

impl Index {
    /// Přenačte tělo každé poznámky ze seznamu `views` (cesty pod `root`).
    pub fn build(root: &Path, views: &[NoteView], rev: u64) -> Self {
        let docs = views
            .iter()
            .filter(|v| !v.done)
            .map(|v| {
                let body = read_body(root, Path::new(&v.path));
                let hay = fold_text(&format!("{}\n{}", v.title, body));
                Doc { view: v.clone(), body, hay }
            })
            .collect();
        Self { rev, docs }
    }

    pub fn rev(&self) -> u64 {
        self.rev
    }

    /// Zásahy pro `query` (nejnovější první, jak jsou poznámky seřazené), nejvýš `limit`. Titulek sedí
    /// → úryvek je prázdný; jinak úryvek z těla.
    pub fn search(&self, query: &str, limit: usize) -> Vec<Hit> {
        let ws = words(query);
        let mut out = Vec::new();
        for d in &self.docs {
            if out.len() >= limit {
                break;
            }
            if !matches(&d.hay, &ws) {
                continue;
            }
            let in_title = matches(&fold_text(&d.view.title), &ws);
            let snip = if in_title { String::new() } else { snippet(&d.body, &ws, 28, 60).unwrap_or_default() };
            out.push(Hit { view: d.view.clone(), snippet: snip });
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fold_is_char_for_char() {
        assert_eq!(fold_text("Příliš ŽLUŤOUČKÝ kůň"), "prilis zlutoucky kun");
        for s in ["", "abc", "Žluťoučký", "İ", "ǅ", "日本語 ok"] {
            assert_eq!(fold_text(s).chars().count(), s.chars().count(), "{s:?}");
        }
    }

    #[test]
    fn folded_maps_back_to_original_bytes() {
        let o = "Řekni Žofii: zítra v 10";
        let f = Folded::new(o);
        let at = f.text.find("zitra").unwrap();
        let (a, b) = f.orig(at, at + 5);
        assert_eq!(&o[a..b], "zítra");
        assert_eq!(f.orig(0, f.text.len()), (0, o.len()));
    }

    #[test]
    fn words_and_matches() {
        assert_eq!(words("  Faktura  NOVÁK "), vec!["faktura", "novak"]);
        assert!(words("   ").is_empty());
        let h = fold_text("Zavolat Novákovi kvůli faktuře");
        assert!(matches(&h, &words("novak faktur")));
        assert!(matches(&h, &words("NOVÁKOVI")));
        assert!(!matches(&h, &words("novak smlouva")));
        assert!(!matches(&h, &[]));
    }

    #[test]
    fn index_finds_title_and_body_and_skips_done() {
        use super::super::testkit::{at, tmp_root};
        use super::super::{NewNote, create, scan, set_done};
        let root = tmp_root();
        let mk = |text: &str, sec: u32| create(root.path(), &NewNote { text: text.into(), ..Default::default() }, &at(&format!("2026-10-20T10:00:{sec:02}+02:00"))).unwrap();
        let nakup = mk("Nákup

koupit mléko a chleba", 1);
        mk("Schůzka s Novákem

řešit fakturu za říjen", 2);
        let build = || Index::build(root.path(), &scan(root.path()), 7);
        let idx = build();
        assert_eq!(idx.rev(), 7);
        // tělo: úryvek obsahuje shodu
        let h = idx.search("FAKTUR říjen", 10);
        assert_eq!(h.len(), 1);
        assert!(h[0].snippet.contains("fakturu") && h[0].view.title.starts_with("Schůzka"), "{h:?}");
        // titulek: bez úryvku; bez diakritiky
        let h = idx.search("novakem", 10);
        assert_eq!((h.len(), h[0].snippet.as_str()), (1, ""));
        // slova z titulku i těla dohromady
        assert_eq!(idx.search("nakup chleb", 10).len(), 1);
        assert_eq!(idx.search("nakup faktur", 10).len(), 0);
        assert_eq!(idx.search("", 10).len(), 0);
        assert_eq!(idx.search("e", 1).len(), 1, "limit");
        // hotová se nehledá
        set_done(root.path(), &nakup, true).unwrap();
        assert_eq!(build().search("mleko", 10).len(), 0);
    }

    #[test]
    fn snippet_table() {
        let long = format!("{} Ředitel schválil rozpočet na příští rok. {}", "a ".repeat(50), "b ".repeat(80));
        let s = snippet(&long, &words("reditel"), 10, 20).unwrap();
        assert!(s.starts_with('…') && s.ends_with('…') && s.contains("Ředitel schválil"), "{s}");
        assert!(s.chars().count() <= 10 + 7 + 20 + 3, "{s}");
        assert_eq!(snippet("krátké\n\n  tělo", &words("telo"), 28, 60).as_deref(), Some("krátké tělo"));
        assert_eq!(snippet("Začátek textu", &words("zacatek"), 28, 60).as_deref(), Some("Začátek textu"));
        assert_eq!(snippet("něco", &words("jiného"), 28, 60), None);
        assert_eq!(snippet("něco", &[], 28, 60), None);
        // shoda na konci: žádné „…“ vzadu, vpředu ano
        let tail = format!("{}konec", "x ".repeat(40));
        let s = snippet(&tail, &words("konec"), 6, 60).unwrap();
        assert!(s.starts_with('…') && s.ends_with("konec"), "{s}");
    }
}
