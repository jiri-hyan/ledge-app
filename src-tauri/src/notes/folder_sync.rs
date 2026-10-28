//! Propojení poznámek se složkou `.md` souborů (Obsidian vault, Logseq `pages`, export Joplinu…):
//! `notes.folderLink`. Jedna poznámka = jeden `.md`, kopíruje se BAJTOVĚ (frontmatter vč. cizích klíčů,
//! `[[vazby]]`, konce řádků), takže se nic neztratí ani při cestě tam a zpět. Směr: jen import, jen
//! export, nebo oba. Mazání se nepřenáší (volba `propagateDeletes`), konflikt nic nepřepíše potichu.
//!
//! Soubory: tady stav (`State`, `note-folder-link.json` v `%LOCALAPPDATA%\Ledge`, mimo poznámky a
//! config), kontrola složky, běh `run` a texty; `folder_sync_plan.rs` čisté rozhodování,
//! `folder_sync_scan.rs` sken a bezpečné cesty, `folder_sync_apply.rs` provedení kroků.
//! Spouští vlákno `notes-folder` (`threads/notes_folder.rs`) — žádný polling, jen události.
//! Git synchronizace (`notesync/`) je nezávislá a tímhle se nemění. Mapa: `docs/codemap/notes.md`.

use super::folder_sync_apply::{ConflictInfo, Env, Report, apply, display};
use super::folder_sync_plan::{Entry, Opts, plan};
use super::folder_sync_scan::{Kind, Prev, count, scan, under_frozen};
use super::{strip_verbatim, write_atomic};
use crate::config::{FolderLinkConfig, LinkDirection};
use crate::configerr::{FieldError, fe};
use crate::i18n::tr;
use chrono::{DateTime, FixedOffset};
use serde::{Deserialize, Serialize};
use std::path::{Component, Path, PathBuf};

/// Kolik posledních konfliktů se pamatuje (do „Rozumím“).
const MAX_CONFLICTS: usize = 30;
const STATE_FILE: &str = "note-folder-link.json";

/// Nastavení jednoho běhu.
#[derive(Debug, Clone)]
pub struct LinkRun {
    pub root: PathBuf,
    pub folder: PathBuf,
    pub dir: LinkDirection,
    pub deletes: bool,
}

/// Soubor stavu: poslední dohoda (cesty, hashe, otisky) + konflikty k zobrazení. Per počítač.
#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct State {
    pub version: u32,
    /// složka, ke které se záznamy vztahují (jiná složka = čistý začátek)
    pub folder: String,
    pub entries: Vec<Entry>,
    pub conflicts: Vec<ConflictInfo>,
}

pub fn state_path() -> PathBuf {
    crate::actionrun::local_dir().join(STATE_FILE)
}

/// Rozbitý / chybějící soubor = čistý stav (první synchronizace spáruje shodné soubory podle obsahu).
pub fn load_state(path: &Path) -> State {
    std::fs::read_to_string(path).ok().and_then(|t| serde_json::from_str(&t).ok()).unwrap_or_default()
}

pub fn save_state(path: &Path, s: &State) -> Result<(), String> {
    if let Some(dir) = path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    write_atomic(path, serde_json::to_string_pretty(s).map_err(|e| e.to_string())?.as_bytes())
}

/// Smaže zapamatované konflikty (tlačítko „Rozumím“). Poznámky ani kopie se nemění.
pub fn clear_conflicts(path: &Path) -> Result<(), String> {
    let mut s = load_state(path);
    s.conflicts.clear();
    save_state(path, &s)
}

/// Kontrola configu (lexikálně — dostupnost složky se zjišťuje při běhu, odpojený disk nesmí shodit
/// uložení nastavení).
pub fn validate_link(c: &FolderLinkConfig) -> Result<(), FieldError> {
    let p = c.path.trim();
    if p.is_empty() {
        return if c.enabled { Err(fe("notes.folderLink.path", tr("notes.folderLink.path: vyber složku, se kterou se mají poznámky propojit.", "notes.folderLink.path: pick the folder to link the notes with."))) } else { Ok(()) };
    }
    let path = Path::new(p);
    if !path.is_absolute() || path.components().any(|c| matches!(c, Component::ParentDir)) {
        return Err(fe("notes.folderLink.path", trf!("notes.folderLink.path „{p}“ musí být absolutní cesta bez „..“.", "notes.folderLink.path “{p}” must be an absolute path without “..”.")));
    }
    Ok(())
}

