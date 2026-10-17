//! Script Commands (jako Raycast): složka skriptů (`scripts.dir`, výchozí `%USERPROFILE%\Ledge\scripts`)
//! → položky palety (a volitelně dlaždice karty Rychlé akce). Skript = `.ps1`, `.cmd`/`.bat`, `.py`,
//! `.js`/`.mjs` s hlavičkou `@ledge.*` (`meta.rs`); spuštění a souhlas v `run.rs` +
//! `commands/scripts.rs`. Načítání bez watcheru: `refresh` při otevření palety / šuplíku (s kartou)
//! / změně configu — jeden výpis složky, hlavička (prvních 4 KB) a SHA-256 jen u změněných souborů
//! (mtime + velikost), strop `MAX_SCRIPTS`. Snapshot čte jen paměť (`snapshot_views`).

pub mod meta;
pub mod run;

use crate::actionrun::{CONSENTS, RUNNING};
use crate::i18n::tr;
use meta::{Lang, Meta, Mode};
use parking_lot::Mutex;
use run::Interp;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::io::Read;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

/// Nejvíc skriptů (soubory s povolenou příponou, podle jména); zbytek se jen spočítá.
pub const MAX_SCRIPTS: usize = 200;
/// Větší soubor se nehashuje ani nespustí.
pub const MAX_FILE: u64 = 1024 * 1024;

fn yes() -> bool {
    true
}

/// Sekce `scripts` v configu.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(default, rename_all = "camelCase")]
pub struct ScriptsConfig {
    /// Skripty v paletě (výchozí true — ale nic se neukáže, dokud složka neexistuje).
    #[serde(default = "yes")]
    pub enabled: bool,
    /// Složka skriptů; chybí / prázdné = `%USERPROFILE%\Ledge\scripts`. `%PROMĚNNÉ%` se rozbalí.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dir: Option<String>,
    /// Skripty i jako dlaždice v kartě Rychlé akce (výchozí false = jen paleta).
    pub in_actions: bool,
}

impl Default for ScriptsConfig {
    fn default() -> Self {
        Self { enabled: true, dir: None, in_actions: false }
    }
}

/// `%USERPROFILE%\Ledge\scripts`.
pub fn default_dir() -> PathBuf {
    let home = std::env::var("USERPROFILE").unwrap_or_else(|_| ".".into());
    Path::new(&home).join("Ledge").join("scripts")
}

/// Složka z configu: prázdné = výchozí, `%VAR%` rozbalené, musí být absolutní (`C:\…`).
pub fn resolve_dir_with(dir: Option<&str>, var: impl Fn(&str) -> Option<String>) -> Result<PathBuf, String> {
    let d = dir.map(str::trim).unwrap_or("");
    if d.is_empty() {
        return Ok(default_dir());
    }
    let x = crate::notes::expand_env(d, var)?;
    let n = crate::personal::normalize(x.trim())?;
    let b = n.as_bytes();
    if b.len() >= 3 && b[0].is_ascii_alphabetic() && b[1] == b':' && (b[2] == b'\\' || b[2] == b'/') && !n.split(['\\', '/']).any(|s| s == "..") {
        Ok(PathBuf::from(n))
    } else {
        Err(tr("scripts.dir musí být absolutní cesta (C:\\…) bez ..", "scripts.dir must be an absolute path (C:\\…) without ..").into())
    }
}

pub fn resolve_dir(c: &ScriptsConfig) -> Result<PathBuf, String> {
    resolve_dir_with(c.dir.as_deref(), |k| std::env::var(k).ok())
}

/// Tvarová kontrola (`config::validate`).
pub fn validate(c: &ScriptsConfig) -> Result<(), String> {
    resolve_dir(c).map(|_| ())
}

// ---------- sken ----------

/// Jeden skript Ledge ve složce.
#[derive(Debug, Clone, PartialEq)]
pub struct Script {
    pub id: String,
    pub file: String,
    pub path: PathBuf,
    pub lang: Lang,
    pub meta: Result<Meta, String>,
    /// SHA-256 celého obsahu (hex); prázdné = nespočítáno (chyba, moc velký)
    pub sha: String,
}

