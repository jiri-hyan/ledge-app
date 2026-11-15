//! Načtení a validace `*.kit.json` — čisté: dostane text, vrátí ověřenou `Spec` (poznámky o ořezaném)
//! nebo důvody, proč soubor nejde použít. Tolerantní jako `desk::load`: neznámé klíče se ignorují,
//! špatný řádek / tlačítko se zahodí s poznámkou, rozbitý JSON je chyba (nikdy panika).
//! Zdroje v textu (`{limits.five.pct|pct}`) Rust neověřuje (pevná tabulka je v UI `SOURCES`), jen syntaxi `{…}`.
//! Akce jen z bílé listiny: `open` (cesta, nebo `drawer:<karta>`), `url` (http/https), `copy` (≤ 4 KB),
//! `action` (id z rychlých akcí; existenci řeší až klik).

use crate::i18n::tr;
use crate::widgets::output::url_ok;
use serde::Serialize;
use serde_json::Value;

/// Poznámka pro člověka (co se zahodilo / ořezalo, nebo proč soubor nejde).
pub type Note = String;

pub const MAX_BYTES: usize = 16 * 1024;
pub const MAX_ROWS: usize = 12;
pub const MAX_BUTTONS: usize = 6;
pub const MAX_TEXT: usize = 120;
pub const MAX_TITLE: usize = 60;
const MAX_ICON: usize = 8;
const MAX_COPY: usize = 4096;
const MAX_PATH: usize = 1024;
const MAX_TONES: usize = 4;
const MAX_SRC: usize = 64;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum KitTone {
    Ok,
    Amber,
    Red,
    Dim,
}

