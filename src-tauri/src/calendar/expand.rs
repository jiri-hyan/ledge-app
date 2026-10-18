//! Opakování událostí (RRULE) — běžné případy z Google a Outlooku: FREQ DAILY/WEEKLY/MONTHLY/YEARLY,
//! INTERVAL, COUNT, UNTIL, BYDAY (u WEEKLY dny týdne, u MONTHLY/YEARLY i s pořadím `2MO`, `-1FR`),
//! BYMONTHDAY (i záporné), BYMONTH, BYSETPOS (Outlook: „poslední pracovní den“), WKST.
//! Ostatní části (BYHOUR, BYWEEKNO…) se ignorují; HOURLY a jemnější FREQ = chyba (událost pak jen
//! jednou, v DTSTART).
//!
//! Počítá se v místním čase události (bez pásma) — týdenní schůzka v 9:00 zůstane v 9:00 i přes změnu
//! letního času; převod na UTC dělá volající (`tz.rs`). Stejné funkce slouží i přechodům VTIMEZONE.

use super::ics::{Dt, datetime15};
use chrono::{Datelike, Duration, NaiveDate, NaiveDateTime, NaiveTime, Weekday};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Freq {
    Daily,
    Weekly,
    Monthly,
    Yearly,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Until {
    Date(NaiveDate),
    /// bez `Z` — místní čas události
    Local(NaiveDateTime),
    Utc(NaiveDateTime),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Rule {
    pub freq: Freq,
    pub interval: u32,
    pub count: Option<u32>,
    pub until: Option<Until>,
    /// (pořadí, den): 0 = každý takový den v období, 2 = druhý, −1 = poslední
    pub byday: Vec<(i32, Weekday)>,
    pub bymonthday: Vec<i32>,
    pub bymonth: Vec<u32>,
    pub bysetpos: Vec<i32>,
    pub wkst: Weekday,
}

/// Nejvíc období (dní, týdnů, měsíců, let) od DTSTART — denní událost by se jinak mohla točit věčně.
const MAX_PERIODS: i64 = 60_000;

fn weekday(s: &str) -> Option<Weekday> {
    Some(match s.to_ascii_uppercase().as_str() {
        "MO" => Weekday::Mon,
        "TU" => Weekday::Tue,
        "WE" => Weekday::Wed,
        "TH" => Weekday::Thu,
        "FR" => Weekday::Fri,
        "SA" => Weekday::Sat,
        "SU" => Weekday::Sun,
        _ => return None,
    })
}

fn ints(v: &str) -> Result<Vec<i32>, String> {
    v.split(',').map(|x| x.trim().trim_start_matches('+').parse::<i32>().map_err(|_| format!("číslo „{x}“"))).collect()
}

pub fn parse_rule(s: &str) -> Result<Rule, String> {
    let mut r = Rule { freq: Freq::Daily, interval: 1, count: None, until: None, byday: vec![], bymonthday: vec![], bymonth: vec![], bysetpos: vec![], wkst: Weekday::Mon };
    let mut freq = None;
    for part in s.trim().trim_start_matches("RRULE:").split(';').filter(|p| !p.trim().is_empty()) {
        let (k, v) = part.split_once('=').ok_or_else(|| format!("část bez = ({part})"))?;
        let v = v.trim();
        match k.trim().to_ascii_uppercase().as_str() {
            "FREQ" => {
                freq = Some(match v.to_ascii_uppercase().as_str() {
                    "DAILY" => Freq::Daily,
                    "WEEKLY" => Freq::Weekly,
                    "MONTHLY" => Freq::Monthly,
                    "YEARLY" => Freq::Yearly,
                    other => return Err(format!("FREQ={other} se nepodporuje")),
                })
            }
            "INTERVAL" => r.interval = v.parse::<u32>().map_err(|_| "INTERVAL".to_string())?.max(1),
            "COUNT" => r.count = Some(v.parse().map_err(|_| "COUNT".to_string())?),
            "UNTIL" => {
                r.until = Some(if let Some(u) = v.strip_suffix(['Z', 'z']) {
                    Until::Utc(datetime15(u).ok_or("UNTIL")?)
                } else if v.len() == 8 {
                    match super::ics::parse_dt(v, None, true) {
                        Some(Dt::Date(d)) => Until::Date(d),
                        _ => return Err("UNTIL".into()),
                    }
                } else {
                    Until::Local(datetime15(v).ok_or("UNTIL")?)
                })
            }
            "BYDAY" => {
                for x in v.split(',') {
                    let x = x.trim();
                    let split = x.len().checked_sub(2).ok_or_else(|| format!("BYDAY {x}"))?;
                    let (ord, day) = x.split_at(split);
                    let ord = if ord.is_empty() { 0 } else { ord.trim_start_matches('+').parse::<i32>().map_err(|_| format!("BYDAY {x}"))? };
                    r.byday.push((ord, weekday(day).ok_or_else(|| format!("BYDAY {x}"))?));
                }
            }
            "BYMONTHDAY" => r.bymonthday = ints(v)?.into_iter().filter(|d| *d != 0 && d.abs() <= 31).collect(),
            "BYMONTH" => r.bymonth = ints(v)?.into_iter().filter(|m| (1..=12).contains(m)).map(|m| m as u32).collect(),
            "BYSETPOS" => r.bysetpos = ints(v)?.into_iter().filter(|p| *p != 0).collect(),
            "WKST" => r.wkst = weekday(v).ok_or("WKST")?,
            _ => {} // BYHOUR, BYMINUTE, BYWEEKNO… — ignorováno
        }
    }
    r.freq = freq.ok_or("chybí FREQ")?;
    Ok(r)
}

pub fn days_in_month(y: i32, m: u32) -> u32 {
    let (ny, nm) = if m == 12 { (y + 1, 1) } else { (y, m + 1) };
    NaiveDate::from_ymd_opt(ny, nm, 1).and_then(|d| d.pred_opt()).map_or(28, |d| d.day())
}

/// n-tý (1 = první, −1 = poslední) prvek; mimo rozsah None.
fn nth<T: Copy>(v: &[T], n: i32) -> Option<T> {
    let i = if n > 0 { n as usize - 1 } else { v.len().checked_sub(n.unsigned_abs() as usize)? };
    v.get(i).copied()
}

/// Dny v období `days` (seřazené) podle BYDAY s pořadím (vůči celému období).
fn by_weekday(days: &[NaiveDate], byday: &[(i32, Weekday)]) -> Vec<NaiveDate> {
    let mut out = Vec::new();
    for &(ord, wd) in byday {
        let matching: Vec<NaiveDate> = days.iter().copied().filter(|d| d.weekday() == wd).collect();
        if ord == 0 {
            out.extend(matching);
        } else if let Some(d) = nth(&matching, ord) {
            out.push(d);
        }
    }
    out
}

/// Dny měsíce podle BYMONTHDAY a BYDAY (průnik, když jsou oba); bez nich `default_day`.
pub fn month_days(y: i32, m: u32, byday: &[(i32, Weekday)], bymonthday: &[i32], default_day: u32) -> Vec<NaiveDate> {
    let last = days_in_month(y, m);
    let all: Vec<NaiveDate> = (1..=last).filter_map(|d| NaiveDate::from_ymd_opt(y, m, d)).collect();
    let md: Vec<NaiveDate> = bymonthday
        .iter()
        .filter_map(|&d| if d > 0 { all.get(d as usize - 1).copied() } else { nth(&all, d) })
        .collect();
    let wd = by_weekday(&all, byday);
    let mut out = match (bymonthday.is_empty(), byday.is_empty()) {
        (false, false) => md.into_iter().filter(|d| wd.contains(d)).collect(),
        (false, true) => md,
        (true, false) => wd,
        (true, true) => NaiveDate::from_ymd_opt(y, m, default_day).into_iter().collect(),
    };
    out.sort_unstable();
    out.dedup();
    out
}

fn setpos(set: Vec<NaiveDate>, pos: &[i32]) -> Vec<NaiveDate> {
    if pos.is_empty() {
        return set;
    }
    let mut out: Vec<NaiveDate> = pos.iter().filter_map(|&p| nth(&set, p)).collect();
    out.sort_unstable();
    out.dedup();
    out
}

fn add_months(y: i32, m: u32, k: i64) -> Option<(i32, u32)> {
    let total = y as i64 * 12 + (m as i64 - 1) + k;
    Some((i32::try_from(total.div_euclid(12)).ok()?, total.rem_euclid(12) as u32 + 1))
}

/// Období č. `k` (od DTSTART): (první den období, kandidáti v něm, seřazení).
fn period(r: &Rule, start: NaiveDate, k: i64) -> Option<(NaiveDate, Vec<NaiveDate>)> {
    let step = k * r.interval as i64;
    let month_ok = |d: &NaiveDate| r.bymonth.is_empty() || r.bymonth.contains(&d.month());
    Some(match r.freq {
        Freq::Daily => {
            let d = start.checked_add_signed(Duration::days(step))?;
            let ok = month_ok(&d)
                && (r.bymonthday.is_empty() || month_days(d.year(), d.month(), &[], &r.bymonthday, 0).contains(&d))
                && (r.byday.is_empty() || r.byday.iter().any(|(_, w)| *w == d.weekday()));
            (d, if ok { vec![d] } else { vec![] })
        }
        Freq::Weekly => {
            let back = (start.weekday().num_days_from_monday() + 7 - r.wkst.num_days_from_monday()) % 7;
            let ws = start.checked_sub_signed(Duration::days(back as i64))?.checked_add_signed(Duration::weeks(step))?;
            let wds: Vec<Weekday> = if r.byday.is_empty() { vec![start.weekday()] } else { r.byday.iter().map(|(_, w)| *w).collect() };
            let mut days: Vec<NaiveDate> = wds
                .iter()
                .filter_map(|w| ws.checked_add_signed(Duration::days(((w.num_days_from_monday() + 7 - r.wkst.num_days_from_monday()) % 7) as i64)))
                .filter(month_ok)
                .collect();
            days.sort_unstable();
            days.dedup();
            (ws, days)
        }
        Freq::Monthly => {
            let (y, m) = add_months(start.year(), start.month(), step)?;
            let first = NaiveDate::from_ymd_opt(y, m, 1)?;
            let days = if month_ok(&first) { month_days(y, m, &r.byday, &r.bymonthday, start.day()) } else { vec![] };
            (first, setpos(days, &r.bysetpos))
        }
        Freq::Yearly => {
            let y = start.year().checked_add(i32::try_from(step).ok()?)?;
            let first = NaiveDate::from_ymd_opt(y, 1, 1)?;
            let days = if r.bymonth.is_empty() && r.bymonthday.is_empty() && !r.byday.is_empty() {
                // BYDAY bez BYMONTH: pořadí vůči celému roku (např. 20MO)
                let all: Vec<NaiveDate> = first.iter_days().take_while(|d| d.year() == y).collect();
                let mut v = by_weekday(&all, &r.byday);
                v.sort_unstable();
                v.dedup();
                v
            } else {
                let months = if r.bymonth.is_empty() { vec![start.month()] } else { r.bymonth.clone() };
                let mut v: Vec<NaiveDate> = months.iter().flat_map(|&m| month_days(y, m, &r.byday, &r.bymonthday, start.day())).collect();
                v.sort_unstable();
                v.dedup();
                v
            };
            (first, setpos(days, &r.bysetpos))
        }
    })
}

/// Výskyty od `start` (místní čas události, vždy první výskyt — i když pravidlu neodpovídá) do `end`
/// a `until` (obojí včetně, místní čas). Vrací jen výskyty ≥ `from`; COUNT se počítá od začátku.
pub fn occurrences(r: &Rule, start: NaiveDateTime, until: Option<NaiveDateTime>, from: NaiveDateTime, end: NaiveDateTime) -> Vec<NaiveDateTime> {
    let mut out = Vec::new();
    let mut n = 0u32;
    let limit = until.map_or(end, |u| u.min(end));
    let mut emit = |t: NaiveDateTime| -> bool {
        if t > limit || r.count.is_some_and(|c| n >= c) {
            return false;
        }
        n += 1;
        if t >= from {
            out.push(t);
        }
        true
    };
    if !emit(start) {
        return out;
    }
    let time: NaiveTime = start.time();
    for k in 0..MAX_PERIODS {
        let Some((first, days)) = period(r, start.date(), k) else { break };
        if first > limit.date() {
            break;
        }
        for d in days {
            let t = d.and_time(time);
            if t <= start {
                continue;
            }
            if !emit(t) {
                return out;
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(y: i32, m: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, day).unwrap()
    }
    fn at(y: i32, m: u32, day: u32, h: u32) -> NaiveDateTime {
        d(y, m, day).and_hms_opt(h, 0, 0).unwrap()
    }
    fn dates(v: &[NaiveDateTime]) -> Vec<String> {
        v.iter().map(|t| t.format("%Y-%m-%d").to_string()).collect()
    }
    /// Výskyty pravidla od `start` do konce roku 2027 jako data.
    fn run(rule: &str, start: NaiveDateTime) -> Vec<String> {
        let r = parse_rule(rule).unwrap();
        dates(&occurrences(&r, start, None, start, at(2027, 12, 31, 23)))
    }

    #[test]
    fn parse_rule_table() {
        let r = parse_rule("FREQ=MONTHLY;INTERVAL=2;BYDAY=2MO,-1FR,+1SU,TU;COUNT=5;WKST=SU").unwrap();
        assert_eq!(r.freq, Freq::Monthly);
        assert_eq!(r.interval, 2);
        assert_eq!(r.count, Some(5));
        assert_eq!(r.byday, vec![(2, Weekday::Mon), (-1, Weekday::Fri), (1, Weekday::Sun), (0, Weekday::Tue)]);
        assert_eq!(r.wkst, Weekday::Sun);
        assert_eq!(parse_rule("FREQ=DAILY;UNTIL=20261231T225959Z").unwrap().until, Some(Until::Utc(d(2026, 12, 31).and_hms_opt(22, 59, 59).unwrap())));
        assert_eq!(parse_rule("FREQ=DAILY;UNTIL=20261231").unwrap().until, Some(Until::Date(d(2026, 12, 31))));
        assert_eq!(parse_rule("FREQ=DAILY;UNTIL=20261231T100000").unwrap().until, Some(Until::Local(at(2026, 12, 31, 10))));
        assert_eq!(parse_rule("RRULE:FREQ=YEARLY;BYMONTH=3;BYDAY=-1SU;BYHOUR=2").unwrap().bymonth, vec![3]);
        assert_eq!(parse_rule("FREQ=WEEKLY;INTERVAL=0").unwrap().interval, 1);
        for bad in ["FREQ=HOURLY", "FREQ=MINUTELY", "INTERVAL=2", "FREQ=DAILY;BYDAY=XX", "FREQ=DAILY;COUNT=x", "FREQ=DAILY;UNTIL=2026", "FREQ"] {
            assert!(parse_rule(bad).is_err(), "{bad}");
        }
    }

    #[test]
    fn days_in_month_table() {
        for (y, m, n) in [(2026, 1, 31), (2026, 2, 28), (2028, 2, 29), (2100, 2, 28), (2026, 4, 30), (2026, 12, 31)] {
            assert_eq!(days_in_month(y, m), n, "{y}-{m}");
        }
    }

    /// (pravidlo, DTSTART, prvních N výskytů)
    #[test]
    fn expansion_table() {
        let mon = at(2026, 10, 19, 9); // pondělí
        let cases: &[(&str, NaiveDateTime, &[&str])] = &[
            ("FREQ=DAILY;COUNT=3", mon, &["2026-10-19", "2026-10-20", "2026-10-21"]),
            ("FREQ=DAILY;INTERVAL=3;COUNT=3", mon, &["2026-10-19", "2026-10-22", "2026-10-25"]),
            ("FREQ=DAILY;BYDAY=MO,FR;COUNT=4", mon, &["2026-10-19", "2026-10-23", "2026-10-26", "2026-10-30"]),
            ("FREQ=DAILY;UNTIL=20261021T090000", mon, &["2026-10-19", "2026-10-20", "2026-10-21"]),
            ("FREQ=WEEKLY;COUNT=3", mon, &["2026-10-19", "2026-10-26", "2026-11-02"]),
            ("FREQ=WEEKLY;BYDAY=MO,WE,FR;COUNT=5", mon, &["2026-10-19", "2026-10-21", "2026-10-23", "2026-10-26", "2026-10-28"]),
            ("FREQ=WEEKLY;INTERVAL=2;BYDAY=TU,TH;COUNT=4", mon, &["2026-10-19", "2026-10-20", "2026-10-22", "2026-11-03"]),
            // DTSTART ve středu, pravidlo jen pondělí: DTSTART se počítá jako první výskyt
            ("FREQ=WEEKLY;BYDAY=MO;COUNT=3", at(2026, 10, 21, 9), &["2026-10-21", "2026-10-26", "2026-11-02"]),
            ("FREQ=MONTHLY;COUNT=3", at(2026, 10, 15, 9), &["2026-10-15", "2026-11-15", "2026-12-15"]),
            // 31. jen v měsících, které ho mají
            ("FREQ=MONTHLY;COUNT=4", at(2026, 10, 31, 9), &["2026-10-31", "2026-12-31", "2027-01-31", "2027-03-31"]),
            ("FREQ=MONTHLY;BYDAY=2MO;COUNT=3", at(2026, 10, 12, 9), &["2026-10-12", "2026-11-09", "2026-12-14"]),
            ("FREQ=MONTHLY;BYDAY=-1FR;COUNT=3", at(2026, 10, 30, 9), &["2026-10-30", "2026-11-27", "2026-12-25"]),
            ("FREQ=MONTHLY;BYMONTHDAY=1,15;COUNT=4", at(2026, 10, 1, 9), &["2026-10-01", "2026-10-15", "2026-11-01", "2026-11-15"]),
            ("FREQ=MONTHLY;BYMONTHDAY=-1;COUNT=3", at(2026, 10, 31, 9), &["2026-10-31", "2026-11-30", "2026-12-31"]),
            // Outlook: poslední pracovní den v měsíci
            ("FREQ=MONTHLY;BYDAY=MO,TU,WE,TH,FR;BYSETPOS=-1;COUNT=3", at(2026, 10, 30, 9), &["2026-10-30", "2026-11-30", "2026-12-31"]),
            // Outlook: druhé úterý přes BYSETPOS
            ("FREQ=MONTHLY;BYDAY=TU;BYSETPOS=2;COUNT=2", at(2026, 10, 13, 9), &["2026-10-13", "2026-11-10"]),
            // pátek 13.
            ("FREQ=MONTHLY;BYDAY=FR;BYMONTHDAY=13;COUNT=2", at(2026, 11, 13, 9), &["2026-11-13", "2027-08-13"]),
            ("FREQ=YEARLY;COUNT=2", at(2026, 10, 19, 9), &["2026-10-19", "2027-10-19"]),
            ("FREQ=YEARLY;BYMONTH=3;BYDAY=-1SU;COUNT=2", at(2026, 3, 29, 2), &["2026-03-29", "2027-03-28"]),
            ("FREQ=YEARLY;BYMONTH=11;BYDAY=1SU;COUNT=2", at(2026, 11, 1, 2), &["2026-11-01", "2027-11-07"]),
            // narozeniny 29. 2. jen v přestupném roce
            ("FREQ=YEARLY;COUNT=2", at(2024, 2, 29, 0), &["2024-02-29", "2028-02-29"]),
        ];
        for (rule, start, want) in cases {
            let r = parse_rule(rule).unwrap();
            // UNTIL bez Z = místní čas události (převod UTC dělá volající)
            let until = match &r.until {
                Some(Until::Local(t)) => Some(*t),
                Some(Until::Date(d)) => d.and_hms_opt(23, 59, 59),
                _ => None,
            };
            let got = dates(&occurrences(&r, *start, until, *start, at(2030, 12, 31, 0)));
            assert_eq!(got, *want, "{rule}");
        }
    }

    #[test]
    fn window_count_until_and_time_of_day() {
        let r = parse_rule("FREQ=DAILY;COUNT=10").unwrap();
        let start = at(2026, 10, 1, 9);
        // okno od 8. 10.: COUNT se počítá od začátku → zbydou 8., 9., 10.
        let v = occurrences(&r, start, None, at(2026, 10, 8, 0), at(2026, 12, 1, 0));
        assert_eq!(dates(&v), ["2026-10-08", "2026-10-09", "2026-10-10"]);
        assert!(v.iter().all(|t| t.time() == start.time()));
        // UNTIL přes parametr (volající převede do místního času)
        let r = parse_rule("FREQ=WEEKLY").unwrap();
        assert_eq!(dates(&occurrences(&r, start, Some(at(2026, 10, 15, 9)), start, at(2027, 1, 1, 0))), ["2026-10-01", "2026-10-08", "2026-10-15"]);
        // konec okna
        assert_eq!(dates(&occurrences(&r, start, None, start, at(2026, 10, 10, 0))), ["2026-10-01", "2026-10-08"]);
        // UNTIL před DTSTART → nic
        assert!(occurrences(&r, start, Some(at(2026, 9, 1, 0)), start, at(2027, 1, 1, 0)).is_empty());
    }

    #[test]
    fn old_daily_event_stays_bounded() {
        // denní od roku 2000 bez konce: do okna 2027 doběhne a nezasekne se
        let r = parse_rule("FREQ=DAILY").unwrap();
        let v = occurrences(&r, at(2000, 1, 1, 8), None, at(2027, 12, 30, 0), at(2027, 12, 31, 23));
        assert_eq!(dates(&v), ["2027-12-30", "2027-12-31"]);
        assert_eq!(run("FREQ=YEARLY;INTERVAL=5", at(2016, 6, 1, 0)), ["2016-06-01", "2021-06-01", "2026-06-01"]);
    }

    #[test]
    fn month_days_table() {
        type Case<'a> = (u32, &'a [(i32, Weekday)], &'a [i32], u32, &'a [u32]);
        let cases: &[Case] = &[
            (10, &[], &[], 19, &[19]),
            (11, &[], &[], 31, &[]),
            (10, &[(0, Weekday::Mon)], &[], 1, &[5, 12, 19, 26]),
            (10, &[(1, Weekday::Thu), (-1, Weekday::Sat)], &[], 1, &[1, 31]),
            (10, &[(5, Weekday::Mon)], &[], 1, &[]),
            (10, &[], &[-1, 1, 40], 1, &[1, 31]),
            (2, &[], &[30], 1, &[]),
        ];
        for (m, byday, bymd, def, want) in cases {
            let got: Vec<u32> = month_days(2026, *m, byday, bymd, *def).iter().map(|d| d.day()).collect();
            assert_eq!(got, *want, "měsíc {m} {byday:?} {bymd:?}");
        }
    }
}
