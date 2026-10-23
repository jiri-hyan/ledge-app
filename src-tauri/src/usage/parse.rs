//! Jeden řádek transcriptu Claude Code (`~/.claude/projects/<slug>/*.jsonl`) → záznam spotřeby.
//! Čisté: bez I/O. Bere jen `type:"assistant"` s `message.usage`; klíč deduplikace = `requestId`
//! + `message.id` (streamované bloky jedné odpovědi i kopie po resume/forku mají oba stejné).

use serde::Deserialize;

/// Součty tokenů (všechno u64; cache zápis rozdělený na 5 min / 1 h).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, serde::Serialize, Deserialize)]
pub struct Tokens {
    pub input: u64,
    pub output: u64,
    /// zápis do cache s TTL 5 min (bez rozpadu v usage = celý `cache_creation_input_tokens`)
    pub cw5: u64,
    /// zápis do cache s TTL 1 h (`cache_creation.ephemeral_1h_input_tokens`)
    pub cw1: u64,
    /// čtení z cache
    pub cr: u64,
}

impl Tokens {
    /// Součty jsou nasycené: rozbitý řádek s `u64::MAX` nesmí shodit Ledge (debug) ani zalamovat do malého čísla.
    pub fn total(&self) -> u64 {
        [self.input, self.output, self.cw5, self.cw1, self.cr].into_iter().fold(0, u64::saturating_add)
    }
    /// Délka promptu jednoho požadavku (vstup + zápis + čtení cache) — pro cenové pásmo Haiku 5.5.
    pub fn prompt(&self) -> u64 {
        [self.input, self.cw5, self.cw1, self.cr].into_iter().fold(0, u64::saturating_add)
    }
    pub fn add(&mut self, o: &Tokens) {
        self.input = self.input.saturating_add(o.input);
        self.output = self.output.saturating_add(o.output);
        self.cw5 = self.cw5.saturating_add(o.cw5);
        self.cw1 = self.cw1.saturating_add(o.cw1);
        self.cr = self.cr.saturating_add(o.cr);
    }
}

/// Jedna odpověď API z transcriptu.
#[derive(Debug, Clone, PartialEq)]
pub struct Rec {
    /// FNV-1a 64 z `requestId` + `message.id` (stabilní napříč verzemi Rustu — ukládá se na disk)
    pub key: u64,
    pub ts_ms: i64,
    pub model: String,
    pub cwd: Option<String>,
    pub tokens: Tokens,
    /// `usage.speed == "fast"`
    pub fast: bool,
    /// `usage.inference_geo == "us"`
    pub geo_us: bool,
}

#[derive(Deserialize)]
struct Line<'a> {
    #[serde(rename = "type", borrow)]
    kind: Option<&'a str>,
    #[serde(rename = "requestId")]
    request_id: Option<String>,
    timestamp: Option<String>,
    cwd: Option<String>,
    message: Option<Msg>,
}

#[derive(Deserialize)]
struct Msg {
    id: Option<String>,
    model: Option<String>,
    usage: Option<Usage>,
}

#[derive(Deserialize, Default)]
struct Usage {
    input_tokens: Option<u64>,
    output_tokens: Option<u64>,
    cache_creation_input_tokens: Option<u64>,
    cache_read_input_tokens: Option<u64>,
    cache_creation: Option<CacheCreation>,
    speed: Option<String>,
    inference_geo: Option<String>,
}

#[derive(Deserialize, Default)]
struct CacheCreation {
    ephemeral_1h_input_tokens: Option<u64>,
}

/// FNV-1a 64 — stabilní hash pro klíč deduplikace.
pub fn fnv(parts: &[&str]) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for (i, p) in parts.iter().enumerate() {
        if i > 0 {
            h ^= 0x1f; // oddělovač částí
            h = h.wrapping_mul(0x0100_0000_01b3);
        }
        for b in p.bytes() {
            h ^= b as u64;
            h = h.wrapping_mul(0x0100_0000_01b3);
        }
    }
    h
}

