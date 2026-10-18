//! Výstup widgetu (stdout) → `WidgetData`. Dva formáty, oba tolerantní (nikdy panika, co nejde
//! přečíst, se přeskočí s poznámkou v `notes`):
//! - **JSON** (doporučený): objekt `{title, text, items[{label, value, icon, color, url | action}],
//!   badge, progress, strip, color}`; samotné pole = položky, samotný řetězec = text.
//! - **Text à la xbar**: první řádek = proužek, další řádky do `---` = text karty, za `---` položky;
//!   parametry za `|` (`href=…`, `color=…`, `copy=…`, `open=…`, `run=… arg=…`, `icon=…`, `value=…`).
//!
//! Klik na položku umí jen předdefinované věci (`Act`): otevřít web (http/https), složku nebo soubor,
//! zkopírovat text, spustit povolený skript ze složky skriptů s argumentem. Libovolný příkaz z výstupu
//! (`bash=`, `shell=` u xbar) se nikdy nespouští.

use crate::i18n::tr;
use serde::Serialize;
use serde_json::Value;

pub const MAX_ITEMS: usize = 50;
const MAX_TITLE: usize = 80;
const MAX_TEXT: usize = 2000;
const MAX_LABEL: usize = 120;
const MAX_VALUE: usize = 80;
const MAX_BADGE: usize = 16;
/// Text v proužku (jako `strip.custom`).
pub const MAX_STRIP: usize = 24;
const MAX_URL: usize = 2048;
const MAX_PATH: usize = 1024;
const MAX_COPY: usize = 4096;
const MAX_ARG: usize = 256;
const MAX_ICON: usize = 8;
const MAX_NOTES: usize = 5;

/// Barva z výstupu = předvolba (jako barvy proužku), žádný volný hex do UI.
#[derive(Serialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum Tone {
    Accent,
    Text,
    Muted,
    Violet,
    Blue,
    Teal,
    Green,
    Amber,
    Red,
}

/// Akce položky — jen tyhle, nic jiného.
#[derive(Serialize, Clone, Debug, PartialEq)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum Act {
    /// složka nebo soubor (při kliku znovu ověřeno: jen dokumenty, obrázky, média, zip, složky)
    Open { path: String },
    /// text do schránky
    Copy { text: String },
    /// povolený skript ze složky skriptů (`scripts.dir`) s jedním argumentem
    Run { script: String, arg: Option<String> },
}

#[derive(Serialize, Clone, Debug, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct Item {
    pub label: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color: Option<Tone>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub action: Option<Act>,
    /// oddělovač (`---` v textovém formátu)
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub sep: bool,
}

#[derive(Serialize, Clone, Debug, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct WidgetData {
    pub title: Option<String>,
    pub text: Option<String>,
    pub items: Vec<Item>,
    pub badge: Option<String>,
    /// 0–1
    pub progress: Option<f32>,
    /// text položky proužku (nejvýš 24 znaků)
    pub strip: Option<String>,
    pub color: Option<Tone>,
    /// co se z výstupu nedalo použít (lidsky, nejvýš 5)
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub notes: Vec<String>,
}

impl WidgetData {
    /// Text pro proužek: `strip`, jinak `badge`, jinak první řádek textu (vše nejvýš 24 znaků).
    pub fn strip_text(&self) -> Option<String> {
        let first = self.text.as_deref().and_then(|t| t.lines().map(str::trim).find(|l| !l.is_empty()));
        self.strip.as_deref().or(self.badge.as_deref()).or(first).map(|s| cut(s, MAX_STRIP)).filter(|s| !s.is_empty())
    }

    fn note(&mut self, n: String) {
        if self.notes.len() < MAX_NOTES {
            self.notes.push(n);
        }
    }
}

/// Jeden řádek bez řídicích znaků, oříznutý na `max` znaků (s „…“).
fn cut(s: &str, max: usize) -> String {
    let clean: String = s.chars().map(|c| if c.is_control() { ' ' } else { c }).collect();
    let t = clean.trim();
    if t.chars().count() <= max { t.to_string() } else { format!("{}…", t.chars().take(max - 1).collect::<String>().trim_end()) }
}

