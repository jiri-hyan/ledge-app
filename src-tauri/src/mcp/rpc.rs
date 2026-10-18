//! JSON-RPC 2.0 pro MCP (ručně, bez knihovny): `initialize` (vyjednání verze), `ping`, `tools/list`,
//! `tools/call`, notifikace bez odpovědi. Chyba nástroje = výsledek s `isError: true` (to řeší `tools`);
//! sem patří jen chyby protokolu (špatná zpráva, neznámá metoda, neznámý nástroj). Čisté funkce.

use serde_json::{Value, json};

/// Podporované verze protokolu MCP, nejnovější první (ta se nabídne, když klient chce jinou).
pub const VERSIONS: [&str; 3] = ["2025-06-18", "2025-03-26", "2024-11-05"];

pub const PARSE_ERROR: i64 = -32700;
pub const INVALID_REQUEST: i64 = -32600;
pub const METHOD_NOT_FOUND: i64 = -32601;
pub const INVALID_PARAMS: i64 = -32602;

/// Chyba protokolu (kód JSON-RPC + zpráva).
#[derive(Debug, PartialEq)]
pub struct RpcErr(pub i64, pub String);

/// Co server umí: seznam nástrojů a jejich volání (`tools::Handler`).
pub trait Tools {
    fn list(&self) -> Value;
    /// Výsledek nástroje (`content` + `isError`), nebo chyba protokolu (neznámý nástroj).
    fn call(&self, name: &str, args: &Value) -> Result<Value, RpcErr>;
    fn version(&self) -> String;
}

/// Výsledek zpracování těla požadavku.
#[derive(Debug, PartialEq)]
pub enum Outcome {
    /// 200 + JSON
    Reply(String),
    /// 400 + JSON-RPC chyba (nečitelné tělo)
    BadRequest(String),
    /// 202 bez těla (jen notifikace / odpovědi klienta)
    Accepted,
}

/// Text pro model: kdy Ledge nastavovat a jak.
pub const INSTRUCTIONS: &str = "Ledge is a Windows drawer at the top edge of the screen (projects/app shortcuts, quick actions and links, cards, a status strip). \
These tools read and change Ledge's settings exactly like its Settings window does. Typical flow: ledge_status → apps_discover → \
tiles_add / actions_add for what the user really uses → layout_get + layout_preview → layout_apply → appearance_get/appearance_set → \
open_drawer to show the result. Every change can be reverted with undo_last. Nothing here launches programs: a command action asks the \
user for consent on its first run.";

/// Zpracuje tělo `POST /mcp` (jedna zpráva nebo dávka).
pub fn handle(body: &str, t: &dyn Tools) -> Outcome {
    let msg: Value = match serde_json::from_str(body) {
        Ok(v) => v,
        Err(e) => return Outcome::BadRequest(error_body(Value::Null, PARSE_ERROR, &format!("Parse error: {e}"))),
    };
    match msg {
        // dávka (2025-03-26; novější klienti posílají po jedné)
        Value::Array(items) if !items.is_empty() => {
            let out: Vec<Value> = items.iter().filter_map(|m| one(m, t)).collect();
            if out.is_empty() { Outcome::Accepted } else { Outcome::Reply(Value::Array(out).to_string()) }
        }
        Value::Array(_) => Outcome::BadRequest(error_body(Value::Null, INVALID_REQUEST, "Empty batch")),
        m => match one(&m, t) {
            Some(r) => Outcome::Reply(r.to_string()),
            None => Outcome::Accepted,
        },
    }
}

/// Chyba JSON-RPC jako text.
pub fn error_body(id: Value, code: i64, message: &str) -> String {
    error(id, code, message).to_string()
}

fn error(id: Value, code: i64, message: &str) -> Value {
    json!({ "jsonrpc": "2.0", "id": id, "error": { "code": code, "message": message } })
}