impl Script {
    /// Otisk pro souhlas (jen platný skript).
    pub fn fingerprint(&self) -> Option<String> {
        let m = self.meta.as_ref().ok()?;
        (!self.sha.is_empty()).then(|| run::fingerprint(&self.path, &self.sha, m.mode, &m.args))
    }

    pub fn confirm_text(&self) -> String {
        let args = self.meta.as_ref().map(|m| m.args.as_slice()).unwrap_or(&[]);
        run::confirm_text(self.lang, &self.path, args, &self.sha)
    }
}

/// Výsledek skenu: skripty podle jména + co se přeskočilo. `stamps` = mtime a velikost každého
/// přečteného souboru (i ne-skriptů), aby se nezměněné nečetly znovu.
#[derive(Debug, Default, Clone)]
pub struct Scan {
    pub dir: PathBuf,
    pub exists: bool,
    pub scripts: Vec<Script>,
    /// souborů s povolenou příponou nad `MAX_SCRIPTS`
    pub skipped: usize,
    pub error: Option<String>,
    stamps: HashMap<String, (Option<SystemTime>, u64, Option<Script>)>,
}

/// Načtený skript: metadata (nebo proč jsou chybná) a SHA-256 obsahu (prázdné = nespočítáno).
pub type Loaded = (Result<Meta, String>, String);

/// Přečte hlavičku (prvních `HEADER_BYTES`) a — jen u skriptu Ledge — dohashuje zbytek souboru.
/// `Ok(None)` = soubor nemá `@ledge.` (pomocný modul).
pub fn load_one(path: &Path, lang: Lang, size: u64) -> Result<Option<Loaded>, String> {
    use sha2::Digest;
    let mut f = std::fs::File::open(path).map_err(|e| e.to_string())?;
    let mut head = Vec::with_capacity(meta::HEADER_BYTES);
    (&mut f).take(meta::HEADER_BYTES as u64).read_to_end(&mut head).map_err(|e| e.to_string())?;
    let parsed = match meta::parse(&meta::header_text(&head, size > head.len() as u64), lang) {
        Ok(None) => return Ok(None),
        Ok(Some(m)) => Ok(m),
        Err(e) => Err(e),
    };
    if size > MAX_FILE {
        return Ok(Some((Err(tr("soubor je větší než 1 MB", "the file is larger than 1 MB").into()), String::new())));
    }
    let mut h = sha2::Sha256::new();
    h.update(&head);
    let mut buf = [0u8; 16 * 1024];
    let mut total = head.len() as u64;
    loop {
        let n = f.read(&mut buf).map_err(|e| e.to_string())?;
        if n == 0 {
            break;
        }
        total += n as u64;
        if total > MAX_FILE {
            return Ok(Some((Err(tr("soubor je větší než 1 MB", "the file is larger than 1 MB").into()), String::new())));
        }
        h.update(&buf[..n]);
    }
    Ok(Some((parsed, run::sha256_hex(h))))
}

/// Projde složku: jeden výpis, stat z výpisu, čte jen soubory se změněným mtime / velikostí
/// (proti `prev`). Bez rekurze, bez skrytých souborů.
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
    let mut found: Vec<(String, PathBuf, Lang, Option<SystemTime>, u64)> = rd
        .flatten()
        .filter_map(|e| {
            let name = e.file_name().to_string_lossy().into_owned();
            let lang = Lang::from_ext(Path::new(&name).extension()?.to_str()?)?;
            let md = e.metadata().ok().filter(|m| m.is_file())?;
            (!name.starts_with('.')).then(|| (name, e.path(), lang, md.modified().ok(), md.len()))
        })
        .collect();
    found.sort_by_key(|(n, ..)| n.to_lowercase());
    out.skipped = found.len().saturating_sub(MAX_SCRIPTS);
    found.truncate(MAX_SCRIPTS);
    for (name, path, lang, mtime, size) in found {
        let cached = prev.and_then(|p| p.stamps.get(&name)).filter(|(m, s, _)| *m == mtime && *s == size && mtime.is_some());
        let script = match cached {
            Some((_, _, s)) => s.clone(),
            None => match load_one(&path, lang, size) {
                Ok(None) => None,
                Ok(Some((meta, sha))) => Some(Script { id: format!("script:{name}"), file: name.clone(), path: path.clone(), lang, meta, sha }),
                Err(e) => Some(Script { id: format!("script:{name}"), file: name.clone(), path: path.clone(), lang, meta: Err(e), sha: String::new() }),
            },
        };
        if let Some(s) = &script {
            out.scripts.push(s.clone());
        }
        out.stamps.insert(name, (mtime, size, script));
    }
    out
}