/// Víc řádků (text karty): řídicí znaky kromě nového řádku pryč.
fn cut_text(s: &str, max: usize) -> String {
    let clean: String = s.replace("\r\n", "\n").chars().map(|c| if c.is_control() && c != '\n' { ' ' } else { c }).collect();
    let t = clean.trim();
    if t.chars().count() <= max { t.to_string() } else { format!("{}…", t.chars().take(max - 1).collect::<String>()) }
}

fn opt(s: &str, max: usize) -> Option<String> {
    Some(cut(s, max)).filter(|s| !s.is_empty())
}

/// Barva podle jména (předvolby, běžné aliasy) nebo `#rgb` / `#rrggbb` podle odstínu.
pub fn tone(s: &str) -> Option<Tone> {
    let l = s.trim().to_ascii_lowercase();
    Some(match l.as_str() {
        "accent" => Tone::Accent,
        "text" | "white" | "default" | "normal" => Tone::Text,
        "muted" | "gray" | "grey" | "dim" | "silver" => Tone::Muted,
        "violet" | "purple" | "magenta" | "pink" => Tone::Violet,
        "blue" | "info" | "navy" => Tone::Blue,
        "teal" | "cyan" | "aqua" => Tone::Teal,
        "green" | "ok" | "success" | "good" | "lime" => Tone::Green,
        "amber" | "yellow" | "orange" | "warn" | "warning" | "gold" => Tone::Amber,
        "red" | "error" | "danger" | "bad" | "fail" | "failed" | "crimson" => Tone::Red,
        _ => return hex_tone(&l),
    })
}

fn hex_tone(s: &str) -> Option<Tone> {
    let h = s.strip_prefix('#').filter(|h| h.chars().all(|c| c.is_ascii_hexdigit()))?;
    let px = |i: usize, n: usize| u8::from_str_radix(&h[i..i + n], 16).ok().map(|v| if n == 1 { v * 17 } else { v });
    let (r, g, b) = match h.len() {
        3 => (px(0, 1)?, px(1, 1)?, px(2, 1)?),
        6 => (px(0, 2)?, px(2, 2)?, px(4, 2)?),
        _ => return None,
    };
    let (r, g, b) = (r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0);
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    if max - min < 0.15 {
        return Some(if max > 0.6 { Tone::Text } else { Tone::Muted });
    }
    let d = max - min;
    let hue = if max == r {
        60.0 * (((g - b) / d) % 6.0)
    } else if max == g {
        60.0 * ((b - r) / d + 2.0)
    } else {
        60.0 * ((r - g) / d + 4.0)
    };
    let hue = if hue < 0.0 { hue + 360.0 } else { hue };
    Some(match hue as u32 {
        0..=19 | 330..=360 => Tone::Red,
        20..=69 => Tone::Amber,
        70..=159 => Tone::Green,
        160..=199 => Tone::Teal,
        200..=259 => Tone::Blue,
        _ => Tone::Violet,
    })
}

/// Web jen http(s), bez mezer, nejvýš 2048 znaků.
pub fn url_ok(u: &str) -> bool {
    let l = u.to_ascii_lowercase();
    (l.starts_with("https://") || l.starts_with("http://")) && u.len() > 8 && u.len() <= MAX_URL && !u.chars().any(|c| c.is_whitespace() || c.is_control())
}

/// Jméno skriptu pro `run`: jen soubor (bez cesty), přípona skriptu.
fn script_ok(s: &str) -> bool {
    !s.is_empty() && s.len() <= 200 && !s.contains(['\\', '/', ':']) && !s.starts_with('.') && std::path::Path::new(s).extension().and_then(|e| e.to_str()).and_then(crate::scripts::meta::Lang::from_ext).is_some()
}

fn arg_ok(a: &str) -> bool {
    a.chars().count() <= MAX_ARG && !a.contains(['"', '\n', '\r', '\0'])
}

