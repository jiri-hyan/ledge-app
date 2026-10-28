//! Zapojení hooků Ledge do `~/.claude/settings.json` (Nastavení → Claude Code → „Zapojit hooky“).
//! Čisté `merge` / `remove` / `status` nad textem (pořadí klíčů zachováno — vlastní `J`, serde_json
//! bez `preserve_order` by klíče přeřadil), cizí klíče i hooky nechá být, své pozná podle URL
//! `http://127.0.0.1|localhost:<port>/permission|/event|/question|/plan`; skupina s matcherem (otázky
//! Clauda, review plánu) se porovnává i podle `matcher`. Víc hooků Ledge pod jednou událostí (PreToolUse:
//! `/question` + `/plan`) se navzájem neodebírá — `merge` odebere jen náš hook, který žádnému řádku
//! `EVENTS` té události nesedí (jiný port, jiný matcher, neznámá cesta). Zápis: záloha `settings.json.bak-pred-ledge`
//! (jen jednou), atomicky (tmp + rename), odsazení 2 mezery. Testy jen nad řetězci a dočasnými soubory.

use serde::de::{Deserializer, MapAccess, SeqAccess, Visitor};
use serde::ser::{SerializeMap, SerializeSeq, Serializer};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Událost → cesta mostu → timeout (s) → matcher. PermissionRequest a PreToolUse(ExitPlanMode) čekají na
/// šéfa (`approvalTimeoutS` ≤ 550), PreToolUse(AskUserQuestion) na odpověď (`questionHoldS` ≤ 550; pevných
/// 600 s, ať změna držení nevyžaduje přepis settings.json); ostatní jen ohlásí stav; Stop v `block`
/// stihne git do 4 s.
pub const EVENTS: &[(&str, &str, u32, Option<&str>)] = &[
    ("PermissionRequest", "/permission", 600, None),
    ("PreToolUse", "/question", 600, Some("AskUserQuestion")),
    ("PreToolUse", "/plan", 600, Some("ExitPlanMode")),
    ("SessionStart", "/event", 5, None),
    ("UserPromptSubmit", "/event", 5, None),
    ("PostToolUse", "/event", 5, None),
    ("PostToolUseFailure", "/event", 5, None),
    ("Stop", "/event", 5, None),
    ("SessionEnd", "/event", 5, None),
    ("Notification", "/event", 5, None),
    ("SubagentStart", "/event", 5, None),
    ("SubagentStop", "/event", 5, None),
];

/// Volitelné hooky (přidané později): jejich absence sama nedělá stav „zapojeno jen zčásti“,
/// jen nabídne „k dispozici aktualizace hooků“.
pub const OPTIONAL_EVENTS: &[&str] = &["PostToolUseFailure"];

/// Hook otázek Clauda — chybí-li jen ten (Ledge starší verze), UI nabídne „Doplnit otázky Clauda“.
pub const QUESTIONS_EVENT: &str = "PreToolUse";
/// Hook review plánu v `missing` (jméno pro UI; otázky zůstávají „PreToolUse“ kvůli starším UI).
pub const PLAN_EVENT: &str = "PreToolUse(ExitPlanMode)";

/// Jméno řádku `EVENTS` v `missing`: událost, u jiného matcheru než otázek „událost(matcher)“.
fn label(ev: &str, m: Option<&str>) -> String {
    match m {
        Some(m) if m != "AskUserQuestion" => format!("{ev}({m})"),
        _ => ev.to_string(),
    }
}

const BACKUP: &str = "settings.json.bak-pred-ledge";

// ---------- JSON se zachovaným pořadím klíčů ----------

#[derive(Debug, Clone, PartialEq)]
pub enum J {
    Null,
    Bool(bool),
    Num(serde_json::Number),
    Str(String),
    Arr(Vec<J>),
    Obj(Vec<(String, J)>),
}

impl J {
    pub(crate) fn get(&self, k: &str) -> Option<&J> {
        match self {
            J::Obj(m) => m.iter().rev().find(|(x, _)| x == k).map(|(_, v)| v),
            _ => None,
        }
    }
    pub(crate) fn get_mut(&mut self, k: &str) -> Option<&mut J> {
        match self {
            J::Obj(m) => m.iter_mut().rev().find(|(x, _)| x == k).map(|(_, v)| v),
            _ => None,
        }
    }
    pub(crate) fn str(&self, k: &str) -> Option<&str> {
        match self.get(k) {
            Some(J::Str(s)) => Some(s),
            _ => None,
        }
    }
    pub(crate) fn obj(pairs: Vec<(&str, J)>) -> J {
        J::Obj(pairs.into_iter().map(|(k, v)| (k.to_string(), v)).collect())
    }
}