fn key(p: &Path) -> String {
    strip_verbatim(p).to_string_lossy().to_lowercase().replace('/', "\\").trim_end_matches('\\').to_string()
}

fn overlaps(a: &Path, b: &Path) -> bool {
    let (a, b) = (key(a), key(b));
    a == b || a.starts_with(&format!("{b}\\")) || b.starts_with(&format!("{a}\\"))
}

/// Složky, které nejsou vault (klíče cest, `key`): profil uživatele, Dokumenty, Plocha, Stažené, OneDrive,
/// AppData, Program Files, Windows. Skládají se z proměnných prostředí (`get`); co chybí, se přeskočí.
pub(super) fn broad_folders(get: &dyn Fn(&str) -> Option<String>) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    let mut add = |p: String| {
        if !p.trim().is_empty() {
            out.push(key(Path::new(p.trim())));
        }
    };
    if let Some(profile) = get("USERPROFILE") {
        add(profile.clone());
        for sub in ["Documents", "Dokumenty", "Desktop", "Plocha", "Downloads", "Stažené", "OneDrive"] {
            add(format!("{profile}\\{sub}"));
        }
    }
    for var in ["OneDrive", "OneDriveConsumer", "OneDriveCommercial", "APPDATA", "LOCALAPPDATA", "ProgramData", "ProgramFiles", "ProgramFiles(x86)", "ProgramW6432", "SystemRoot", "windir"] {
        if let Some(p) = get(var) {
            add(p);
        }
    }
    out
}

/// Je `folder` (klíč cesty) příliš široká? Rovná se známé složce, nebo je její nadřazená (`C:\Users`, kořen
/// disku, `AppData`…) — vault tam nikdy není a sken by prošel půl disku. Čistá funkce.
pub(super) fn is_broad(folder: &str, known: &[String]) -> bool {
    let prefix = format!("{}\\", folder.trim_end_matches('\\'));
    known.iter().any(|k| k == folder || k.starts_with(&prefix))
}

fn too_broad() -> String {
    tr(
        "Tahle složka je moc široká (profil, Dokumenty, Plocha, Stažené, OneDrive, systémová složka…) — vyber přímo složku s poznámkami, třeba vault Obsidianu.",
        "That folder is too broad (profile, Documents, Desktop, Downloads, OneDrive, a system folder…) — pick the folder that holds your notes, e.g. your Obsidian vault.",
    )
    .into()
}

/// Vybraná složka → kanonická cesta, nebo srozumitelná chyba (neexistuje, kořen disku, příliš široká,
/// překrývá se se složkou poznámek).
pub fn check_folder(path: &str, root: &Path) -> Result<PathBuf, String> {
    check_folder_with(path, root, &|n| std::env::var(n).ok())
}

/// `check_folder` s dodaným prostředím (testy nesahají na skutečný profil).
pub(super) fn check_folder_with(path: &str, root: &Path, get: &dyn Fn(&str) -> Option<String>) -> Result<PathBuf, String> {
    let p = PathBuf::from(path.trim());
    if !p.is_absolute() || p.components().any(|c| matches!(c, Component::ParentDir)) {
        return Err(trf!("Cesta „{}“ musí být absolutní.", "The path “{}” must be absolute.", p.display()));
    }
    let c = p.canonicalize().ok().filter(|c| c.is_dir()).ok_or_else(|| super::folder_sync_scan::not_a_folder(&p))?;
    if c.components().filter(|x| matches!(x, Component::Normal(_))).count() == 0 {
        return Err(super::folder_sync_scan::too_shallow());
    }
    if is_broad(&key(&c), &broad_folders(get)) {
        return Err(too_broad());
    }
    let root_c = root.canonicalize().unwrap_or_else(|_| root.to_path_buf());
    if overlaps(&c, &root_c) {
        return Err(tr(
            "Tahle složka se překrývá s úložištěm poznámek Ledge — vyber jinou (třeba vault Obsidianu).",
            "That folder overlaps Ledge's notes storage — pick another one (e.g. your Obsidian vault).",
        )
        .into());
    }
    Ok(c)
}