/// Akce z hodnot (`open`, `copy`, `run` + `arg`); `Err` = lidský důvod, proč ne.
fn make_act(open: Option<&str>, copy: Option<&str>, run: Option<&str>, arg: Option<&str>) -> Result<Option<Act>, String> {
    if let Some(p) = open.map(str::trim).filter(|p| !p.is_empty()) {
        return if p.chars().count() <= MAX_PATH && !p.contains('\n') { Ok(Some(Act::Open { path: p.to_string() })) } else { Err(tr("cesta je moc dlouhá", "the path is too long").into()) };
    }
    if let Some(t) = copy {
        return if t.chars().count() <= MAX_COPY { Ok(Some(Act::Copy { text: t.to_string() })) } else { Err(tr("text ke zkopírování je delší než 4096 znaků", "the text to copy is longer than 4096 characters").into()) };
    }
    if let Some(s) = run.map(str::trim).filter(|s| !s.is_empty()) {
        if !script_ok(s) {
            return Err(trf!("„{s}“ není skript ze složky skriptů (jen jméno souboru .ps1/.cmd/.py/.js)", "“{s}” isn't a script from the scripts folder (just a .ps1/.cmd/.py/.js file name)"));
        }
        if let Some(a) = arg
            && !arg_ok(a)
        {
            return Err(tr("argument skriptu: nejvýš 256 znaků, bez uvozovek a nových řádků", "script argument: at most 256 characters, no quotes or new lines").into());
        }
        return Ok(Some(Act::Run { script: s.to_string(), arg: arg.map(str::to_string) }));
    }
    Ok(None)
}

/// Parsuje stdout. JSON, když začíná `{` / `[` / `"` a jde přečíst; jinak textový formát.
pub fn parse(stdout: &str) -> WidgetData {
    let s = stdout.trim_start_matches('\u{feff}').trim();
    if s.starts_with(['{', '[', '"'])
        && let Ok(v) = serde_json::from_str::<Value>(s)
    {
        return from_json(&v);
    }
    let mut d = from_text(s);
    if s.starts_with('{') {
        d.note(tr("výstup vypadá jako JSON, ale nejde přečíst — beru ho jako text", "the output looks like JSON but can't be read — treating it as text").into());
    }
    d
}

fn scalar(v: &Value) -> Option<String> {
    match v {
        Value::String(s) => Some(s.clone()),
        Value::Number(n) => Some(n.to_string()),
        Value::Bool(b) => Some(if *b { tr("ano", "yes") } else { tr("ne", "no") }.into()),
        _ => None,
    }
}

fn progress(v: &Value) -> Option<f32> {
    let n = match v {
        Value::Number(n) => n.as_f64()?,
        Value::String(s) => {
            let t = s.trim();
            let pct = t.ends_with('%');
            let x: f64 = t.trim_end_matches('%').trim().replace(',', ".").parse().ok()?;
            if pct { x / 100.0 } else { x }
        }
        _ => return None,
    };
    let n = if n > 1.0 { n / 100.0 } else { n };
    n.is_finite().then_some(n.clamp(0.0, 1.0) as f32)
}

fn from_json(v: &Value) -> WidgetData {
    let mut d = WidgetData::default();
    match v {
        Value::Array(_) => {
            json_items(&mut d, v);
            return d;
        }
        Value::String(s) => {
            d.text = Some(cut_text(s, MAX_TEXT)).filter(|t| !t.is_empty());
            return d;
        }
        Value::Object(_) => {}
        _ => {
            d.text = scalar(v);
            return d;
        }
    }
    let get = |k: &str| v.get(k).filter(|x| !x.is_null());
    d.title = get("title").and_then(scalar).and_then(|s| opt(&s, MAX_TITLE));
    d.text = get("text").and_then(scalar).map(|s| cut_text(&s, MAX_TEXT)).filter(|t| !t.is_empty());
    d.badge = get("badge").and_then(scalar).and_then(|s| opt(&s, MAX_BADGE));
    d.strip = get("strip").and_then(scalar).and_then(|s| opt(&s, MAX_STRIP));
    d.color = get("color").and_then(Value::as_str).and_then(tone);
    if let Some(p) = get("progress") {
        d.progress = progress(p);
        if d.progress.is_none() {
            d.note(tr("progress: čekám číslo 0–1 nebo procenta", "progress: expected a number 0–1 or a percentage").into());
        }
    }
    if let Some(items) = get("items") {
        json_items(&mut d, items);
    }
    d
}

