//! Zástupci (config `projects`, karta Projekty) a rychlé akce (`actions`) přes MCP: přidat, upravit,
//! odebrat. Změna jde přes typy configu a jejich validaci (`config::validate_field` volá `tools`);
//! tady jsou jen pravidla navíc pro MCP:
//!
//! - **Zástupce nesmí dostat libovolný příkaz.** Spuštění zástupce (`run`, `args`) se na rozdíl od akce
//!   nepotvrzuje, takže MCP smí dát jen příkaz, který Ledge sám najde ve složce projektu
//!   (`detected_runs`: návrh průvodce i Nastavení → Přidat) nebo skript balíčkovače (`npm run dev`).
//!   Cokoli jiného patří do rychlé akce `command` — ta se před prvním spuštěním zeptá (`actions.md`).
//! - Akce přidaná přes MCP souhlas nikdy nedostane (`Consents::grant` volá jen UI po kliku člověka).

use super::{Host, schema};
use crate::i18n::tr;
use crate::actions::Action;
use crate::config::{Config, Project, ProjectKind};
use regex::Regex;
use serde_json::{Map, Value, json};
use std::path::Path;
use std::sync::LazyLock;

/// Skript balíčkovače: `npm run dev`, `pnpm dev`, `yarn start`, `bun run build`, `npm start`.
static PM_SCRIPT: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^(npm|pnpm|yarn|bun)( run)? [a-z0-9][a-z0-9:._-]{0,63}$").unwrap());

/// (příkaz, argumenty), které Ledge sám navrhne pro složku (průvodce `repos::suggest` + `projectdetect`).
pub fn detected_runs(dir: &Path) -> Vec<(String, Vec<String>)> {
    let mut out = Vec::new();
    if !dir.is_dir() {
        return out;
    }
    let name = dir.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
    let facts = crate::onboarding::repos::facts(dir);
    if let (_, _, Some(p)) = crate::onboarding::repos::suggest(&facts, &name, &dir.to_string_lossy(), None) {
        out.push((p.run.unwrap_or_default(), p.args));
    }
    if let Ok(d) = crate::projectdetect::detect(dir, &[]) {
        out.push((d.run.unwrap_or_default(), Vec::new()));
    }
    out
}

/// Smí MCP dát zástupci tenhle příkaz a argumenty? Kontroluje jen to, co se proti `old` změnilo.
pub fn check_cmd(old: Option<&Project>, new: &Project, host: &dyn Host) -> Result<(), String> {
    let run_changed = old.map(|o| o.run != new.run).unwrap_or(new.run.is_some());
    let args_changed = old.map(|o| o.args != new.args).unwrap_or(!new.args.is_empty());
    if let Some(exe) = new.exe.as_deref()
        && old.map(|o| o.exe.as_deref() != Some(exe)).unwrap_or(true)
        && (!exe.trim().to_ascii_lowercase().ends_with(".exe") || exe.contains(['"', '\0', '\n']))
    {
        return Err(trf!("exe „{exe}“: čekám cestu k souboru .exe.", "exe “{exe}”: expected a path to an .exe file."));
    }
    if !run_changed && !args_changed {
        return Ok(());
    }
    let found = new.path.as_deref().map(|p| host.detected_runs(Path::new(p))).unwrap_or_default();
    if run_changed && let Some(run) = new.run.as_deref().map(str::trim).filter(|r| !r.is_empty()) {
        let ok = PM_SCRIPT.is_match(&run.to_ascii_lowercase()) || found.iter().any(|(r, _)| r == run);
        if !ok {
            let known: Vec<&str> = found.iter().map(|(r, _)| r.as_str()).filter(|r| !r.is_empty()).collect();
            let hint = if known.is_empty() { String::new() } else { trf!(" Ve složce jsem našel: {}.", " Found in the folder: {}.", known.join(", ")) };
            return Err(trf!(
                "Příkaz „{run}“ zástupci přes MCP dát nejde: zástupce se spouští bez potvrzení, proto jen skript balíčkovače (npm/pnpm/yarn/bun run <skript>) nebo příkaz, který Ledge sám najde ve složce projektu.{hint} Jiný příkaz přidej jako rychlou akci typu command (actions_add) — ta se před prvním spuštěním zeptá.",
                "The command “{run}” can't be given to a shortcut over MCP: shortcuts start without confirmation, so only a package-manager script (npm/pnpm/yarn/bun run <script>) or a command Ledge finds in the project folder itself is allowed.{hint} Add any other command as a quick action of kind command (actions_add) — it asks before its first run."
            ));
        }
    }
    if args_changed && !new.args.is_empty() && !found.iter().any(|(_, a)| *a == new.args) {
        return Err(tr(
            "Argumenty zástupci přes MCP dát nejde (spouští se bez potvrzení). Aplikaci s argumenty přidej jako rychlou akci typu app (actions_add) — ta se před prvním spuštěním zeptá.",
            "Arguments can't be given to a shortcut over MCP (it starts without confirmation). Add an app with arguments as a quick action of kind app (actions_add) — it asks before its first run."
        )
        .into());
    }
    Ok(())
}

