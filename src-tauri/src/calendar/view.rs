//! Co karta Kalendář ukáže: Dnes (bez skončených), Zítra, a když obojí chybí, nejbližší další den
//! s událostí („Příští“). Celodenní nahoře, pak podle začátku; probíhající označená. Vrací i čas
//! další změny (začátek / konec dnešní události, půlnoc), aby emit vlákno probudilo šuplík jen tehdy.

use super::tz::LocalTz;
use super::{AHEAD_DAYS, Occ, ms, naive};
use chrono::{Datelike, Duration, NaiveDate, NaiveTime};
use serde::Serialize;

#[derive(Serialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Item {
    /// ms epochy; u celodenní půlnoc data jako UTC (UI čas neukazuje)
    pub start: i64,
    pub end: i64,
    pub all_day: bool,
    pub title: String,
    pub location: String,
    /// index kalendáře v `personal.calendars`
    pub cal: usize,
    pub ongoing: bool,
    pub tz_guess: bool,
    /// odkaz „Připojit“ (`meeting::Join`), `null` = událost žádný nemá
    pub join: Option<super::meeting::Join>,
}

#[derive(Serialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Day {
    /// today · tomorrow · later
    pub kind: &'static str,
    /// místní datum `YYYY-MM-DD`
    pub date: String,
    pub items: Vec<Item>,
}

#[derive(Debug, Default, PartialEq)]
pub struct Picked {
    pub days: Vec<Day>,
    /// kolik událostí se nevešlo do `max_rows`
    pub more: usize,
    /// kdy se výběr změní (ms epochy)
    pub next_change: Option<i64>,
}

/// Nulová událost (připomínka) „trvá“ minutu, ať ji karta stihne ukázat.
fn eff_end(o: &Occ) -> i64 {
    o.end.max(o.start + 60_000)
}

fn ymd(d: NaiveDate) -> String {
    format!("{:04}-{:02}-{:02}", d.year(), d.month(), d.day())
}

/// Probíhá teď nějaká časovaná (ne celodenní) událost? Nulová událost „trvá“ minutu jako v `pick`.
pub fn any_ongoing<'a>(occs: impl IntoIterator<Item = &'a Occ>, now: i64) -> bool {
    occs.into_iter().any(|o| !o.all_day && o.start <= now && now < eff_end(o))
}

