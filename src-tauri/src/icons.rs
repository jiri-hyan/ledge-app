//! Logo dlaždice projektu. Pořadí: `logo` z configu (když soubor existuje) → u nainstalované appky
//! ikona z exe → ikony, které deklaruje web projektu (`<link rel=icon>`, web manifest) a mělký seznam
//! obvyklých souborů (`PATTERNS`), seřazené SVG > větší rastr > deklarované. Rastry převádí `iconimg`
//! na PNG ≤ 128 px. Čte disk/registr/COM → jen vlákno `logos`; výsledky drží [`Store`] (lib.rs).

use crate::config::{Project, ProjectKind};
use crate::iconimg;
use regex::Regex;
use std::collections::HashMap;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::LazyLock;
use std::time::{Duration, Instant};

/// Strop jednoho loga (data URL jde přes IPC).
pub const LIMIT: usize = 512 * 1024;
/// Při otevření šuplíku se loga přepočítají nejdřív po téhle době (nové soubory v repu).
pub const STALE: Duration = Duration::from_secs(300);
/// Větší soubor se ani nečte (ochrana před omylem nasměrovaným `logo`).
const MAX_READ: u64 = 16 * 1024 * 1024;

/// Obvyklá umístění ikon vůči kořeni webu (jedna `*` jen ve jménu souboru, bez rozlišení velikosti).
const PATTERNS: &[&str] = &[
    "public/favicon.svg",
    "public/favicon.png",
    "public/favicon.ico",
    "public/favicon_io/*.png",
    "public/favicon_io/*.ico",
    "public/icon*.svg",
    "public/icon*.png",
    "public/logo*.svg",
    "public/logo*.png",
    "build/icon.svg",
    "build/icon.png",
    "build/icon.ico",
    "src-tauri/icons/icon.png",
    "src-tauri/icons/128x128.png",
    "src-tauri/icons/128x128@2x.png",
    "assets/icon*.png",
    "assets/icon*.svg",
    "resources/icon.png",
    "app/favicon.ico",
    "src/app/favicon.ico",
    "static/favicon.*",
];
/// Kde hledat `index.html` vůči kořeni webu.
const HTML: &[&str] = &["index.html", "public/index.html", "src/index.html", "src/renderer/index.html"];
const MANIFESTS: &[&str] = &["public/manifest.json", "public/site.webmanifest", "public/manifest.webmanifest", "manifest.webmanifest"];
/// Výchozí obrázky šablon (Vite, React, Tauri…) — nejsou logem projektu.
const TEMPLATE_ASSETS: &[&str] = &["vite.svg", "react.svg", "tauri.svg", "vue.svg", "svelte.svg", "next.svg", "vercel.svg", "electron.svg"];
/// Podsložky, které nejsou kořenem webu (a nesmí se procházet).
const SKIP_DIRS: &[&str] = &["node_modules", "target", "dist", "out", "build", "coverage", "docs", ".git", ".next", ".svelte-kit"];

static LINK_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?is)<link\b[^>]*>").unwrap());
static REL_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r#"(?i)\brel\s*=\s*["']([^"']*)["']"#).unwrap());
static HREF_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r#"(?i)\bhref\s*=\s*["']([^"']*)["']"#).unwrap());

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Logo {
    pub mime: &'static str,
    pub bytes: Vec<u8>,
}

impl Logo {
    pub fn data_url(&self) -> String {
        format!("data:{};base64,{}", self.mime, b64(&self.bytes))
    }
}

/// Kdy brát ikonu z exe.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExeIcon {
    /// nainstalovaná appka (`appName`) — její ikona je to pravé logo
    First,
    /// vlastní exe v repu — až když repo žádnou ikonu nemá
    Fallback,
    /// dev projekt nebo generické `electron.exe` (ikona Electronu není logo projektu)
    No,
}