fn ids_of<'a>(it: impl Iterator<Item = &'a str>) -> String {
    let v: Vec<&str> = it.collect();
    if v.is_empty() { "—".into() } else { v.join(", ") }
}

/// Jednoznačné id ze jména (slug, kolize `-2…`).
pub fn unique_id(name: &str, taken: &[&str]) -> String {
    let base = crate::onboarding::repos::slug(name);
    if !taken.contains(&base.as_str()) {
        return base;
    }
    (2..).map(|n| format!("{base}-{n}")).find(|id| !taken.contains(&id.as_str())).unwrap()
}

fn obj(args: &Value) -> Result<&Map<String, Value>, String> {
    args.as_object().ok_or_else(|| tr("Čekám objekt s poli.", "Expected an object with fields.").into())
}

fn str_arg<'a>(args: &'a Value, k: &str) -> Option<&'a str> {
    args.get(k).and_then(Value::as_str).map(str::trim).filter(|s| !s.is_empty())
}

/// Změny bez `id` (id se mění jen odebráním a přidáním).
fn without_id(args: &Value) -> Value {
    let mut m = args.as_object().cloned().unwrap_or_default();
    m.remove("id");
    Value::Object(m)
}

// ---------- zástupci (projects) ----------

pub fn tiles_list(cfg: &Config) -> Value {
    json!({
        "tiles": cfg.projects,
        "projectsCardOnTabs": cfg.layout.pages.iter().filter(|p| p.cards.iter().any(|c| c.id == "projects")).map(|p| p.id.clone()).collect::<Vec<_>>(),
    })
}

