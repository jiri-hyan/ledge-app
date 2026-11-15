//! Čtecí výřez Ledge pro jiné nástroje (`live_state`): sessions Claude, limity, dnešní kalendář, čekající schválení
//! (jen kdo a kdy, ne co), běžící dev servery s maskovanými chybovými řádky a počty čekajících zadání. Jen čtení:
//! nic se nepotvrzuje, nespouští ani nemění (povolit akci Clauda smí jen člověk). Soukromí řeší POVOLENÝ SEZNAM
//! polí (`ALLOW`, funkce `exposed`): co v něm není, se nevystaví — nové pole zdroje je tedy skryté, dokud ho
//! někdo nepřidá do tabulky i do testu. Zdroj dat dodává `Host::live_raw`, tady je jen čistý tvar výstupu.

use serde_json::{Map, Value, json};
use std::collections::BTreeSet;

/// Části výstupu (parametr `part`); bez něj všechny.
pub const PARTS: &[&str] = &["sessions", "limits", "agenda", "approvals", "devServers", "briefs"];

/// Vystavená pole (tečková cesta, pole v poli beze indexu). Nic jiného ven nejde — záměrně chybí: titulky a
/// jména sessions (text promptů), cesty (`cwd`, soubory, `configPath`), příkazy a shrnutí schválení, poslední
/// zpráva a soubor aktivity, umístění a účastníci událostí, odkazy na schůzky, `cmd` / `cwd` / `tail` / `lastLine`
/// procesů, názvy a cesty poznámek, e-maily.
pub const ALLOW: &[&str] = &[
    "sessions.id", "sessions.project", "sessions.folder", "sessions.status", "sessions.since", "sessions.updatedAt",
    "sessions.ended", "sessions.endReason", "sessions.alive", "sessions.uncommittedCount", "sessions.unpushed",
    "sessions.subagentsRunning", "sessions.activity.phase", "sessions.activity.tool", "sessions.activity.at",
    "limits.at", "limits.source", "limits.staleMs", "limits.fiveHour.pct", "limits.fiveHour.resetsAt",
    "limits.sevenDay.pct", "limits.sevenDay.resetsAt",
    "agenda.start", "agenda.end", "agenda.allDay", "agenda.title", "agenda.ongoing",
    "approvals.questions", "approvals.pending.tool", "approvals.pending.folder", "approvals.pending.createdAt", "approvals.pending.expiresAt",
    "devServers.id", "devServers.phase", "devServers.port", "devServers.url", "devServers.startedAt", "devServers.readyAt",
    "devServers.exitCode", "devServers.errors", "devServers.errorLines",
    "briefs.waiting", "briefs.review",
];

/// Smí pole (skalár) nebo kontejner na `path` ven? Kontejner = cesta, pod kterou je něco povoleno.
pub fn exposed(path: &str) -> bool {
    ALLOW.iter().any(|a| *a == path || a.strip_prefix(path).is_some_and(|r| r.starts_with('.')))
}

/// Zkopíruje jen povolené. Objekt i pole se procházejí dál (povolená cesta objektu neznamená, že smí celý), skaláry
/// a pole skalárů jen když je jejich cesta přesně v `ALLOW`.
pub fn filter(v: &Value, path: &str) -> Option<Value> {
    match v {
        Value::Object(o) => {
            let mut out = Map::new();
            for (k, x) in o {
                let p = if path.is_empty() { k.clone() } else { format!("{path}.{k}") };
                if exposed(&p)
                    && let Some(f) = filter(x, &p)
                {
                    out.insert(k.clone(), f);
                }
            }
            Some(Value::Object(out))
        }
        Value::Array(a) => Some(Value::Array(a.iter().filter_map(|x| filter(x, path)).collect())),
        scalar => ALLOW.contains(&path).then(|| scalar.clone()),
    }
}

/// Poslední složka cesty (`C:\x\atlas` → `atlas`) — z cesty ven jde jen název složky.
fn folder(cwd: &str) -> String {
    cwd.trim_end_matches(['/', '\\']).rsplit(['/', '\\']).next().unwrap_or("").to_string()
}