// ---------- pro UI ----------

/// Skript pro UI — stejný tvar jako `ActionView` (dlaždice, paleta, potvrzení) + pár polí navíc.
#[derive(Serialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ScriptView {
    id: String,
    label: String,
    kind: &'static str,
    /// jméno souboru (podtitulek v paletě)
    target: String,
    group: Option<String>,
    show_on: &'static str,
    icon_key: Option<String>,
    monogram: String,
    color: &'static str,
    needs_confirm: bool,
    confirm_text: String,
    /// proč se ptá (poprvé / změna souboru / `@ledge.confirm`)
    confirm_note: Option<String>,
    problem: Option<String>,
    running: bool,
    script: bool,
    description: Option<String>,
    mode: Mode,
    lang: Lang,
}

/// Interprety pro jazyky v seznamu (PATH se čte jednou na sken).
pub fn interps(scripts: &[Script], path_var: &std::ffi::OsStr) -> HashMap<Lang, Result<Interp, String>> {
    let langs: HashSet<Lang> = scripts.iter().map(|s| s.lang).collect();
    langs.into_iter().map(|l| (l, run::interp(l, path_var))).collect()
}

/// Proč skript nejde spustit (hlavička, interpret, cesta), jinak `None`.
pub fn problem(s: &Script, interp: Option<&Result<Interp, String>>) -> Option<String> {
    let m = match &s.meta {
        Err(e) => return Some(e.clone()),
        Ok(m) => m,
    };
    match interp {
        Some(Err(e)) => Some(e.clone()),
        Some(Ok(i)) => run::plan(&s.path, i, &m.args, m.mode == Mode::Terminal).err(),
        None => Some(tr("interpret nejde zjistit", "can't find the interpreter").into()),
    }
}

pub fn view(s: &Script, interp: Option<&Result<Interp, String>>, consented: bool, running: bool) -> ScriptView {
    let (label, description, icon, group, confirm, mode) = match &s.meta {
        Ok(m) => (m.title.clone(), m.description.clone(), m.icon.clone(), m.group.clone(), m.confirm, m.mode),
        Err(_) => (s.file.clone(), None, None, None, false, Mode::Output),
    };
    let note = if confirm {
        tr("Skript se ptá před každým spuštěním (@ledge.confirm).", "The script asks before every run (@ledge.confirm).")
    } else {
        tr("Skript je cizí kód: spusť ho, jen když víš, co dělá. Potvrzení platí pro tenhle obsah — po změně souboru se Ledge zeptá znovu.", "A script is foreign code: run it only if you know what it does. The confirmation covers this content — after the file changes, Ledge asks again.")
    };
    let needs = confirm || !consented;
    ScriptView {
        id: s.id.clone(),
        monogram: icon.unwrap_or_else(|| crate::snapshot::monogram(&label)),
        color: crate::actions::monogram_color(&label),
        label,
        kind: "command",
        target: s.file.clone(),
        group: Some(group.unwrap_or_else(|| tr("Skripty", "Scripts").into())),
        show_on: "both",
        icon_key: None,
        needs_confirm: needs,
        confirm_text: s.confirm_text(),
        confirm_note: needs.then(|| note.to_string()),
        problem: problem(s, interp),
        running,
        script: true,
        description,
        mode,
        lang: s.lang,
    }
}

struct State {
    scan: Scan,
    interps: HashMap<Lang, Result<Interp, String>>,
    last: Vec<ScriptView>,
}

static STATE: Mutex<Option<State>> = Mutex::new(None);

/// Co ví paleta a Nastavení.
#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Listing {
    pub enabled: bool,
    pub dir: String,
    pub exists: bool,
    pub scripts: Vec<ScriptView>,
    pub skipped: usize,
    pub error: Option<String>,
}

fn views_of(st: &State, load: bool) -> Vec<ScriptView> {
    let running = RUNNING.lock().clone();
    let mut c = CONSENTS.lock();
    st.scan
        .scripts
        .iter()
        .map(|s| {
            let consented = s.fingerprint().is_some_and(|fp| if load { c.has_fp(&fp) } else { c.has_cached_fp(&fp) });
            view(s, st.interps.get(&s.lang), consented, running.contains(&s.id))
        })
        .collect()
}

