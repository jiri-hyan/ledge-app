//! Ikony rychlých akcí: odkud ikona přijde (`spec`), lokální ikony (vlastní obrázek, ikona
//! souboru/složky/exe přes `iconimg`) a favicon webu — stažený JEDNOU (při přidání akce nebo na
//! výslovné „Obnovit ikonu“), s termínem a stropem velikosti, převedený na PNG a uložený v
//! `%LOCALAPPDATA%\Ledge\icons\<host>.png` (neúspěch = `<host>.miss`, další pokus jen na klik).
//! Nikdy periodicky. Stav a vlákno drží `actionstate.rs`.

use crate::actions::{Action, ActionKind};
use crate::{actionrun, iconimg, icons, personal};
use std::path::{Path, PathBuf};
use std::time::Duration;

/// Největší vlastní obrázek / stažená ikona / HTML stránky, které se čtou.
const MAX_IMAGE: u64 = 1 << 20;
const MAX_ICON_DOWNLOAD: usize = 256 * 1024;
const MAX_HTML: usize = 512 * 1024;
const NET_TIMEOUT: Duration = Duration::from_secs(5);
/// Kolik kandidátů favicon se nejvýš zkusí stáhnout.
const MAX_TRIES: usize = 4;

/// Odkud vzít ikonu akce.
#[derive(Debug, PartialEq)]
pub enum Spec {
    Monogram,
    /// vlastní obrázek z `icon`
    Image(PathBuf),
    /// ikona souboru / složky ze shellu
    Shell(PathBuf),
    /// ikona aplikace (cíl ověří `personal::classify_fs` — i holé jméno ze složky Windows)
    Exe(String),
    /// favicon webu: `origin` = `https://host[:port]`, `key` = jméno souboru v cache
    Favicon { origin: String, key: String },
}

/// Čisté rozhodnutí podle configu: `icon` chybí / `auto` → podle typu, `monogram` → písmena,
/// jinak cesta k obrázku.
pub fn spec(a: &Action) -> Spec {
    match a.icon.as_deref().map(str::trim) {
        Some("monogram") => return Spec::Monogram,
        Some(p) if !p.is_empty() && p != "auto" => return Spec::Image(PathBuf::from(p)),
        _ => {}
    }
    let t = a.target.trim();
    match a.kind {
        ActionKind::Url | ActionKind::Web => origin(t).map_or(Spec::Monogram, |(origin, key)| Spec::Favicon { origin, key }),
        ActionKind::File | ActionKind::Folder => personal::normalize(t).map_or(Spec::Monogram, |n| Spec::Shell(PathBuf::from(n))),
        ActionKind::App => Spec::Exe(t.to_string()),
        ActionKind::Command => Spec::Monogram,
    }
}

/// `https://Host:8080/cesta?x` → (`https://host:8080`, `host_8080`). Jen http(s) a rozumný host.
pub fn origin(url: &str) -> Option<(String, String)> {
    let (scheme, rest) = url.split_once("://")?;
    let scheme = scheme.to_ascii_lowercase();
    if scheme != "http" && scheme != "https" {
        return None;
    }
    let authority = rest.split(['/', '?', '#']).next()?;
    let hostport = authority.rsplit('@').next()?.to_ascii_lowercase();
    let (host, port) = match hostport.rsplit_once(':') {
        Some((h, p)) if !p.is_empty() && p.chars().all(|c| c.is_ascii_digit()) => (h.to_string(), Some(p.to_string())),
        _ => (hostport.clone(), None),
    };
    if host.is_empty() || host.len() > 253 || !host.chars().all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '-') {
        return None;
    }
    let origin = match &port {
        Some(p) => format!("{scheme}://{host}:{p}"),
        None => format!("{scheme}://{host}"),
    };
    let key = match port {
        Some(p) => format!("{host}_{p}"),
        None => host,
    };
    Some((origin, key))
}

/// `href` z `<link rel=icon>` → absolutní URL (http/https); `data:`, `javascript:` apod. ne.
pub fn join_href(origin: &str, href: &str) -> Option<String> {
    let h = href.trim();
    let lower = h.to_ascii_lowercase();
    if h.is_empty() {
        return None;
    }
    if lower.starts_with("http://") || lower.starts_with("https://") {
        return Some(h.to_string());
    }
    if let Some(rest) = h.strip_prefix("//") {
        let scheme = origin.split("://").next()?;
        return Some(format!("{scheme}://{rest}"));
    }
    if lower.contains(':') {
        return None; // jiné schéma
    }
    Some(if h.starts_with('/') { format!("{origin}{h}") } else { format!("{origin}/{h}") })
}

