//! Drát mezi Ledge a rozšířením: NDJSON po stdin/stdout dítěte, `v:1` (kontrakt `docs/codemap/ext.md`).
//! Čisté funkce — kódování, dekódování, limit řádku. Neznámá pole a typy zpráv se ignorují (dopředná
//! kompatibilita), jiná verze `v` = chyba `Version`.

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::io::BufRead;

/// Verze kontraktu; nesedí-li, Ledge pošle `quit` a označí rozšíření za nekompatibilní.
pub const PROTO: u32 = 1;
/// Strop jednoho řádku (bajty). Delší = chyba, spojení se zavře.
pub const MAX_LINE: usize = 256 * 1024;

/// ext → Ledge.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "t", rename_all = "lowercase")]
pub enum Up {
    Hello {
        id: String,
        pid: u32,
        #[serde(default)]
        wants: Vec<String>,
    },
    Ready,
    /// uživatelská akce v rozšíření (vynuluje nečinnost)
    Active,
    Req {
        what: String,
        #[serde(default, rename = "do")]
        act: Option<String>,
        #[serde(default)]
        key: Option<String>,
    },
    Err {
        msg: String,
    },
    Bye,
}

/// Ledge → ext.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "t", rename_all = "lowercase")]
pub enum Down {
    Init {
        cfg: Value,
        lang: String,
        dead: bool,
        #[serde(default)]
        rev: Value,
    },
    State {
        pause: bool,
        #[serde(default)]
        why: Option<String>,
    },
    Data {
        topic: String,
        rev: u64,
        d: Value,
    },
    Cmd {
        #[serde(default)]
        name: String,
        #[serde(default)]
        args: Value,
    },
    Quit,
}

const UP_TAGS: &[&str] = &["hello", "ready", "active", "req", "err", "bye"];
const DOWN_TAGS: &[&str] = &["init", "state", "data", "cmd", "quit"];

#[derive(Debug, Clone, PartialEq)]
pub enum Decoded<T> {
    Msg(T),
    /// typ zprávy, který tahle strana nezná (novější protistrana) — zahodit
    Unknown(String),
}

#[derive(Debug, Clone, PartialEq)]
pub enum WireError {
    /// není JSON objekt
    Malformed,
    /// `v` chybí nebo nesedí
    Version(Option<u64>),
    /// známý typ, ale špatný tvar polí
    Bad(String),
    TooLong,
}

impl std::fmt::Display for WireError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WireError::Malformed => write!(f, "řádek není JSON objekt"),
            WireError::Version(v) => write!(f, "verze kontraktu {v:?}, očekáváno {PROTO}"),
            WireError::Bad(m) => write!(f, "vadná zpráva: {m}"),
            WireError::TooLong => write!(f, "řádek delší než {MAX_LINE} bajtů"),
        }
    }
}

fn decode<T: for<'de> Deserialize<'de>>(line: &str, tags: &[&str]) -> Result<Decoded<T>, WireError> {
    let v: Value = serde_json::from_str(line.trim()).map_err(|_| WireError::Malformed)?;
    let obj = v.as_object().ok_or(WireError::Malformed)?;
    if obj.get("v").and_then(Value::as_u64) != Some(PROTO as u64) {
        return Err(WireError::Version(obj.get("v").and_then(Value::as_u64)));
    }
    let t = obj.get("t").and_then(Value::as_str).ok_or(WireError::Malformed)?;
    if !tags.contains(&t) {
        return Ok(Decoded::Unknown(t.to_string()));
    }
    serde_json::from_value(v).map(Decoded::Msg).map_err(|e| WireError::Bad(e.to_string()))
}

pub fn decode_up(line: &str) -> Result<Decoded<Up>, WireError> {
    decode(line, UP_TAGS)
}

pub fn decode_down(line: &str) -> Result<Decoded<Down>, WireError> {
    decode(line, DOWN_TAGS)
}

fn encode<T: Serialize>(m: &T) -> String {
    let mut v = serde_json::to_value(m).unwrap_or_else(|_| json!({}));
    if let Some(o) = v.as_object_mut() {
        o.insert("v".into(), json!(PROTO));
    }
    let mut s = v.to_string();
    s.push('\n');
    s
}

pub fn encode_up(m: &Up) -> String {
    encode(m)
}

pub fn encode_down(m: &Down) -> String {
    encode(m)
}