fn json_items(d: &mut WidgetData, v: &Value) {
    let list: Vec<&Value> = match v {
        Value::Array(a) => a.iter().collect(),
        Value::Object(_) => vec![v],
        _ => {
            d.note(tr("items má být pole položek", "items should be an array of items").into());
            return;
        }
    };
    if list.len() > MAX_ITEMS {
        d.note(trf!("položek je {}, ukážu prvních {MAX_ITEMS}", "there are {} items, showing the first {MAX_ITEMS}", list.len()));
    }
    for (i, it) in list.into_iter().take(MAX_ITEMS).enumerate() {
        let n = i + 1;
        if let Some(s) = scalar(it) {
            if let Some(label) = opt(&s, MAX_LABEL) {
                d.items.push(Item { label, ..Default::default() });
            }
            continue;
        }
        if !it.is_object() {
            d.note(trf!("položka {n}: čekám objekt nebo text", "item {n}: expected an object or text"));
            continue;
        }
        let get = |k: &str| it.get(k).filter(|x| !x.is_null());
        let gs = |k: &str| get(k).and_then(scalar);
        let Some(label) = gs("label").or_else(|| gs("title")).or_else(|| gs("name")).and_then(|s| opt(&s, MAX_LABEL)) else {
            if it.get("sep").and_then(Value::as_bool) == Some(true) {
                d.items.push(Item { sep: true, ..Default::default() });
            } else {
                d.note(trf!("položka {n}: chybí label", "item {n}: label is missing"));
            }
            continue;
        };
        let mut item = Item {
            label,
            value: gs("value").and_then(|s| opt(&s, MAX_VALUE)),
            icon: gs("icon").and_then(|s| opt(&s, MAX_ICON)).filter(|s| !s.contains(char::is_whitespace)),
            color: get("color").and_then(Value::as_str).and_then(tone),
            ..Default::default()
        };
        if let Some(u) = gs("url").or_else(|| gs("href")) {
            if url_ok(u.trim()) {
                item.url = Some(u.trim().to_string());
            } else {
                d.note(trf!("položka {n}: odkaz jen http(s)://", "item {n}: links only http(s)://"));
            }
        }
        let act = match get("action") {
            Some(Value::Object(a)) => {
                let s = |k: &str| a.get(k).and_then(scalar);
                match a.get("type").and_then(Value::as_str).unwrap_or("") {
                    "open" => make_act(s("path").as_deref(), None, None, None),
                    "copy" => make_act(None, s("text").as_deref().or(Some("")), None, None),
                    "run" => make_act(None, None, s("script").as_deref().or(Some("")), s("arg").as_deref()),
                    t => Err(trf!("neznámá akce „{t}“ (open, copy, run)", "unknown action “{t}” (open, copy, run)")),
                }
            }
            Some(_) => Err(tr("action má být objekt {\"type\": …}", "action should be an object {\"type\": …}").into()),
            None => make_act(gs("open").as_deref(), gs("copy").as_deref(), gs("run").as_deref(), gs("arg").as_deref()),
        };
        match act {
            Ok(a) => item.action = a,
            Err(e) => d.note(trf!("položka {n}: {e}", "item {n}: {e}")),
        }
        d.items.push(item);
    }
}

/// `text | k=v k2="v 2" k3='v 3'` → (text, parametry). Bez `|` = jen text.
fn split_params(line: &str) -> (&str, Vec<(String, String)>) {
    let Some((text, rest)) = line.split_once('|') else { return (line, vec![]) };
    let mut out = Vec::new();
    let mut chars = rest.chars().peekable();
    loop {
        while chars.peek().is_some_and(|c| c.is_whitespace()) {
            chars.next();
        }
        let mut key = String::new();
        while let Some(&c) = chars.peek() {
            if c == '=' || c.is_whitespace() {
                break;
            }
            key.push(c);
            chars.next();
        }
        if key.is_empty() {
            break;
        }
        let mut val = String::new();
        if chars.peek() == Some(&'=') {
            chars.next();
            match chars.peek().copied() {
                Some(q @ ('"' | '\'')) => {
                    chars.next();
                    for c in chars.by_ref() {
                        if c == q {
                            break;
                        }
                        val.push(c);
                    }
                }
                _ => {
                    while let Some(&c) = chars.peek() {
                        if c.is_whitespace() {
                            break;
                        }
                        val.push(c);
                        chars.next();
                    }
                }
            }
        }
        out.push((key.to_ascii_lowercase(), val));
    }
    (text, out)
}