pub fn pick(occs: &[(usize, &Occ)], now: i64, local: &dyn LocalTz, max_rows: usize) -> Picked {
    let now_n = naive(now);
    let today = (now_n + Duration::seconds(local.offset_utc(now_n) as i64)).date();
    // místní půlnoc dne → ms epochy
    let midnight = |d: NaiveDate| {
        let t = d.and_time(NaiveTime::MIN);
        ms(t - Duration::seconds(local.offset_local(t) as i64))
    };
    let on_day = |o: &Occ, d: NaiveDate| {
        if o.all_day {
            naive(o.start).date() <= d && d < naive(o.end).date()
        } else {
            o.start < midnight(d + Duration::days(1)) && eff_end(o) > midnight(d)
        }
    };
    let items = |d: NaiveDate, only_left: bool| -> Vec<Item> {
        let mut v: Vec<Item> = occs
            .iter()
            .filter(|(_, o)| on_day(o, d) && (!only_left || o.all_day || eff_end(o) > now))
            .map(|&(cal, o)| Item {
                start: o.start,
                end: o.end,
                all_day: o.all_day,
                title: o.title.clone(),
                location: o.location.clone(),
                cal,
                ongoing: !o.all_day && o.start <= now && now < eff_end(o),
                tz_guess: o.tz_guess,
                join: o.join.clone(),
            })
            .collect();
        v.sort_by(|a, b| (!a.all_day, a.start, &a.title, a.cal).cmp(&(!b.all_day, b.start, &b.title, b.cal)));
        v
    };
    let mut days = Vec::new();
    let t = items(today, true);
    let next_change = t
        .iter()
        .filter(|i| !i.all_day)
        .flat_map(|i| [i.start, i.end.max(i.start + 60_000)])
        .filter(|&x| x > now)
        .fold(midnight(today + Duration::days(1)), i64::min);
    if !t.is_empty() {
        days.push(Day { kind: "today", date: ymd(today), items: t });
    }
    let tomorrow = today + Duration::days(1);
    let m = items(tomorrow, false);
    if !m.is_empty() {
        days.push(Day { kind: "tomorrow", date: ymd(tomorrow), items: m });
    }
    if days.is_empty() {
        for k in 2..=AHEAD_DAYS {
            let d = today + Duration::days(k);
            let v = items(d, false);
            if !v.is_empty() {
                days.push(Day { kind: "later", date: ymd(d), items: v });
                break;
            }
        }
    }
    // strop řádků přes všechny dny
    let (mut left, mut more) = (max_rows, 0);
    for d in &mut days {
        let keep = d.items.len().min(left);
        more += d.items.len() - keep;
        d.items.truncate(keep);
        left -= keep;
    }
    days.retain(|d| !d.items.is_empty());
    Picked { days, more, next_change: Some(next_change) }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::calendar::tz::builtin;

    fn utc(d: u32, h: u32, mi: u32) -> i64 {
        ms(NaiveDate::from_ymd_opt(2026, 10, d).unwrap().and_hms_opt(h, mi, 0).unwrap())
    }
    fn timed(title: &str, start: i64, end: i64) -> Occ {
        Occ { start, end, title: title.into(), ..Occ::default() }
    }
    #[test]
    fn any_ongoing_table() {
        let t = |s: i64, e: i64| timed("x", s, e);
        let ad = allday("Svátek", 15, 16);
        let now = utc(15, 10, 0);
        type Case<'a> = (&'a str, Vec<Occ>, bool);
        let cases: Vec<Case> = vec![
            ("nic", vec![], false),
            ("probíhá", vec![t(now - 1000, now + 1000)], true),
            ("začíná přesně teď", vec![t(now, now + 60_000)], true),
            ("končí přesně teď", vec![t(now - 60_000, now)], false),
            ("ještě nezačala", vec![t(now + 1, now + 120_000)], false),
            ("už skončila", vec![t(now - 120_000, now - 1)], false),
            ("celodenní se nepočítá", vec![ad.clone()], false),
            ("nulová trvá minutu", vec![t(now - 30_000, now - 30_000)], true),
            ("nulová po minutě ne", vec![t(now - 60_000, now - 60_000)], false),
            ("celodenní + probíhající", vec![ad, t(now - 1, now + 1)], true),
        ];
        for (name, occs, want) in cases {
            assert_eq!(any_ongoing(&occs, now), want, "{name}");
        }
    }

    fn allday(title: &str, from: u32, to: u32) -> Occ {
        Occ { start: utc(from, 0, 0), end: utc(to, 0, 0), all_day: true, title: title.into(), ..Occ::default() }
    }
    /// (druh dne, názvy, probíhající názvy)
    fn shape(p: &Picked) -> Vec<(&'static str, Vec<String>, Vec<String>)> {
        p.days
            .iter()
            .map(|d| (d.kind, d.items.iter().map(|i| i.title.clone()).collect(), d.items.iter().filter(|i| i.ongoing).map(|i| i.title.clone()).collect()))
            .collect()
    }

    #[test]
    fn today_tomorrow_later_table() {
        let tz = builtin("Europe/Prague").unwrap();
        // 18. 10. 2026 12:00 v Praze = 10:00 UTC
        let now = utc(18, 10, 0);
        let s = |v: &[&str]| v.iter().map(|x| x.to_string()).collect::<Vec<_>>();
        type Shape<'a> = Vec<(&'a str, Vec<String>, Vec<String>)>;
        let cases: Vec<(&str, Vec<Occ>, Shape)> = vec![
            ("nic", vec![], vec![]),
            (
                "dnes: skončené pryč, probíhající, celodenní nahoře",
                vec![timed("Ráno", utc(18, 6, 0), utc(18, 7, 0)), timed("Teď", utc(18, 9, 30), utc(18, 10, 30)), timed("Večer", utc(18, 17, 0), utc(18, 18, 0)), allday("Svátek", 18, 19)],
                vec![("today", s(&["Svátek", "Teď", "Večer"]), s(&["Teď"]))],
            ),
            (
                "dnes i zítra; 23:30 místního patří ještě dnešku, 0:30 už zítřku",
                vec![timed("Pozdě", utc(18, 21, 30), utc(18, 21, 45)), timed("Po půlnoci", utc(18, 22, 30), utc(18, 23, 0)), timed("Zítra", utc(19, 7, 0), utc(19, 8, 0))],
                vec![("today", s(&["Pozdě"]), vec![]), ("tomorrow", s(&["Po půlnoci", "Zítra"]), vec![])],
            ),
            (
                "jen zítra",
                vec![timed("Ráno", utc(18, 6, 0), utc(18, 7, 0)), allday("Výlet", 19, 21)],
                vec![("tomorrow", s(&["Výlet"]), vec![])],
            ),
            (
                "dnes ani zítra nic → nejbližší další den",
                vec![timed("Za týden", utc(25, 9, 0), utc(25, 10, 0)), timed("Za dva", utc(27, 9, 0), utc(27, 10, 0))],
                vec![("later", s(&["Za týden"]), vec![])],
            ),
            (
                "vícedenní časová přes půlnoc je dnes i zítra",
                vec![timed("Noc", utc(18, 19, 0), utc(19, 6, 0))],
                vec![("today", s(&["Noc"]), vec![]), ("tomorrow", s(&["Noc"]), vec![])],
            ),
            ("nulová připomínka minutu", vec![timed("Pípnutí", utc(18, 10, 0), utc(18, 10, 0))], vec![("today", s(&["Pípnutí"]), s(&["Pípnutí"]))]),
        ];
        for (name, occs, want) in cases {
            let refs: Vec<(usize, &Occ)> = occs.iter().map(|o| (0, o)).collect();
            assert_eq!(shape(&pick(&refs, now, &tz, 20)), want, "{name}");
        }
    }

    #[test]
    fn rows_limit_dates_and_next_change() {
        let tz = builtin("Europe/Prague").unwrap();
        let now = utc(18, 10, 0);
        let occs: Vec<Occ> = (0..6).map(|i| timed(&format!("E{i}"), utc(18, 11 + i, 0), utc(18, 11 + i, 30))).chain([timed("Z", utc(19, 8, 0), utc(19, 9, 0))]).collect();
        let refs: Vec<(usize, &Occ)> = occs.iter().enumerate().map(|(i, o)| (i % 2, o)).collect();
        let p = pick(&refs, now, &tz, 4);
        assert_eq!(p.days.len(), 1);
        assert_eq!(p.days[0].date, "2026-10-18");
        assert_eq!(p.days[0].items.len(), 4);
        assert_eq!(p.more, 3);
        assert_eq!(p.days[0].items[1].cal, 1);
        // nejbližší změna = začátek E0 (11:00 UTC)
        assert_eq!(p.next_change, Some(utc(18, 11, 0)));
        // bez dnešních událostí = místní půlnoc (22:00 UTC v letním čase)
        let z = [timed("Z", utc(19, 8, 0), utc(19, 9, 0))];
        let p = pick(&[(0, &z[0])], now, &tz, 4);
        assert_eq!(p.next_change, Some(utc(18, 22, 0)));
        assert_eq!((p.days[0].kind, p.days[0].date.as_str()), ("tomorrow", "2026-10-19"));
        // probíhající: další změna je její konec
        let r = [timed("Teď", utc(18, 9, 0), utc(18, 10, 15))];
        assert_eq!(pick(&[(0, &r[0])], now, &tz, 4).next_change, Some(utc(18, 10, 15)));
    }
}