/// Jedna zpráva → odpověď (`None` = notifikace / odpověď klienta, nic se nevrací).
fn one(m: &Value, t: &dyn Tools) -> Option<Value> {
    let Some(obj) = m.as_object() else { return Some(error(Value::Null, INVALID_REQUEST, "Invalid request")) };
    let id = obj.get("id").cloned();
    let id_ok = matches!(&id, Some(Value::String(_)) | Some(Value::Number(_)));
    if obj.get("jsonrpc").and_then(Value::as_str) != Some("2.0") {
        return Some(error(if id_ok { id.unwrap() } else { Value::Null }, INVALID_REQUEST, "jsonrpc must be \"2.0\""));
    }
    let Some(method) = obj.get("method").and_then(Value::as_str) else {
        // odpověď klienta na náš požadavek (žádné neposíláme) — přijmout a zahodit
        return if obj.contains_key("result") || obj.contains_key("error") { None } else { Some(error(Value::Null, INVALID_REQUEST, "Missing method")) };
    };
    // notifikace (notifications/initialized, cancelled…): bez odpovědi
    id.as_ref()?;
    if !id_ok {
        return Some(error(Value::Null, INVALID_REQUEST, "id must be a string or a number"));
    }
    let id = id.unwrap();
    let params = obj.get("params").cloned().unwrap_or(Value::Null);
    let res = match method {
        "initialize" => Ok(initialize(&params, &t.version())),
        "ping" => Ok(json!({})),
        "tools/list" => Ok(json!({ "tools": t.list() })),
        "tools/call" => call(&params, t),
        _ => Err(RpcErr(METHOD_NOT_FOUND, format!("Method not found: {method}"))),
    };
    Some(match res {
        Ok(r) => json!({ "jsonrpc": "2.0", "id": id, "result": r }),
        Err(RpcErr(code, msg)) => error(id, code, &msg),
    })
}

/// Vyjednaná verze: klientova, když ji umíme, jinak naše nejnovější (klient se rozhodne, jestli pokračovat).
pub fn negotiate(asked: Option<&str>) -> &'static str {
    asked.and_then(|a| VERSIONS.iter().find(|v| **v == a)).copied().unwrap_or(VERSIONS[0])
}

fn initialize(params: &Value, version: &str) -> Value {
    json!({
        "protocolVersion": negotiate(params.get("protocolVersion").and_then(Value::as_str)),
        "capabilities": { "tools": { "listChanged": false } },
        "serverInfo": { "name": "ledge", "title": "Ledge", "version": version },
        "instructions": INSTRUCTIONS,
    })
}