/// xbar parametry, které Ledge záměrně neprovádí (příkazy z výstupu).
const IGNORED_RUN: [&str; 3] = ["bash", "shell", "terminal"];

fn from_text(s: &str) -> WidgetData {
    let mut d = WidgetData::default();
    let lines: Vec<&str> = s.lines().map(str::trim_end).collect();
    let split = lines.iter().position(|l| l.trim() == "---");
    let (head, body) = match split {
        Some(i) => (&lines[..i], &lines[i + 1..]),
        None => (&lines[..], &[][..]),
    };
    let head: Vec<&str> = head.iter().copied().filter(|l| !l.trim().is_empty()).collect();
    if let Some(first) = head.first() {
        let (text, params) = split_params(first);
        d.strip = opt(text, MAX_STRIP);
        d.color = params.iter().find(|(k, _)| k == "color").and_then(|(_, v)| tone(v));
    }
    let rest: Vec<String> = head.iter().skip(1).map(|l| split_params(l).0.trim().to_string()).filter(|l| !l.is_empty()).collect();
    if !rest.is_empty() {
        d.text = Some(cut_text(&rest.join("\n"), MAX_TEXT));
    }
    let mut ignored = false;
    let mut count = 0usize;
    for line in body {
        let t = line.trim_start_matches('-').trim();
        let is_sep = line.trim() == "---";
        if t.is_empty() && !is_sep {
            continue;
        }
        count += 1;
        if d.items.len() >= MAX_ITEMS {
            continue;
        }
        if is_sep {
            if d.items.last().is_some_and(|i| !i.sep) {
                d.items.push(Item { sep: true, ..Default::default() });
            }
            continue;
        }
        let (text, params) = split_params(t);
        let Some(label) = opt(text, MAX_LABEL) else { continue };
        let p = |k: &str| params.iter().find(|(pk, _)| pk == k).map(|(_, v)| v.as_str());
        let mut item = Item {
            label,
            value: p("value").and_then(|v| opt(v, MAX_VALUE)),
            icon: p("icon").or_else(|| p("emoji")).and_then(|v| opt(v, MAX_ICON)),
            color: p("color").and_then(tone),
            ..Default::default()
        };
        if let Some(u) = p("href").or_else(|| p("url")) {
            if url_ok(u) {
                item.url = Some(u.to_string());
            } else {
                d.note(trf!("„{}“: odkaz jen http(s)://", "“{}”: links only http(s)://", item.label));
            }
        }
        match make_act(p("open"), p("copy"), p("run"), p("arg")) {
            Ok(a) => item.action = a,
            Err(e) => d.note(format!("„{}“: {e}", item.label)),
        }
        if IGNORED_RUN.iter().any(|k| p(k).is_some()) {
            ignored = true;
        }
        d.items.push(item);
    }
    if d.items.last().is_some_and(|i| i.sep) {
        d.items.pop();
    }
    if count > MAX_ITEMS {
        d.note(trf!("položek je {count}, ukážu prvních {MAX_ITEMS}", "there are {count} items, showing the first {MAX_ITEMS}"));
    }
    if ignored {
        d.note(tr("bash= / shell= Ledge nespouští — místo nich použij run= (skript ze složky skriptů)", "Ledge doesn't run bash= / shell= — use run= (a script from the scripts folder) instead").into());
    }
    d
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn json_full() {
        let d = parse(
            r#"{"title":"Docker","text":"2 běží","badge":3,"progress":"42%","strip":"🐳 2/3 běží a ještě dlouhý text","color":"green",
            "items":[{"label":"web","value":"Up 2h","icon":"🟢","color":"ok","url":"https://localhost:8080"},
                     {"label":"db","action":{"type":"copy","text":"postgres://x"}},
                     {"label":"logy","action":{"type":"open","path":"C:\\logs"}},
                     {"label":"restart","action":{"type":"run","script":"restart.ps1","arg":"web"}},
                     {"label":"zkratky","copy":"ahoj"}]}"#,
        );
        assert_eq!(d.title.as_deref(), Some("Docker"));
        assert_eq!(d.text.as_deref(), Some("2 běží"));
        assert_eq!(d.badge.as_deref(), Some("3"));
        assert_eq!(d.progress, Some(0.42));
        assert_eq!(d.strip.as_ref().map(|s| s.chars().count()), Some(MAX_STRIP));
        assert_eq!(d.color, Some(Tone::Green));
        assert_eq!(d.items.len(), 5);
        assert_eq!((d.items[0].color, d.items[0].url.as_deref(), d.items[0].value.as_deref()), (Some(Tone::Green), Some("https://localhost:8080"), Some("Up 2h")));
        assert_eq!(d.items[1].action, Some(Act::Copy { text: "postgres://x".into() }));
        assert_eq!(d.items[2].action, Some(Act::Open { path: r"C:\logs".into() }));
        assert_eq!(d.items[3].action, Some(Act::Run { script: "restart.ps1".into(), arg: Some("web".into()) }));
        assert_eq!(d.items[4].action, Some(Act::Copy { text: "ahoj".into() }));
        assert!(d.notes.is_empty(), "{:?}", d.notes);
    }

    /// (vstup, počet položek, text, proužek, počet poznámek)
    #[test]
    fn tolerant_table() {
        type Case = (&'static str, usize, Option<&'static str>, Option<&'static str>, usize);
        let cases: Vec<Case> = vec![
            ("", 0, None, None, 0),
            ("   \n\n", 0, None, None, 0),
            (r#"["a", "b", 3]"#, 3, None, None, 0),
            (r#""jen text""#, 0, Some("jen text"), None, 0),
            (r#"{"items": {"label": "jedna"}}"#, 1, None, None, 0),
            (r#"{"items": "nesmysl"}"#, 0, None, None, 1),
            (r#"{"items": [{"value": "bez labelu"}, 7, {"label": "ok"}]}"#, 2, None, None, 1),
            (r#"{"items": [{"label": "x", "url": "javascript:alert(1)"}]}"#, 1, None, None, 1),
            (r#"{"items": [{"label": "x", "url": "file:///C:/a"}]}"#, 1, None, None, 1),
            (r#"{"items": [{"label": "x", "action": {"type": "exec", "cmd": "del"}}]}"#, 1, None, None, 1),
            (r#"{"items": [{"label": "x", "action": "copy"}]}"#, 1, None, None, 1),
            (r#"{"items": [{"label": "x", "run": "..\\evil.ps1"}]}"#, 1, None, None, 1),
            (r#"{"items": [{"label": "x", "run": "a.ps1", "arg": "a\"b"}]}"#, 1, None, None, 1),
            (r#"{"progress": "hodně"}"#, 0, None, None, 1),
            (r#"{"text": 42}"#, 0, Some("42"), None, 0),
            // rozbitý JSON = text (první řádek do proužku) + poznámka
            ("{\"title\": \"bez konce", 0, None, Some("{\"title\": \"bez konce"), 1),
            ("42", 0, None, Some("42"), 0),
        ];
        for (inp, items, text, strip, notes) in cases {
            let d = parse(inp);
            assert_eq!(d.items.len(), items, "{inp:?}: položky {:?}", d.items);
            assert_eq!(d.text.as_deref(), text, "{inp:?}: text");
            assert_eq!(d.strip.as_deref(), strip, "{inp:?}: proužek");
            assert_eq!(d.notes.len(), notes, "{inp:?}: poznámky {:?}", d.notes);
        }
    }

    #[test]
    fn text_format_like_xbar() {
        let d = parse("☀ 18 °C | color=amber\nLisabon, polojasno\n---\nPředpověď | href=https://open-meteo.com\n--Zítra 21 °C\nZkopírovat | copy=\"18 °C\"\n---\nSložka | open=C:\\Temp\nSmazat | bash=/bin/rm param1=-rf\nRestart | run=restart.ps1 arg=web\nNebezpečné | href=file:///C:/x\n---\n");
        assert_eq!(d.strip.as_deref(), Some("☀ 18 °C"));
        assert_eq!(d.color, Some(Tone::Amber));
        assert_eq!(d.text.as_deref(), Some("Lisabon, polojasno"));
        let labels: Vec<&str> = d.items.iter().map(|i| if i.sep { "—" } else { i.label.as_str() }).collect();
        assert_eq!(labels, ["Předpověď", "Zítra 21 °C", "Zkopírovat", "—", "Složka", "Smazat", "Restart", "Nebezpečné"]);
        assert_eq!(d.items[0].url.as_deref(), Some("https://open-meteo.com"));
        assert_eq!(d.items[2].action, Some(Act::Copy { text: "18 °C".into() }));
        assert_eq!(d.items[4].action, Some(Act::Open { path: r"C:\Temp".into() }));
        assert_eq!(d.items[5].action, None, "bash= se nikdy nespustí");
        assert_eq!(d.items[6].action, Some(Act::Run { script: "restart.ps1".into(), arg: Some("web".into()) }));
        assert_eq!(d.items[7].url, None);
        assert_eq!(d.notes.len(), 2, "{:?}", d.notes);
    }

    #[test]
    fn limits_and_control_chars() {
        let many: String = (0..80).map(|i| format!("r{i}\n")).collect();
        let d = parse(&format!("head\n---\n{many}"));
        assert_eq!(d.items.len(), MAX_ITEMS);
        assert_eq!(d.notes.len(), 1);
        let d = parse(&format!(r#"{{"items":[{}]}}"#, (0..80).map(|i| format!("\"{i}\"")).collect::<Vec<_>>().join(",")));
        assert_eq!(d.items.len(), MAX_ITEMS);
        let d = parse("a\u{7}b\tc\n---\nx\u{1b}[31my");
        assert_eq!(d.strip.as_deref(), Some("a b c"));
        assert_eq!(d.items[0].label, "x [31my");
        let long = "x".repeat(5000);
        assert_eq!(parse(&format!(r#"{{"text":"{long}"}}"#)).text.unwrap().chars().count(), MAX_TEXT);
        assert!(parse("\u{feff}{\"text\":\"bom\"}").notes.is_empty());
    }

    #[test]
    fn strip_text_fallbacks() {
        let d = |j: &str| parse(j).strip_text();
        assert_eq!(d(r#"{"strip":"S","badge":"B","text":"T"}"#).as_deref(), Some("S"));
        assert_eq!(d(r#"{"badge":"B","text":"T"}"#).as_deref(), Some("B"));
        assert_eq!(d(r#"{"text":"\n  první\ndruhý"}"#).as_deref(), Some("první"));
        assert_eq!(d(r#"{"items":["a"]}"#), None);
        assert_eq!(parse(""), WidgetData::default());
    }

    #[test]
    fn tones() {
        let cases: &[(&str, Option<Tone>)] = &[
            ("green", Some(Tone::Green)),
            ("WARNING", Some(Tone::Amber)),
            ("error", Some(Tone::Red)),
            ("#f00", Some(Tone::Red)),
            ("#22c55e", Some(Tone::Green)),
            ("#3b82f6", Some(Tone::Blue)),
            ("#888888", Some(Tone::Muted)),
            ("#ffffff", Some(Tone::Text)),
            ("#f59e0b", Some(Tone::Amber)),
            ("#12345", None),
            ("#ččč", None),
            ("rainbow", None),
        ];
        for (s, want) in cases {
            assert_eq!(tone(s), *want, "{s}");
        }
    }

    #[test]
    fn url_rules() {
        for (u, ok) in [("https://x.cz", true), ("http://localhost:5173/a?b=c", true), ("HTTPS://X.CZ", true), ("https://", false), ("ftp://x", false), ("https://a b", false), ("javascript:x", false)] {
            assert_eq!(url_ok(u), ok, "{u}");
        }
    }

    #[test]
    fn serde_shape() {
        let d = parse(r#"{"items":[{"label":"a","action":{"type":"run","script":"x.py"}}]}"#);
        let v = serde_json::to_value(&d).unwrap();
        assert_eq!(v["items"][0]["action"], serde_json::json!({"type":"run","script":"x.py","arg":null}));
        assert!(v["items"][0].get("sep").is_none());
        assert!(v.get("notes").is_none());
    }
}
