//! Výstup nástroje `apps_discover`: aplikace (Start menu + Microsoft Store) a repozitáře jako malý
//! JSON. Čistá funkce nad výsledky skenu — sken (`Host::discover`) dělá `host.rs`. Úspora tokenů:
//! `alreadyInLedge` jen když je `true`, prázdné hodnoty se vynechají.

use crate::onboarding::apps::FoundApp;
use crate::onboarding::repos::FoundRepo;
use crate::onboarding::store::StoreApp;
use serde_json::{Map, Value, json};

/// Nejvíc aplikací / repozitářů ve výstupu (zbytek model stejně nepřečte).
const MAX_APPS: usize = 300;
const MAX_REPOS: usize = 200;

fn flag(m: &mut Map<String, Value>, already: bool) {
    if already {
        m.insert("alreadyInLedge".into(), Value::Bool(true));
    }
}

pub fn output(apps: &[FoundApp], store: &[StoreApp], repos: &[FoundRepo]) -> Value {
    let mut list: Vec<Value> = apps
        .iter()
        .map(|a| {
            let mut m = Map::new();
            m.insert("name".into(), json!(a.name));
            m.insert("exe".into(), json!(a.exe));
            if !a.args.is_empty() {
                m.insert("args".into(), json!(a.args));
            }
            flag(&mut m, a.already);
            Value::Object(m)
        })
        .collect();
    // Store: bez exe, s `aumid` — do tiles_add jako {"kind":"app","aumid":…}
    list.extend(store.iter().map(|a| {
        let mut m = Map::new();
        m.insert("name".into(), json!(a.name));
        m.insert("aumid".into(), json!(a.aumid));
        m.insert("source".into(), json!("store"));
        flag(&mut m, a.already);
        Value::Object(m)
    }));
    list.truncate(MAX_APPS);
    let repos: Vec<Value> = repos
        .iter()
        .take(MAX_REPOS)
        .map(|r| {
            let mut m = Map::new();
            m.insert("name".into(), json!(r.name));
            m.insert("path".into(), json!(r.path));
            if let Some(x) = &r.remote {
                m.insert("remote".into(), json!(x));
            }
            m.insert("kind".into(), json!(r.kind));
            if !r.hint.is_empty() {
                m.insert("hint".into(), json!(r.hint));
            }
            if let Some(ms) = r.last_ms {
                m.insert("lastActiveMs".into(), json!(ms));
            }
            if let Some(p) = &r.project {
                m.insert("suggestion".into(), serde_json::to_value(p).unwrap_or(Value::Null));
            }
            flag(&mut m, r.already);
            Value::Object(m)
        })
        .collect();
    json!({ "apps": list, "repos": repos })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn output_is_small_and_marks_store() {
        let app = |n: &str, already: bool, args: &[&str]| FoundApp { name: n.into(), exe: format!(r"C:\Programy\{n}.exe"), args: args.iter().map(|s| s.to_string()).collect(), already };
        let apps = [app("Alfa", false, &[]), app("Beta", true, &["--x", r"C:\Program Files\Beta"])];
        let store = [StoreApp { name: "Kalkulačka".into(), aumid: "Microsoft.WindowsCalculator_8wekyb3d8bbwe!App".into(), already: false }];
        let repo = FoundRepo { name: "web".into(), path: r"C:\code\web".into(), remote: None, last_ms: None, kind: "none", hint: String::new(), project: None, already: false };
        let v = output(&apps, &store, &[repo]);
        let text = v.to_string();
        assert_eq!(
            text,
            concat!(
                r#"{"apps":[{"exe":"C:\\Programy\\Alfa.exe","name":"Alfa"},"#,
                r#"{"alreadyInLedge":true,"args":["--x","C:\\Program Files\\Beta"],"exe":"C:\\Programy\\Beta.exe","name":"Beta"},"#,
                r#"{"aumid":"Microsoft.WindowsCalculator_8wekyb3d8bbwe!App","name":"Kalkulačka","source":"store"}],"#,
                r#""repos":[{"kind":"none","name":"web","path":"C:\\code\\web"}]}"#
            )
        );
        assert!(!text.contains("false") && !text.contains("null"));
    }
}