impl KitTone {
    fn parse(s: &str) -> Option<Self> {
        Some(match s {
            "ok" => Self::Ok,
            "amber" => Self::Amber,
            "red" => Self::Red,
            "dim" => Self::Dim,
            _ => return None,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Tone {
    /// `[práh, tón]`, vzestupně podle prahu
    pub over: Vec<(f64, KitTone)>,
}

/// Akce řádku / tlačítka (nejvýš jedna; pořadí `open` › `url` › `copy` › `action`).
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct Acts {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub open: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub copy: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub action: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Row {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    /// cesta zdroje průběhu (např. `limits.five.pct`)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bar: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tone: Option<Tone>,
    #[serde(flatten)]
    pub act: Acts,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Button {
    pub label: String,
    #[serde(flatten)]
    pub act: Acts,
}

/// Ověřená šablona; tvar JSON = `KitSpec` v `src/types/widgets.ts`.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Spec {
    pub kit: u8,
    pub title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,
    pub rows: Vec<Row>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub buttons: Vec<Button>,
    /// co se při načtení zahodilo / ořezalo (v `spec` se neposílá, jde zvlášť jako `notes`)
    #[serde(skip)]
    pub notes: Vec<Note>,
}

fn clip(s: &str, max: usize) -> (String, bool) {
    let t = s.trim();
    if t.chars().count() <= max { (t.to_string(), false) } else { (t.chars().take(max).collect(), true) }
}

/// Syntaxe `{zdroj}` / `{zdroj|formát}` v textu: závorky párové, nezanořené, obsah neprázdný a jen
/// `a–z A–Z 0–9 _ . |` (≤ 64 znaků). `Err` = co je špatně.
pub fn template_ok(s: &str) -> Result<(), String> {
    let mut inside: Option<String> = None;
    for c in s.chars() {
        match (c, &mut inside) {
            ('{', None) => inside = Some(String::new()),
            ('{', Some(_)) => return Err(tr("zanořená závorka {", "nested {").into()),
            ('}', None) => return Err(tr("} bez {", "} without {").into()),
            ('}', Some(b)) => {
                if b.is_empty() || b.starts_with('|') || b.ends_with('|') || b.chars().count() > MAX_SRC || b.matches('|').count() > 1 {
                    return Err(tr("prázdný nebo chybný zdroj v {…}", "empty or invalid source in {…}").into());
                }
                inside = None;
            }
            (c, Some(b)) => {
                if !(c.is_ascii_alphanumeric() || matches!(c, '_' | '.' | '|')) {
                    return Err(tr("v {…} jsou jen písmena, číslice, _ . a |", "{…} allows only letters, digits, _ . and |").into());
                }
                b.push(c);
            }
            _ => {}
        }
    }
    if inside.is_some() { Err(tr("chybí }", "missing }").into()) } else { Ok(()) }
}

fn source_ok(s: &str) -> bool {
    !s.is_empty() && s.len() <= MAX_SRC && !s.starts_with('.') && !s.ends_with('.') && s.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '.'))
}

fn card_ok(c: &str) -> bool {
    !c.is_empty() && c.len() <= 64 && c.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '.' | ':'))
}

/// Text ≤ 120 znaků se syntaxí šablony; ořezání = poznámka, chybná syntaxe = `None` + poznámka.
fn text_field(v: &Value, key: &str, who: &str, notes: &mut Vec<Note>) -> Option<String> {
    let s = v.get(key).filter(|x| !x.is_null())?;
    let Some(s) = s.as_str() else {
        notes.push(trf!("{who}: „{key}“ není text.", "{who}: “{key}” is not text."));
        return None;
    };
    let (t, cut) = clip(s, MAX_TEXT);
    if cut {
        notes.push(trf!("{who}: „{key}“ je delší než {MAX_TEXT} znaků — ořezáno.", "{who}: “{key}” is longer than {MAX_TEXT} characters — cut."));
    }
    if let Err(e) = template_ok(&t) {
        notes.push(trf!("{who}: „{key}“ — {e}.", "{who}: “{key}” — {e}."));
        return None;
    }
    (!t.is_empty()).then_some(t)
}

/// Akce z `open` / `url` / `copy` / `action`; víc najednou = první platná, ostatní jen poznámka.
fn acts(v: &Value, who: &str, notes: &mut Vec<Note>) -> Acts {
    let mut a = Acts::default();
    let present: Vec<&str> = ["open", "url", "copy", "action"].into_iter().filter(|k| v.get(k).is_some_and(|x| !x.is_null())).collect();
    if present.len() > 1 {
        let list = present.join(", ");
        notes.push(trf!("{who}: víc akcí najednou ({list}) — platí první.", "{who}: several actions ({list}) — the first one wins."));
    }
    for k in present {
        let Some(s) = v[k].as_str().map(str::trim) else {
            notes.push(trf!("{who}: „{k}“ není text.", "{who}: “{k}” is not text."));
            continue;
        };
        let ok = match k {
            "open" => match s.strip_prefix("drawer:") {
                Some(card) => card_ok(card),
                None => !s.is_empty() && s.chars().count() <= MAX_PATH && !s.contains(['\n', '\r', '\0']),
            },
            "url" => url_ok(s),
            "copy" => !s.is_empty() && s.len() <= MAX_COPY && template_ok(s).is_ok(),
            _ => crate::actions::valid_id(s),
        };
        if !ok {
            notes.push(trf!("{who}: akce „{k}“ je mimo povolené (zahozeno).", "{who}: the “{k}” action is not allowed (dropped)."));
            continue;
        }
        if a != Acts::default() {
            continue;
        }
        let s = Some(s.to_string());
        match k {
            "open" => a.open = s,
            "url" => a.url = s,
            "copy" => a.copy = s,
            _ => a.action = s,
        }
    }
    a
}

fn tone(v: &Value, who: &str, notes: &mut Vec<Note>) -> Option<Tone> {
    let t = v.get("tone").filter(|x| !x.is_null())?;
    let Some(arr) = t.get("over").and_then(Value::as_array) else {
        notes.push(trf!("{who}: „tone“ čeká {{\"over\": [[práh, tón], …]}}.", "{who}: “tone” expects {{\"over\": [[threshold, tone], …]}}."));
        return None;
    };
    let mut over: Vec<(f64, KitTone)> = Vec::new();
    for e in arr.iter().take(MAX_TONES) {
        let p = e.as_array().filter(|p| p.len() == 2);
        match p.and_then(|p| Some((p[0].as_f64().filter(|n| n.is_finite())?, KitTone::parse(p[1].as_str()?)?))) {
            Some(x) => over.push(x),
            None => notes.push(trf!("{who}: práh {e} zahozen (čekám [číslo, ok|amber|red|dim]).", "{who}: threshold {e} dropped (expected [number, ok|amber|red|dim]).")),
        }
    }
    if arr.len() > MAX_TONES {
        notes.push(trf!("{who}: nejvýš {MAX_TONES} prahů — zbytek zahozen.", "{who}: at most {MAX_TONES} thresholds — the rest dropped."));
    }
    over.sort_by(|a, b| a.0.total_cmp(&b.0));
    (!over.is_empty()).then_some(Tone { over })
}

fn row(v: &Value, i: usize, notes: &mut Vec<Note>) -> Option<Row> {
    let who = trf!("řádek {n}", "row {n}", n = i + 1);
    if !v.is_object() {
        notes.push(trf!("{who}: není objekt (zahozeno).", "{who}: is not an object (dropped)."));
        return None;
    }
    let text = text_field(v, "text", &who, notes);
    let label = text_field(v, "label", &who, notes);
    let bar = v.get("bar").and_then(Value::as_str).map(str::trim).filter(|b| !b.is_empty()).and_then(|b| {
        if source_ok(b) {
            Some(b.to_string())
        } else {
            notes.push(trf!("{who}: „bar“ není cesta zdroje.", "{who}: “bar” is not a source path."));
            None
        }
    });
    if text.is_none() && bar.is_none() {
        notes.push(trf!("{who}: bez „text“ a „bar“ (zahozeno).", "{who}: has neither “text” nor “bar” (dropped)."));
        return None;
    }
    Some(Row { text, bar, label, tone: tone(v, &who, notes), act: acts(v, &who, notes) })
}

fn button(v: &Value, i: usize, notes: &mut Vec<Note>) -> Option<Button> {
    let who = trf!("tlačítko {n}", "button {n}", n = i + 1);
    let Some(label) = v.is_object().then(|| text_field(v, "label", &who, notes)).flatten() else {
        notes.push(trf!("{who}: bez „label“ (zahozeno).", "{who}: has no “label” (dropped)."));
        return None;
    };
    let act = acts(v, &who, notes);
    if act == Acts::default() {
        notes.push(trf!("{who}: bez povolené akce (zahozeno).", "{who}: has no allowed action (dropped)."));
        return None;
    }
    Some(Button { label, act })
}

/// `*.kit.json` → `Spec`. `Err` = soubor nejde použít (důvody), `Ok` nese i `notes` o zahozeném.
pub fn parse(text: &str) -> Result<Spec, Vec<Note>> {
    let fail = |s: String| Err(vec![s]);
    if text.len() > MAX_BYTES {
        return fail(trf!("soubor je větší než {n} KB", "the file is larger than {n} KB", n = MAX_BYTES / 1024));
    }
    let text = text.strip_prefix('\u{feff}').unwrap_or(text);
    let v: Value = match serde_json::from_str(text) {
        Ok(v) => v,
        Err(e) => return fail(trf!("není platný JSON: {e}", "not valid JSON: {e}")),
    };
    if !v.is_object() {
        return fail(tr("kořen musí být objekt", "the root must be an object").into());
    }
    if v.get("kit").and_then(Value::as_u64) != Some(1) {
        return fail(tr("chybí \"kit\": 1", "missing \"kit\": 1").into());
    }
    let mut notes = Vec::new();
    let (title, cut) = clip(v.get("title").and_then(Value::as_str).unwrap_or(""), MAX_TITLE);
    if title.is_empty() {
        return fail(tr("chybí „title“", "missing “title”").into());
    }
    if cut {
        notes.push(trf!("title je delší než {MAX_TITLE} znaků — ořezáno.", "title is longer than {MAX_TITLE} characters — cut."));
    }
    let icon = v.get("icon").and_then(Value::as_str).map(str::trim).filter(|s| !s.is_empty()).map(|s| clip(s, MAX_ICON).0);
    let Some(rows_v) = v.get("rows").and_then(Value::as_array) else {
        return fail(tr("chybí pole „rows“", "missing the “rows” array").into());
    };
    if rows_v.len() > MAX_ROWS {
        notes.push(trf!("nejvýš {MAX_ROWS} řádků — zbytek zahozen.", "at most {MAX_ROWS} rows — the rest dropped."));
    }
    let rows: Vec<Row> = rows_v.iter().take(MAX_ROWS).enumerate().filter_map(|(i, r)| row(r, i, &mut notes)).collect();
    let btn_v = v.get("buttons").and_then(Value::as_array).map(Vec::as_slice).unwrap_or(&[]);
    if btn_v.len() > MAX_BUTTONS {
        notes.push(trf!("nejvýš {MAX_BUTTONS} tlačítek — zbytek zahozen.", "at most {MAX_BUTTONS} buttons — the rest dropped."));
    }
    let buttons: Vec<Button> = btn_v.iter().take(MAX_BUTTONS).enumerate().filter_map(|(i, b)| button(b, i, &mut notes)).collect();
    if rows.is_empty() && buttons.is_empty() {
        notes.push(tr("nic k zobrazení", "nothing to show").into());
        return Err(notes);
    }
    Ok(Spec { kit: 1, title, icon, rows, buttons, notes })
}

#[cfg(test)]
mod tests;
