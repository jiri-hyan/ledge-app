//! Tauri příkazy widgetů ze skriptů: seznam (`widgets_list` — sken při otevření katalogu / Nastavení),
//! souhlas (`widget_consent` — musí sedět text s otiskem obsahu, který člověk viděl), „Obnovit teď“,
//! „Zobrazit výstup“, „Upravit skript“ (sloveso `edit`, jinak Poznámkový blok — nikdy „otevřít“, to by
//! `.py` / `.js` / `.cmd` spustilo), složka, galerie příkladů a klik na položku. Klik umí jen web
//! (http/https), složku / dokument (`personal::classify_fs` — žádné programy), schránku a povolený
//! skript ze složky skriptů s argumentem. Logika v `widgets/*`.

use crate::actionrun::{self, CONSENTS, Outcome, RunGuard};
use crate::i18n::tr;
use crate::personal::{self, LinkTarget};
use crate::scripts::{self, meta::Mode};
use crate::state::{UiCmd, core, log};
use crate::widgets::{self, live, output::Act, templates};
use std::path::Path;
use tauri_plugin_opener::OpenerExt;

#[tauri::command(async)]
pub(crate) fn widgets_list() -> live::Listing {
    let cfg = core().cfg.read().clone();
    live::listing(&cfg)
}

#[tauri::command(async)]
pub(crate) fn widget_consent(id: String, confirmed: String) -> Result<(), String> {
    let cfg = core().cfg.read().widgets.clone();
    live::grant(&cfg, &id, &confirmed)
}

#[tauri::command]
pub(crate) fn widget_refresh(id: String) {
    live::refresh_now(&id);
}

#[tauri::command]
pub(crate) fn widget_output(id: String) -> Option<String> {
    live::output_of(&id)
}

/// Otevře soubor k úpravě: sloveso `edit` (pro `.ps1`, `.cmd`, `.js` ho Windows mají), jinak Poznámkový blok.
fn open_in_editor(path: &Path) -> Result<(), String> {
    use windows::Win32::UI::Shell::ShellExecuteW;
    use windows::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;
    use windows::core::{HSTRING, PCWSTR, w};
    let file = HSTRING::from(path.as_os_str());
    let r = unsafe { ShellExecuteW(None, w!("edit"), &file, PCWSTR::null(), PCWSTR::null(), SW_SHOWNORMAL) };
    if r.0 as isize > 32 {
        return Ok(());
    }
    std::process::Command::new("notepad.exe").arg(path).spawn().map(|_| ()).map_err(|e| crate::errors::spawn(tr("Editor nejde otevřít", "Can't open an editor"), &e))
}

#[tauri::command(async)]
pub(crate) fn widget_edit(id: String) -> Result<(), String> {
    let c = core();
    let cfg = c.cfg.read().widgets.clone();
    let path = live::edit_path(&cfg, &id)?;
    open_in_editor(&path)?;
    c.send_ui(UiCmd::Hide);
    Ok(())
}

/// „Otevřít složku widgetů“: založí ji (výslovný klik) a otevře v Průzkumníkovi.
#[tauri::command(async)]
pub(crate) fn widgets_open_dir() -> Result<String, String> {
    let c = core();
    let cfg = c.cfg.read().widgets.clone();
    let dir = widgets::resolve_dir(&cfg)?;
    std::fs::create_dir_all(&dir).map_err(|e| crate::errors::io(&trf!("Složku {} nejde založit", "Can't create the folder {}", dir.display()), &e))?;
    c.app.opener().open_path(dir.to_string_lossy(), None::<&str>).map_err(|e| crate::errors::any(tr("Složku nejde otevřít", "Can't open the folder"), &e))?;
    Ok(dir.display().to_string())
}

#[tauri::command(async)]
pub(crate) fn widget_templates() -> Vec<templates::TemplateView> {
    let cfg = core().cfg.read().widgets.clone();
    widgets::resolve_dir(&cfg).map(|d| templates::views(&d)).unwrap_or_default()
}

/// Zkopíruje příklad z galerie do složky widgetů; vrací id nového widgetu. Nic nespouští.
#[tauri::command(async)]
pub(crate) fn widget_install(file: String) -> Result<String, String> {
    let cfg = core().cfg.read().widgets.clone();
    let dir = widgets::resolve_dir(&cfg)?;
    let path = templates::install(&dir, &file)?;
    log(&format!("widget: příklad {file} zkopírován"));
    live::rescan(&cfg)?;
    Ok(format!("{}{}", widgets::plan::PREFIX, path.file_name().map(|f| f.to_string_lossy().into_owned()).unwrap_or(file)))
}

