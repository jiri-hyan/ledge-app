//! Gramatika řádku úkolu — JEDINÉ místo (čisté funkce, bez I/O). Text za značkou `- [ ] ` je
//! `text [⏰ YYYY-MM-DD[ HH:MM]] [! | !!] [ — důkaz: …]`: ocas (termín, priorita) stojí na konci textu,
//! tokeny jsou oddělené právě jednou mezerou. Čtou ho `md::tasks`, `brief::scan_body`, `connector::plan`,
//! rychlý zápis (`taskquick`) i index úkolů (`taskindex`); TS zrcadlo `src/notes/taskline.ts` sdílí tabulku
//! `taskline_cases.json`.
//!
//! Pravidla (round-trip): `render(parse_tail(x)) == x` pro KAŽDÝ text (i neplatný — co gramatika nezná,
//! zůstane v `text` bajt po bajtu, včetně koncových mezer a důkazu); `!` je priorita jen jako samostatný
//! token na konci („Pozor!“, `a!b`, `Co?!`, `!!!`, URL ne); víc `⏰` → platí poslední v ocase, ostatní zůstanou
//! v textu; `📅 YYYY-MM-DD` (Obsidian Tasks) se čte jako celodenní termín a při zápisu beze změny termínu
//! zůstane `📅`; čas je místní (bez offsetu), bez času = celodenní.

use super::search::fold_text;
use chrono::{Datelike, NaiveDate, NaiveDateTime, NaiveTime, Timelike};

/// Značka termínu (zapisuje se) a Obsidian Tasks (jen čtení).
pub const DUE_MARK: &str = "⏰";
pub const CAL_MARK: &str = "📅";
/// Značka řádku pod bodem odeslaným do propojené aplikace (píše `connector::link`: `↳ [{název}](url)`,
/// `↳ [{název}] id`); rozpozná jakýkoli název i holé `↳ [`. Bod se pak nepočítá jako vlastní úkol.
/// Shodné s `SENT_MARK` v `src/notes/taskline.ts`; společné případy `sent_mark_cases.json`.
pub const SENT_MARK: &str = "↳ [";

/// Rozebraný řádek úkolu (výpůjčky z původního textu).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TaskLine<'a> {
    /// text bez ocasu (termín, priorita), důkazu a koncových mezer
    pub text: &'a str,
    /// text + tokeny (bez důkazu a koncových mezer) — jak ho dosud znal `brief::Item::text`
    pub head: &'a str,
    pub due: Option<NaiveDateTime>,
    /// termín bez času (`due` má 00:00)
    pub all_day: bool,
    /// termín přečtený z `📅` (zapíše se zpět jako `📅`, dokud ho nezměníš)
    pub cal: bool,
    /// 0 žádná, 1 = `!`, 2 = `!!`
    pub prio: u8,
    /// ` — důkaz: …` včetně mezer před pomlčkou, jinak `""`
    pub evidence_raw: &'a str,
    /// koncové bílé znaky (jen bez důkazu)
    pub trail: &'a str,
}

impl TaskLine<'_> {
    /// Text důkazu bez značky (`None` = žádný nebo prázdný).
    pub fn evidence(&self) -> Option<&str> {
        evidence_text(self.evidence_raw)
    }
}

/// Začátek důkazu (pomlčka s mezerami + `důkaz:` / `dukaz:` / `evidence:`): (index mezery před pomlčkou
/// po ořezu mezer, index za dvojtečkou). Platí nejdřívější shoda.
fn evidence_at(text: &str) -> Option<(usize, usize)> {
    let mut best: Option<(usize, usize)> = None;
    for dash in [" — ", " – ", " - "] {
        for (i, _) in text.match_indices(dash) {
            let rest = &text[i + dash.len()..];
            for kw in ["důkaz", "dukaz", "evidence"] {
                if rest.get(..kw.len()).is_some_and(|h| h.to_lowercase() == kw)
                    && rest[kw.len()..].starts_with(':')
                    && best.is_none_or(|(b, _)| i < b)
                {
                    best = Some((i, i + dash.len() + kw.len() + 1));
                }
            }
        }
    }
    best.map(|(i, e)| (text[..i].trim_end().len(), e))
}

fn evidence_text(raw: &str) -> Option<&str> {
    let (_, e) = evidence_at(raw)?;
    Some(raw[e..].trim()).filter(|e| !e.is_empty())
}