/// Výsledek běhu pro UI.
#[derive(Debug)]
pub struct Outcome {
    pub report: Report,
    pub message: String,
    /// všechny pamatované konflikty (nejnovější první)
    pub conflicts: Vec<ConflictInfo>,
}

/// Jeden běh: sken obou stran → plán → provedení → uložení stavu. Chyba = nic se nesmazalo ani
/// nepřepsalo (sken přerušený uprostřed by vypadal jako hromadné smazání, proto se nepokračuje).
pub fn run(link: &LinkRun, state_file: &Path, now: DateTime<FixedOffset>) -> Result<Outcome, String> {
    std::fs::create_dir_all(&link.root).map_err(|e| crate::errors::io(&trf!("Složku {} nejde založit", "Can't create the folder {}", link.root.display()), &e))?;
    let root = link.root.canonicalize().map_err(|e| crate::errors::io(&trf!("Složku {} nejde přečíst", "Can't read the folder {}", link.root.display()), &e))?;
    let base = check_folder(&link.folder.to_string_lossy(), &root)?;
    let mut state = load_state(state_file);
    if state.folder != key(&base) {
        state = State { version: 1, folder: key(&base), ..State::default() };
    }
    let prev = |side_ledge: bool| -> Prev {
        state.entries.iter().map(|e| if side_ledge { (e.ledge.to_lowercase(), (e.lm, e.ls, e.hash.clone())) } else { (e.ext.to_lowercase(), (e.em, e.es, e.hash.clone())) }).collect()
    };
    let (ls, es) = (scan(&root, Kind::Ledge, &prev(true))?, scan(&base, Kind::Ext, &prev(false))?);
    let frozen = under_frozen;
    let (held, active): (Vec<Entry>, Vec<Entry>) = std::mem::take(&mut state.entries).into_iter().partition(|e| frozen(&ls.frozen, &e.ledge) || frozen(&es.frozen, &e.ext));
    // položka se jménem mimo UTF-8 nejde spárovat se záznamem → soubor by vypadal smazaný; mazání se tentokrát nepřenáší
    let deletes = link.deletes && ls.odd_names + es.odd_names == 0;
    let opts = Opts { dir: link.dir, deletes, today: now.format("%Y-%m-%d").to_string() };
    let planned = plan(&active, &ls.files, &es.files, &opts);
    let mut report = Report::default();
    let mut entries = apply(&Env { root: &root, base: &base, now_ms: now.timestamp_millis() }, planned.steps, &mut report);
    entries.extend(held);
    entries.sort_by(|a, b| a.ledge.cmp(&b.ledge));
    entries.dedup_by(|a, b| a.ledge.eq_ignore_ascii_case(&b.ledge));
    state.entries = entries;
    let mut conflicts = report.conflicts.clone();
    conflicts.extend(std::mem::take(&mut state.conflicts));
    conflicts.truncate(MAX_CONFLICTS);
    state.conflicts = conflicts.clone();
    state.version = 1;
    save_state(state_file, &state)?;
    let message = summary(&report, planned.held_deletes, es.too_big + ls.too_big, &base);
    Ok(Outcome { report, message, conflicts })
}