/// Kandidáti ke stažení: deklarované ikony (apple-touch-icon první — bývá největší; SVG a manifest
/// vynechané, WIC je neumí), pak `/favicon.ico`. Bez duplicit, nejvýš `MAX_TRIES`.
pub fn candidates(origin: &str, links: &[(String, String)]) -> Vec<String> {
    let mut ranked: Vec<(u8, String)> = links
        .iter()
        .filter(|(rel, href)| !rel.split_whitespace().any(|r| r == "manifest") && !href.to_ascii_lowercase().split(['?', '#']).next().unwrap_or("").ends_with(".svg"))
        .filter_map(|(rel, href)| Some((if rel.contains("apple-touch-icon") { 0 } else { 1 }, join_href(origin, href)?)))
        .collect();
    ranked.sort_by_key(|(r, _)| *r);
    let mut out: Vec<String> = Vec::new();
    for u in ranked.into_iter().map(|(_, u)| u).chain([format!("{origin}/favicon.ico")]) {
        if !out.contains(&u) {
            out.push(u);
        }
    }
    out.truncate(MAX_TRIES);
    out
}

pub fn cache_dir() -> PathBuf {
    actionrun::local_dir().join("icons")
}

/// Stav faviconu v cache: `Some(Some(png))` = máme, `Some(None)` = minule nevyšlo, `None` = nezkoušeno.
pub fn cached(dir: &Path, key: &str) -> Option<Option<Vec<u8>>> {
    if let Ok(b) = std::fs::read(dir.join(format!("{key}.png"))) {
        return Some(Some(b));
    }
    dir.join(format!("{key}.miss")).exists().then_some(None)
}

/// Zapíše výsledek stažení (PNG nebo značku neúspěchu).
pub fn store(dir: &Path, key: &str, png: Option<&[u8]>) {
    let _ = std::fs::create_dir_all(dir);
    match png {
        Some(b) => {
            let _ = std::fs::write(dir.join(format!("{key}.png")), b);
            let _ = std::fs::remove_file(dir.join(format!("{key}.miss")));
        }
        None => {
            let _ = std::fs::write(dir.join(format!("{key}.miss")), b"");
        }
    }
}

/// Smaže výsledek pro `key` (výslovné „Obnovit ikonu“ → příště se stáhne znovu).
pub fn forget(dir: &Path, key: &str) {
    let _ = std::fs::remove_file(dir.join(format!("{key}.png")));
    let _ = std::fs::remove_file(dir.join(format!("{key}.miss")));
}

/// Ikona z disku / shellu (bez sítě): `(mime, bytes)`. Stat a COM — jen vlákno `action-icons`.
pub fn local(spec: &Spec) -> Option<(&'static str, Vec<u8>)> {
    match spec {
        Spec::Monogram | Spec::Favicon { .. } => None,
        Spec::Image(p) => {
            let meta = std::fs::metadata(p).ok()?;
            if !meta.is_file() || meta.len() > MAX_IMAGE {
                return None;
            }
            let bytes = std::fs::read(p).ok()?;
            if p.extension().is_some_and(|e| e.eq_ignore_ascii_case("svg")) {
                return Some(("image/svg+xml", bytes));
            }
            iconimg::to_png(&bytes).map(|b| ("image/png", b))
        }
        Spec::Shell(p) => p.exists().then(|| iconimg::shell_png(p)).flatten().map(|b| ("image/png", b)),
        Spec::Exe(t) => match personal::classify_fs(t) {
            personal::LinkTarget::App(p) => iconimg::exe_png(&p).map(|b| ("image/png", b)),
            _ => None,
        },
    }
}

/// Stáhne nejvýš `limit` bajtů (víc = `None`, ne useknutý soubor).
async fn get_limited(client: &reqwest::Client, url: &str, limit: usize) -> Option<Vec<u8>> {
    let mut resp = client.get(url).send().await.ok()?;
    if !resp.status().is_success() || resp.content_length().is_some_and(|n| n > limit as u64) {
        return None;
    }
    let mut buf = Vec::new();
    while let Some(chunk) = resp.chunk().await.ok()? {
        buf.extend_from_slice(&chunk);
        if buf.len() > limit {
            return None;
        }
    }
    Some(buf)
}