pub fn tile_add(cfg: &Config, args: &Value, host: &dyn Host) -> Result<(Config, String), String> {
    let m = obj(args)?;
    let name = str_arg(args, "name").ok_or_else(|| tr("Chybí name (název zástupce).", "name (the shortcut's name) is missing."))?;
    let root = schema::of::<Project>();
    if let Some(k) = schema::unknown_key(&root, &root, args, "") {
        return Err(trf!("Neznámé pole „{k}“ (jde: {}).", "Unknown field “{k}” (allowed: {}).", schema::fields(&root, &root).join(", ")));
    }
    let taken: Vec<&str> = cfg.projects.iter().map(|p| p.id.as_str()).collect();
    let id = match str_arg(args, "id") {
        Some(id) if taken.contains(&id) => return Err(trf!("Zástupce s id „{id}“ už je — uprav ho přes tiles_update.", "A shortcut with id “{id}” already exists — change it with tiles_update.")),
        Some(id) => id.to_string(),
        None => unique_id(name, &taken),
    };
    let mut v = json!({ "id": id, "name": name, "kind": "dev" });
    // složka bez druhu / příkazu = návrh jako Nastavení → Projekty → Přidat (jen pole, která vstup nemá)
    if let Some(path) = str_arg(args, "path")
        && !(m.contains_key("kind") && (m.contains_key("run") || m.contains_key("exe") || m.contains_key("appName")))
    {
        if !Path::new(path).is_absolute() {
            return Err(trf!("path „{path}“: čekám absolutní cestu ke složce (C:\\…).", "path “{path}”: expected an absolute folder path (C:\\…)."));
        }
        let owned: Vec<String> = taken.iter().map(|s| s.to_string()).collect();
        let d = host.detect(Path::new(path), &owned)?;
        let dv = serde_json::to_value(&d).map_err(|e| e.to_string())?;
        for k in ["kind", "remote", "subdir", "run", "port", "exe"] {
            if let Some(x) = dv.get(k).filter(|x| !x.is_null()) {
                v[k] = x.clone();
            }
        }
    }
    schema::merge(&mut v, &without_id(args));
    // `passPort` / `openBrowser` patří dev serveru; aplikace je nemá (výchozí v configu je `true`)
    if v["kind"] == "app" {
        v["passPort"] = Value::Bool(false);
        v["openBrowser"] = Value::Bool(false);
    }
    let p: Project = serde_json::from_value(v).map_err(|e| schema::serde_msg(&e))?;
    check_cmd(None, &p, host)?;
    let summary = describe(&p);
    let mut next = cfg.clone();
    next.projects.push(p);
    let shown = crate::layout::placed(&next.layout, "projects");
    let tail = if shown { "" } else { tr(" Karta Projekty ale není na žádné záložce — přidej ji (layout_apply).", " But the Projects card isn't on any tab — add it (layout_apply).") };
    Ok((next, trf!("Přidán zástupce {summary}.{tail}", "Added the shortcut {summary}.{tail}")))
}

fn describe(p: &Project) -> String {
    let what = match (p.run.as_deref(), p.exe.as_deref().or(p.app_name.as_deref()).or(p.aumid.as_deref())) {
        (Some(r), _) => format!("„{r}“"),
        (None, Some(e)) => format!("„{e}“"),
        _ => String::new(),
    };
    format!("„{}“ (id {}{}{what})", p.name, p.id, if what.is_empty() { "" } else { ", " })
}

pub fn tile_update(cfg: &Config, args: &Value, host: &dyn Host) -> Result<(Config, String), String> {
    obj(args)?;
    let id = str_arg(args, "id").ok_or_else(|| tr("Chybí id zástupce (tiles_list).", "The shortcut id is missing (tiles_list)."))?;
    let i = cfg.projects.iter().position(|p| p.id == id).ok_or_else(|| trf!("Zástupce „{id}“ není (jsou: {}).", "There's no shortcut “{id}” (there are: {}).", ids_of(cfg.projects.iter().map(|p| p.id.as_str()))))?;
    let old = &cfg.projects[i];
    let mut new: Project = schema::patch(old, &without_id(args), "tiles")?;
    if new.kind == ProjectKind::App {
        (new.pass_port, new.open_browser) = (false, false);
    }
    check_cmd(Some(old), &new, host)?;
    let mut next = cfg.clone();
    next.projects[i] = new;
    Ok((next, trf!("Upraven zástupce „{id}“.", "Updated the shortcut “{id}”.")))
}

pub fn tile_remove(cfg: &Config, args: &Value) -> Result<(Config, String), String> {
    let id = str_arg(args, "id").ok_or_else(|| tr("Chybí id zástupce (tiles_list).", "The shortcut id is missing (tiles_list)."))?;
    let mut next = cfg.clone();
    let n = next.projects.len();
    next.projects.retain(|p| p.id != id);
    if next.projects.len() == n {
        return Err(trf!("Zástupce „{id}“ není (jsou: {}).", "There's no shortcut “{id}” (there are: {}).", ids_of(cfg.projects.iter().map(|p| p.id.as_str()))));
    }
    Ok((next, trf!("Odebrán zástupce „{id}“.", "Removed the shortcut “{id}”.")))
}

// ---------- rychlé akce a odkazy (actions) ----------

