//! Rozpoznání času v českém textu poznámky („zítra v 10“, „v pondělí“, „za 2 h“, „25. 10. v 14:00“) pro
//! nabídku připomínky. Čistá funkce `parse_when(text, now)` bez I/O; diakritika a velikost písmen se
//! skládají (`search::Folded`), takže rozsah shody (bajty v originálu) sedí na původní text.
//! Pravidla: datum bez času = 9:00; „ráno / dopoledne / odpoledne / večer“ = 9 / 10 / 14 / 19 h; samotný
//! čas = dnes, nebo zítra, když už byl; den v týdnu = nejbližší budoucí; datum bez roku = letos, nebo
//! příští rok, když už byl; výsledek je vždy v budoucnosti (minulost = žádná shoda). Opatrnost proti
//! falešným shodám: verze „2.10“, „10 %“, „1. bod“, telefonní čísla, „kapitola 3.2.“ nic nenabídnou.

use super::search::Folded;
use super::{iso, local_from_naive};
use crate::i18n::tr;
use chrono::{DateTime, Datelike, Duration, FixedOffset, NaiveDate, NaiveDateTime, NaiveTime, Timelike, Weekday};
use regex::{Captures, Regex};
use std::sync::LazyLock;

/// Nalezený čas: rozsah shody v původním textu (bajty) a čas připomínky.
#[derive(Debug, Clone, PartialEq)]
pub struct When {
    pub start: usize,
    pub end: usize,
    pub at: DateTime<FixedOffset>,
}

/// Hodina bez udání času u data.
const DEFAULT_HOUR: u32 = 9;

const NUM_WORDS: &str = "jeden|jednu|jedno|dva|dve|tri|ctyri|pet|sest|sedm|osm|devet|deset";

/// První část shody (datum / relativní údaj / samotný čas / část dne). Pořadí větví = přednost při
/// shodě na stejném místě.
static PRIMARY: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(&format!(
        r"(?x)
        \b(?P<day>dnes|dneska|zitra|zejtra|pozitri|pozejtri)\b
      | \b(?:(?:v|ve|na)\s+)?(?:pristi\s+)?(?P<wd>pondeli|utery|streda|stredu|ctvrtek|patek|sobota|sobotu|nedele|nedeli)\b
      | \b(?P<d>\d{{1,2}})\.\s*(?:(?P<m>\d{{1,2}})\.|(?P<mn>ledna|unora|brezna|dubna|kvetna|cervna|cervence|srpna|zari|rijna|listopadu|prosince)\b)(?:\s*(?P<y>20\d{{2}})\b)?
      | \bza\s+(?P<pul>pul\s+hodiny)\b
      | \bza\s+(?:(?P<n>\d{{1,3}}|{NUM_WORDS})\s*)?(?P<u>minut\w*|min|hodin\w*|hod|h|dny|dnu|dni|den|tyd\w*)\b
      | (?:\b(?:v|ve)\s+)?\b(?P<hh>\d{{1,2}}):(?P<mm>\d{{2}})\b
      | \b(?:v|ve)\s+(?P<vh>\d{{1,2}})(?:[:.](?P<vm>\d{{2}}))?\b(?:\s*(?:h|hod\w*)\b)?
      | \b(?P<pod>rano|dopoledne|odpoledne|vecer)\b
        "
    ))
    .expect("PRIMARY")
});