/// Favicon webu jako PNG: HTML úvodní stránky → `<link rel=icon>` kandidáti → `/favicon.ico`.
/// Každý požadavek s termínem 5 s, stažené soubory se stropem.
pub async fn fetch_favicon(origin: &str) -> Option<Vec<u8>> {
    let client = reqwest::Client::builder()
        .timeout(NET_TIMEOUT)
        .redirect(reqwest::redirect::Policy::limited(5))
        .user_agent(concat!("Ledge/", env!("CARGO_PKG_VERSION")))
        .build()
        .ok()?;
    let links = match get_limited(&client, &format!("{origin}/"), MAX_HTML).await {
        Some(html) => icons::icon_links(&String::from_utf8_lossy(&html)),
        None => Vec::new(),
    };
    for url in candidates(origin, &links) {
        if let Some(bytes) = get_limited(&client, &url, MAX_ICON_DOWNLOAD).await
            && let Some(png) = iconimg::to_png(&bytes)
        {
            return Some(png);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn act(kind: ActionKind, target: &str, icon: Option<&str>) -> Action {
        Action { id: "a".into(), label: "A".into(), kind, target: target.into(), icon: icon.map(String::from), ..Default::default() }
    }

    #[test]
    fn spec_table() {
        let fav = |o: &str, k: &str| Spec::Favicon { origin: o.into(), key: k.into() };
        let cases = [
            (act(ActionKind::Url, "https://Mapy.cz/x?y", None), fav("https://mapy.cz", "mapy.cz")),
            (act(ActionKind::Url, "http://localhost:5173/", Some("auto")), fav("http://localhost:5173", "localhost_5173")),
            (act(ActionKind::Url, "https://mapy.cz", Some("monogram")), Spec::Monogram),
            (act(ActionKind::Url, "https://mapy.cz", Some(r"C:\i\m.png")), Spec::Image(r"C:\i\m.png".into())),
            (act(ActionKind::Folder, r"C:\Fotky\", None), Spec::Shell(r"C:\Fotky\".into())),
            (act(ActionKind::File, r"C:\a.txt. ", None), Spec::Shell(r"C:\a.txt".into())),
            (act(ActionKind::App, "notepad.exe", None), Spec::Exe("notepad.exe".into())),
            (act(ActionKind::Command, "dir", None), Spec::Monogram),
            (act(ActionKind::Web, "https://music.youtube.com/", None), fav("https://music.youtube.com", "music.youtube.com")),
        ];
        for (a, want) in cases {
            assert_eq!(spec(&a), want, "{a:?}");
        }
    }

    #[test]
    fn origin_rejects_odd_hosts() {
        assert_eq!(origin("https://user:pw@example.com:8443/a"), Some(("https://example.com:8443".into(), "example.com_8443".into())));
        for bad in ["ftp://x.cz", "https://", "https://a b/", "https://..\\x/", "mapy.cz"] {
            assert_eq!(origin(bad), None, "{bad}");
        }
    }

    #[test]
    fn candidates_order_and_resolution() {
        let links = vec![
            ("icon".to_string(), "/favicon-32.png".to_string()),
            ("apple-touch-icon".to_string(), "img/apple.png?v=2".to_string()),
            ("icon".to_string(), "/logo.svg".to_string()),
            ("manifest".to_string(), "/site.webmanifest".to_string()),
            ("icon".to_string(), "//cdn.example.com/f.ico".to_string()),
            ("icon".to_string(), "data:image/png;base64,xx".to_string()),
        ];
        assert_eq!(
            candidates("https://x.cz", &links),
            vec!["https://x.cz/img/apple.png?v=2", "https://x.cz/favicon-32.png", "https://cdn.example.com/f.ico", "https://x.cz/favicon.ico"]
        );
        assert_eq!(candidates("https://x.cz", &[]), vec!["https://x.cz/favicon.ico"]);
        assert_eq!(join_href("https://x.cz", "javascript:alert(1)"), None);
    }

    #[test]
    fn disk_cache_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let d = dir.path().join("icons");
        assert_eq!(cached(&d, "x.cz"), None);
        store(&d, "x.cz", None);
        assert_eq!(cached(&d, "x.cz"), Some(None));
        store(&d, "x.cz", Some(b"png"));
        assert_eq!(cached(&d, "x.cz"), Some(Some(b"png".to_vec())));
        forget(&d, "x.cz");
        assert_eq!(cached(&d, "x.cz"), None);
    }

    #[test]
    fn local_icons_from_disk() {
        let dir = tempfile::tempdir().unwrap();
        let svg = dir.path().join("i.svg");
        std::fs::write(&svg, "<svg/>").unwrap();
        assert_eq!(local(&Spec::Image(svg)).unwrap().0, "image/svg+xml");
        let png = dir.path().join("i.png");
        std::fs::write(&png, iconimg::png_from_bgra(2, 2, &[1, 2, 3, 255].repeat(4))).unwrap();
        assert_eq!(local(&Spec::Image(png)).unwrap().0, "image/png");
        assert!(local(&Spec::Image(dir.path().join("chybi.png"))).is_none());
        assert!(local(&Spec::Shell(dir.path().to_path_buf())).is_some());
        assert!(local(&Spec::Exe("explorer.exe".into())).is_some());
        assert!(local(&Spec::Monogram).is_none());
    }
}