/// Text bodu rozdělený na text (včetně tokenů) a důkaz (` — důkaz: …`, též `–`/`-`).
pub fn split_evidence(text: &str) -> (&str, Option<&str>) {
    match evidence_at(text) {
        Some((i, e)) => (&text[..i], Some(text[e..].trim()).filter(|e| !e.is_empty())),
        None => (text.trim_end(), None),
    }
}

/// `YYYY-MM-DD` (přesně, platné datum).
fn parse_date(s: &str) -> Option<NaiveDate> {
    let b = s.as_bytes();
    let ok = b.len() == 10 && b[4] == b'-' && b[7] == b'-' && b.iter().enumerate().all(|(i, c)| matches!(i, 4 | 7) || c.is_ascii_digit());
    ok.then(|| NaiveDate::from_ymd_opt(s[..4].parse().ok()?, s[5..7].parse().ok()?, s[8..].parse().ok()?)).flatten()
}

/// `HH:MM` (přesně, 24 h).
fn parse_hm(s: &str) -> Option<NaiveTime> {
    let b = s.as_bytes();
    let ok = b.len() == 5 && b[2] == b':' && b.iter().enumerate().all(|(i, c)| i == 2 || c.is_ascii_digit());
    ok.then(|| NaiveTime::from_hms_opt(s[..2].parse().ok()?, s[3..].parse().ok()?, 0)).flatten()
}

/// Odděl poslední token (oddělený jednou mezerou). `head` nesmí být prázdný ani končit mezerou, jinak by
/// zápis tokenu nebyl bajtově obnovitelný.
fn last_token(s: &str) -> (Option<&str>, &str) {
    match s.rsplit_once(' ') {
        Some((h, t)) => (Some(h), t),
        None => (None, s),
    }
}

fn clean_head(h: Option<&str>) -> Option<&str> {
    match h {
        None => Some(""),
        Some(h) if !h.is_empty() && !h.ends_with(' ') => Some(h),
        _ => None,
    }
}

/// Priorita na konci: samostatné `!` / `!!`.
fn peel_prio(s: &str) -> Option<(&str, u8)> {
    let (h, t) = last_token(s);
    let p = match t {
        "!" => 1,
        "!!" => 2,
        _ => return None,
    };
    Some((clean_head(h)?, p))
}

/// Termín na konci: `⏰ DATUM [ČAS]` nebo `📅 DATUM`. (zbytek, čas, celodenní, z 📅)
fn peel_due(s: &str) -> Option<(&str, NaiveDateTime, bool, bool)> {
    let (h1, t1) = last_token(s);
    let (date_tok, time, h) = match parse_hm(t1) {
        Some(tm) => {
            let (h2, d) = last_token(h1?);
            (d, Some(tm), h2)
        }
        None => (t1, None, h1),
    };
    let date = parse_date(date_tok)?;
    let (h3, mark) = last_token(h?);
    let cal = match mark {
        DUE_MARK => false,
        CAL_MARK if time.is_none() => true,
        _ => return None,
    };
    Some((clean_head(h3)?, date.and_time(time.unwrap_or_default()), time.is_none(), cal))
}

/// Rozebere text za značkou úkolu (`- [ ] ` už je pryč).
pub fn parse_tail(s: &str) -> TaskLine<'_> {
    let (pre, evidence_raw) = match evidence_at(s) {
        Some((i, _)) => (&s[..i], &s[i..]),
        None => (s, ""),
    };
    let head = pre.trim_end();
    let trail = &pre[head.len()..];
    let mut text = head;
    let (mut prio, mut due, mut all_day, mut cal) = (0, None, false, false);
    if let Some((h, p)) = peel_prio(text) {
        (text, prio) = (h, p);
    }
    if let Some((h, d, a, c)) = peel_due(text) {
        (text, due, all_day, cal) = (h, Some(d), a, c);
    }
    TaskLine { text, head, due, all_day, cal, prio, evidence_raw, trail }
}

/// Kanonický zápis termínu (`YYYY-MM-DD` nebo `YYYY-MM-DD HH:MM`).
pub fn due_text(d: NaiveDateTime, all_day: bool) -> String {
    let date = format!("{:04}-{:02}-{:02}", d.year(), d.month(), d.day());
    if all_day { date } else { format!("{date} {:02}:{:02}", d.hour(), d.minute()) }
}