/// Čas hned za datem: `v 10:30` / `10:30` / `v 10` / `v 10 hod` / `ráno` …
static TAIL_COLON: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^\s*,?\s*(?:(?:v|ve)\s+)?(\d{1,2}):(\d{2})\b").expect("TAIL_COLON"));
static TAIL_V: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^\s*,?\s*(?:v|ve)\s+(\d{1,2})(?:\.(\d{2}))?\b(?:\s*(?:h|hod\w*)\b)?").expect("TAIL_V"));
static TAIL_POD: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^\s*,?\s*(?:v\s+)?(rano|dopoledne|odpoledne|vecer)\b").expect("TAIL_POD"));
static POD_AFTER: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^\s+(rano|dopoledne|odpoledne|vecer)\b").expect("POD_AFTER"));
/// Slova, po kterých číslo nejspíš není čas ani datum (verze, kapitola, telefon, poměr…).
const NOT_BEFORE: [&str; 21] = [
    "verze", "verzi", "version", "ver", "release", "build", "kapitola", "kapitole", "bod", "bodu", "odstavec", "sekce", "cislo", "c", "no", "tel", "telefon", "pomer", "skore", "port", "id",
];
/// Slova, po kterých „v 2“ není čas (v 2 týdnech, v 5 případech…).
const NOT_AFTER: [&str; 9] = ["tyd", "dn", "mes", "rok", "let", "kus", "proc", "pripad", "krok"];

fn word_before(s: &str, at: usize) -> &str {
    let head = s[..at].trim_end_matches([' ', '\t', ':', ',']);
    let start = head.rfind(|c: char| !c.is_alphanumeric()).map_or(0, |i| i + head[i..].chars().next().map_or(1, char::len_utf8));
    &head[start..]
}

/// Číslo nebo číslovka slovem.
fn count(s: &str) -> Option<i64> {
    s.parse().ok().or(match s {
        "jeden" | "jednu" | "jedno" => Some(1),
        "dva" | "dve" => Some(2),
        "tri" => Some(3),
        "ctyri" => Some(4),
        "pet" => Some(5),
        "sest" => Some(6),
        "sedm" => Some(7),
        "osm" => Some(8),
        "devet" => Some(9),
        "deset" => Some(10),
        _ => None,
    })
}

fn hm(h: &str, m: Option<&str>) -> Option<NaiveTime> {
    NaiveTime::from_hms_opt(h.parse().ok()?, m.map_or(Some(0), |m| m.parse().ok())?, 0)
}

fn pod_time(w: &str) -> Option<NaiveTime> {
    NaiveTime::from_hms_opt(
        match w {
            "rano" => 9,
            "dopoledne" => 10,
            "odpoledne" => 14,
            "vecer" => 19,
            _ => return None,
        },
        0,
        0,
    )
}

fn weekday(w: &str) -> Option<Weekday> {
    Some(match w {
        "pondeli" => Weekday::Mon,
        "utery" => Weekday::Tue,
        "streda" | "stredu" => Weekday::Wed,
        "ctvrtek" => Weekday::Thu,
        "patek" => Weekday::Fri,
        "sobota" | "sobotu" => Weekday::Sat,
        "nedele" | "nedeli" => Weekday::Sun,
        _ => return None,
    })
}

fn month(w: &str) -> Option<u32> {
    ["ledna", "unora", "brezna", "dubna", "kvetna", "cervna", "cervence", "srpna", "zari", "rijna", "listopadu", "prosince"].iter().position(|m| *m == w).map(|i| i as u32 + 1)
}

/// Čas za primární shodou (`t` = zbytek složeného textu): (čas, kolik bajtů zabral).
fn tail(t: &str) -> Option<(NaiveTime, usize)> {
    if let Some(c) = TAIL_COLON.captures(t)
        && let Some(time) = hm(&c[1], Some(&c[2]))
    {
        return Some((time, c.get(0)?.end()));
    }
    if let Some(c) = TAIL_V.captures(t) {
        let end = c.get(0)?.end();
        let rest = &t[end..];
        let after = rest.trim_start();
        // „v 2 týdnech“ není čas
        if NOT_AFTER.iter().any(|w| after.starts_with(w)) {
            return None;
        }
        let mut time = hm(&c[1], c.get(2).map(|m| m.as_str()))?;
        let mut end = end;
        // „v 3 odpoledne“ = 15:00
        if let Some(p) = POD_AFTER.captures(rest) {
            if matches!(&p[1], "odpoledne" | "vecer") && time.hour() < 12 {
                time = time.with_hour(time.hour() + 12)?;
            }
            end += p.get(0)?.end();
        }
        return Some((time, end));
    }
    if let Some(c) = TAIL_POD.captures(t) {
        return Some((pod_time(&c[1])?, c.get(0)?.end()));
    }
    None
}

