//! Pomůcky nad JSON Schema configu (`schemars`): neznámá pole ve vstupu (překlep = chyba, ne tiché
//! ignorování), sloučení změn (RFC 7396: objekt se slučuje, `null` maže, jinak nahradí), povolené
//! hodnoty výčtů (katalogy pro `*_get` a hlášky). Čisté funkce.

use schemars::JsonSchema;
use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::{Map, Value};

/// Schéma typu jako JSON.
pub fn of<T: JsonSchema>() -> Value {
    serde_json::to_value(schemars::schema_for!(T)).unwrap_or(Value::Null)
}

/// Rozbalí `$ref` / `allOf[0].$ref` / `anyOf` s jedním ne-null členem (Option) na cílovou definici.
pub fn deref<'a>(root: &'a Value, v: &'a Value) -> &'a Value {
    let mut cur = v;
    for _ in 0..8 {
        let r = cur.get("$ref").or_else(|| cur.get("allOf").and_then(|a| a.get(0)).and_then(|x| x.get("$ref")));
        if let Some(path) = r.and_then(Value::as_str).and_then(|r| r.strip_prefix("#/")) {
            match root.pointer(&format!("/{path}")) {
                Some(t) => {
                    cur = t;
                    continue;
                }
                None => return cur,
            }
        }
        // Option<T>: anyOf [T, null]
        if let Some(any) = cur.get("anyOf").and_then(Value::as_array) {
            let real: Vec<&Value> = any.iter().filter(|x| x.get("type").and_then(Value::as_str) != Some("null")).collect();
            if real.len() == 1 && (real[0].get("$ref").is_some() || real[0].get("properties").is_some()) {
                cur = real[0];
                continue;
            }
        }
        return cur;
    }
    cur
}

/// Hodnoty výčtu (enum / oneOf const / anyOf enum); `None` = není výčet.
pub fn enum_values(root: &Value, node: &Value) -> Option<Vec<String>> {
    let n = deref(root, node);
    if let Some(e) = n.get("enum").and_then(Value::as_array) {
        return Some(e.iter().filter_map(|x| x.as_str().map(String::from)).collect());
    }
    if let Some(c) = n.get("const").and_then(Value::as_str) {
        return Some(vec![c.to_string()]);
    }
    let variants = n.get("oneOf").or_else(|| n.get("anyOf")).and_then(Value::as_array)?;
    let mut out = Vec::new();
    for v in variants {
        let v = deref(root, v);
        if let Some(c) = v.get("const").and_then(Value::as_str) {
            out.push(c.to_string());
        } else if let Some(e) = v.get("enum").and_then(Value::as_array) {
            out.extend(e.iter().filter_map(|x| x.as_str().map(String::from)));
        }
    }
    (!out.is_empty()).then_some(out)
}

/// Katalog povolených hodnot: `{ "theme": [...], "strip.style": [...] }` (pole s výčtem, vnořeně).
pub fn catalog(root: &Value) -> Map<String, Value> {
    let mut out = Map::new();
    walk(root, root, "", &mut out, 0);
    out
}

fn walk(root: &Value, node: &Value, prefix: &str, out: &mut Map<String, Value>, depth: usize) {
    if depth > 4 {
        return;
    }
    let n = deref(root, node);
    let Some(props) = n.get("properties").and_then(Value::as_object) else { return };
    for (k, v) in props {
        let key = if prefix.is_empty() { k.clone() } else { format!("{prefix}.{k}") };
        if let Some(vals) = enum_values(root, v) {
            out.insert(key, Value::from(vals));
        } else if let Some(items) = deref(root, v).get("items")
            && let Some(vals) = enum_values(root, items)
        {
            out.insert(format!("{key}[]"), Value::from(vals));
        } else {
            walk(root, v, &key, out, depth + 1);
        }
    }
}

/// Neznámé pole ve vstupu (cesta) — podle `properties` schématu, vnořeně i v polích. Mapy
/// (`additionalProperties`) a volné hodnoty se nekontrolují.
pub fn unknown_key(root: &Value, node: &Value, input: &Value, path: &str) -> Option<String> {
    let n = deref(root, node);
    match input {
        Value::Object(m) => {
            let props = n.get("properties").and_then(Value::as_object)?;
            for (k, v) in m {
                let at = if path.is_empty() { k.clone() } else { format!("{path}.{k}") };
                match props.get(k) {
                    None => return Some(at),
                    Some(s) => {
                        if let Some(u) = unknown_key(root, s, v, &at) {
                            return Some(u);
                        }
                    }
                }
            }
            None
        }
        Value::Array(a) => {
            let items = n.get("items")?;
            a.iter().enumerate().find_map(|(i, v)| unknown_key(root, items, v, &format!("{path}.{i}")))
        }
        _ => None,
    }
}

