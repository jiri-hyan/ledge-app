//! Validace configu: první chyba i s cestou k poli.

use super::Config;
use super::hotkeys::hotkey_collision;
use super::notes::{validate_note_kinds, validate_pin_defaults};
use super::{MAX_LINK_TARGET, MAX_LINKS, ProjectKind, Trigger, valid_aumid, valid_notes_remote, valid_repo};
use crate::configerr::{FieldError, fe};
use crate::i18n::tr;

/// Validace s textem chyby (zpětně kompatibilní tvar; pole viz `validate_field`).
pub fn validate(cfg: &Config) -> Result<(), String> {
    validate_field(cfg).map_err(|e| e.message)
}

/// Validace configu: první chyba i s cestou k poli (Nastavení ji ukáže u pole).
pub fn validate_field(cfg: &Config) -> Result<(), FieldError> {
    let mut seen = std::collections::HashSet::new();
    if cfg.git_alerts.read.len() > 64 || !cfg.git_alerts.read.chars().all(|c| c.is_ascii_alphanumeric()) {
        return Err(fe("gitAlerts.read", tr("Otisk „Přečteno“ má nečekaný tvar — smaž ho.", "The “Mark as read” fingerprint has an unexpected form — delete it.")));
    }
    for (i, p) in cfg.projects.iter().enumerate() {
        let at = |f: &str| format!("projects.{i}.{f}");
        if p.id.trim().is_empty() {
            return Err(fe(at("id"), trf!("Projekt „{}“ nemá id.", "Project “{}” has no id.", p.name)));
        }
        if !seen.insert(p.id.as_str()) {
            return Err(fe(at("id"), trf!("Id projektu „{}“ je dvakrát.", "Project id “{}” is there twice.", p.id)));
        }
        if p.kind == ProjectKind::Dev && p.run.as_deref().unwrap_or("").trim().is_empty() {
            return Err(fe(at("run"), trf!("Projekt „{}“ (dev) nemá příkaz `run`.", "Project “{}” (dev) has no `run` command.", p.id)));
        }
        if p.kind == ProjectKind::App && p.exe.is_none() && p.app_name.is_none() && p.aumid.is_none() {
            return Err(fe(at("exe"), trf!("Projekt „{}“ (app) potřebuje `exe`, `appName` nebo `aumid`.", "Project “{}” (app) needs `exe`, `appName` or `aumid`.", p.id)));
        }
        if let Some(a) = p.aumid.as_deref()
            && !valid_aumid(a)
        {
            return Err(fe(at("aumid"), trf!("Projekt „{}“: aumid „{a}“ nesedí (čekám tvar Jmeno_hash!App, jen A–Z a–z 0–9 . _ ! -).", "Project “{}”: aumid “{a}” doesn't fit (expected the form Name_hash!App, only A–Z a–z 0–9 . _ ! -).", p.id)));
        }
        if let Some(a) = &p.accent {
            let ok = a.len() == 7 && a.starts_with('#') && a[1..].chars().all(|c| c.is_ascii_hexdigit());
            if !ok {
                return Err(fe(at("accent"), trf!("Projekt „{}“: accent musí být #rrggbb.", "Project “{}”: accent must be #rrggbb.", p.id)));
            }
        }
    }
    let w = &cfg.personal.weather;
    if w.lat.is_some() != w.lon.is_some() {
        let missing = if w.lat.is_none() { "personal.weather.lat" } else { "personal.weather.lon" };
        return Err(fe(missing, tr("personal.weather: vyplň zeměpisnou šířku i délku, nebo obě smaž.", "personal.weather: fill in both latitude and longitude, or clear both.")));
    }
    if w.lat.is_some_and(|v| !(-90.0..=90.0).contains(&v)) {
        return Err(fe("personal.weather.lat", tr("personal.weather.lat musí být −90 až 90.", "personal.weather.lat must be −90 to 90.")));
    }
    if w.lon.is_some_and(|v| !(-180.0..=180.0).contains(&v)) {
        return Err(fe("personal.weather.lon", tr("personal.weather.lon musí být −180 až 180.", "personal.weather.lon must be −180 to 180.")));
    }
    let links = &cfg.personal.links;
    if links.len() > MAX_LINKS {
        return Err(fe("personal.links", trf!("personal.links: nejvýš {MAX_LINKS} odkazů (je jich {}).", "personal.links: at most {MAX_LINKS} links (there are {}).", links.len())));
    }
    for (i, l) in links.iter().enumerate() {
        let n = i + 1;
        if l.name.trim().is_empty() {
            return Err(fe(format!("personal.links.{i}.name"), trf!("Odkaz č. {n} nemá název.", "Link no. {n} has no name.")));
        }
        if l.target.trim().is_empty() {
            return Err(fe(format!("personal.links.{i}.target"), trf!("Odkaz „{}“ nemá cíl.", "Link “{}” has no target.", l.name)));
        }
        if l.target.chars().count() > MAX_LINK_TARGET {
            return Err(fe(format!("personal.links.{i}.target"), trf!("Odkaz „{}“: cíl je delší než {MAX_LINK_TARGET} znaků.", "Link “{}”: the target is longer than {MAX_LINK_TARGET} characters.", l.name)));
        }
    }
    crate::calendar::conf::validate(&cfg.personal.calendars).map_err(|(path, msg)| fe(path, msg))?;
    validate_note_kinds(&cfg.notes.kinds)?;
    validate_pin_defaults(&cfg.notes.pins, &cfg.notes.tag_colors)?;
    let sync = &cfg.notes.sync;
    if !(1..=120).contains(&sync.interval_min) {
        return Err(fe("notes.sync.intervalMin", tr("notes.sync.intervalMin musí být 1–120 minut.", "notes.sync.intervalMin must be 1–120 minutes.")));
    }
    if (sync.enabled || !sync.remote.trim().is_empty()) && !valid_notes_remote(&sync.remote) {
        return Err(fe(
            "notes.sync.remote",
            trf!(
                "notes.sync.remote „{}“: chce https://…, git@…, ssh://… (nebo absolutní cestu / file:// k testovacímu repu).",
                "notes.sync.remote “{}”: needs https://…, git@…, ssh://… (or an absolute path / file:// to a test repo).",
                sync.remote
            ),
        ));
    }
    crate::notes::folder_sync::validate_link(&cfg.notes.folder_link)?;
    if !(1..=48).contains(&cfg.update.interval_h) {
        return Err(fe("update.intervalH", tr("update.intervalH musí být 1–48 (hodin).", "update.intervalH must be 1–48 (hours).")));
    }
    if !valid_repo(&cfg.update.repo) {
        return Err(fe("update.repo", trf!("update.repo „{}“ musí mít tvar owner/name (např. jiri-hyan/ledge-app).", "update.repo “{}” must have the form owner/name (e.g. jiri-hyan/ledge-app).", cfg.update.repo)));
    }
    if !(0.0..=1.0).contains(&cfg.strip.opacity) {
        return Err(fe("strip.opacity", tr("strip.opacity musí být 0–1.", "strip.opacity must be 0–1.")));
    }
    // bez myši i bez zkratky by šuplík nešel otevřít vůbec
    if cfg.drawer.trigger == Trigger::Off && (!cfg.hotkeys.enabled || cfg.hotkeys.toggle.trim().is_empty()) {
        return Err(fe("drawer.trigger", tr(
                "drawer.trigger „off“ jde jen se zapnutou zkratkou (hotkeys.enabled + hotkeys.toggle) — bez zkratek musí jít šuplík otevřít myší.",
                "drawer.trigger “off” only works with the shortcut on (hotkeys.enabled + hotkeys.toggle) — without shortcuts the drawer must open with the mouse.",
            )));
    }
    hotkey_collision(cfg)?;
    crate::actions::validate(&cfg.actions)?;
    crate::scripts::validate(&cfg.scripts).map_err(|e| fe("scripts.dir", e))?;
    crate::widgets::validate(&cfg.widgets).map_err(|e| fe("widgets.dir", e))?;
    crate::appearance::validate(&cfg.appearance)?;
    crate::stripconf::validate(&cfg.strip)?;
    crate::timer::validate(&cfg.timer)?;
    crate::smartconf::validate(&cfg.smart)?;
    crate::winquick::validate(&cfg.winquick)?;
    crate::mediaconf::validate(&cfg.media)?;
    crate::cliphist::validate(&cfg.clipboard)?;
    crate::github::conf::validate(&cfg.github)?;
    crate::sysmon::conf::validate(&cfg.system)?;
    crate::layout::validate(&cfg.layout)?;
    crate::accounts::conf::validate(&cfg.accounts)?;
    crate::tilelook::validate(cfg)?;
    Ok(())
}