fn at_naive(n: NaiveDateTime) -> Option<DateTime<FixedOffset>> {
    local_from_naive(n)
}

/// První použitelná shoda v `text` vůči `now` (lokální čas). `None`, když v textu není čas, nebo by
/// vyšel v minulosti.
pub fn parse_when(text: &str, now: &DateTime<FixedOffset>) -> Option<When> {
    let f = Folded::new(text);
    let nown = now.naive_local();
    let today = nown.date();
    let mut pos = 0;
    while pos < f.text.len() {
        let c = PRIMARY.captures_at(&f.text, pos)?;
        let m = c.get(0)?;
        pos = m.end();
        if let Some((end, n)) = candidate(&f.text, &c, m.start(), m.end(), nown, today) {
            // shoda i s časem za datem se přeskočí celá (minulé „dnes v 8:00“ nemá nabídnout „v 8:00“ zítra)
            pos = pos.max(end);
            if let Some(at) = at_naive(n).filter(|a| a > now) {
                let (s, e) = f.orig(m.start(), end);
                return Some(When { start: s, end: e, at });
            }
        }
    }
    None
}

/// Čas shody jako naivní lokální čas + konec shody (včetně času za datem).
fn candidate(s: &str, c: &Captures, start: usize, end: usize, now: NaiveDateTime, today: NaiveDate) -> Option<(usize, NaiveDateTime)> {
    let with_time = |date: NaiveDate| -> (usize, NaiveDateTime) {
        match tail(&s[end..]) {
            Some((t, used)) => (end + used, date.and_time(t)),
            None => (end, date.and_time(NaiveTime::from_hms_opt(DEFAULT_HOUR, 0, 0).unwrap_or_default())),
        }
    };
    let guarded = |at: usize| NOT_BEFORE.contains(&word_before(s, at));
    if let Some(d) = c.name("day") {
        let date = match d.as_str() {
            "dnes" | "dneska" => today,
            "zitra" | "zejtra" => today + Duration::days(1),
            _ => today + Duration::days(2),
        };
        return Some(with_time(date));
    }
    if let Some(w) = c.name("wd") {
        let wd = weekday(w.as_str())?;
        let ahead = (7 + wd.num_days_from_monday() as i64 - today.weekday().num_days_from_monday() as i64) % 7;
        let (e, mut n) = with_time(today + Duration::days(ahead));
        if n <= now {
            n += Duration::days(7);
        }
        return Some((e, n));
    }
    if let Some(d) = c.name("d") {
        // „2.10.“ po verzi / číslu kapitoly, „1.25.10.“ a „25.10.20“ nejsou datum
        let prev = s[..start].chars().next_back();
        let next = s[end..].chars().next();
        if guarded(start) || prev.is_some_and(|p| p.is_ascii_digit() || matches!(p, '.' | ',' | '/' | '-' | '+' | ':')) || next.is_some_and(|n| n.is_ascii_digit()) {
            return None;
        }
        let day: u32 = d.as_str().parse().ok()?;
        let mon = match c.name("m") {
            Some(m) => m.as_str().parse().ok()?,
            None => month(c.name("mn")?.as_str())?,
        };
        let build = |y: i32| NaiveDate::from_ymd_opt(y, mon, day);
        let (e, n) = match c.name("y") {
            Some(y) => with_time(build(y.as_str().parse().ok()?)?),
            None => {
                let (e, n) = with_time(build(today.year())?);
                if n > now { (e, n) } else { with_time(build(today.year() + 1)?) }
            }
        };
        return Some((e, n));
    }
    if c.name("pul").is_some() {
        return Some((end, now.with_second(0)? + Duration::minutes(30)));
    }
    if let Some(u) = c.name("u") {
        let n = c.name("n").map_or(Some(1), |n| count(n.as_str()))?;
        let u = u.as_str();
        let base = now.with_second(0)?;
        return Some(match u {
            _ if u.starts_with("min") => (end, base + Duration::minutes(n)),
            _ if u.starts_with("hod") || u == "h" => (end, base + Duration::hours(n)),
            _ if u.starts_with("tyd") => with_time(today + Duration::weeks(n)),
            _ => with_time(today + Duration::days(n)),
        });
    }
    if let (Some(h), Some(m)) = (c.name("hh"), c.name("mm")) {
        if guarded(start) {
            return None;
        }
        let t = hm(h.as_str(), Some(m.as_str()))?;
        return Some((end, next_time(now, t)));
    }
    if let Some(h) = c.name("vh") {
        let after = s[end..].trim_start();
        if guarded(start) || NOT_AFTER.iter().any(|w| after.starts_with(w)) {
            return None;
        }
        let mut t = hm(h.as_str(), c.name("vm").map(|m| m.as_str()))?;
        let mut end = end;
        if let Some(p) = POD_AFTER.captures(&s[end..]) {
            if matches!(&p[1], "odpoledne" | "vecer") && t.hour() < 12 {
                t = t.with_hour(t.hour() + 12)?;
            }
            end += p.get(0)?.end();
        }
        return Some((end, next_time(now, t)));
    }
    if let Some(p) = c.name("pod") {
        return Some((end, next_time(now, pod_time(p.as_str())?)));
    }
    None
}

