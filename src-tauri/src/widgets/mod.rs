//! Widgety ze skriptů (jako xbar / SwiftBar / Übersicht): soubor ve složce widgetů (`widgets.dir`,
//! výchozí `%USERPROFILE%\Ledge\widgets`) s hlavičkou `@ledge.widget` vypíše na stdout JSON nebo text
//! à la xbar → Ledge z toho vykreslí kartu (`widget:<soubor>` v katalogu karet) a/nebo položku proužku.
//!
//! - `meta.rs` hlavička, `output.rs` parser výstupu, `plan.rs` plánovač (čisté, tabulkové testy)
//! - `consent.rs` souhlas (otisk obsahu jako Script Commands, jeden na soubor)
//! - `run.rs` spuštění bez okna s termínem a stropem výstupu (Job Object zabije celý strom)
//! - `live.rs` stav v paměti, vlákno `widgets` (spí, dokud nic není vidět), pohled pro UI a proužek
//! - `templates.rs` galerie vestavěných příkladů (kopie do složky na klik)
//!
//! Sken složky bez watcheru (jako `scripts`): výpis + čtení jen změněných souborů (mtime + velikost),
//! strop `MAX_WIDGETS`. Program (`.exe`) má hlavičku v `<jméno>.exe.ledge` vedle sebe.

pub mod consent;
pub mod live;
pub mod meta;
pub mod output;
pub mod plan;
pub mod run;
pub mod templates;

use crate::i18n::tr;
use crate::scripts::meta as smeta;
use meta::{Kind, WidgetMeta};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

/// Nejvýš widgetů ve složce (podle jména); zbytek se jen spočítá.
pub const MAX_WIDGETS: usize = 50;
/// Větší skript se nehashuje ani nespustí.
pub const MAX_FILE: u64 = 1024 * 1024;
/// Větší program se nehashuje ani nespustí.
pub const MAX_EXE: u64 = 64 * 1024 * 1024;
const MAX_SIDECAR: u64 = 16 * 1024;
/// Ukázka začátku skriptu v potvrzení.
const PREVIEW_CHARS: usize = 1500;
const PREVIEW_LINES: usize = 30;

fn yes() -> bool {
    true
}

/// Sekce `widgets` v configu.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(default, rename_all = "camelCase")]
pub struct WidgetsConfig {
    /// Widgety ze skriptů (výchozí true — ale nic neběží, dokud widget nepřidáš na záložku nebo do
    /// proužku a nepovolíš ho).
    #[serde(default = "yes")]
    pub enabled: bool,
    /// Složka widgetů; chybí / prázdné = `%USERPROFILE%\Ledge\widgets`. `%PROMĚNNÉ%` se rozbalí.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dir: Option<String>,
}

impl Default for WidgetsConfig {
    fn default() -> Self {
        Self { enabled: true, dir: None }
    }
}

/// `%USERPROFILE%\Ledge\widgets`.
pub fn default_dir() -> PathBuf {
    let home = std::env::var("USERPROFILE").unwrap_or_else(|_| ".".into());
    Path::new(&home).join("Ledge").join("widgets")
}

pub fn resolve_dir_with(dir: Option<&str>, var: impl Fn(&str) -> Option<String>) -> Result<PathBuf, String> {
    match dir.map(str::trim).filter(|d| !d.is_empty()) {
        None => Ok(default_dir()),
        Some(d) => crate::scripts::resolve_dir_with(Some(d), var).map_err(|e| e.replace("scripts.dir", "widgets.dir")),
    }
}

pub fn resolve_dir(c: &WidgetsConfig) -> Result<PathBuf, String> {
    resolve_dir_with(c.dir.as_deref(), |k| std::env::var(k).ok())
}

/// Tvarová kontrola (`config::validate`).
pub fn validate(c: &WidgetsConfig) -> Result<(), String> {
    resolve_dir(c).map(|_| ())
}

// ---------- sken ----------

#[derive(Debug, Clone, PartialEq)]
pub struct Widget {
    /// `widget:<soubor>`
    pub id: String,
    pub file: String,
    pub path: PathBuf,
    pub kind: Kind,
    pub meta: Result<WidgetMeta, String>,
    /// SHA-256 obsahu (u programu i hlavičky); prázdné = nespočítáno
    pub sha: String,
    /// začátek skriptu (u programu jeho hlavička) pro potvrzení
    pub head: String,
}

impl Widget {
    pub fn fingerprint(&self) -> Option<String> {
        let m = self.meta.as_ref().ok()?;
        (!self.sha.is_empty()).then(|| consent::fingerprint(&self.path, &self.sha, self.kind, &m.args))
    }