/// Jeden řádek ze streamu: `None` = konec. Delší než `MAX_LINE` → `TooLong` (zbytek řádku se zahodí).
pub fn read_line(reader: &mut impl BufRead, buf: &mut Vec<u8>) -> std::io::Result<Option<Result<String, WireError>>> {
    buf.clear();
    let total = crate::procman::read_line_capped(reader, buf, MAX_LINE)?;
    if total == 0 {
        return Ok(None);
    }
    if total > MAX_LINE + 2 {
        return Ok(Some(Err(WireError::TooLong)));
    }
    Ok(Some(Ok(String::from_utf8_lossy(buf).into_owned())))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn up_decode_table() {
        let cases: &[(&str, Result<Decoded<Up>, WireError>)] = &[
            (
                r#"{"v":1,"t":"hello","id":"wall","pid":12,"wants":["media"]}"#,
                Ok(Decoded::Msg(Up::Hello { id: "wall".into(), pid: 12, wants: vec!["media".into()] })),
            ),
            // `wants` je volitelné, neznámá pole se ignorují
            (
                r#"{"v":1,"t":"hello","id":"x","pid":1,"novinka":{"a":1}}"#,
                Ok(Decoded::Msg(Up::Hello { id: "x".into(), pid: 1, wants: vec![] })),
            ),
            (r#"{"v":1,"t":"ready"}"#, Ok(Decoded::Msg(Up::Ready))),
            (r#"{"v":1,"t":"active"}"#, Ok(Decoded::Msg(Up::Active))),
            (r#"{"v":1,"t":"bye"}"#, Ok(Decoded::Msg(Up::Bye))),
            (
                r#"{"v":1,"t":"req","what":"media","do":"next"}"#,
                Ok(Decoded::Msg(Up::Req { what: "media".into(), act: Some("next".into()), key: None })),
            ),
            (r#"{"v":1,"t":"err","msg":"boom"}"#, Ok(Decoded::Msg(Up::Err { msg: "boom".into() }))),
            (r#"{"v":1,"t":"budoucnost","x":1}"#, Ok(Decoded::Unknown("budoucnost".into()))),
            // směr: Ledge→ext typ není pro ext→Ledge známý
            (r#"{"v":1,"t":"quit"}"#, Ok(Decoded::Unknown("quit".into()))),
            (r#"{"v":2,"t":"ready"}"#, Err(WireError::Version(Some(2)))),
            (r#"{"t":"ready"}"#, Err(WireError::Version(None))),
            (r#"{"v":1}"#, Err(WireError::Malformed)),
            ("nejson", Err(WireError::Malformed)),
            ("[1,2]", Err(WireError::Malformed)),
        ];
        for (line, want) in cases {
            assert_eq!(&decode_up(line), want, "{line}");
        }
        assert!(matches!(decode_up(r#"{"v":1,"t":"hello","id":5}"#), Err(WireError::Bad(_))));
    }

    #[test]
    fn down_roundtrip_and_unknown() {
        let msgs = [
            Down::Init { cfg: json!({"a": 1}), lang: "cs".into(), dead: false, rev: json!({}) },
            Down::State { pause: true, why: Some("game".into()) },
            Down::Data { topic: "media".into(), rev: 17, d: json!({"title": "x"}) },
            Down::Cmd { name: "open".into(), args: json!(null) },
            Down::Quit,
        ];
        for m in msgs {
            let line = encode_down(&m);
            assert!(line.ends_with('\n') && line.matches('\n').count() == 1, "jeden řádek");
            assert!(line.contains(r#""v":1"#));
            assert_eq!(decode_down(&line).unwrap(), Decoded::Msg(m));
        }
        assert_eq!(decode_down(r#"{"v":1,"t":"hello","id":"a","pid":1}"#), Ok(Decoded::Unknown("hello".into())));
        // `state` bez `why`
        assert_eq!(decode_down(r#"{"v":1,"t":"state","pause":false}"#), Ok(Decoded::Msg(Down::State { pause: false, why: None })));
    }

    #[test]
    fn up_encode_carries_version_and_do_key() {
        let l = encode_up(&Up::Req { what: "media".into(), act: Some("next".into()), key: None });
        let v: Value = serde_json::from_str(&l).unwrap();
        assert_eq!(v, json!({"v": 1, "t": "req", "what": "media", "do": "next", "key": null}));
        assert_eq!(decode_up(&encode_up(&Up::Bye)), Ok(Decoded::Msg(Up::Bye)));
    }

    fn lines(input: &[u8]) -> Vec<Result<String, WireError>> {
        let mut r = std::io::BufReader::with_capacity(16, input);
        let mut buf = Vec::new();
        let mut out = Vec::new();
        while let Some(l) = read_line(&mut r, &mut buf).unwrap() {
            out.push(l);
        }
        out
    }

    #[test]
    fn read_line_limit_table() {
        assert_eq!(lines(b"a\nb\r\n"), vec![Ok("a\n".into()), Ok("b\r\n".into())]);
        assert!(lines(b"").is_empty());
        assert_eq!(lines(b"bez-konce"), vec![Ok("bez-konce".into())]);
        // přesně na stropu projde, o kus nad ním = TooLong a další řádek se čte dál
        let ok = format!("{}\n", "x".repeat(MAX_LINE - 1));
        let long = format!("{}\n", "y".repeat(MAX_LINE + 10));
        let mut input = ok.clone().into_bytes();
        input.extend_from_slice(long.as_bytes());
        input.extend_from_slice(b"{\"v\":1}\n");
        let got = lines(&input);
        assert_eq!(got.len(), 3);
        assert!(got[0].is_ok());
        assert_eq!(got[1], Err(WireError::TooLong));
        assert_eq!(got[2], Ok("{\"v\":1}\n".into()));
    }
}