/// Dnešní `t`, nebo zítřejší, když dnes už bylo.
fn next_time(now: NaiveDateTime, t: NaiveTime) -> NaiveDateTime {
    let n = now.date().and_time(t);
    if n > now { n } else { n + Duration::days(1) }
}

const CS_DAYS: [&str; 7] = ["v pondělí", "v úterý", "ve středu", "ve čtvrtek", "v pátek", "v sobotu", "v neděli"];
const EN_DAYS: [&str; 7] = ["Monday", "Tuesday", "Wednesday", "Thursday", "Friday", "Saturday", "Sunday"];
const EN_MONTHS: [&str; 12] = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];

/// Krátký popis pro tlačítko „Připomenout …?“: `zítra 10:00`, `v pondělí 09:00`, `25. 10. 14:00`
/// (s rokem, když není letos).
pub fn label(at: &DateTime<FixedOffset>, now: &DateTime<FixedOffset>) -> String {
    let d = at.date_naive();
    let days = (d - now.date_naive()).num_days();
    let time = format!("{:02}:{:02}", at.hour(), at.minute());
    let cs = tr("cs", "en") == "cs";
    let wd = d.weekday().num_days_from_monday() as usize;
    let day = match days {
        0 => tr("dnes", "today").to_string(),
        1 => tr("zítra", "tomorrow").to_string(),
        2 if cs => "pozítří".to_string(),
        2..=6 => if cs { CS_DAYS[wd].to_string() } else { EN_DAYS[wd].to_string() },
        _ if cs => format!("{}. {}.{}", d.day(), d.month(), if d.year() == now.year() { String::new() } else { format!(" {}", d.year()) }),
        _ => format!("{} {}{}", d.day(), EN_MONTHS[d.month0() as usize], if d.year() == now.year() { String::new() } else { format!(" {}", d.year()) }),
    };
    format!("{day} {time}")
}

/// Návrh připomínky pro frontend: popis + ISO s offsetem.
#[derive(Debug, Clone, serde::Serialize, PartialEq)]
pub struct Suggest {
    pub label: String,
    pub iso: String,
}