/// Opak `parse_tail`: `render(parse_tail(x)) == x`.
pub fn render(t: &TaskLine) -> String {
    let mut o = String::with_capacity(t.head.len() + t.evidence_raw.len() + t.trail.len() + 24);
    o.push_str(t.text);
    let sep = |o: &mut String| {
        if !o.is_empty() {
            o.push(' ');
        }
    };
    if let Some(d) = t.due {
        sep(&mut o);
        o.push_str(if t.cal && t.all_day { CAL_MARK } else { DUE_MARK });
        o.push(' ');
        o.push_str(&due_text(d, t.all_day));
    }
    if t.prio > 0 {
        sep(&mut o);
        o.push_str(if t.prio >= 2 { "!!" } else { "!" });
    }
    o.push_str(t.evidence_raw);
    o.push_str(t.trail);
    o
}

/// Nový termín (`None` = odebrat; `all_day` → čas se zahodí). Zbytek řádku beze změny.
pub fn set_due(s: &str, due: Option<(NaiveDateTime, bool)>) -> String {
    let mut t = parse_tail(s);
    match due {
        Some((d, all)) => {
            let d = if all { d.date().and_time(NaiveTime::default()) } else { d };
            (t.due, t.all_day, t.cal) = (Some(d), all, false);
        }
        None => (t.due, t.all_day, t.cal) = (None, false, false),
    }
    render(&t)
}

/// Termín z UI / příkazu: `YYYY-MM-DD` (celodenní), `YYYY-MM-DD HH:MM` (též s `T`), prázdné = odebrat.
pub fn parse_due_arg(s: &str) -> Result<Option<(NaiveDateTime, bool)>, String> {
    let s = s.trim();
    if s.is_empty() {
        return Ok(None);
    }
    let bad = || trf!("Termín „{s}“ nejde přečíst (YYYY-MM-DD nebo YYYY-MM-DD HH:MM).", "Can’t read the date “{s}” (YYYY-MM-DD or YYYY-MM-DD HH:MM).");
    let (d, t) = match s.split_once([' ', 'T']) {
        Some((d, t)) => (d, Some(t.trim())),
        None => (s, None),
    };
    let date = parse_date(d).ok_or_else(bad)?;
    match t {
        None => Ok(Some((date.and_time(NaiveTime::default()), true))),
        Some(t) => Ok(Some((date.and_time(parse_hm(t.get(..5).unwrap_or(t)).ok_or_else(bad)?), false))),
    }
}

/// Nová priorita (0 = odebrat, 1 = `!`, 2+ = `!!`).
#[allow(dead_code)] // zápis priority z UI (U2); zatím jen testy
pub fn set_prio(s: &str, prio: u8) -> String {
    let mut t = parse_tail(s);
    t.prio = prio.min(2);
    render(&t)
}

/// Priorita 1–5 ze slova: `!` = 4, `!!` / `!!!` = 5, `!1`…`!5` = N; „Pozor!“ ani `!` uprostřed slova ne.
pub fn prio_word(w: &str) -> Option<u8> {
    let core = w.trim_end_matches([',', ';', '.']);
    let n = core.chars().count();
    if (1..=3).contains(&n) && core.chars().all(|c| c == '!') {
        return Some(if n == 1 { 4 } else { 5 });
    }
    match core.strip_prefix('!')?.as_bytes() {
        [c @ b'1'..=b'5'] => Some(c - b'0'),
        _ => None,
    }
}

/// Změní jediný řádek úkolu v těle (`line` od 0 jako `md::tasks`) funkcí nad textem za značkou; zbytek těla
/// (i `\r`) zůstane bajt po bajtu. Řádek, který úkolem není, = chyba.
pub fn edit_line(body: &str, line: usize, f: impl FnOnce(&str) -> String) -> Result<String, String> {
    let nope = || crate::i18n::tr("Na tomhle řádku už úkol není — poznámka se mezitím změnila.", "There's no task on this line any more — the note changed meanwhile.").to_string();
    let (pos, raw) = body.split('\n').scan(0usize, |p, l| {
        let at = *p;
        *p += l.len() + 1;
        Some((at, l))
    }).nth(line).ok_or_else(nope)?;
    if !super::md::task_lines(body).any(|(i, _, _)| i == line) {
        return Err(nope());
    }
    let l = raw.trim_end_matches('\r');
    let (_, _, rest) = super::md::list_item(l).ok_or_else(nope)?;
    let mark = l.len() - rest.len() + 3; // „[ ]“
    let after = &l[mark..];
    let lead = &after[..after.len() - after.trim_start().len()];
    let new = format!("{}{lead}{}", &l[..mark], f(after.trim_start()));
    let mut out = String::with_capacity(body.len() + 32);
    out.push_str(&body[..pos]);
    out.push_str(&new);
    out.push_str(&body[pos + l.len()..]);
    Ok(out)
}