/// Jména polí objektu ve schématu (pro hlášku „povolené: …“).
pub fn fields(root: &Value, node: &Value) -> Vec<String> {
    deref(root, node).get("properties").and_then(Value::as_object).map(|m| m.keys().cloned().collect()).unwrap_or_default()
}

/// Sloučení podle RFC 7396 (JSON Merge Patch).
pub fn merge(target: &mut Value, patch: &Value) {
    match (target, patch) {
        (Value::Object(t), Value::Object(p)) => {
            for (k, v) in p {
                if v.is_null() {
                    t.remove(k);
                } else if v.is_object() && t.get(k).is_some_and(Value::is_object) {
                    merge(t.get_mut(k).unwrap(), v);
                } else {
                    t.insert(k.clone(), v.clone());
                }
            }
        }
        (t, p) => *t = p.clone(),
    }
}

/// Lidská věta z chyby serde (bez technického „at line 1 column 42“).
pub fn serde_msg(e: &serde_json::Error) -> String {
    let s = e.to_string();
    match s.find(" at line ") {
        Some(i) => s[..i].to_string(),
        None => s,
    }
}

/// Změna části configu: kontrola neznámých polí, sloučení, převod zpět na typ. `what` = jméno sekce
/// do hlášky (`appearance`).
pub fn patch<T: Serialize + DeserializeOwned + JsonSchema>(cur: &T, change: &Value, what: &str) -> Result<T, String> {
    if !change.is_object() {
        return Err(trf!("{what}: čekám objekt se změnami.", "{what}: expected an object with changes."));
    }
    let root = of::<T>();
    if let Some(k) = unknown_key(&root, &root, change, "") {
        let top = fields(&root, &root).join(", ");
        return Err(trf!("{what}: neznámé pole „{k}“ (jde: {top}).", "{what}: unknown field “{k}” (allowed: {top})."));
    }
    let mut v = serde_json::to_value(cur).map_err(|e| e.to_string())?;
    merge(&mut v, change);
    serde_json::from_value(v).map_err(|e| trf!("{what}: {}", "{what}: {}", serde_msg(&e)))
}

/// Celá hodnota typu ze vstupu (s kontrolou neznámých polí).
pub fn parse<T: DeserializeOwned + JsonSchema>(input: &Value, what: &str) -> Result<T, String> {
    let root = of::<T>();
    if let Some(k) = unknown_key(&root, &root, input, "") {
        return Err(trf!("{what}: neznámé pole „{k}“.", "{what}: unknown field “{k}”."));
    }
    serde_json::from_value(input.clone()).map_err(|e| trf!("{what}: {}", "{what}: {}", serde_msg(&e)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::appearance::AppearanceConfig;
    use serde_json::json;

    #[test]
    fn merge_rfc7396() {
        let mut t = json!({ "a": 1, "b": { "c": 2, "d": 3 }, "e": [1] });
        merge(&mut t, &json!({ "a": null, "b": { "c": 9 }, "e": [2, 3], "f": "x" }));
        assert_eq!(t, json!({ "b": { "c": 9, "d": 3 }, "e": [2, 3], "f": "x" }));
    }

    #[test]
    fn appearance_catalog_and_unknown_keys() {
        let root = of::<AppearanceConfig>();
        let c = catalog(&root);
        let list = |k: &str| c.get(k).and_then(Value::as_array).map(|a| a.iter().filter_map(Value::as_str).map(String::from).collect::<Vec<_>>()).unwrap_or_default();
        assert!(list("theme").contains(&"nord".to_string()), "{c:?}");
        assert!(list("density").contains(&"comfy".to_string()));
        assert!(list("strip.style").len() >= 4, "{c:?}");
        assert!(list("motion.style").contains(&"none".to_string()), "{c:?}");
        assert_eq!(unknown_key(&root, &root, &json!({ "theme": "dark", "strip": { "size": "large" } }), ""), None);
        assert_eq!(unknown_key(&root, &root, &json!({ "themes": "dark" }), "").as_deref(), Some("themes"));
        assert_eq!(unknown_key(&root, &root, &json!({ "strip": { "sise": "large" } }), "").as_deref(), Some("strip.sise"));
    }

    #[test]
    fn patch_ok_and_errors() {
        let a = AppearanceConfig::default();
        let b = patch(&a, &json!({ "theme": "nord", "strip": { "size": "large" } }), "appearance").unwrap();
        assert_eq!(serde_json::to_value(&b).unwrap()["theme"], "nord");
        assert_eq!(serde_json::to_value(&b).unwrap()["strip"]["size"], "large");
        let e = patch(&a, &json!({ "theme": "fialova" }), "appearance").unwrap_err();
        assert!(e.contains("fialova") && e.contains("nord"), "povolené hodnoty v hlášce: {e}");
        assert!(!e.contains("line"), "{e}");
        assert!(patch(&a, &json!({ "barva": 1 }), "appearance").unwrap_err().contains("barva"));
        assert!(patch(&a, &json!([1]), "appearance").is_err());
    }
}