fn add_folder(item: &mut Value) {
    if let Some(f) = item.get("cwd").and_then(Value::as_str).map(folder) {
        item["folder"] = json!(f);
    }
}

fn count(item: &Value, key: &str) -> usize {
    item.get(key).and_then(Value::as_array).map_or(0, Vec::len)
}

/// Odvozená pole, která zdroj nemá: název složky místo cesty, počty místo seznamů souborů a subagentů.
fn derive(raw: &Value) -> Value {
    let mut v = raw.clone();
    for s in v.get_mut("sessions").and_then(Value::as_array_mut).into_iter().flatten() {
        add_folder(s);
        s["uncommittedCount"] = json!(count(s, "uncommitted"));
        s["subagentsRunning"] = json!(s.get("subagents").map_or(0, |a| count(a, "running")));
    }
    for p in v.get_mut("approvals").and_then(|a| a.get_mut("pending")).and_then(Value::as_array_mut).into_iter().flatten() {
        add_folder(p);
    }
    v
}

/// Výstup nástroje: povolená pole ze `raw` (tvar viz `LiveHost::live_raw`), případně jen jedna část. Neznámá část = chyba.
pub fn build(raw: &Value, part: Option<&str>) -> Result<Value, String> {
    if let Some(p) = part
        && !PARTS.contains(&p)
    {
        return Err(trf!("Neznámá část „{p}“ (jsou: {}).", "Unknown part “{p}” (there are: {}).", PARTS.join(", ")));
    }
    let Some(Value::Object(all)) = filter(&derive(raw), "") else { return Ok(json!({})) };
    let keep: BTreeSet<&str> = part.map_or_else(|| PARTS.iter().copied().collect(), |p| [p].into());
    Ok(Value::Object(all.into_iter().filter(|(k, _)| keep.contains(k.as_str())).collect()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exposed_table() {
        for (path, want) in [
            // povoleno
            ("sessions.status", true),
            ("sessions.folder", true),
            ("sessions.uncommittedCount", true),
            ("sessions.activity.phase", true),
            ("limits.fiveHour.pct", true),
            ("agenda.title", true),
            ("devServers.errorLines", true),
            ("devServers.errors", true),
            ("approvals.pending.tool", true),
            ("briefs.waiting", true),
            // kontejnery (kvůli rekurzi)
            ("sessions", true),
            ("sessions.activity", true),
            ("limits.fiveHour", true),
            ("approvals.pending", true),
            // zakázáno: obsah promptů, cesty, příkazy, soubory, lidé
            ("sessions.title", false),
            ("sessions.name", false),
            ("sessions.cwd", false),
            ("sessions.uncommitted", false),
            ("sessions.subagents", false),
            ("sessions.activity.message", false),
            ("sessions.activity.file", false),
            ("sessions.job", false),
            ("approvals.pending.summary", false),
            ("approvals.pending.cwd", false),
            ("approvals.pending.preview", false),
            ("approvals.pending.plan", false),
            ("approvals.pending.always", false),
            ("agenda.location", false),
            ("agenda.join", false),
            ("agenda.people", false),
            ("devServers.cmd", false),
            ("devServers.cwd", false),
            ("devServers.tail", false),
            ("devServers.lastLine", false),
            ("devServers.message", false),
            ("briefs.first", false),
            ("briefs.first.title", false),
            ("briefs.first.path", false),
            ("configPath", false),
            ("notes", false),
            ("mail", false),
            // předpona jména není kontejner
            ("session", false),
            ("sessions.stat", false),
            ("", false),
        ] {
            assert_eq!(exposed(path), want, "{path}");
        }
    }

    fn raw() -> Value {
        json!({
            "sessions": [{
                "id": "s1", "name": "oprav heslo admina", "title": "tajný prompt", "cwd": "C:\\Users\\jan\\Desktop\\atlas", "project": "Atlas", "status": "working",
                "since": 5, "updatedAt": 6, "ended": false, "alive": true, "uncommitted": ["a.rs", "b.rs"], "unpushed": 1, "entrypoint": "cli", "pid": 1234,
                "subagents": { "running": [{ "id": "a", "name": "tajný agent" }, { "id": "b" }], "done": [] },
                "activity": { "phase": "editing", "tool": "Edit", "file": "C:/x/.env", "message": "heslo je hunter2", "at": 7 },
                "job": { "id": "abcd1234" }
            }],
            "approvals": { "questions": 1, "pending": [{ "id": 1, "sessionId": "s1", "cwd": "C:/x/atlas", "tool": "Bash", "summary": "curl -H 'Authorization: Bearer k'", "createdAt": 1, "expiresAt": 2, "preview": { "x": 1 } }] },
            "limits": { "at": 9, "source": "statusLine", "fiveHour": { "pct": 42.5, "resetsAt": 10 }, "sevenDay": null, "staleMs": 300000, "statusLine": "full", "history5h": [[1.0, 2.0]] },
            "agenda": [{ "start": 1, "end": 2, "allDay": false, "title": "Porada", "location": "Zasedačka 3", "cal": 0, "ongoing": false, "join": { "url": "https://x" }, "people": { "a": 1 } }],
            "devServers": [{ "id": "atlas", "phase": "failed", "port": 5173, "url": "http://localhost:5173", "startedAt": 1, "readyAt": null, "exitCode": 1, "message": "C:\\x\\y", "lastLine": "tajné", "errors": 2, "errorLines": ["Error: x", "at y"], "cwd": "C:/x", "cmd": "npm run dev --token=abc", "tail": ["a"] }],
            "briefs": { "waiting": 1, "review": 2, "first": { "path": "C:/n/a.md", "title": "Soukromá poznámka", "status": "waiting" } },
            "configPath": "C:/Users/jan/AppData/Roaming/Ledge/config.json"
        })
    }

    #[test]
    fn build_exposes_only_allowlist() {
        let out = build(&raw(), None).unwrap();
        // nic zakázaného není nikde ve výstupu (ani zanořeně, ani jako hodnota)
        let text = out.to_string();
        for secret in ["tajný", "oprav heslo", "hunter2", ".env", "Users", "Bearer", "Zasedačka", "https://x", "npm run dev", "tajné", "Soukromá", "config.json", "preview", "cwd", "summary"] {
            assert!(!text.contains(secret), "{secret} ve výstupu: {text}");
        }
        // vystavená pole jsou
        assert_eq!(out["sessions"][0]["folder"], "atlas");
        assert_eq!(out["sessions"][0]["uncommittedCount"], 2);
        assert_eq!(out["sessions"][0]["subagentsRunning"], 2);
        assert_eq!(out["sessions"][0]["activity"]["phase"], "editing");
        assert_eq!(out["approvals"]["pending"][0], json!({ "tool": "Bash", "folder": "atlas", "createdAt": 1, "expiresAt": 2 }));
        assert_eq!(out["limits"]["fiveHour"]["pct"], 42.5);
        assert_eq!(out["agenda"][0]["title"], "Porada");
        assert_eq!(out["devServers"][0]["errorLines"], json!(["Error: x", "at y"]));
        assert_eq!(out["briefs"], json!({ "waiting": 1, "review": 2 }));
        assert!(out.get("configPath").is_none());
    }

    #[test]
    fn build_parts_and_empty() {
        let one = build(&raw(), Some("limits")).unwrap();
        assert_eq!(one.as_object().unwrap().keys().collect::<Vec<_>>(), ["limits"]);
        assert!(build(&raw(), Some("notes")).is_err());
        // bez dat prázdné části, ne vymyšlené hodnoty
        assert_eq!(build(&Value::Null, None).unwrap(), json!({}));
        assert_eq!(folder("C:\\a\\b\\"), "b");
        assert_eq!(folder("/x/y"), "y");
    }
}