// ---------- slova řádku: priorita a řešitel (z `connector::plan`, beze změny chování) ----------

/// Koncová interpunkce slova.
pub const TRAIL: [char; 6] = [',', ';', '.', ':', '!', '?'];

fn is_keyword(w: &str) -> bool {
    matches!(fold_text(w.trim_end_matches(TRAIL)).as_str(), "resitel" | "resitelka" | "assignee")
}

/// Slovo jako část jména: písmena, `-`, apostrof; bez čísel a značek.
fn name_like(w: &str) -> bool {
    !w.is_empty() && w.chars().all(|c| c.is_alphabetic() || c == '-' || c == '\'')
}

/// Zmínka o člověku v řádku (bez párování se členy týmu).
#[derive(Debug, Clone, PartialEq)]
pub struct Mention {
    /// index slova s „řešitel“ / `@jméno`
    pub at: usize,
    /// kolik slov za klíčovým slovem je spojka („je“, „bude“) — patří k frázi, ale není jméno
    pub skip: usize,
    /// kandidátní slova jména (až 3; u `@x` jediné), bez `@` a koncové interpunkce
    pub cand: Vec<String>,
    /// `@jméno` (jedno slovo, žádné zkracování délky shody)
    pub at_sign: bool,
}

/// Rozebraný řádek bodu — jen text, žádní členové týmu, žádný plán.
#[derive(Debug, Clone, PartialEq)]
pub struct LineParts {
    pub words: Vec<String>,
    /// slova, která jsou značka priority (`!`, `!!`, `!N`)
    pub prio_words: Vec<usize>,
    pub priority: Option<u8>,
    pub mention: Option<Mention>,
}

/// Rozebere text bodu: slova, priorita a první zmínka o řešiteli. Čistá funkce (tabulkový test `parse_line_table`).
pub fn parse_line(text: &str) -> LineParts {
    let words: Vec<String> = text.split_whitespace().map(str::to_string).collect();
    let mut prio_words = Vec::new();
    let mut priority = None;
    for (i, w) in words.iter().enumerate() {
        if let Some(p) = prio_word(w) {
            prio_words.push(i);
            priority.get_or_insert(p);
        }
    }
    let mut mention = None;
    for (i, w) in words.iter().enumerate() {
        if is_keyword(w) {
            // až 3 slova jména za klíčovým slovem; čárka / středník / tečka jméno ukončí
            let skip = usize::from(words.get(i + 1).is_some_and(|n| matches!(fold_text(n.trim_end_matches(TRAIL)).as_str(), "je" | "bude" | "is")));
            let mut cand: Vec<String> = Vec::new();
            for (k, nw) in words[i + 1 + skip..].iter().take(3).enumerate() {
                let core = nw.trim_end_matches(TRAIL);
                let core = if k == 0 { core.strip_prefix('@').unwrap_or(core) } else { core };
                if !name_like(core) {
                    break;
                }
                cand.push(core.to_string());
                if nw.ends_with([',', ';', '.']) {
                    break;
                }
            }
            if !cand.is_empty() {
                mention = Some(Mention { at: i, skip, cand, at_sign: false });
                break;
            }
        } else if let Some(core) = w.strip_prefix('@') {
            let core = core.trim_end_matches(TRAIL);
            // `@src/App.tsx:4` (odkaz na soubor) ani e-mail není zmínka o člověku
            if !core.is_empty() && !core.contains(['/', '\\', ':']) && core.chars().all(|c| c.is_alphanumeric() || matches!(c, '.' | '_' | '-')) {
                mention = Some(Mention { at: i, skip: 0, cand: vec![core.to_string()], at_sign: true });
                break;
            }
        }
    }
    LineParts { words, prio_words, priority, mention }
}

#[cfg(test)]
#[path = "taskline_tests.rs"]
mod tests;