/// Sken (stat + změněné soubory) a seznam pro UI. Volat mimo UI vlákno (disk).
pub fn refresh(cfg: &ScriptsConfig) -> Listing {
    let dir = resolve_dir(cfg);
    let shown = dir.as_ref().map(|d| d.display().to_string()).unwrap_or_default();
    let dir = match (cfg.enabled, dir) {
        (true, Ok(d)) => d,
        (_, r) => {
            let changed = STATE.lock().take().is_some_and(|s| !s.last.is_empty());
            if changed && cfg.in_actions {
                crate::state::mark_dirty();
            }
            return Listing { enabled: cfg.enabled, dir: shown, exists: false, scripts: vec![], skipped: 0, error: r.err() };
        }
    };
    // kopie, ne `take`: snapshot mezitím dál vidí starý stav (dlaždice nezablikají)
    let (prev_scan, prev_last) = STATE.lock().as_ref().map(|p| (p.scan.clone(), p.last.clone())).unzip();
    let scan = scan_dir(&dir, prev_scan.as_ref());
    let path_var = std::env::var_os("PATH").unwrap_or_default();
    let interps = interps(&scan.scripts, &path_var);
    let mut st = State { scan, interps, last: vec![] };
    let views = views_of(&st, true);
    let changed = prev_last != Some(views.clone());
    st.last = views.clone();
    let listing = Listing { enabled: true, dir: shown, exists: st.scan.exists, scripts: views, skipped: st.scan.skipped, error: st.scan.error.clone() };
    *STATE.lock() = Some(st);
    if changed && cfg.in_actions {
        crate::state::mark_dirty();
    }
    listing
}

/// `refresh` na krátkém vlákně (otevření šuplíku s kartou, změna configu).
pub fn refresh_async() {
    let cfg = crate::state::core().cfg.read().scripts.clone();
    if !cfg.enabled {
        STATE.lock().take();
        return;
    }
    std::thread::Builder::new().name("scripts".into()).spawn(move || drop(refresh(&cfg))).ok();
}

/// Snapshot: jen paměť, a jen když jsou skripty i v kartě.
pub fn snapshot_views(cfg: &ScriptsConfig) -> Vec<ScriptView> {
    if !(cfg.enabled && cfg.in_actions) {
        return vec![];
    }
    STATE.lock().as_ref().map(|st| views_of(st, false)).unwrap_or_default()
}

/// Skript `id` čerstvě z disku (spuštění): soubor se přečte a zahashuje vždy znovu, ne z cache.
pub fn fresh(cfg: &ScriptsConfig, id: &str) -> Result<(Script, Interp), String> {
    if !cfg.enabled {
        return Err(tr("Skripty jsou vypnuté (Nastavení → Rychlé akce).", "Scripts are off (Settings → Quick actions).").into());
    }
    let dir = resolve_dir(cfg)?;
    let file = id.strip_prefix("script:").filter(|f| !f.is_empty() && !f.contains(['\\', '/', ':'])).ok_or_else(|| tr("Neznámý skript.", "Unknown script."))?;
    let path = dir.join(file);
    let lang = Path::new(file).extension().and_then(|e| e.to_str()).and_then(Lang::from_ext).ok_or_else(|| tr("Neznámý skript.", "Unknown script."))?;
    let md = std::fs::metadata(&path).ok().filter(|m| m.is_file()).ok_or_else(|| trf!("Skript {file} už ve složce není.", "The script {file} is no longer in the folder."))?;
    let (meta, sha) = load_one(&path, lang, md.len())?.ok_or_else(|| trf!("{file} už nemá hlavičku @ledge.", "{file} no longer has an @ledge header."))?;
    let s = Script { id: id.to_string(), file: file.to_string(), path, lang, meta, sha };
    let path_var = std::env::var_os("PATH").unwrap_or_default();
    let interp = run::interp(lang, &path_var);
    if let Some(p) = problem(&s, Some(&interp)) {
        return Err(format!("{}: {p}", s.file));
    }
    Ok((s, interp?))
}

#[cfg(test)]
mod tests;