/// Skript ze složky skriptů s argumentem z widgetu — jen povolený (souhlas z palety) a bez `@ledge.confirm`.
fn run_script_with(file: &str, arg: Option<&str>) -> Result<Outcome, String> {
    let cfg = core().cfg.read().scripts.clone();
    let (s, interp) = scripts::fresh(&cfg, &format!("script:{file}"))?;
    let meta = s.meta.clone()?;
    let fp = s.fingerprint().ok_or_else(|| tr("Skript nejde ověřit.", "Can't verify the script."))?;
    if meta.confirm || !CONSENTS.lock().has_fp(&fp) {
        return Err(trf!(
            "Skript „{}“ ještě nemáš povolený — spusť ho jednou z palety (Ctrl+K) a potvrď.",
            "You haven't allowed the script “{}” yet — run it once from the palette (Ctrl+K) and confirm.",
            meta.title
        ));
    }
    let mut args = meta.args.clone();
    args.extend(arg.map(str::to_string));
    let dir = s.path.parent().map(|d| d.to_path_buf()).unwrap_or_default();
    let mut cmd = scripts::run::plan(&s.path, &interp, &args, meta.mode == Mode::Terminal)?.command(&dir);
    let res = match meta.mode {
        Mode::Terminal => actionrun::spawn_console(&mut cmd, &meta.title),
        mode => {
            let _guard = RunGuard::take(&s.id).ok_or_else(|| trf!("{} už běží.", "{} is already running.", meta.title))?;
            actionrun::capture(&mut cmd, &meta.title, meta.timeout_s, mode == Mode::Output)
        }
    };
    log(&format!("widget → skript {}: {}", s.file, match &res {
        Ok(o) => o.message.clone(),
        Err(e) => e.lines().next().unwrap_or("").to_string(),
    }));
    res
}

/// Klik na položku widgetu. `label` = co UI ukazovalo (data se mezitím mohla obnovit → odmítnout).
/// Vrací hlášku pro UI (zkopírováno / výsledek skriptu), `None` = otevřelo se něco jiného.
#[tauri::command(async)]
pub(crate) fn widget_click(id: String, index: usize, label: String) -> Result<Option<String>, String> {
    let c = core();
    let item = live::item(&id, index, &label).ok_or_else(|| tr("Widget se mezitím obnovil — klikni znovu.", "The widget refreshed meanwhile — click again."))?;
    if let Some(url) = item.url.as_deref().filter(|u| widgets::output::url_ok(u)) {
        c.app.opener().open_url(url, None::<&str>).map_err(|e| crate::errors::any(tr("Odkaz nejde otevřít", "Can't open the link"), &e))?;
        c.send_ui(UiCmd::Hide);
        return Ok(None);
    }
    match item.action {
        None => Ok(None),
        Some(Act::Copy { text }) => {
            crate::shelf::clip::set_text(&text)?;
            Ok(Some(tr("Zkopírováno do schránky.", "Copied to the clipboard.").into()))
        }
        Some(Act::Open { path }) => {
            match personal::classify_fs(&path) {
                LinkTarget::Folder(p) | LinkTarget::File(p) => c.app.opener().open_path(p.to_string_lossy(), None::<&str>).map_err(|e| crate::errors::any(tr("Odkaz nejde otevřít", "Can't open the link"), &e))?,
                LinkTarget::Web(u) => c.app.opener().open_url(u, None::<&str>).map_err(|e| crate::errors::any(tr("Odkaz nejde otevřít", "Can't open the link"), &e))?,
                LinkTarget::App(_) => return Err(tr("Widget nesmí spouštět programy — jen otevřít složku nebo dokument.", "A widget may not launch programs — only open a folder or a document.").into()),
                LinkTarget::Invalid(why) => return Err(format!("{path}: {why}")),
            }
            c.send_ui(UiCmd::Hide);
            Ok(None)
        }
        Some(Act::Run { script, arg }) => {
            let o = run_script_with(&script, arg.as_deref())?;
            if o.hide {
                c.send_ui(UiCmd::Hide);
            }
            Ok(Some(match o.output {
                Some(out) => format!("{}\n{out}", o.message),
                None => o.message,
            }))
        }
    }
}