pub fn suggest(text: &str, now: &DateTime<FixedOffset>) -> Option<Suggest> {
    let w = parse_when(text, now)?;
    Some(Suggest { label: label(&w.at, now), iso: iso(&w.at) })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Pátek 23. 10. 2026 14:05 (lokálně).
    fn now() -> DateTime<FixedOffset> {
        local_from_naive(NaiveDate::from_ymd_opt(2026, 10, 23).unwrap().and_hms_opt(14, 5, 0).unwrap()).unwrap()
    }

    fn got(text: &str) -> Option<(String, String)> {
        parse_when(text, &now()).map(|w| (w.at.naive_local().format("%Y-%m-%d %H:%M").to_string(), text[w.start..w.end].to_string()))
    }

    #[test]
    fn recognises_czech_times() {
        let t = |s: &str, at: &str, m: &str| assert_eq!(got(s), Some((at.to_string(), m.to_string())), "{s}");
        // dnes / zítra / pozítří
        t("zítra v 10", "2026-10-24 10:00", "zítra v 10");
        t("Zavolat Novákovi zítra v 10 kvůli faktuře", "2026-10-24 10:00", "zítra v 10");
        t("ZÍTRA V 10:00", "2026-10-24 10:00", "ZÍTRA V 10:00");
        t("zítra", "2026-10-24 09:00", "zítra");
        t("zejtra v 7.30", "2026-10-24 07:30", "zejtra v 7.30");
        t("pozítří v 10:30", "2026-10-25 10:30", "pozítří v 10:30");
        t("pozitri ve 14", "2026-10-25 14:00", "pozitri ve 14");
        t("dnes v 15:30", "2026-10-23 15:30", "dnes v 15:30");
        t("dnes 16:00", "2026-10-23 16:00", "dnes 16:00");
        t("dnes večer", "2026-10-23 19:00", "dnes večer");
        t("zítra ráno", "2026-10-24 09:00", "zítra ráno");
        t("zítra odpoledne", "2026-10-24 14:00", "zítra odpoledne");
        t("zítra v 3 odpoledne", "2026-10-24 15:00", "zítra v 3 odpoledne");
        t("dnes v 5 večer", "2026-10-23 17:00", "dnes v 5 večer");
        t("zítra, v 10 hod", "2026-10-24 10:00", "zítra, v 10 hod");
        // dny v týdnu = nejbližší budoucí
        t("v pondělí", "2026-10-26 09:00", "v pondělí");
        t("v pondělí v 9", "2026-10-26 09:00", "v pondělí v 9");
        t("ve středu 14:00", "2026-10-28 14:00", "ve středu 14:00");
        t("příští úterý", "2026-10-27 09:00", "příští úterý");
        t("na sobotu", "2026-10-24 09:00", "na sobotu");
        t("v neděli ráno", "2026-10-25 09:00", "v neděli ráno");
        t("v pátek", "2026-10-30 09:00", "v pátek");
        t("v pátek v 16:00", "2026-10-23 16:00", "v pátek v 16:00");
        t("ve čtvrtek", "2026-10-29 09:00", "ve čtvrtek");
        // za …
        t("za 2 h", "2026-10-23 16:05", "za 2 h");
        t("za 30 min", "2026-10-23 14:35", "za 30 min");
        t("za hodinu", "2026-10-23 15:05", "za hodinu");
        t("za půl hodiny", "2026-10-23 14:35", "za půl hodiny");
        t("za dvě hodiny", "2026-10-23 16:05", "za dvě hodiny");
        t("za 3 dny", "2026-10-26 09:00", "za 3 dny");
        t("za týden", "2026-10-30 09:00", "za týden");
        t("za 2 týdny", "2026-11-06 09:00", "za 2 týdny");
        t("za dva dny v 10", "2026-10-25 10:00", "za dva dny v 10");
        t("za 10 minut zavolat", "2026-10-23 14:15", "za 10 minut");
        // datum
        t("25. 10.", "2026-10-25 09:00", "25. 10.");
        t("25. 10. 2026", "2026-10-25 09:00", "25. 10. 2026");
        t("25.10. v 14:00", "2026-10-25 14:00", "25.10. v 14:00");
        t("30. 10. 17:30", "2026-10-30 17:30", "30. 10. 17:30");
        t("1. 10.", "2027-10-01 09:00", "1. 10.");
        t("5. listopadu", "2026-11-05 09:00", "5. listopadu");
        t("25. října v 8", "2026-10-25 08:00", "25. října v 8");
        t("termín 24.12. do 12:00", "2026-12-24 09:00", "24.12.");
        // samotný čas / část dne
        t("v 15:30", "2026-10-23 15:30", "v 15:30");
        t("15:30", "2026-10-23 15:30", "15:30");
        t("v 10", "2026-10-24 10:00", "v 10");
        t("ve 14:00", "2026-10-24 14:00", "ve 14:00");
        t("ráno", "2026-10-24 09:00", "ráno");
        t("večer", "2026-10-23 19:00", "večer");
        t("odpoledne", "2026-10-24 14:00", "odpoledne");
        t("v 3 odpoledne", "2026-10-23 15:00", "v 3 odpoledne");
        // první použitelná shoda; minulá se přeskočí
        t("dnes v 8:00 nebo zítra v 9", "2026-10-24 09:00", "zítra v 9");
    }

    #[test]
    fn ignores_what_is_not_a_time() {
        for s in [
            "",
            "nic tu není",
            "verze 2.10",
            "verze 2.10.",
            "ve verzi 2.10.",
            "v1.2.3",
            "2.10.1",
            "10 %",
            "1. bod",
            "1. 5 položek",
            "kapitola 3.2.",
            "603 123 456",
            "+420 603 123 456",
            "tel. 123 45 67",
            "rok 2026",
            "0.5 l mléka",
            "poměr 3:20",
            "v 2 týdnech",
            "v 5 případech",
            "dnes v 8:00",
            "dnes",
            "1. 10. 2026",
            "99. 99.",
            "32. 1.",
            "30. 2.",
            "25. 13.",
            "25.10.20",
            "ve 25:00",
            "v 10:75",
            "dnesní schůzka",
            "zítřejší",
            "za 5",
            "za kolik",
        ] {
            assert_eq!(got(s), None, "{s:?}");
        }
    }

    #[test]
    fn match_range_is_in_original_bytes() {
        let s = "Řekni Žofii: zítra v 10 ať přinese šéfovi dokumenty";
        let w = parse_when(s, &now()).unwrap();
        assert_eq!(&s[w.start..w.end], "zítra v 10");
        // shoda na konci a na začátku
        let w = parse_when("v pondělí", &now()).unwrap();
        assert_eq!((w.start, w.end), (0, "v pondělí".len()));
    }

    #[test]
    fn labels_are_short() {
        let n = now();
        let l = |s: &str| suggest(s, &n).map(|x| x.label);
        assert_eq!(l("zítra v 10").as_deref(), Some("zítra 10:00"));
        assert_eq!(l("dnes večer").as_deref(), Some("dnes 19:00"));
        assert_eq!(l("pozítří").as_deref(), Some("pozítří 09:00"));
        assert_eq!(l("ve středu 14:00").as_deref(), Some("ve středu 14:00"));
        assert_eq!(l("25. 11.").as_deref(), Some("25. 11. 09:00"));
        assert_eq!(l("1. 10.").as_deref(), Some("1. 10. 2027 09:00"));
        let s = suggest("zítra v 10", &n).unwrap();
        assert!(s.iso.starts_with("2026-10-24T10:00:00"), "{}", s.iso);
        crate::i18n::with_lang(crate::uiconf::Lang::En, || {
            assert_eq!(label(&parse_when("zítra v 10", &n).unwrap().at, &n), "tomorrow 10:00");
            assert_eq!(label(&parse_when("ve středu 14:00", &n).unwrap().at, &n), "Wednesday 14:00");
            assert_eq!(label(&parse_when("25. 11.", &n).unwrap().at, &n), "25 Nov 09:00");
        });
    }
}