/// ISO 8601 (`2026-10-17T15:17:08.995Z`, i s offsetem) → ms od epochy.
pub fn iso_ms(s: &str) -> Option<i64> {
    chrono::DateTime::parse_from_rfc3339(s).ok().map(|d| d.timestamp_millis())
}

/// Levný předfiltr nad bajty: řádek, který nemůže být odpověď API s usage, se neparsuje
/// (většina bajtů transcriptu jsou výsledky nástrojů v řádcích `user`).
pub fn maybe_usage(line: &[u8]) -> bool {
    memchr::memmem::find(line, b"\"assistant\"").is_some() && memchr::memmem::find(line, b"\"usage\"").is_some()
}

/// Řádek → záznam. `None` = jiný typ řádku, chybí usage / klíč / čas, nebo jsou všechny tokeny 0
/// (`<synthetic>` chybové zprávy Claude Code).
pub fn parse_line(line: &[u8]) -> Option<Rec> {
    if !maybe_usage(line) {
        return None;
    }
    let l: Line = serde_json::from_slice(line).ok()?;
    if l.kind != Some("assistant") {
        return None;
    }
    let m = l.message?;
    let u = m.usage?;
    let (rid, mid) = (l.request_id.unwrap_or_default(), m.id.unwrap_or_default());
    if rid.is_empty() && mid.is_empty() {
        return None;
    }
    let cw_total = u.cache_creation_input_tokens.unwrap_or(0);
    // rozpad 5m/1h z `cache_creation`, když ho usage nese; zbytek (i bez rozpadu) = 5 min (výchozí TTL)
    let cw1 = u.cache_creation.and_then(|c| c.ephemeral_1h_input_tokens).unwrap_or(0).min(cw_total);
    let cw5 = cw_total - cw1;
    let tokens = Tokens {
        input: u.input_tokens.unwrap_or(0),
        output: u.output_tokens.unwrap_or(0),
        cw5,
        cw1,
        cr: u.cache_read_input_tokens.unwrap_or(0),
    };
    if tokens.total() == 0 {
        return None;
    }
    Some(Rec {
        key: fnv(&[&rid, &mid]),
        ts_ms: iso_ms(l.timestamp.as_deref()?)?,
        model: m.model.unwrap_or_default(),
        cwd: l.cwd.filter(|c| !c.is_empty()),
        tokens,
        fast: u.speed.as_deref() == Some("fast"),
        geo_us: u.inference_geo.as_deref() == Some("us"),
    })
}