    pub fn confirm_text(&self) -> String {
        let args = self.meta.as_ref().map(|m| m.args.as_slice()).unwrap_or(&[]);
        consent::confirm_text(self.kind, &self.path, args, &self.sha)
    }

    pub fn title(&self) -> String {
        self.meta.as_ref().map(|m| m.title.clone()).unwrap_or_else(|_| self.file.clone())
    }
}

/// `program.exe` → `program.exe.ledge` (hlavička programu).
pub fn sidecar_of(path: &Path) -> PathBuf {
    let mut s = path.as_os_str().to_owned();
    s.push(".ledge");
    PathBuf::from(s)
}

fn preview(text: &str) -> String {
    let mut out: String = text.lines().take(PREVIEW_LINES).collect::<Vec<_>>().join("\n");
    if out.chars().count() > PREVIEW_CHARS {
        out = format!("{}…", out.chars().take(PREVIEW_CHARS).collect::<String>());
    }
    out
}

/// SHA-256 souboru proudem (`head` = už přečtený začátek), nejvýš `max` bajtů.
fn hash_rest(f: &mut std::fs::File, head: &[u8], max: u64) -> Result<Option<String>, String> {
    use sha2::Digest;
    let mut h = sha2::Sha256::new();
    h.update(head);
    let mut total = head.len() as u64;
    let mut buf = [0u8; 16 * 1024];
    loop {
        let n = f.read(&mut buf).map_err(|e| e.to_string())?;
        if n == 0 {
            break;
        }
        total += n as u64;
        if total > max {
            return Ok(None);
        }
        h.update(&buf[..n]);
    }
    Ok(Some(crate::scripts::run::sha256_hex(h)))
}

/// Načtený widget: metadata (nebo proč jsou chybná), SHA-256 a ukázka začátku.
pub type Loaded = (Result<WidgetMeta, String>, String, String);

/// Přečte hlavičku a dohashuje soubor. `Ok(None)` = není widget (bez `@ledge.widget`, program bez
/// `.exe.ledge`).
pub fn load_one(path: &Path, kind: Kind, size: u64) -> Result<Option<Loaded>, String> {
    use sha2::Digest;
    let too_big = |max: &str| trf!("soubor je větší než {max}", "the file is larger than {max}");
    let Some(lang) = kind.lang() else {
        // program: hlavička vedle, obsah = program + hlavička
        let side = sidecar_of(path);
        let Ok(md) = std::fs::metadata(&side) else { return Ok(None) };
        if md.len() > MAX_SIDECAR {
            return Ok(Some((Err(tr("hlavička .exe.ledge je větší než 16 KB", "the .exe.ledge header is larger than 16 KB").into()), String::new(), String::new())));
        }
        let text = std::fs::read(&side).map_err(|e| e.to_string())?;
        let text = smeta::header_text(&text, false);
        let parsed = match meta::parse(&text, smeta::Lang::Python) {
            Ok(None) => return Ok(None),
            Ok(Some(m)) => Ok(m),
            Err(e) => Err(e),
        };
        if size > MAX_EXE {
            return Ok(Some((Err(too_big("64 MB")), String::new(), preview(&text))));
        }
        let mut f = std::fs::File::open(path).map_err(|e| e.to_string())?;
        let Some(exe) = hash_rest(&mut f, &[], MAX_EXE)? else { return Ok(Some((Err(too_big("64 MB")), String::new(), preview(&text)))) };
        let mut h = sha2::Sha256::new();
        h.update(exe.as_bytes());
        h.update([0u8]);
        h.update(text.as_bytes());
        return Ok(Some((parsed, crate::scripts::run::sha256_hex(h), preview(&text))));
    };
    let mut f = std::fs::File::open(path).map_err(|e| e.to_string())?;
    let mut head = Vec::with_capacity(smeta::HEADER_BYTES);
    (&mut f).take(smeta::HEADER_BYTES as u64).read_to_end(&mut head).map_err(|e| e.to_string())?;
    let text = smeta::header_text(&head, size > head.len() as u64);
    let parsed = match meta::parse(&text, lang) {
        Ok(None) => return Ok(None),
        Ok(Some(m)) => Ok(m),
        Err(e) => Err(e),
    };
    if size > MAX_FILE {
        return Ok(Some((Err(too_big("1 MB")), String::new(), preview(&text))));
    }
    match hash_rest(&mut f, &head, MAX_FILE)? {
        Some(sha) => Ok(Some((parsed, sha, preview(&text)))),
        None => Ok(Some((Err(too_big("1 MB")), String::new(), preview(&text)))),
    }
}