impl<'de> Deserialize<'de> for J {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<J, D::Error> {
        struct V;
        impl<'de> Visitor<'de> for V {
            type Value = J;
            fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                f.write_str("JSON")
            }
            fn visit_unit<E>(self) -> Result<J, E> {
                Ok(J::Null)
            }
            fn visit_none<E>(self) -> Result<J, E> {
                Ok(J::Null)
            }
            fn visit_bool<E>(self, b: bool) -> Result<J, E> {
                Ok(J::Bool(b))
            }
            fn visit_i64<E>(self, n: i64) -> Result<J, E> {
                Ok(J::Num(n.into()))
            }
            fn visit_u64<E>(self, n: u64) -> Result<J, E> {
                Ok(J::Num(n.into()))
            }
            fn visit_f64<E: serde::de::Error>(self, n: f64) -> Result<J, E> {
                serde_json::Number::from_f64(n).map(J::Num).ok_or_else(|| E::custom("číslo"))
            }
            fn visit_str<E>(self, s: &str) -> Result<J, E> {
                Ok(J::Str(s.to_string()))
            }
            fn visit_string<E>(self, s: String) -> Result<J, E> {
                Ok(J::Str(s))
            }
            fn visit_seq<A: SeqAccess<'de>>(self, mut a: A) -> Result<J, A::Error> {
                let mut v = Vec::new();
                while let Some(x) = a.next_element()? {
                    v.push(x);
                }
                Ok(J::Arr(v))
            }
            fn visit_map<A: MapAccess<'de>>(self, mut a: A) -> Result<J, A::Error> {
                let mut v = Vec::new();
                while let Some((k, x)) = a.next_entry::<String, J>()? {
                    v.push((k, x));
                }
                Ok(J::Obj(v))
            }
        }
        d.deserialize_any(V)
    }
}

impl Serialize for J {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        match self {
            J::Null => s.serialize_unit(),
            J::Bool(b) => s.serialize_bool(*b),
            J::Num(n) => n.serialize(s),
            J::Str(x) => s.serialize_str(x),
            J::Arr(v) => {
                let mut q = s.serialize_seq(Some(v.len()))?;
                for x in v {
                    q.serialize_element(x)?;
                }
                q.end()
            }
            J::Obj(m) => {
                let mut q = s.serialize_map(Some(m.len()))?;
                for (k, x) in m {
                    q.serialize_entry(k, x)?;
                }
                q.end()
            }
        }
    }
}

// ---------- čistá logika ----------

/// Je to hook Ledge? → (port, cesta). Jen http na loopback s cestou mostu.
pub fn ours(hook: &J) -> Option<(u16, &str)> {
    if hook.str("type") != Some("http") {
        return None;
    }
    let rest = hook.str("url")?.strip_prefix("http://")?;
    let rest = rest.strip_prefix("127.0.0.1:").or_else(|| rest.strip_prefix("localhost:"))?;
    let (port, path) = rest.split_at(rest.find('/')?);
    matches!(path, "/permission" | "/event" | "/question" | "/plan").then_some(())?;
    Some((port.parse().ok()?, path))
}

fn want_url(port: u16, path: &str) -> String {
    format!("http://127.0.0.1:{port}{path}")
}

pub(crate) fn parse(text: &str) -> Result<J, String> {
    let t = text.trim_start_matches('\u{feff}');
    if t.trim().is_empty() {
        return Ok(J::Obj(vec![]));
    }
    let v: J = serde_json::from_str(t).map_err(|e| crate::errors::json(crate::i18n::tr("settings.json Clauda", "Claude's settings.json"), &e))?;
    match v {
        J::Obj(_) => Ok(v),
        _ => Err(crate::i18n::tr("settings.json není JSON objekt.", "settings.json isn't a JSON object.").into()),
    }
}

pub(crate) fn render(v: &J, original: &str) -> String {
    let mut out = serde_json::to_string_pretty(v).unwrap_or_default();
    if original.is_empty() || original.ends_with('\n') {
        out.push('\n');
    }
    out
}