/// `cwd` z libovolného řádku (i `user`) — záloha pro záznamy bez vlastního `cwd`.
pub fn line_cwd(line: &[u8]) -> Option<String> {
    let i = memchr::memmem::find(line, b"\"cwd\":\"")?;
    let rest = &line[i + 6..];
    let mut de = serde_json::Deserializer::from_slice(rest).into_iter::<String>();
    de.next()?.ok().filter(|c| !c.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn line(rid: &str, mid: &str, usage: &str) -> String {
        format!(
            r#"{{"parentUuid":"p","type":"assistant","requestId":"{rid}","timestamp":"2026-10-17T15:17:08.995Z","cwd":"C:\\Users\\x\\Ledge","message":{{"id":"{mid}","model":"claude-opus-5-5","usage":{usage},"content":[{{"type":"tool_use","input":{{"model":"sonnet","cwd":"D:\\jinde"}}}}]}}}}"#
        )
    }

    #[test]
    fn parses_rows_table() {
        let full = r#"{"input_tokens":3,"output_tokens":50,"cache_creation_input_tokens":1000,"cache_read_input_tokens":20000,"cache_creation":{"ephemeral_5m_input_tokens":400,"ephemeral_1h_input_tokens":600},"speed":"standard","inference_geo":"global"}"#;
        let r = parse_line(line("req_1", "msg_1", full).as_bytes()).expect("záznam");
        assert_eq!(r.tokens, Tokens { input: 3, output: 50, cw5: 400, cw1: 600, cr: 20000 });
        assert_eq!(r.model, "claude-opus-5-5", "model ze zprávy, ne z tool_use");
        assert_eq!(r.cwd.as_deref(), Some("C:\\Users\\x\\Ledge"));
        assert_eq!(r.ts_ms, iso_ms("2026-10-17T15:17:08.995Z").unwrap());
        assert!(!r.fast && !r.geo_us);
        assert_eq!(r.key, fnv(&["req_1", "msg_1"]));

        let cases: &[(&str, String, Option<Tokens>)] = &[
            ("bez rozpadu cache = 5 min", line("r", "m", r#"{"input_tokens":1,"output_tokens":2,"cache_creation_input_tokens":70}"#), Some(Tokens { input: 1, output: 2, cw5: 70, cw1: 0, cr: 0 })),
            ("jen 1h", line("r", "m", r#"{"input_tokens":0,"output_tokens":1,"cache_creation_input_tokens":70,"cache_creation":{"ephemeral_1h_input_tokens":70}}"#), Some(Tokens { input: 0, output: 1, cw5: 0, cw1: 70, cr: 0 })),
            ("null pole", line("r", "m", r#"{"input_tokens":null,"output_tokens":5,"cache_creation":null}"#), Some(Tokens { output: 5, ..Tokens::default() })),
            ("samé nuly (synthetic)", line("r", "m", r#"{"input_tokens":0,"output_tokens":0}"#), None),
            ("user řádek", r#"{"type":"user","message":{"usage":{"input_tokens":5}},"text":"assistant"}"#.into(), None),
            ("bez usage", r#"{"type":"assistant","requestId":"r","timestamp":"2026-10-17T15:17:08Z","message":{"id":"m"}}"#.into(), None),
            ("bez klíče", r#"{"type":"assistant","timestamp":"2026-10-17T15:17:08Z","message":{"usage":{"output_tokens":5}}}"#.into(), None),
            ("bez času", r#"{"type":"assistant","requestId":"r","message":{"usage":{"output_tokens":5}}}"#.into(), None),
            ("rozbitý JSON", r#"{"type":"assistant","usage":{"#.into(), None),
        ];
        for (name, l, want) in cases {
            assert_eq!(parse_line(l.as_bytes()).map(|r| r.tokens), *want, "{name}");
        }
    }

    #[test]
    fn flags_and_keys() {
        let r = parse_line(line("r", "m", r#"{"output_tokens":1,"speed":"fast","inference_geo":"us"}"#).as_bytes()).unwrap();
        assert!(r.fast && r.geo_us);
        assert_ne!(fnv(&["ab", "c"]), fnv(&["a", "bc"]), "oddělovač částí");
        assert_eq!(fnv(&["req_x", ""]), fnv(&["req_x", ""]));
        let only_msg = r#"{"type":"assistant","timestamp":"2026-10-17T15:17:08Z","message":{"id":"m1","usage":{"output_tokens":5}}}"#;
        assert_eq!(parse_line(only_msg.as_bytes()).unwrap().key, fnv(&["", "m1"]), "bez requestId stačí message.id");
    }

    #[test]
    fn hostile_lines_never_panic() {
        let max = u64::MAX;
        // obří čísla: součty se nasytí, nepřetečou
        let huge = format!(r#"{{"input_tokens":{max},"output_tokens":{max},"cache_creation_input_tokens":{max},"cache_read_input_tokens":{max}}}"#);
        let r = parse_line(line("r", "m", &huge).as_bytes()).expect("záznam");
        assert_eq!(r.tokens.total(), max);
        assert_eq!(r.tokens.prompt(), max);
        let mut sum = Tokens::default();
        sum.add(&r.tokens);
        sum.add(&r.tokens);
        assert_eq!(sum.input, max);
        // záporné / desetinné / textové tokeny = řádek se zahodí, ne pád
        let bad: [(&str, String); 11] = [
            ("záporné", line("r", "m", r#"{"input_tokens":-5}"#)),
            ("desetinné", line("r", "m", r#"{"input_tokens":1.5}"#)),
            ("text", line("r", "m", r#"{"input_tokens":"5"}"#)),
            ("přes u64", line("r", "m", r#"{"input_tokens":18446744073709551616}"#)),
            ("usage pole", line("r", "m", "[1,2]")),
            ("usage null", line("r", "m", "null")),
            ("message pole", r#"{"type":"assistant","requestId":"r","timestamp":"2026-10-17T15:17:08Z","message":[1],"usage":1}"#.into()),
            ("čas není čas", r#"{"type":"assistant","requestId":"r","timestamp":"včera","message":{"usage":{"output_tokens":5}}}"#.into()),
            ("čas číslo", r#"{"type":"assistant","requestId":"r","timestamp":1760000000,"message":{"usage":{"output_tokens":5}}}"#.into()),
            ("prázdný", String::new()),
            ("jen mezery", "   \r\n".into()),
        ];
        for (name, l) in &bad {
            assert_eq!(parse_line(l.as_bytes()), None, "{name}");
        }
        // ne-UTF-8 bajty uvnitř řádku nebo před ním: JSON je neplatný → None, ne pád
        let mut raw = line("r", "m", r#"{"output_tokens":5}"#).into_bytes();
        raw.splice(0..0, [0xff, 0xfe]);
        assert_eq!(parse_line(&raw), None);
        let mut raw = line("r", "m", r#"{"output_tokens":5}"#).into_bytes();
        let at = raw.windows(4).position(|w| w == b"opus").unwrap();
        raw[at] = 0xc3; // osiřelý začátek vícebajtové sekvence uvnitř řetězce
        assert_eq!(parse_line(&raw), None);
        // CRLF na konci neškodí (soubor upravený editorem)
        let mut crlf = line("r", "m", r#"{"output_tokens":5}"#);
        crlf.push_str("\r\n");
        assert!(parse_line(crlf.as_bytes()).is_some());
        // obří řádek (4 MB výsledek nástroje) s usage na konci se zpracuje
        let big = format!(r#"{{"type":"assistant","requestId":"r","timestamp":"2026-10-17T15:17:08Z","note":"{}","message":{{"id":"m","usage":{{"output_tokens":7}}}}}}"#, "x".repeat(4_000_000));
        assert_eq!(parse_line(big.as_bytes()).map(|r| r.tokens.output), Some(7));
    }

    #[test]
    fn timestamps_with_offset_and_garbage() {
        let cases: [(&str, Option<i64>); 7] = [
            ("1970-01-01T00:00:00Z", Some(0)),
            ("2026-10-17T15:17:08.995Z", Some(1_792_250_228_995)),
            ("2026-10-17T17:17:08.995+02:00", Some(1_792_250_228_995)),
            ("2026-10-17", None),
            ("", None),
            ("2026-13-45T99:99:99Z", None),
            ("   2026-10-17T15:17:08Z", None),
        ];
        for (s, want) in cases {
            assert_eq!(iso_ms(s), want, "{s:?}");
        }
    }

    #[test]
    fn cwd_from_any_line() {
        assert_eq!(line_cwd(br#"{"type":"user","cwd":"C:\\a\\b","x":1}"#).as_deref(), Some("C:\\a\\b"));
        assert_eq!(line_cwd(br#"{"type":"user"}"#), None);
        assert_eq!(line_cwd(br#"{"cwd":""}"#), None);
        // useknutý / rozbitý / ne-UTF-8 řetězec = None
        assert_eq!(line_cwd(br#"{"cwd":"C:\\a"#), None);
        assert_eq!(line_cwd(b"{\"cwd\":\"\xff\xfe\"}"), None);
        assert_eq!(line_cwd(br#"{"cwd":"#), None);
        assert_eq!(line_cwd(br#"{"cwd":123}"#), None);
    }
}