pub fn exe_policy(p: &Project) -> ExeIcon {
    match (p.kind, &p.exe, &p.app_name) {
        (ProjectKind::App, None, Some(_)) => ExeIcon::First,
        (ProjectKind::App, Some(e), _) if !e.to_lowercase().replace('\\', "/").ends_with("electron.exe") => ExeIcon::Fallback,
        _ => ExeIcon::No,
    }
}

/// Vstup pro [`resolve`] — sestaví se pod zámky v lib.rs, počítá se bez nich.
#[derive(Debug, Clone)]
pub struct Source {
    pub dir: Option<PathBuf>,
    pub logo: Option<String>,
    /// vlastní exe (cesta už spojená se složkou projektu)
    pub exe: Option<PathBuf>,
    /// nainstalovaná appka — exe se dohledá v registru až na pozadí
    pub app_name: Option<String>,
    pub policy: ExeIcon,
}

pub fn resolve(src: &Source) -> Option<Logo> {
    let exe = || {
        let path = src.exe.clone().or_else(|| src.app_name.as_deref().and_then(crate::resolve::installed_app_exe))?;
        iconimg::exe_png(&path).map(|bytes| Logo { mime: "image/png", bytes })
    };
    let configured = src.logo.as_deref().and_then(|rel| {
        let p = Path::new(rel);
        load(&if p.is_absolute() { p.to_path_buf() } else { src.dir.as_ref()?.join(p) })
    });
    configured
        .or_else(|| (src.policy == ExeIcon::First).then(exe).flatten())
        .or_else(|| src.dir.as_deref().and_then(|d| candidates(d).iter().find_map(|p| load(p))))
        .or_else(|| (src.policy == ExeIcon::Fallback).then(exe).flatten())
}

/// Soubor → logo: SVG beze změny, rastr přes WIC na PNG ≤ 128 px (malé PNG beze změny).
pub fn load(path: &Path) -> Option<Logo> {
    let meta = std::fs::metadata(path).ok()?;
    if !meta.is_file() || meta.len() > MAX_READ {
        return None;
    }
    let bytes = std::fs::read(path).ok()?;
    if ext(path) == "svg" {
        let head = String::from_utf8_lossy(&bytes[..bytes.len().min(4096)]).to_lowercase();
        return (bytes.len() <= LIMIT && head.contains("<svg")).then_some(Logo { mime: "image/svg+xml", bytes });
    }
    let small_png = bytes.starts_with(PNG_SIG)
        && bytes.len() <= LIMIT
        && raster_size(&bytes).is_some_and(|(w, h)| w <= iconimg::MAX_PX && h <= iconimg::MAX_PX);
    let png = if small_png { bytes } else { iconimg::to_png(&bytes)? };
    (png.len() <= LIMIT).then_some(Logo { mime: "image/png", bytes: png })
}

fn ext(p: &Path) -> String {
    p.extension().map(|e| e.to_string_lossy().to_lowercase()).unwrap_or_default()
}

const PNG_SIG: &[u8] = b"\x89PNG\r\n\x1a\n";

/// Rozměr z hlavičky PNG nebo největší položky ICO (bez dekódování).
pub fn raster_size(b: &[u8]) -> Option<(u32, u32)> {
    let be = |i: usize| u32::from_be_bytes(b[i..i + 4].try_into().unwrap());
    if b.len() >= 24 && b.starts_with(PNG_SIG) {
        return Some((be(16), be(20)));
    }
    if b.len() >= 6 && b[..4] == [0, 0, 1, 0] {
        let n = u16::from_le_bytes([b[4], b[5]]) as usize;
        let px = |v: u8| if v == 0 { 256 } else { v as u32 };
        return (0..n)
            .map(|i| 6 + 16 * i)
            .take_while(|&o| o + 16 <= b.len())
            .map(|o| (px(b[o]), px(b[o + 1])))
            .max_by_key(|&(w, h)| w * h);
    }
    None
}

