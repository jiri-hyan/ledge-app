//! Review plánu: `PermissionRequest` s `tool_name == "ExitPlanMode"` (Claude chce opustit plan mode).
//! Čisté kromě `from_request` (případné čtení `planFilePath`).
//!
//! - Plán = `tool_input.plan` (Markdown); prázdný → zkusí soubor `planFilePath` (stejné limity jako
//!   náhled diffu). Nic → `None` a karta je obyčejné schválení jako dřív.
//! - Tři akce karty: **Schválit** = `allow`; **Schválit a povolit úpravy** = `allow` + `updatedPermissions:
//!   [{type:"setMode", mode:"acceptEdits", destination:"session"}]`; **Vrátit s připomínkou** = `deny` +
//!   `message`. „Vždy povolit“ u plánu není (`resolve` ho odmítne).
//! - Zda Claude Code ExitPlanMode přes `PermissionRequest` posílá, dokumentace neříká — když nepřijde,
//!   nic z toho se nespustí.
//!
//! Mapa: `docs/codemap/sessions.md` → Schvalování.

use super::preview::{Disk, READ_MAX, READ_WAIT, read_with_deadline};
use super::rules::{Extra, clean_reason};
use super::{Decision, paths::absolutize};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::path::PathBuf;

pub const TOOL: &str = "ExitPlanMode";
/// Nejdelší plán, který karta nese (znaky); delší se ořízne (`cut`).
pub const PLAN_MAX: usize = 64 * 1024;
/// Nejdelší titulek (peek, řádek karty).
pub const TITLE_MAX: usize = 80;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Plan {
    pub markdown: String,
    /// první nadpis, jinak první neprázdný řádek
    pub title: String,
    /// `planFilePath`, když ho Claude poslal
    pub file: Option<String>,
    /// plán byl delší než `PLAN_MAX`
    pub cut: bool,
}

/// Režim po schválení plánu — z karty jde jen tenhle (jiné hodnoty IPC odmítne).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Mode {
    AcceptEdits,
}

/// Titulek plánu: první nadpis Markdownu mimo blok kódu, jinak první neprázdný řádek; bez `#`, `>`,
/// odrážek a `**`, nejvýš `TITLE_MAX` znaků.
pub fn title(md: &str) -> String {
    let mut fence = false;
    let mut first: Option<&str> = None;
    let mut heading: Option<&str> = None;
    for l in md.lines() {
        let t = l.trim();
        if t.starts_with("```") || t.starts_with("~~~") {
            fence = !fence;
            continue;
        }
        if fence || t.is_empty() {
            continue;
        }
        if first.is_none() {
            first = Some(t);
        }
        if t.starts_with('#') {
            heading = Some(t);
            break;
        }
    }
    let raw = heading.or(first).unwrap_or("");
    let t = raw.trim_start_matches(['#', '>', '-', '*', ' ']).replace("**", "").replace('`', "");
    let t = t.split_whitespace().collect::<Vec<_>>().join(" ");
    if t.chars().count() > TITLE_MAX { t.chars().take(TITLE_MAX).collect::<String>().trim_end().to_string() + "…" } else { t }
}

/// Plán z textu (+ cesta k souboru). Prázdný → `None`.
pub fn parse(text: &str, file: Option<String>) -> Option<Plan> {
    let text = text.trim_matches(['\n', '\r']);
    if text.trim().is_empty() {
        return None;
    }
    let cut = text.chars().count() > PLAN_MAX;
    let markdown = if cut { text.chars().take(PLAN_MAX).collect() } else { text.to_string() };
    Some(Plan { title: title(&markdown), markdown, file, cut })
}

/// Plán z `tool_input`: `plan`, jinak obsah `planFilePath` (čtení s limitem a termínem).
pub fn from_request(input: &Value, cwd: &str, read: impl FnOnce(&str) -> Disk) -> Option<Plan> {
    let file = ["planFilePath", "plan_file_path"].iter().find_map(|k| input.get(*k).and_then(Value::as_str)).filter(|f| !f.trim().is_empty()).map(str::to_string);
    let text = input.get("plan").and_then(Value::as_str).unwrap_or("");
    if !text.trim().is_empty() {
        return parse(text, file);
    }
    let path = absolutize(file.as_deref()?, cwd);
    match read(&path) {
        Disk::Text(t) => parse(&t, file),
        _ => None,
    }
}

/// Produkce: `from_request` se čtením přes `read_with_deadline`.
pub fn for_request(input: &Value, cwd: &str) -> Option<Plan> {
    from_request(input, cwd, |p| read_with_deadline(PathBuf::from(p), READ_MAX, READ_WAIT))
}

/// Klik na kartě plánu. `None` = nesmyslná kombinace (Vždy povolit, režim + vrátit) → karta zůstane.
pub fn resolve(allow: bool, extra: &Extra) -> Option<Decision> {
    if extra.always.is_some() {
        return None;
    }
    match (allow, extra.mode) {
        (true, None) => Some(Decision::Allow),
        (true, Some(m)) => Some(Decision::AllowMode(m)),
        (false, Some(_)) => None,
        (false, None) => Some(match extra.reason.as_deref().and_then(clean_reason) {
            Some(r) => Decision::ReturnPlan(r),
            None => Decision::Deny,
        }),
    }
}