/// Lidská věta o tom, co se stalo.
pub fn summary(r: &Report, held: usize, big: usize, base: &Path) -> String {
    let mut parts: Vec<String> = Vec::new();
    let mut add = |n: usize, cs: &'static str, en: &'static str| {
        if n > 0 {
            parts.push(format!("{n} {}", tr(cs, en)));
        }
    };
    add(r.imported, "nových ve Ledge", "new in Ledge");
    add(r.updated_ledge, "upravených ve Ledge", "updated in Ledge");
    add(r.exported, "nových ve složce", "new in the folder");
    add(r.updated_folder, "upravených ve složce", "updated in the folder");
    add(r.deleted_ledge, "do Archivu Ledge", "to Ledge's Archive");
    add(r.deleted_folder, "do .trash ve složce", "to .trash in the folder");
    add(r.conflicts.len(), "konfliktů", "conflicts");
    let mut out = if parts.is_empty() { trf!("Beze změn — {} je v souladu s poznámkami.", "No changes — {} is in step with your notes.", display(base)) } else { format!("{}: {}.", tr("Hotovo", "Done"), parts.join(", ")) };
    if held > 0 {
        out += &trf!(" Smazání ({held}) se nepřeneslo — bylo jich najednou podezřele moc, zkontroluj složku.", " Deletions ({held}) weren't carried over — suspiciously many at once, check the folder.");
    }
    if big > 0 {
        out += &trf!(" Přeskočeno {big} velkých souborů (nad 1 MB).", " Skipped {big} large files (over 1 MB).");
    }
    if r.skipped > 0 {
        out += &trf!(" {} se změnilo za běhu, dořeší se příště.", " {} changed meanwhile, handled next time.", r.skipped);
    }
    if let Some(e) = r.errors.first() {
        out += &format!(" {e}");
    }
    out
}

/// Souborová událost ve složce, která stojí za synchronizaci: poznámka (`.md` / `.markdown`) nebo
/// složka / soubor bez přípony (přejmenování složky); ne skryté části (`.obsidian`, `.trash`, `.git`),
/// `logseq/` ani obrázky a dočasné soubory.
pub fn event_relevant(base: &Path, paths: &[PathBuf]) -> bool {
    if paths.is_empty() {
        return true;
    }
    let bases = [base.to_path_buf(), strip_verbatim(base)];
    paths.iter().any(|p| {
        let rel = bases.iter().find_map(|b| p.strip_prefix(b).ok()).unwrap_or(p.as_path());
        let hidden = rel.components().any(|c| matches!(c, Component::Normal(n) if super::folder_sync_scan::skip_dir(&n.to_string_lossy())));
        let ext = p.extension().map(|e| e.to_string_lossy().to_ascii_lowercase());
        !hidden && matches!(ext.as_deref(), None | Some("md") | Some("markdown"))
    })
}

/// Znaky vaultu: skrytá složka editoru / `logseq/` v kořeni. Bez nich a s hodně `.md` to nejspíš není vault.
const VAULT_SIGNS: [&str; 6] = [".obsidian", "logseq", ".logseq", ".foam", ".zk", ".silverbullet"];
/// Kolik `.md` bez znaku vaultu už stojí za varování.
const WARN_NOTES: usize = 200;

/// Varovat před zapnutím? Ve složce není žádný znak vaultu a je v ní víc než [`WARN_NOTES`] `.md`. Čistá funkce.
pub fn warn_not_vault(has_sign: bool, md: usize) -> bool {
    !has_sign && md > WARN_NOTES
}

/// Výsledek kontroly vybrané složky pro UI: počet poznámek a případné varování (zobrazí se před zapnutím).
#[derive(Debug, Serialize)]
pub struct FolderCheck {
    pub n: usize,
    pub warn: Option<String>,
}

/// Kolik poznámek by se ve složce našlo (pro „Nalezeno N poznámek“ po výběru složky) a varování, když
/// složka nevypadá jako vault.
pub fn count_notes(path: &str, root: &Path) -> Result<FolderCheck, String> {
    let base = check_folder(path, root)?;
    let n = count(&base, Kind::Ext)?;
    let has_sign = VAULT_SIGNS.iter().any(|s| base.join(s).is_dir());
    let warn = warn_not_vault(has_sign, n).then(|| {
        trf!(
            "Ve složce je {n} poznámek, ale nevypadá jako vault (chybí .obsidian ani logseq). Opravdu je to ta správná složka? Všechny se zkopírují do Ledge.",
            "There are {n} notes in the folder, but it doesn't look like a vault (no .obsidian or logseq). Is this really the right folder? All of them will be copied into Ledge."
        )
    });
    Ok(FolderCheck { n, warn })
}