/// Pořadí kandidáta: SVG, pak velikost (do 128 px; protáhlé obrázky = nápis, ne ikona), pak deklarované.
fn rank(path: &Path, declared: bool) -> (bool, u32, bool) {
    if ext(path) == "svg" {
        return (true, 0, declared);
    }
    let mut head = Vec::new();
    let _ = std::fs::File::open(path).and_then(|f| f.take(6 + 16 * 64).read_to_end(&mut head));
    let px = raster_size(&head).map_or(16, |(w, h)| {
        let (lo, hi) = (w.min(h), w.max(h));
        let px = lo.min(iconimg::MAX_PX);
        if hi > lo + lo / 3 { px / 4 } else { px }
    });
    (false, px, declared)
}

/// Kandidáti v repu, nejlepší první. Mělce: kořen, jeho přímé podsložky s webem, `apps/*`, `packages/*`.
pub fn candidates(dir: &Path) -> Vec<PathBuf> {
    let mut found: Vec<(PathBuf, bool)> = Vec::new();
    let mut push = |p: PathBuf, declared: bool| {
        let name = p.file_name().map(|n| n.to_string_lossy().to_lowercase()).unwrap_or_default();
        if !TEMPLATE_ASSETS.contains(&name.as_str()) && p.is_file() && !found.iter().any(|(q, _)| same(q, &p)) {
            found.push((p, declared));
        }
    };
    for root in web_roots(dir) {
        for p in declared(&root) {
            push(p, true);
        }
        for pat in PATTERNS {
            for p in glob1(&root, pat) {
                push(p, false);
            }
        }
    }
    let mut ranked: Vec<_> = found.into_iter().enumerate().map(|(i, (p, d))| (rank(&p, d), i, p)).collect();
    // vyšší rank první, při shodě dřívější nález
    ranked.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)));
    ranked.into_iter().map(|(_, _, p)| p).collect()
}

fn same(a: &Path, b: &Path) -> bool {
    a.to_string_lossy().to_lowercase().replace('/', "\\") == b.to_string_lossy().to_lowercase().replace('/', "\\")
}

fn web_roots(dir: &Path) -> Vec<PathBuf> {
    let mut roots = vec![dir.to_path_buf()];
    let is_web = |p: &Path| p.join("public").is_dir() || p.join("index.html").is_file();
    for base in [dir.to_path_buf(), dir.join("apps"), dir.join("packages")] {
        let Ok(rd) = std::fs::read_dir(&base) else { continue };
        for e in rd.flatten() {
            let name = e.file_name().to_string_lossy().to_lowercase();
            let p = e.path();
            if roots.len() < 24 && !name.starts_with('.') && !SKIP_DIRS.contains(&name.as_str()) && p.is_dir() && is_web(&p) {
                roots.push(p);
            }
        }
    }
    roots
}

/// Ikony z `<link rel="icon|apple-touch-icon">` v index.html a z web manifestů (i odkázaných z HTML).
fn declared(root: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let mut manifests: Vec<PathBuf> = MANIFESTS.iter().map(|m| root.join(m)).collect();
    for h in HTML {
        let html = root.join(h);
        let Ok(text) = std::fs::read_to_string(&html) else { continue };
        let base = html.parent().unwrap_or(root);
        for (rel, href) in icon_links(&text) {
            let hits = resolve_href(root, base, &href);
            if rel.split_whitespace().any(|r| r == "manifest") {
                manifests.extend(hits);
            } else {
                out.extend(hits);
            }
        }
    }
    for m in manifests {
        let Ok(text) = std::fs::read_to_string(&m) else { continue };
        let base = m.parent().unwrap_or(root);
        for src in manifest_icons(&text) {
            out.extend(resolve_href(root, base, &src));
        }
    }
    out
}

/// `(rel, href)` odkazů na ikonu nebo manifest (bez `mask-icon` — jednobarevná šablona Safari).
pub fn icon_links(html: &str) -> Vec<(String, String)> {
    LINK_RE
        .find_iter(html)
        .filter_map(|m| {
            let tag = m.as_str();
            let rel = REL_RE.captures(tag)?[1].to_lowercase();
            let href = HREF_RE.captures(tag)?[1].trim().to_string();
            let wanted = rel.split_whitespace().any(|r| matches!(r, "icon" | "apple-touch-icon" | "manifest"));
            (wanted && !href.is_empty()).then_some((rel, href))
        })
        .collect()
}