/// `decision` pro „Schválit a povolit úpravy“.
pub fn allow_mode(m: Mode) -> Value {
    json!({"behavior": "allow", "updatedPermissions": [{"type": "setMode", "mode": m, "destination": "session"}]})
}

/// `decision.message` pro „Vrátit s připomínkou“.
pub fn return_message(r: &str) -> String {
    format!("Plán vrácen z Ledge s připomínkou: {r}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::claude::rules::{Destination, body};

    #[test]
    fn title_table() {
        let long = format!("# {}", "slovo ".repeat(30));
        let cases: Vec<(&str, &str)> = vec![
            ("# Plán: přihlášení\n\n1. krok", "Plán: přihlášení"),
            ("Úvod bez nadpisu\n## Kroky", "Kroky"),
            ("Jen text\nna dvou řádcích", "Jen text"),
            ("```\n# ne v kódu\n```\n## **Opravdový** `nadpis`", "Opravdový nadpis"),
            ("\n\n- odrážka první\n", "odrážka první"),
            ("> citát", "citát"),
            ("", ""),
        ];
        for (md, want) in cases {
            assert_eq!(title(md), want, "{md:?}");
        }
        let t = title(&long);
        assert!(t.ends_with('…') && t.chars().count() <= TITLE_MAX + 1, "{t}");
    }

    #[test]
    fn plan_from_input() {
        let read_none = |_: &str| -> Disk { panic!("plán ve vstupu = nečíst soubor") };
        let p = from_request(&json!({"plan": "# A\n\nkroky", "planFilePath": "C:/p/.claude/plans/a.md"}), "C:/p", read_none).unwrap();
        assert_eq!((p.title.as_str(), p.file.as_deref(), p.cut), ("A", Some("C:/p/.claude/plans/a.md"), false));
        assert_eq!(from_request(&json!({"plan": "  \n "}), "C:/p", |_| Disk::Missing), None);
        assert_eq!(from_request(&json!({}), "C:/p", |_| Disk::Missing), None);
        let big = "x".repeat(PLAN_MAX + 5);
        let p = from_request(&json!({"plan": big}), "", read_none).unwrap();
        assert_eq!((p.cut, p.markdown.chars().count()), (true, PLAN_MAX));
        // jen soubor (tempdir)
        let t = tempfile::tempdir().unwrap();
        std::fs::write(t.path().join("plan.md"), "## Ze souboru\n- a").unwrap();
        let dir = t.path().to_string_lossy().replace('\\', "/");
        let p = from_request(&json!({"planFilePath": "plan.md"}), &dir, |p| crate::claude::preview::read_capped(std::path::Path::new(p), READ_MAX)).unwrap();
        assert_eq!(p.title, "Ze souboru");
        assert_eq!(from_request(&json!({"planFilePath": "chybi.md"}), &dir, |p| crate::claude::preview::read_capped(std::path::Path::new(p), READ_MAX)), None);
    }

    /// Tři akce karty plánu → tělo odpovědi hooku.
    #[test]
    fn plan_actions_body() {
        let dec = |d: Option<Decision>| -> Value {
            let b = body(&d.expect("rozhodnutí"));
            let v: Value = serde_json::from_str(&b).unwrap();
            assert_eq!(v["hookSpecificOutput"]["hookEventName"], "PermissionRequest");
            v["hookSpecificOutput"]["decision"].clone()
        };
        let ex = |mode: Option<Mode>, reason: Option<&str>| Extra { mode, reason: reason.map(str::to_string), ..Extra::default() };
        assert_eq!(dec(resolve(true, &ex(None, None))), json!({"behavior": "allow"}), "Schválit");
        assert_eq!(
            dec(resolve(true, &ex(Some(Mode::AcceptEdits), None))),
            json!({"behavior": "allow", "updatedPermissions": [{"type": "setMode", "mode": "acceptEdits", "destination": "session"}]}),
            "Schválit a povolit úpravy"
        );
        assert_eq!(
            dec(resolve(false, &ex(None, Some("  nejdřív testy \"x\" ")))),
            json!({"behavior": "deny", "message": "Plán vrácen z Ledge s připomínkou: nejdřív testy \"x\""}),
            "Vrátit s připomínkou"
        );
        assert_eq!(dec(resolve(false, &ex(None, Some("  ")))), json!({"behavior": "deny", "message": "Zamítnuto v Ledge."}), "prázdná připomínka = zamítnout");
        assert_eq!(resolve(false, &ex(Some(Mode::AcceptEdits), None)), None, "režim + vrátit = nic");
        assert_eq!(resolve(true, &Extra { always: Some(Destination::Session), ..Extra::default() }), None, "Vždy povolit u plánu ne");
    }

    #[test]
    fn mode_from_ipc() {
        let e: Extra = serde_json::from_value(json!({"mode": "acceptEdits"})).unwrap();
        assert_eq!(e.mode, Some(Mode::AcceptEdits));
        assert!(serde_json::from_value::<Extra>(json!({"mode": "bypassPermissions"})).is_err(), "jiný režim z karty nejde");
    }
}
