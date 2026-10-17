//! Chyba validace configu s cestou k poli (`projects.2.run`, `notes.sync.remote`) — Nastavení podle ní
//! ukáže chybu přímo u pole a přepne na správný panel. Text zůstává česky a stejný jako dřív
//! (`config::validate` vrací jen `message`, zpětně kompatibilně).

use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FieldError {
    /// Cesta k poli v JSON configu oddělená tečkami, indexy pole jako čísla. `None` = nejde přiřadit.
    pub field: Option<String>,
    pub message: String,
}

pub fn fe(field: impl Into<String>, message: impl Into<String>) -> FieldError {
    FieldError { field: Some(field.into()), message: message.into() }
}

/// Cesta z textu chyby, který začíná `sekce.pole:` nebo `sekce.pole musí…` (validace jiných modulů
/// vracejí jen String). Bere jen tvar `[a-zA-Z0-9_.]` s aspoň jednou tečkou, ne název souboru `*.json`.
pub fn field_from_message(msg: &str) -> Option<String> {
    let head: String = msg.chars().take_while(|c| c.is_ascii_alphanumeric() || *c == '.' || *c == '_').collect();
    let head = head.trim_end_matches('.');
    let rest = &msg[head.len()..];
    let followed = rest.starts_with(':') || rest.starts_with(' ');
    let path_like = head.contains('.') && !head.ends_with(".json") && head.chars().next().is_some_and(|c| c.is_ascii_lowercase());
    (followed && path_like).then(|| head.to_string())
}

impl From<String> for FieldError {
    fn from(message: String) -> Self {
        FieldError { field: field_from_message(&message), message }
    }
}

impl From<&str> for FieldError {
    fn from(message: &str) -> Self {
        message.to_string().into()
    }
}

impl From<FieldError> for String {
    fn from(e: FieldError) -> Self {
        e.message
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn field_from_message_table() {
        let cases = [
            ("notes.sync.intervalMin musí být 1–120 minut.", Some("notes.sync.intervalMin")),
            ("appearance.accent: chce #rrggbb.", Some("appearance.accent")),
            ("strip.opacity musí být 0–1.", Some("strip.opacity")),
            ("Projekt „x“ nemá id.", None),
            ("Odkaz č. 1 nemá název.", None),
            ("config.json: expected value", None),
            ("bezTecky: něco", None),
            ("", None),
        ];
        for (msg, want) in cases {
            assert_eq!(field_from_message(msg).as_deref(), want, "{msg}");
        }
    }

    #[test]
    fn conversions_keep_message() {
        let e: FieldError = "update.repo „x“ musí mít tvar owner/name".into();
        assert_eq!(e.field.as_deref(), Some("update.repo"));
        let s: String = e.clone().into();
        assert_eq!(s, e.message);
        let v = serde_json::to_value(fe("projects.0.run", "chybí")).unwrap();
        assert_eq!(v, serde_json::json!({ "field": "projects.0.run", "message": "chybí" }));
    }

    /// Každá chyba `config::validate_field` míří na pole, které Nastavení umí ukázat.
    #[test]
    fn validate_field_paths_table() {
        use crate::config::{Config, validate_field};
        let base = || -> serde_json::Value { serde_json::to_value(Config::default()).unwrap() };
        let proj = |extra: serde_json::Value| {
            let mut p = serde_json::json!({ "id": "a", "name": "A", "kind": "dev", "run": "npm run dev" });
            p.as_object_mut().unwrap().extend(extra.as_object().unwrap().clone());
            p
        };
        let cases: Vec<(&str, serde_json::Value)> = vec![
            ("projects.0.id", serde_json::json!({ "projects": [proj(serde_json::json!({ "id": " " }))] })),
            ("projects.1.id", serde_json::json!({ "projects": [proj(serde_json::json!({})), proj(serde_json::json!({}))] })),
            ("projects.0.run", serde_json::json!({ "projects": [proj(serde_json::json!({ "run": "" }))] })),
            ("projects.0.exe", serde_json::json!({ "projects": [proj(serde_json::json!({ "kind": "app" }))] })),
            ("projects.0.accent", serde_json::json!({ "projects": [proj(serde_json::json!({ "accent": "red" }))] })),
            ("personal.weather.lon", serde_json::json!({ "personal": { "weather": { "lat": 50.0 } } })),
            ("personal.weather.lat", serde_json::json!({ "personal": { "weather": { "lat": 95.0, "lon": 1.0 } } })),
            ("personal.links.0.target", serde_json::json!({ "personal": { "links": [{ "name": "X", "target": " " }] } })),
            ("notes.kinds.1.color", serde_json::json!({ "notes": { "kinds": [{ "id": "a", "label": "A" }, { "id": "b", "label": "B", "color": "zelena" }] } })),
            ("notes.sync.intervalMin", serde_json::json!({ "notes": { "sync": { "intervalMin": 0 } } })),
            ("notes.sync.remote", serde_json::json!({ "notes": { "sync": { "remote": "http://x/y" } } })),
            ("update.repo", serde_json::json!({ "update": { "repo": "bez-lomitka" } })),
            ("strip.opacity", serde_json::json!({ "strip": { "opacity": 2.0 } })),
            ("drawer.trigger", serde_json::json!({ "drawer": { "trigger": "off" } })),
            ("hotkeys.note", serde_json::json!({ "hotkeys": { "enabled": true, "note": "Ctrl+Alt+D" } })),
        ];
        for (want, patch) in cases {
            let mut v = base();
            for (k, val) in patch.as_object().unwrap() {
                if let (Some(dst), Some(src)) = (v[k].as_object_mut(), val.as_object()) {
                    for (kk, vv) in src {
                        match (dst.get_mut(kk).and_then(|d| d.as_object_mut()), vv.as_object()) {
                            (Some(d2), Some(s2)) => d2.extend(s2.clone()),
                            _ => {
                                dst.insert(kk.clone(), vv.clone());
                            }
                        }
                    }
                } else {
                    v[k] = val.clone();
                }
            }
            let cfg: Config = serde_json::from_value(v).unwrap();
            let e = validate_field(&cfg).unwrap_err();
            assert_eq!(e.field.as_deref(), Some(want), "{}", e.message);
            assert!(!e.message.is_empty());
        }
        assert!(validate_field(&Config::default()).is_ok());
    }
}