fn call(params: &Value, t: &dyn Tools) -> Result<Value, RpcErr> {
    let name = params.get("name").and_then(Value::as_str).ok_or_else(|| RpcErr(INVALID_PARAMS, "tools/call needs params.name".into()))?;
    let args = match params.get("arguments") {
        None | Some(Value::Null) => json!({}),
        Some(a @ Value::Object(_)) => a.clone(),
        Some(_) => return Err(RpcErr(INVALID_PARAMS, "params.arguments must be an object".into())),
    };
    t.call(name, &args)
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Fake;
    impl Tools for Fake {
        fn list(&self) -> Value {
            json!([{ "name": "echo" }])
        }
        fn call(&self, name: &str, args: &Value) -> Result<Value, RpcErr> {
            match name {
                "echo" => Ok(json!({ "content": [{ "type": "text", "text": args.to_string() }], "isError": false })),
                _ => Err(RpcErr(INVALID_PARAMS, format!("Unknown tool: {name}"))),
            }
        }
        fn version(&self) -> String {
            "9.9.9".into()
        }
    }

    fn reply(body: &str) -> Value {
        match handle(body, &Fake) {
            Outcome::Reply(r) | Outcome::BadRequest(r) => serde_json::from_str(&r).unwrap(),
            Outcome::Accepted => Value::Null,
        }
    }

    #[test]
    fn initialize_negotiates_version() {
        for (asked, want) in [("2025-06-18", "2025-06-18"), ("2025-03-26", "2025-03-26"), ("2024-11-05", "2024-11-05"), ("2099-01-01", "2025-06-18")] {
            let r = reply(&format!(r#"{{"jsonrpc":"2.0","id":1,"method":"initialize","params":{{"protocolVersion":"{asked}","capabilities":{{}},"clientInfo":{{"name":"t","version":"1"}}}}}}"#));
            assert_eq!(r["result"]["protocolVersion"], want, "{asked}");
            assert_eq!(r["result"]["serverInfo"]["name"], "ledge");
            assert_eq!(r["result"]["serverInfo"]["version"], "9.9.9");
            assert_eq!(r["result"]["capabilities"]["tools"]["listChanged"], false);
            assert_eq!(r["id"], 1);
        }
        assert_eq!(negotiate(None), VERSIONS[0]);
    }

    #[test]
    fn protocol_table() {
        // (tělo, očekávané: kód chyby / "ok" / "202")
        let cases: &[(&str, &str)] = &[
            (r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#, "202"),
            (r#"{"jsonrpc":"2.0","id":"a","result":{}}"#, "202"),
            (r#"{"jsonrpc":"2.0","id":7,"method":"ping"}"#, "ok"),
            (r#"{"jsonrpc":"2.0","id":7,"method":"tools/list"}"#, "ok"),
            (r#"{"jsonrpc":"2.0","id":7,"method":"tools/call","params":{"name":"echo","arguments":{"x":1}}}"#, "ok"),
            (r#"{"jsonrpc":"2.0","id":7,"method":"tools/call","params":{"name":"echo"}}"#, "ok"),
            (r#"{"jsonrpc":"2.0","id":7,"method":"tools/call","params":{"name":"nic"}}"#, "-32602"),
            (r#"{"jsonrpc":"2.0","id":7,"method":"tools/call","params":{"arguments":{}}}"#, "-32602"),
            (r#"{"jsonrpc":"2.0","id":7,"method":"tools/call","params":{"name":"echo","arguments":[1]}}"#, "-32602"),
            (r#"{"jsonrpc":"2.0","id":7,"method":"resources/list"}"#, "-32601"),
            (r#"{"jsonrpc":"1.0","id":7,"method":"ping"}"#, "-32600"),
            (r#"{"jsonrpc":"2.0","id":null,"method":"ping"}"#, "-32600"),
            (r#"{"jsonrpc":"2.0","id":{"x":1},"method":"ping"}"#, "-32600"),
            (r#"{"jsonrpc":"2.0","id":7}"#, "-32600"),
            (r#"[1]"#, "batch"),
            (r#"[]"#, "-32600"),
            (r#"{nejson"#, "-32700"),
            (r#""text""#, "-32600"),
        ];
        for (body, want) in cases {
            let out = handle(body, &Fake);
            match *want {
                "202" => assert_eq!(out, Outcome::Accepted, "{body}"),
                "ok" => {
                    let r = reply(body);
                    assert!(r.get("result").is_some() && r.get("error").is_none(), "{body}: {r}");
                }
                "batch" => assert!(matches!(out, Outcome::Reply(ref r) if r.contains("-32600")), "{body}"),
                code => {
                    let r = reply(body);
                    assert_eq!(r["error"]["code"].to_string(), *code, "{body}: {r}");
                }
            }
        }
        // nečitelné tělo a prázdná dávka = 400, ostatní chyby jdou s 200 (JSON-RPC odpověď)
        assert!(matches!(handle("{nejson", &Fake), Outcome::BadRequest(_)));
        assert!(matches!(handle(r#"{"jsonrpc":"2.0","id":7,"method":"x"}"#, &Fake), Outcome::Reply(_)));
    }

    #[test]
    fn call_result_and_batch() {
        let r = reply(r#"{"jsonrpc":"2.0","id":"q","method":"tools/call","params":{"name":"echo","arguments":{"a":"b"}}}"#);
        assert_eq!(r["id"], "q");
        assert_eq!(r["result"]["content"][0]["type"], "text");
        assert_eq!(r["result"]["content"][0]["text"], r#"{"a":"b"}"#);
        let b = reply(r#"[{"jsonrpc":"2.0","id":1,"method":"ping"},{"jsonrpc":"2.0","method":"notifications/initialized"},{"jsonrpc":"2.0","id":2,"method":"tools/list"}]"#);
        let ids: Vec<i64> = b.as_array().unwrap().iter().map(|x| x["id"].as_i64().unwrap()).collect();
        assert_eq!(ids, [1, 2]);
        assert_eq!(handle(r#"[{"jsonrpc":"2.0","method":"notifications/initialized"}]"#, &Fake), Outcome::Accepted);
        // odpověď je jeden řádek (stdio proxy ji posílá jako řádek)
        assert!(matches!(handle(r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}"#, &Fake), Outcome::Reply(ref s) if !s.contains('\n')));
    }
}