pub fn manifest_icons(json: &str) -> Vec<String> {
    let v: serde_json::Value = serde_json::from_str(json).unwrap_or_default();
    v.get("icons")
        .and_then(|i| i.as_array())
        .map(|a| a.iter().filter_map(|i| i.get("src")?.as_str().map(str::to_string)).collect())
        .unwrap_or_default()
}

/// Cesty, kde může `href` ležet: `/x` = kořen webu (u Vite složka `public`), relativní = vedle
/// dokumentu nebo v `public`. Externí URL a data: nic.
pub fn resolve_href(root: &Path, doc_dir: &Path, href: &str) -> Vec<PathBuf> {
    let href = href.split(['?', '#']).next().unwrap_or("");
    let lower = href.to_lowercase();
    if href.is_empty() || lower.starts_with("data:") || lower.starts_with("//") || lower.contains("://") || href.contains("..") {
        return Vec::new();
    }
    let rel = href.trim_start_matches("./");
    match rel.strip_prefix('/') {
        Some(abs) => vec![root.join("public").join(abs), root.join(abs)],
        None => vec![doc_dir.join(rel), root.join("public").join(rel)],
    }
}

/// `dir/pre*post` (jedna hvězdička ve jménu, bez ohledu na velikost písmen) nebo přesná cesta.
fn glob1(root: &Path, pat: &str) -> Vec<PathBuf> {
    let (parent, name) = pat.rsplit_once('/').unwrap_or(("", pat));
    let Some((pre, post)) = name.split_once('*') else { return vec![root.join(pat)] };
    let (pre, post) = (pre.to_lowercase(), post.to_lowercase());
    let Ok(rd) = std::fs::read_dir(root.join(parent)) else { return Vec::new() };
    let mut hits: Vec<PathBuf> = rd
        .flatten()
        .filter(|e| {
            let n = e.file_name().to_string_lossy().to_lowercase();
            n.len() >= pre.len() + post.len() && n.starts_with(&pre) && n.ends_with(&post)
        })
        .map(|e| e.path())
        .collect();
    hits.sort();
    hits
}

pub fn b64(data: &[u8]) -> String {
    const T: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(data.len().div_ceil(3) * 4);
    for c in data.chunks(3) {
        let n = (c[0] as u32) << 16 | (*c.get(1).unwrap_or(&0) as u32) << 8 | *c.get(2).unwrap_or(&0) as u32;
        out.push(T[(n >> 18) as usize & 63] as char);
        out.push(T[(n >> 12) as usize & 63] as char);
        out.push(if c.len() > 1 { T[(n >> 6) as usize & 63] as char } else { '=' });
        out.push(if c.len() > 2 { T[n as usize & 63] as char } else { '=' });
    }
    out
}

/// Hotová loga: id projektu → (klíč obsahu, data URL). Snapshot posílá jen klíč, data URL si UI
/// vyžádá jednou přes `project_logo` — base64 tak nejde s každým stavem (~8×/s při otevřeném šuplíku).
#[derive(Default)]
pub struct Store {
    generation: u64,
    running: Option<u64>,
    done_at: Option<Instant>,
    logos: HashMap<String, (String, String)>,
}

impl Store {
    /// Zahájí výpočet → jeho generace. `force` (config, repo index) vždy; jinak jen když nic neběží
    /// a výsledek je starší než [`STALE`].
    pub fn begin(&mut self, force: bool, now: Instant) -> Option<u64> {
        let fresh = self.done_at.is_some_and(|t| now.duration_since(t) < STALE);
        if !force && (self.running.is_some() || fresh) {
            return None;
        }
        self.generation += 1;
        self.running = Some(self.generation);
        Some(self.generation)
    }