/// Matcher skupiny; prázdný = žádný (Claude Code ho bere jako „vše“).
fn matcher(group: &J) -> Option<&str> {
    group.str("matcher").filter(|m| !m.is_empty())
}

/// Je ve skupinách `[{matcher?, hooks:[…]}]` s matcherem `m` hook Ledge s URL `url`?
fn has_hook(groups: &[J], url: &str, m: Option<&str>) -> bool {
    groups.iter().any(|g| matcher(g) == m && matches!(g.get("hooks"), Some(J::Arr(hs)) if hs.iter().any(|h| ours(h).is_some() && h.str("url") == Some(url))))
}

/// Odebere z pole skupin hooky, pro které `drop(matcher skupiny, hook)` vrátí true. Skupina, která
/// tím zůstane bez hooků, zmizí; skupiny bez Ledge hooků se nemění.
fn strip(groups: &mut Vec<J>, drop: &dyn Fn(Option<&str>, &J) -> bool) {
    groups.retain_mut(|g| {
        let m = matcher(g).map(str::to_string);
        let Some(J::Arr(hs)) = g.get_mut("hooks") else { return true };
        let before = hs.len();
        hs.retain(|h| !drop(m.as_deref(), h));
        !(hs.is_empty() && before > 0)
    });
}

fn hooks_obj(root: &mut J, create: bool) -> Result<Option<&mut Vec<(String, J)>>, String> {
    let J::Obj(m) = root else { unreachable!("parse vrací objekt") };
    if !m.iter().any(|(k, _)| k == "hooks") {
        if !create {
            return Ok(None);
        }
        m.push(("hooks".into(), J::Obj(vec![])));
    }
    match root.get_mut("hooks") {
        Some(J::Obj(h)) => Ok(Some(h)),
        _ => Err(crate::i18n::tr("Klíč „hooks“ v settings.json nemá tvar objektu — uprav ho ručně.", "The “hooks” key in settings.json isn't an object — fix it by hand.").into()),
    }
}

/// Přidá chybějící hooky Ledge (staré s jiným portem nahradí). Vrací (nový text, změněno?).
pub fn merge(text: &str, port: u16) -> Result<(String, bool), String> {
    let old = parse(text)?;
    let mut root = old.clone();
    let hooks = hooks_obj(&mut root, true)?.expect("vytvořeno");
    for (ev, path, timeout, m) in EVENTS {
        let url = want_url(port, path);
        if !hooks.iter().any(|(k, _)| k == ev) {
            hooks.push((ev.to_string(), J::Arr(vec![])));
        }
        let Some((_, J::Arr(groups))) = hooks.iter_mut().rev().find(|(k, _)| k == ev) else {
            return Err(trf!("hooks.{ev} v settings.json není pole — uprav ho ručně.", "hooks.{ev} in settings.json isn't an array — fix it by hand."));
        };
        // náš hook, který nesedí žádnému řádku téhle události (jiný port / matcher / cesta) → pryč
        let wanted: Vec<(String, Option<&str>)> = EVENTS.iter().filter(|(e, ..)| e == ev).map(|(_, p, _, wm)| (want_url(port, p), *wm)).collect();
        strip(groups, &|gm, h| ours(h).is_some() && !wanted.iter().any(|(u, wm)| h.str("url") == Some(u.as_str()) && gm == *wm));
        if !has_hook(groups, &url, *m) {
            let hook = J::obj(vec![("type", J::Str("http".into())), ("url", J::Str(url)), ("timeout", J::Num((*timeout).into()))]);
            let mut group = m.map(|m| vec![("matcher", J::Str(m.into()))]).unwrap_or_default();
            group.push(("hooks", J::Arr(vec![hook])));
            groups.push(J::obj(group));
        }
    }
    let changed = root != old;
    Ok((if changed { render(&root, text) } else { text.to_string() }, changed))
}