type Stamp = (Option<SystemTime>, u64, Option<(Option<SystemTime>, u64)>);

/// Výsledek skenu složky widgetů.
#[derive(Debug, Default, Clone)]
pub struct Scan {
    pub dir: PathBuf,
    pub exists: bool,
    pub widgets: Vec<Widget>,
    /// souborů nad `MAX_WIDGETS`
    pub skipped: usize,
    pub error: Option<String>,
    stamps: HashMap<String, (Stamp, Option<Widget>)>,
}

impl Scan {
    pub fn get(&self, id: &str) -> Option<&Widget> {
        self.widgets.iter().find(|w| w.id == id)
    }
}

fn stamp_of(path: &Path, md: &std::fs::Metadata, kind: Kind) -> Stamp {
    let side = (kind == Kind::Exe).then(|| std::fs::metadata(sidecar_of(path)).map(|m| (m.modified().ok(), m.len())).unwrap_or((None, 0)));
    (md.modified().ok(), md.len(), side)
}

fn widget(name: &str, path: &Path, kind: Kind, size: u64) -> Option<Widget> {
    let mk = |meta, sha, head| Widget { id: format!("{}{name}", plan::PREFIX), file: name.to_string(), path: path.to_path_buf(), kind, meta, sha, head };
    match load_one(path, kind, size) {
        Ok(None) => None,
        Ok(Some((m, sha, head))) => Some(mk(m, sha, head)),
        Err(e) => Some(mk(Err(e), String::new(), String::new())),
    }
}

/// Projde složku: jeden výpis, čte jen změněné soubory (proti `prev`). Bez rekurze a skrytých.
pub fn scan_dir(dir: &Path, prev: Option<&Scan>) -> Scan {
    let mut out = Scan { dir: dir.to_path_buf(), ..Default::default() };
    let rd = match std::fs::read_dir(dir) {
        Ok(rd) => rd,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return out,
        Err(e) => {
            out.exists = dir.is_dir();
            out.error = Some(e.to_string());
            return out;
        }
    };
    out.exists = true;
    let prev = prev.filter(|p| p.dir == dir);
    let mut found: Vec<(String, PathBuf, Kind, std::fs::Metadata)> = rd
        .flatten()
        .filter_map(|e| {
            let name = e.file_name().to_string_lossy().into_owned();
            let kind = Kind::from_ext(Path::new(&name).extension()?.to_str()?)?;
            let md = e.metadata().ok().filter(|m| m.is_file())?;
            (!name.starts_with('.')).then(|| (name, e.path(), kind, md))
        })
        .collect();
    found.sort_by_key(|(n, ..)| n.to_lowercase());
    out.skipped = found.len().saturating_sub(MAX_WIDGETS);
    found.truncate(MAX_WIDGETS);
    for (name, path, kind, md) in found {
        let st = stamp_of(&path, &md, kind);
        let cached = prev.and_then(|p| p.stamps.get(&name)).filter(|(s, _)| *s == st && st.0.is_some());
        let w = match cached {
            Some((_, w)) => w.clone(),
            None => widget(&name, &path, kind, md.len()),
        };
        if let Some(w) = &w {
            out.widgets.push(w.clone());
        }
        out.stamps.insert(name, (st, w));
    }
    out
}

/// Widget `id` čerstvě z disku (před během): soubor se přečte a zahashuje znovu, ne z cache.
pub fn fresh(dir: &Path, id: &str) -> Result<Widget, String> {
    let file = plan::file_of(id).ok_or_else(|| tr("Neznámý widget.", "Unknown widget."))?;
    let path = dir.join(file);
    let kind = Path::new(file).extension().and_then(|e| e.to_str()).and_then(Kind::from_ext).ok_or_else(|| tr("Neznámý widget.", "Unknown widget."))?;
    let md = std::fs::metadata(&path).ok().filter(|m| m.is_file()).ok_or_else(|| trf!("Widget {file} už ve složce není.", "The widget {file} is no longer in the folder."))?;
    widget(file, &path, kind, md.len()).ok_or_else(|| trf!("{file} už nemá hlavičku @ledge.widget.", "{file} no longer has an @ledge.widget header."))
}

/// Proč widget nejde spustit (hlavička, interpret, cesta), jinak `None`. `path_var` = PATH.
pub fn problem(w: &Widget, path_var: &std::ffi::OsStr) -> Option<String> {
    let m = match &w.meta {
        Err(e) => return Some(e.clone()),
        Ok(m) => m,
    };
    if w.sha.is_empty() {
        return Some(tr("soubor nejde přečíst", "can't read the file").into());
    }
    run::command(w, m, path_var).err()
}

#[cfg(test)]
mod tests;