    /// Uloží výsledek generace `generation` (starší výpočet novější nepřepíše) → změnilo se něco?
    pub fn finish(&mut self, generation: u64, results: Vec<(String, Option<Logo>)>, now: Instant) -> bool {
        if generation != self.generation {
            return false;
        }
        self.running = None;
        self.done_at = Some(now);
        let logos: HashMap<String, (String, String)> = results
            .into_iter()
            .filter_map(|(id, l)| {
                let l = l?;
                Some((id, (format!("{:016x}", fnv1a(&l.bytes)), l.data_url())))
            })
            .collect();
        let changed = logos.len() != self.logos.len() || logos.iter().any(|(id, (k, _))| self.logos.get(id).map(|e| &e.0) != Some(k));
        self.logos = logos;
        changed
    }

    pub fn key(&self, id: &str) -> Option<String> {
        self.logos.get(id).map(|e| e.0.clone())
    }

    pub fn url(&self, id: &str) -> Option<String> {
        self.logos.get(id).map(|e| e.1.clone())
    }
}

pub(crate) fn fnv1a(b: &[u8]) -> u64 {
    b.iter().fold(0xcbf2_9ce4_8422_2325, |h, &x| (h ^ x as u64).wrapping_mul(0x0000_0100_0000_01b3))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::iconimg::png_from_bgra;

    fn png(w: u32, h: u32) -> Vec<u8> {
        png_from_bgra(w, h, &[10, 20, 30, 255].repeat((w * h) as usize))
    }

    fn put(root: &Path, rel: &str, bytes: &[u8]) {
        let p = root.join(rel);
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(p, bytes).unwrap();
    }

    fn names(dir: &Path) -> Vec<String> {
        candidates(dir).iter().map(|p| p.strip_prefix(dir).unwrap().to_string_lossy().replace('\\', "/")).collect()
    }

    const SVG: &[u8] = b"<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 1 1'/>";

    #[test]
    fn svg_beats_png_and_bigger_png_beats_smaller() {
        let t = tempfile::tempdir().unwrap();
        put(t.path(), "public/favicon_io/favicon-32x32.png", &png(32, 32));
        put(t.path(), "src-tauri/icons/128x128.png", &png(128, 128));
        put(t.path(), "public/logo-wide.png", &png(400, 100));
        assert_eq!(names(t.path())[0], "src-tauri/icons/128x128.png");
        assert_eq!(names(t.path()).last().unwrap(), "public/logo-wide.png", "nápis (široký) je poslední");
        put(t.path(), "build/icon.svg", SVG);
        assert_eq!(names(t.path())[0], "build/icon.svg");
    }

    /// Atlas: config ukazuje na smazaný PNG, deklarovaný favicon.ico v podsložce existuje.
    #[test]
    fn missing_configured_logo_falls_back_to_declared_ico() {
        let t = tempfile::tempdir().unwrap();
        put(t.path(), "index.html", br#"<link rel="icon" type="image/x-icon" href="/favicon_io/favicon.ico" /><link rel="icon" href="/favicon_io/favicon-32x32.png"><link rel="manifest" href="/favicon_io/site.webmanifest" />"#);
        put(t.path(), "public/favicon_io/site.webmanifest", br#"{"icons":[{"src":"/favicon_io/android-chrome-192x192.png"}]}"#);
        let mut ico = vec![0, 0, 1, 0, 1, 0, 48, 48, 0, 0, 1, 0, 32, 0];
        let data = png(48, 48);
        ico.extend((data.len() as u32).to_le_bytes());
        ico.extend(22u32.to_le_bytes());
        ico.extend(&data);
        put(t.path(), "public/favicon_io/favicon.ico", &ico);
        put(t.path(), "public/vite.svg", SVG);
        let src = Source { dir: Some(t.path().into()), logo: Some("public/favicon_io/android-chrome-192x192.png".into()), exe: None, app_name: None, policy: ExeIcon::No };
        let logo = resolve(&src).unwrap();
        assert_eq!(logo.mime, "image/png");
        assert_eq!(raster_size(&logo.bytes), Some((48, 48)));
    }

    /// Pixel Press: web v podsložce `Code/`, ikona z `<link rel=icon href="./images/…">`, obří PNG zmenšený.
    #[test]
    fn web_root_in_subfolder_and_big_png_shrinks() {
        let t = tempfile::tempdir().unwrap();
        put(t.path(), "Code/public/index.html", br#"<link rel="icon" href="./images/mascot.png" />"#);
        put(t.path(), "Code/public/images/mascot.png", &png(512, 512));
        put(t.path(), "node_modules/x/public/favicon.svg", SVG);
        assert_eq!(names(t.path()), vec!["Code/public/images/mascot.png"]);
        let logo = resolve(&Source { dir: Some(t.path().into()), logo: None, exe: None, app_name: None, policy: ExeIcon::No }).unwrap();
        assert_eq!(raster_size(&logo.bytes), Some((128, 128)));
    }

    #[test]
    fn configured_logo_wins_and_template_assets_are_ignored() {
        let t = tempfile::tempdir().unwrap();
        put(t.path(), "index.html", br#"<link rel="icon" href="/vite.svg">"#);
        put(t.path(), "public/vite.svg", SVG);
        assert!(names(t.path()).is_empty());
        put(t.path(), "art/mine.svg", SVG);
        put(t.path(), "public/favicon.svg", SVG);
        let src = Source { dir: Some(t.path().into()), logo: Some("art/mine.svg".into()), exe: None, app_name: None, policy: ExeIcon::No };
        assert_eq!(resolve(&src).unwrap().bytes, SVG);
        // nenaklonovaný projekt bez exe → nic (monogram)
        assert!(resolve(&Source { dir: None, logo: Some("x.svg".into()), exe: None, app_name: None, policy: ExeIcon::First }).is_none());
    }

    #[test]
    fn installed_app_icon_comes_from_exe() {
        let t = tempfile::tempdir().unwrap();
        put(t.path(), "public/favicon.svg", SVG);
        let exe = Some(PathBuf::from(r"C:\Windows\explorer.exe"));
        let first = resolve(&Source { dir: Some(t.path().into()), logo: None, exe: exe.clone(), app_name: None, policy: ExeIcon::First }).unwrap();
        assert_eq!(first.mime, "image/png");
        let fallback = resolve(&Source { dir: Some(t.path().into()), logo: None, exe, app_name: None, policy: ExeIcon::Fallback }).unwrap();
        assert_eq!(fallback.mime, "image/svg+xml", "repo má ikonu → exe až po ní");
    }

    #[test]
    fn exe_policy_table() {
        let mut p = crate::config::Project {
            id: "acme-studio".into(),
            name: "Acme Studio".into(),
            enabled: true,
            kind: ProjectKind::App,
            path: None,
            remote: Some("acme/studio".into()),
            subdir: None,
            run: None,
            port: None,
            pass_port: false,
            open_browser: false,
            exe: None,
            app_name: Some("Acme Studio".into()),
            aumid: None,
            args: vec![],
            accent: None,
            monogram: None,
            logo: None,
            links: vec![],
            look: None,
            mute_dirty: false,
        };
        assert_eq!(exe_policy(&p), ExeIcon::First);
        p.exe = Some("node_modules/electron/dist/electron.exe".into());
        assert_eq!(exe_policy(&p), ExeIcon::No);
        p.exe = Some(r"target\release\studio.exe".into());
        assert_eq!(exe_policy(&p), ExeIcon::Fallback);
        p.kind = ProjectKind::Dev;
        assert_eq!(exe_policy(&p), ExeIcon::No);
    }

    #[test]
    fn hrefs_and_links_parse() {
        let r = Path::new(r"C:\r");
        let d = Path::new(r"C:\r\Code");
        assert_eq!(resolve_href(r, d, "/a/b.png?v=2"), vec![r.join("public").join("a/b.png"), r.join("a/b.png")]);
        assert_eq!(resolve_href(r, d, "./i.svg"), vec![d.join("i.svg"), r.join("public").join("i.svg")]);
        for bad in ["https://x/y.png", "//cdn/y.png", "data:image/png;base64,AA", "../up.png", ""] {
            assert!(resolve_href(r, d, bad).is_empty(), "{bad}");
        }
        let html = r#"<LINK href='a.ico' REL='shortcut icon'><link rel="mask-icon" href="m.svg"><link rel="stylesheet" href="s.css"><link rel="apple-touch-icon" href="t.png">"#;
        let hrefs: Vec<String> = icon_links(html).into_iter().map(|(_, h)| h).collect();
        assert_eq!(hrefs, vec!["a.ico", "t.png"]);
        assert_eq!(manifest_icons(r#"{"icons":[{"src":"/x.png"},{"sizes":"1"}]}"#), vec!["/x.png"]);
        assert!(manifest_icons("nejson").is_empty());
    }

    #[test]
    fn big_svg_and_non_svg_are_rejected() {
        let t = tempfile::tempdir().unwrap();
        put(t.path(), "a.svg", b"not svg");
        put(t.path(), "b.svg", &[b"<svg>".as_slice(), &vec![b' '; LIMIT]].concat());
        assert!(load(&t.path().join("a.svg")).is_none() && load(&t.path().join("b.svg")).is_none());
    }

    #[test]
    fn store_generations() {
        let t0 = Instant::now();
        let mut s = Store::default();
        let logo = || Some(Logo { mime: "image/png", bytes: vec![1, 2, 3] });
        let g1 = s.begin(false, t0).unwrap();
        assert!(s.begin(false, t0).is_none(), "běží → druhý nezačne");
        let g2 = s.begin(true, t0).unwrap();
        assert!(!s.finish(g1, vec![("a".into(), logo())], t0), "starší výpočet se zahodí");
        assert!(s.key("a").is_none());
        assert!(s.finish(g2, vec![("a".into(), logo()), ("b".into(), None)], t0));
        assert_eq!(s.url("a").unwrap(), "data:image/png;base64,AQID");
        assert!(s.key("b").is_none());
        assert!(s.begin(false, t0 + Duration::from_secs(10)).is_none(), "čerstvé");
        let g3 = s.begin(false, t0 + STALE).unwrap();
        assert!(!s.finish(g3, vec![("a".into(), logo())], t0 + STALE), "stejný obsah = beze změny");
    }

    /// Měření nad skutečnými repy (jen čtení): `LEDGE_BENCH_ROOT=C:\Users\…\Desktop cargo test
    /// measure_real_logos -- --ignored --nocapture`. Vypíše zdroj, velikost data URL a čas na projekt.
    #[test]
    #[ignore]
    fn measure_real_logos() {
        let root = std::env::var("LEDGE_BENCH_ROOT").expect("LEDGE_BENCH_ROOT");
        let repos = crate::onboarding::repos::scan(&[PathBuf::from(root)], Instant::now() + Duration::from_secs(30));
        let mut total = 0;
        for d in repos {
            let id = d.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
            let dir = Some(d);
            let src = Source { dir: dir.clone(), logo: None, exe: None, app_name: None, policy: ExeIcon::No };
            let t = Instant::now();
            let first = dir.as_deref().and_then(|d| candidates(d).into_iter().next());
            let logo = resolve(&src);
            let len = logo.as_ref().map_or(0, |l| l.data_url().len());
            total += len;
            println!("{:<16} {:>7} B  {:>6.1} ms  {:?}", id, len, t.elapsed().as_secs_f64() * 1e3, first.or(dir));
        }
        println!("celkem data URL: {total} B");
    }

    #[test]
    fn b64_matches_reference() {
        for (i, o) in [("", ""), ("f", "Zg=="), ("fo", "Zm8="), ("foo", "Zm9v"), ("foobar", "Zm9vYmFy")] {
            assert_eq!(b64(i.as_bytes()), o);
        }
    }
}