pub fn actions_list(cfg: &Config) -> Value {
    json!({ "actions": cfg.actions, "max": crate::actions::MAX_ACTIONS })
}

fn consent_note(a: &Action) -> &'static str {
    if a.risky() {
        tr(" Před prvním spuštěním se Ledge uživatele zeptá (souhlas).", " Ledge asks the user before its first run (consent).")
    } else {
        ""
    }
}

pub fn action_add(cfg: &Config, args: &Value) -> Result<(Config, String), String> {
    obj(args)?;
    let label = str_arg(args, "label").ok_or_else(|| tr("Chybí label (popisek akce).", "label (the action's label) is missing."))?;
    let taken: Vec<&str> = cfg.actions.iter().map(|a| a.id.as_str()).collect();
    let id = match str_arg(args, "id") {
        Some(id) if taken.contains(&id) => return Err(trf!("Akce s id „{id}“ už je — uprav ji přes actions_update.", "An action with id “{id}” already exists — change it with actions_update.")),
        Some(id) => id.to_string(),
        None => unique_id(label, &taken),
    };
    let mut v = without_id(args);
    v["id"] = Value::from(id);
    let a: Action = schema::parse(&v, "actions")?;
    crate::actions::validate_one(&a)?;
    let note = consent_note(&a);
    let mut next = cfg.clone();
    next.actions.push(a.clone());
    Ok((next, trf!("Přidána akce „{}“ (id {}).{note}", "Added the action “{}” (id {}).{note}", a.label, a.id)))
}

pub fn action_update(cfg: &Config, args: &Value) -> Result<(Config, String), String> {
    obj(args)?;
    let id = str_arg(args, "id").ok_or_else(|| tr("Chybí id akce (actions_list).", "The action id is missing (actions_list)."))?;
    let i = cfg.actions.iter().position(|a| a.id == id).ok_or_else(|| trf!("Akce „{id}“ není (jsou: {}).", "There's no action “{id}” (there are: {}).", ids_of(cfg.actions.iter().map(|a| a.id.as_str()))))?;
    let new: Action = schema::patch(&cfg.actions[i], &without_id(args), "actions")?;
    crate::actions::validate_one(&new)?;
    // jiný příkaz = jiný otisk → případný dřívější souhlas přestane platit sám (`actions::fingerprint`)
    let note = if crate::actions::fingerprint(&new) != crate::actions::fingerprint(&cfg.actions[i]) { consent_note(&new) } else { "" };
    let mut next = cfg.clone();
    next.actions[i] = new;
    Ok((next, trf!("Upravena akce „{id}“.{note}", "Updated the action “{id}”.{note}")))
}

pub fn action_remove(cfg: &Config, args: &Value) -> Result<(Config, String), String> {
    let id = str_arg(args, "id").ok_or_else(|| tr("Chybí id akce (actions_list).", "The action id is missing (actions_list)."))?;
    let mut next = cfg.clone();
    let n = next.actions.len();
    next.actions.retain(|a| a.id != id);
    if next.actions.len() == n {
        return Err(trf!("Akce „{id}“ není (jsou: {}).", "There's no action “{id}” (there are: {}).", ids_of(cfg.actions.iter().map(|a| a.id.as_str()))));
    }
    Ok((next, trf!("Odebrána akce „{id}“.", "Removed the action “{id}”.")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pm_script_table() {
        for (run, ok) in [
            ("npm run dev", true),
            ("pnpm dev", true),
            ("yarn start", true),
            ("bun run build:web", true),
            ("npm start", true),
            ("npm run dev && del x", false),
            ("npm run dev; calc", false),
            ("powershell -c x", false),
            ("npx something", false),
            // „npm run“ bez skriptu jen vypíše skripty (projde jako skript „run“) — neškodné
        ] {
            assert_eq!(PM_SCRIPT.is_match(run), ok, "{run}");
        }
    }

    #[test]
    fn unique_id_table() {
        assert_eq!(unique_id("Poznámkový blok", &[]), "poznamkovy-blok");
        assert_eq!(unique_id("app", &["app", "app-2"]), "app-3");
    }
}