/// Odebere všechny hooky Ledge (jakýkoli port). Události, které tím zůstanou prázdné, zmizí.
pub fn remove(text: &str) -> Result<(String, bool), String> {
    let old = parse(text)?;
    let mut root = old.clone();
    if let Some(hooks) = hooks_obj(&mut root, false)? {
        hooks.retain_mut(|(_, v)| {
            let J::Arr(groups) = v else { return true };
            let before = groups.len();
            strip(groups, &|_, h| ours(h).is_some());
            !(groups.is_empty() && before > 0)
        });
        if hooks.is_empty()
            && let J::Obj(m) = &mut root
        {
            m.retain(|(k, _)| k != "hooks");
        }
    }
    let changed = root != old;
    Ok((if changed { render(&root, text) } else { text.to_string() }, changed))
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HooksStatus {
    /// none | partial | full | error
    pub state: &'static str,
    /// události, kterým hook Ledge (na aktuálním portu) chybí
    pub missing: Vec<String>,
    pub error: Option<String>,
    pub path: String,
    /// lidská hláška ke stavu `partial` (chybí jen hook otázek → „Doplnit otázky Clauda“)
    pub hint: Option<String>,
}

/// Hláška k chybějícím hookům pro UI; `None` = nic zvláštního (vše zapojené / nic zapojené).
pub fn hint(missing: &[String]) -> Option<String> {
    match missing {
        [] => None,
        [one] if OPTIONAL_EVENTS.contains(&one.as_str()) => Some(
            crate::i18n::tr(
                "K dispozici je aktualizace hooků (hlášení chyb nástrojů) — klikni „Zapojit“ znovu.",
                "A hook update is available (tool error reporting) — click “Connect” again.",
            )
            .into(),
        ),
        [one] if one == QUESTIONS_EVENT => {
            Some(
                crate::i18n::tr(
                    "Chybí hook pro otázky Clauda (zapojený starší verzí Ledge) — klikni „Doplnit otázky Clauda“.",
                    "The hook for Claude's questions is missing (connected by an older Ledge) — click “Add Claude's questions”.",
                )
                .into(),
            )
        }
        [one] if one == PLAN_EVENT => Some(
            crate::i18n::tr(
                "Chybí hook pro review plánu (zapojený starší verzí Ledge) — klikni „Doplnit review plánu“.",
                "The hook for plan review is missing (connected by an older Ledge) — click “Add plan review”.",
            )
            .into(),
        ),
        [a, b] if [a, b].iter().all(|x| *x == QUESTIONS_EVENT || *x == PLAN_EVENT) => Some(
            crate::i18n::tr(
                "Chybí hooky pro otázky Clauda a review plánu (zapojené starší verzí Ledge) — klikni „Doplnit otázky a review plánu“.",
                "The hooks for Claude's questions and plan review are missing (connected by an older Ledge) — click “Add questions and plan review”.",
            )
            .into(),
        ),
        _ if missing.len() == EVENTS.len() => None,
        _ => Some(trf!("Chybí: {}", "Missing: {}", missing.join(", "))),
    }
}

pub fn status(text: &str, port: u16) -> (&'static str, Vec<String>) {
    let Ok(root) = parse(text) else { return ("error", vec![]) };
    let has = |ev: &str, path: &str, m: Option<&str>| {
        matches!(root.get("hooks").and_then(|h| h.get(ev)), Some(J::Arr(gs)) if has_hook(gs, &want_url(port, path), m))
    };
    let missing: Vec<String> = EVENTS.iter().filter(|(ev, p, _, m)| !has(ev, p, *m)).map(|(ev, _, _, m)| label(ev, *m)).collect();
    let required = missing.iter().filter(|m| !OPTIONAL_EVENTS.contains(&m.as_str())).count();
    let state = match required {
        0 => "full",
        _ if missing.len() == EVENTS.len() => "none",
        _ => "partial",
    };
    (state, missing)
}

// ---------- soubor ----------

pub fn settings_path() -> Option<PathBuf> {
    crate::sessionlive::claude_dir().map(|d| d.join("settings.json"))
}

pub(crate) fn read(path: &Path) -> Result<String, String> {
    match std::fs::read_to_string(path) {
        Ok(t) => Ok(t),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(String::new()),
        Err(e) => Err(crate::errors::io(&trf!("{} nejde přečíst", "Can't read {}", path.display()), &e)),
    }
}

pub fn status_at(path: &Path, port: u16) -> HooksStatus {
    let p = path.display().to_string();
    match read(path) {
        Ok(t) => match parse(&t) {
            Ok(_) => {
                let (state, missing) = status(&t, port);
                HooksStatus { state, hint: hint(&missing), missing, error: None, path: p }
            }
            Err(e) => HooksStatus { state: "error", missing: vec![], error: Some(e), path: p, hint: None },
        },
        Err(e) => HooksStatus { state: "error", missing: vec![], error: Some(e), path: p, hint: None },
    }
}

/// `status_at` pro snapshot: soubor se čte a parsuje jen po změně (mtime, délka) nebo portu.
pub fn status_cached(port: u16) -> HooksStatus {
    use parking_lot::Mutex;
    // jazyk v klíči: hláška (`hint`, chyby) je v jazyce UI
    type Key = (Option<std::time::SystemTime>, u64, u16, crate::uiconf::Lang);
    static CACHE: Mutex<Option<(Key, HooksStatus)>> = Mutex::new(None);
    let Some(path) = settings_path() else {
        return HooksStatus { state: "error", missing: vec![], error: Some(crate::i18n::tr("Nevím, kde je složka .claude.", "I don't know where the .claude folder is.").into()), path: String::new(), hint: None };
    };
    let meta = std::fs::metadata(&path).ok();
    let key = (meta.as_ref().and_then(|m| m.modified().ok()), meta.map_or(0, |m| m.len()), port, crate::i18n::lang());
    let mut c = CACHE.lock();
    if let Some((k, st)) = c.as_ref()
        && *k == key
    {
        return st.clone();
    }
    let st = status_at(&path, port);
    *c = Some((key, st.clone()));
    st
}

/// Zapíše výsledek `f` (merge/remove) atomicky; před první změnou existujícího souboru záloha.
pub fn apply(path: &Path, f: impl FnOnce(&str) -> Result<(String, bool), String>) -> Result<bool, String> {
    let text = read(path)?;
    let (new, changed) = f(&text)?;
    if !changed {
        return Ok(false);
    }
    let dir = path.parent().ok_or_else(|| crate::i18n::tr("settings.json nemá složku", "settings.json has no folder"))?;
    std::fs::create_dir_all(dir).map_err(|e| crate::errors::io(&trf!("Složku {} nejde založit", "Can't create the folder {}", dir.display()), &e))?;
    let backup = dir.join(BACKUP);
    if path.exists() && !backup.exists() {
        std::fs::copy(path, &backup).map_err(|e| crate::errors::io(&trf!("Zálohu {} nejde vytvořit", "Can't create the backup {}", backup.display()), &e))?;
    }
    let tmp = dir.join("settings.json.ledge-tmp");
    std::fs::write(&tmp, new.as_bytes()).map_err(|e| crate::errors::io(&trf!("{} nejde zapsat", "Can't write {}", path.display()), &e))?;
    std::fs::rename(&tmp, path).map_err(|e| {
        let _ = std::fs::remove_file(&tmp);
        crate::errors::io(&trf!("{} nejde zapsat", "Can't write {}", path.display()), &e)
    })?;
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{Value, json};

    fn v(t: &str) -> Value {
        serde_json::from_str(t).unwrap()
    }

    const FOREIGN: &str = r#"{
  "model": "opus",
  "permissions": { "allow": ["Bash(git status)"] },
  "hooks": {
    "PostToolUse": [
      { "matcher": "Edit|Write", "hooks": [{ "type": "command", "command": "npx prettier --write" }] }
    ],
    "Stop": [
      { "hooks": [{ "type": "http", "url": "http://127.0.0.1:47000/event", "timeout": 5 }, { "type": "command", "command": "echo hotovo" }] }
    ]
  },
  "zzz": 1.5
}
"#;

    #[test]
    fn merge_adds_keeps_foreign_and_is_idempotent() {
        let (out, changed) = merge(FOREIGN, 47800).unwrap();
        assert!(changed);
        let j = v(&out);
        // cizí klíče, pořadí a hooky zůstanou
        assert_eq!(j["model"], "opus");
        assert_eq!(j["zzz"], 1.5);
        assert!(out.find("\"model\"").unwrap() < out.find("\"permissions\"").unwrap() && out.find("\"hooks\"").unwrap() < out.find("\"zzz\"").unwrap());
        assert_eq!(j["hooks"]["PostToolUse"][0]["hooks"][0]["command"], "npx prettier --write");
        // starý port Ledge nahrazen, cizí command ve stejné skupině zůstal
        assert_eq!(j["hooks"]["Stop"][0]["hooks"], json!([{ "type": "command", "command": "echo hotovo" }]));
        assert_eq!(j["hooks"]["Stop"][1]["hooks"][0], json!({ "type": "http", "url": "http://127.0.0.1:47800/event", "timeout": 5 }));
        assert_eq!(j["hooks"]["PermissionRequest"][0]["hooks"][0]["url"], "http://127.0.0.1:47800/permission");
        assert_eq!(j["hooks"]["PermissionRequest"][0]["hooks"][0]["timeout"], 600);
        for (ev, ..) in EVENTS {
            assert!(j["hooks"][ev].is_array(), "{ev}");
        }
        assert!(out.starts_with("{\n  \"model\""), "odsazení 2 mezery");
        assert!(out.ends_with("}\n"));
        assert_eq!(status(&out, 47800), ("full", vec![]));
        // podruhé nic
        let (again, changed) = merge(&out, 47800).unwrap();
        assert!(!changed);
        assert_eq!(again, out);
    }

    #[test]
    fn remove_takes_only_ours() {
        let (with, _) = merge(FOREIGN, 47800).unwrap();
        let (out, changed) = remove(&with).unwrap();
        assert!(changed);
        let j = v(&out);
        assert_eq!(j["hooks"]["PostToolUse"][0]["hooks"][0]["command"], "npx prettier --write");
        assert_eq!(j["hooks"]["Stop"], json!([{ "hooks": [{ "type": "command", "command": "echo hotovo" }] }]));
        assert!(j["hooks"].get("PermissionRequest").is_none() && j["hooks"].get("SubagentStop").is_none());
        assert_eq!(status(&out, 47800).0, "none");
        assert!(!remove(&out).unwrap().1, "podruhé nic");
        // jen Ledge hooky → zmizí i prázdné „hooks“
        let (only, _) = merge("{}", 47800).unwrap();
        assert_eq!(v(&remove(&only).unwrap().0), json!({}));
    }

    #[test]
    fn missing_only_optional_hook_is_full_with_update_hint() {
        let (full, _) = merge("{}", 47800).unwrap();
        let old = full.replace("PostToolUseFailure", "XPostToolUseFailure");
        let (st, missing) = status(&old, 47800);
        assert_eq!((st, missing.clone()), ("full", vec!["PostToolUseFailure".to_string()]));
        assert!(hint(&missing).unwrap().contains("aktualizace") || hint(&missing).unwrap().contains("update"));
    }

    #[test]
    fn status_table() {
        let one = r#"{"hooks":{"Stop":[{"hooks":[{"type":"http","url":"http://127.0.0.1:47800/event"}]}]}}"#;
        let (st, missing) = status(one, 47800);
        assert_eq!(st, "partial");
        assert_eq!(missing.len(), EVENTS.len() - 1);
        assert!(!missing.contains(&"Stop".to_string()));
        assert_eq!(status(one, 47801).0, "none", "jiný port = chybí");
        assert_eq!(status("", 47800).0, "none");
        assert_eq!(status("{nejson", 47800).0, "error");
        // localhost se počítá jako náš, ale chce se 127.0.0.1 → merge ho nahradí
        let lh = r#"{"hooks":{"Stop":[{"hooks":[{"type":"http","url":"http://localhost:47800/event"}]}]}}"#;
        let (out, _) = merge(lh, 47800).unwrap();
        assert_eq!(v(&out)["hooks"]["Stop"].as_array().unwrap().len(), 1);
    }

    #[test]
    fn question_hook_has_matcher_and_keeps_foreign_pretooluse() {
        let foreign = r#"{"hooks":{"PreToolUse":[{"matcher":"Bash","hooks":[{"type":"command","command":"guard.sh"}]}]}}"#;
        let (out, changed) = merge(foreign, 47800).unwrap();
        assert!(changed);
        let j = v(&out);
        assert_eq!(j["hooks"]["PreToolUse"][0], json!({"matcher":"Bash","hooks":[{"type":"command","command":"guard.sh"}]}), "cizí PreToolUse zůstane");
        assert_eq!(j["hooks"]["PreToolUse"][1], json!({"matcher":"AskUserQuestion","hooks":[{"type":"http","url":"http://127.0.0.1:47800/question","timeout":600}]}));
        assert_eq!(j["hooks"]["PreToolUse"][2], json!({"matcher":"ExitPlanMode","hooks":[{"type":"http","url":"http://127.0.0.1:47800/plan","timeout":600}]}));
        assert_eq!(j["hooks"]["PreToolUse"].as_array().unwrap().len(), 3, "otázky a plán se navzájem neodebraly");
        assert!(out.find("\"AskUserQuestion\"").unwrap() < out.find("/question").unwrap(), "matcher před hooks");
        assert_eq!(status(&out, 47800), ("full", vec![]));
        assert!(!merge(&out, 47800).unwrap().1, "idempotentní");
        // náš hook pod jiným matcherem (ručně přesunutý) = chybí → merge ho přesune pod AskUserQuestion
        let moved = r#"{"hooks":{"PreToolUse":[{"matcher":"Bash","hooks":[{"type":"http","url":"http://127.0.0.1:47800/question"},{"type":"command","command":"guard.sh"}]}]}}"#;
        assert!(status(moved, 47800).1.contains(&"PreToolUse".to_string()));
        let j = v(&merge(moved, 47800).unwrap().0);
        assert_eq!(j["hooks"]["PreToolUse"][0], json!({"matcher":"Bash","hooks":[{"type":"command","command":"guard.sh"}]}));
        assert_eq!(j["hooks"]["PreToolUse"][1]["matcher"], "AskUserQuestion");
        assert_eq!(j["hooks"]["PreToolUse"][2]["matcher"], "ExitPlanMode");
        // remove vezme jen náš, cizí PreToolUse nechá
        let j = v(&remove(&out).unwrap().0);
        assert_eq!(j["hooks"]["PreToolUse"], json!([{"matcher":"Bash","hooks":[{"type":"command","command":"guard.sh"}]}]));
    }

    #[test]
    fn old_install_without_questions_is_partial_with_hint() {
        let full = merge("{}", 47800).unwrap().0;
        let mut j: Value = v(&full);
        j["hooks"]["PreToolUse"].as_array_mut().unwrap().retain(|g| g["matcher"] != "AskUserQuestion");
        let old = serde_json::to_string(&j).unwrap();
        let (st, missing) = status(&old, 47800);
        assert_eq!((st, missing.as_slice()), ("partial", ["PreToolUse".to_string()].as_slice()));
        assert!(hint(&missing).unwrap().contains("Doplnit otázky Clauda"));
        let (fixed, changed) = merge(&old, 47800).unwrap();
        assert!(changed);
        assert_eq!(status(&fixed, 47800).0, "full");
        let cases: [(&[&str], bool); 3] = [(&[], false), (&["Stop", "PreToolUse"], true), (&["PreToolUse"], true)];
        for (m, some) in cases {
            let m: Vec<String> = m.iter().map(|x| x.to_string()).collect();
            assert_eq!(hint(&m).is_some(), some, "{m:?}");
        }
        let all: Vec<String> = EVENTS.iter().map(|(e, ..)| e.to_string()).collect();
        assert_eq!(hint(&all), None, "nezapojené = žádná zvláštní hláška");
    }

    /// Přechod z Ledge bez review plánu: settings.json s hooky vlny 4 (otázky ano, plán ne) + cizí PreToolUse.
    /// `merge` doplní jen skupinu `ExitPlanMode`, nic jiného nepohne, podruhé nic; `remove` vezme obě naše.
    #[test]
    fn upgrade_adds_plan_hook_safely() {
        let v4 = r#"{
  "hooks": {
    "PreToolUse": [
      { "matcher": "Bash", "hooks": [{ "type": "command", "command": "guard.sh" }] },
      { "matcher": "AskUserQuestion", "hooks": [{ "type": "http", "url": "http://127.0.0.1:47800/question", "timeout": 600 }] }
    ]
  }
}
"#;
        let mut j: Value = v(&merge("{}", 47800).unwrap().0);
        j["hooks"]["PreToolUse"] = v(v4)["hooks"]["PreToolUse"].clone();
        let old = serde_json::to_string_pretty(&j).unwrap();
        let (st, missing) = status(&old, 47800);
        assert_eq!((st, missing.as_slice()), ("partial", [PLAN_EVENT.to_string()].as_slice()));
        assert!(hint(&missing).unwrap().contains("Doplnit review plánu"));
        let (out, changed) = merge(&old, 47800).unwrap();
        assert!(changed);
        let pre = v(&out)["hooks"]["PreToolUse"].clone();
        assert_eq!(pre[0], v(v4)["hooks"]["PreToolUse"][0], "cizí hook netknutý");
        assert_eq!(pre[1], v(v4)["hooks"]["PreToolUse"][1], "otázky netknuté");
        assert_eq!(pre[2], json!({"matcher":"ExitPlanMode","hooks":[{"type":"http","url":"http://127.0.0.1:47800/plan","timeout":600}]}));
        assert_eq!(status(&out, 47800), ("full", vec![]));
        assert!(!merge(&out, 47800).unwrap().1, "idempotentní");
        // jiný port: obě naše skupiny přepsané na nový port, žádná navíc
        let moved = v(&merge(&out, 47801).unwrap().0)["hooks"]["PreToolUse"].clone();
        assert_eq!(moved.as_array().unwrap().len(), 3);
        assert_eq!((moved[1]["hooks"][0]["url"].as_str(), moved[2]["hooks"][0]["url"].as_str()), (Some("http://127.0.0.1:47801/question"), Some("http://127.0.0.1:47801/plan")));
        // náš /plan pod špatným matcherem → přesune se pod ExitPlanMode
        let wrong = out.replace("\"ExitPlanMode\"", "\"Edit\"");
        assert_eq!(status(&wrong, 47800).1, vec![PLAN_EVENT.to_string()]);
        let fixed = v(&merge(&wrong, 47800).unwrap().0)["hooks"]["PreToolUse"].clone();
        assert_eq!((fixed.as_array().unwrap().len(), fixed[2]["matcher"].as_str()), (3, Some("ExitPlanMode")));
        // remove: jen cizí zůstane
        assert_eq!(v(&remove(&out).unwrap().0)["hooks"]["PreToolUse"], json!([{"matcher":"Bash","hooks":[{"type":"command","command":"guard.sh"}]}]));
        // hlášky ke chybějícím doplňkům PreToolUse
        let both = vec![QUESTIONS_EVENT.to_string(), PLAN_EVENT.to_string()];
        assert!(hint(&both).unwrap().contains("otázky a review plánu"));
    }

    #[test]
    fn ours_table() {
        let h = |t: &str, u: &str| J::obj(vec![("type", J::Str(t.into())), ("url", J::Str(u.into()))]);
        let cases = [
            (h("http", "http://127.0.0.1:47800/event"), Some((47800, "/event"))),
            (h("http", "http://localhost:1/permission"), Some((1, "/permission"))),
            (h("http", "http://127.0.0.1:47800/question"), Some((47800, "/question"))),
            (h("http", "http://127.0.0.1:47800/plan"), Some((47800, "/plan"))),
            (h("http", "http://127.0.0.1:47800/jine"), None),
            (h("http", "http://example.com:47800/event"), None),
            (h("http", "https://127.0.0.1:47800/event"), None),
            (h("command", "http://127.0.0.1:47800/event"), None),
            (h("http", "http://127.0.0.1:x/event"), None),
        ];
        for (hook, want) in cases {
            assert_eq!(ours(&hook), want, "{hook:?}");
        }
    }

    #[test]
    fn broken_shapes_are_errors_not_overwrites() {
        assert!(merge("[1]", 1).is_err());
        assert!(merge("{nejson", 1).is_err());
        assert!(merge(r#"{"hooks":[]}"#, 1).is_err());
        assert!(merge(r#"{"hooks":{"Stop":{}}}"#, 1).is_err());
        // BOM a prázdný soubor projdou
        assert!(merge("\u{feff}{}", 1).unwrap().1);
        assert!(merge("", 1).unwrap().0.ends_with("}\n"));
    }

    #[test]
    fn apply_writes_backup_once_and_atomically() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("settings.json");
        // soubor neexistuje → vznikne, záloha ne (není co zálohovat)
        assert!(apply(&path, |t| merge(t, 47800)).unwrap());
        assert!(!dir.path().join(BACKUP).exists());
        assert_eq!(status_at(&path, 47800).state, "full");
        // existující soubor: záloha jednou, pak se nepřepisuje
        std::fs::write(&path, FOREIGN).unwrap();
        assert!(apply(&path, |t| merge(t, 47800)).unwrap());
        assert_eq!(std::fs::read_to_string(dir.path().join(BACKUP)).unwrap(), FOREIGN);
        assert!(apply(&path, remove).unwrap());
        assert_eq!(std::fs::read_to_string(dir.path().join(BACKUP)).unwrap(), FOREIGN);
        assert!(!apply(&path, remove).unwrap(), "beze změny se nezapisuje");
        assert!(!dir.path().join("settings.json.ledge-tmp").exists());
        // nevalidní soubor zůstane netknutý
        std::fs::write(&path, "{rozbité").unwrap();
        assert!(apply(&path, |t| merge(t, 47800)).is_err());
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "{rozbité");
        assert_eq!(status_at(&path, 47800).state, "error");
    }
}
