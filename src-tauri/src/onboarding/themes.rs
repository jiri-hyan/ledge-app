//! Průvodce, krok Vzhled: hotové motivy se čtou ze schématu configu (`appearance.theme`, enum
//! z `schemars`) — průvodce tak nabízí přesně to, co umí sekce `appearance`, a nic nevykresluje sám.
//! Když config `appearance` nemá, seznam je prázdný a krok se přeskočí.

use serde::Serialize;
use serde_json::Value;

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct ThemeOption {
    pub value: String,
    /// popis varianty z dokumentačního komentáře (prázdné = jen hodnota)
    pub label: String,
}

fn deref<'a>(root: &'a Value, v: &'a Value) -> &'a Value {
    let r = v.get("$ref").or_else(|| v.get("allOf").and_then(|a| a.get(0)).and_then(|x| x.get("$ref")));
    match r.and_then(Value::as_str).and_then(|r| r.strip_prefix("#/")) {
        Some(path) => root.pointer(&format!("/{path}")).unwrap_or(v),
        None => v,
    }
}

/// Hodnoty `appearance.theme` ze schématu. Čistá funkce.
pub fn theme_options(schema: &Value) -> Vec<ThemeOption> {
    let Some(app) = schema.pointer("/properties/appearance") else { return vec![] };
    let Some(theme) = deref(schema, app).pointer("/properties/theme") else { return vec![] };
    let theme = deref(schema, theme);
    let plain = |v: &Value| v.as_str().map(|s| ThemeOption { value: s.into(), label: String::new() });
    if let Some(e) = theme.get("enum").and_then(Value::as_array) {
        return e.iter().filter_map(plain).collect();
    }
    let variants = theme.get("oneOf").or_else(|| theme.get("anyOf")).and_then(Value::as_array);
    variants
        .into_iter()
        .flatten()
        .flat_map(|v| {
            let label = v.get("description").and_then(Value::as_str).unwrap_or("").trim().to_string();
            let values: Vec<String> = match v.get("const").and_then(Value::as_str) {
                Some(c) => vec![c.into()],
                None => v.get("enum").and_then(Value::as_array).map(|a| a.iter().filter_map(|x| x.as_str().map(String::from)).collect()).unwrap_or_default(),
            };
            values.into_iter().map(move |value| ThemeOption { value, label: label.clone() })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn enum_and_one_of_shapes() {
        let plain = serde_json::json!({
            "properties": { "appearance": { "$ref": "#/$defs/Appearance" } },
            "$defs": {
                "Appearance": { "properties": { "theme": { "$ref": "#/$defs/Theme" } } },
                "Theme": { "type": "string", "enum": ["dark", "light"] }
            }
        });
        let v: Vec<String> = theme_options(&plain).into_iter().map(|o| o.value).collect();
        assert_eq!(v, ["dark", "light"]);
        let documented = serde_json::json!({
            "properties": { "appearance": { "allOf": [{ "$ref": "#/$defs/A" }] } },
            "$defs": {
                "A": { "properties": { "theme": { "$ref": "#/$defs/T" } } },
                "T": { "oneOf": [ { "const": "dark", "description": "Tmavý" }, { "enum": ["midnight"], "description": "Půlnoc" } ] }
            }
        });
        let o = theme_options(&documented);
        assert_eq!(o, [ThemeOption { value: "dark".into(), label: "Tmavý".into() }, ThemeOption { value: "midnight".into(), label: "Půlnoc".into() }]);
    }

    #[test]
    fn missing_appearance_or_free_text_gives_nothing() {
        assert!(theme_options(&serde_json::json!({ "properties": {} })).is_empty());
        let free = serde_json::json!({ "properties": { "appearance": { "properties": { "theme": { "type": "string" } } } } });
        assert!(theme_options(&free).is_empty());
        // skutečné schéma: buď prázdné (sekce ještě není), nebo samé neprázdné hodnoty
        let real: Value = serde_json::from_str(&crate::config::schema_json()).unwrap();
        assert!(theme_options(&real).iter().all(|o| !o.value.is_empty()));
    }
}
