//! Kalendář z odkazu ICS (jen čtení, bez OAuth) pro kartu Kalendář na záložce Osobní.
//!
//! Google i Outlook umí „tajnou adresu ve formátu iCal“ — Ledge ji stáhne (`live.rs`), rozparsuje
//! (`ics.rs`), rozbalí opakování (`expand.rs`) v pásmu události (`tz.rs`) a nechá si jen výskyty
//! v okně −`PAST_DAYS` / +`AHEAD_DAYS` dní (`collect`). Karta pak ukáže Dnes / Zítra / Příští
//! (`view.rs`). Config a ochrana adresy (tajemství: nelogovat, v UI jen doména) v `conf.rs`.
//! Zdroje jako abstrakce (`source.rs`: odkaz ICS, klasický Outlook v PC `outlook.rs`, místo pro účty
//! Microsoft/Google), odkaz „Připojit“ (`meeting.rs`) a připomínka schůzky (`remind.rs`).

pub mod cache;
pub mod conf;
pub mod expand;
// adaptéry kalendářů účtů (Microsoft Graph, Google) — zapojené v `source::adapter` a `accounts::calsync`
pub mod google;
pub mod graph;
pub mod ics;
pub mod live;
pub mod meeting;
pub mod outlook;
pub mod remind;
pub mod source;
pub mod tz;
pub mod view;

use chrono::{DateTime, Duration, NaiveDateTime};
use ics::{Dt, Event};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tz::{LocalTz, Zones};

/// Okno rozbalených výskytů kolem času stažení (dny).
pub const PAST_DAYS: i64 = 1;
pub const AHEAD_DAYS: i64 = 14;
/// Nejvíc výskytů z jednoho feedu (paměť i velikost cache).
pub const MAX_OCCS: usize = 1500;
pub(crate) const MAX_TITLE: usize = 120;
pub(crate) const MAX_LOCATION: usize = 120;

/// Jeden výskyt v okně. Časy v ms epochy (UTC); celodenní = půlnoc data brané jako UTC (datum bez
/// pásma — `view.rs` ho čte zpět jako datum), `end` výlučně.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct Occ {
    pub start: i64,
    pub end: i64,
    #[serde(default)]
    pub all_day: bool,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub location: String,
    /// pásmo události Ledge nezná → čas je brán jako místní
    #[serde(default)]
    pub tz_guess: bool,
    /// odkaz „Připojit“ (Teams, Meet, Zoom, Webex, Jitsi — `meeting.rs`); stará cache bez pole = žádný
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub join: Option<meeting::Join>,
}

#[derive(Debug)]
pub struct Parsed {
    /// VEVENT ve feedu celkem
    pub events: usize,
    /// název kalendáře z feedu (`X-WR-CALNAME`), prázdný = feed ho nemá
    pub name: String,
    pub occs: Vec<Occ>,
}

pub(crate) fn ms(t: NaiveDateTime) -> i64 {
    t.and_utc().timestamp_millis()
}

pub(crate) fn naive(ms: i64) -> NaiveDateTime {
    DateTime::from_timestamp_millis(ms).map(|d| d.naive_utc()).unwrap_or_default()
}

/// Jeden řádek: nové řádky a vícenásobné mezery → mezera, nejvýš `max` znaků (pak „…“).
pub use crate::util::clip_line as clip;

/// Rozparsuje feed a vrátí výskyty v okně kolem `now_ms`, seřazené podle začátku.
pub fn collect(bytes: &[u8], now_ms: i64, local: &dyn LocalTz) -> Result<Parsed, String> {
    let now = naive(now_ms);
    let (ws, we) = (now - Duration::days(PAST_DAYS), now + Duration::days(AHEAD_DAYS));
    // hrubý filtr na časech bez pásma (pásma feedu ještě neznáme): rezerva 2 dny
    let (lo, hi) = (ws - Duration::days(2), we + Duration::days(2));
    let in_rough = |s: NaiveDateTime, e: NaiveDateTime| e >= lo && s <= hi;
    let feed = ics::parse(bytes, &mut |e| {
        let s = e.start.as_ref().map(Dt::naive).unwrap_or_default();
        let end = e.end.as_ref().map(Dt::naive).unwrap_or(s).max(s + Duration::seconds(e.duration.unwrap_or(0).clamp(0, 400 * 86_400)));
        match &e.recurrence_id {
            // přepis výskytu: zajímá, když je v okně původní termín nebo nový
            Some(rid) => in_rough(rid.naive(), rid.naive() + Duration::days(1)) || in_rough(s, end),
            None => e.rrule.is_some() || !e.rdates.is_empty() || in_rough(s, end),
        }
    })?;
    let zones = Zones::new(&feed.zones, local);
    let mut replaced: HashMap<&str, Vec<&Dt>> = HashMap::new();
    for e in &feed.events {
        if let Some(rid) = &e.recurrence_id {
            replaced.entry(e.uid.as_str()).or_default().push(rid);
        }
    }
    let mut out = Vec::new();
    for e in feed.events.iter().filter(|e| !e.cancelled) {
        let master = e.recurrence_id.is_none();
        let skip = if master { replaced.get(e.uid.as_str()).map(Vec::as_slice).unwrap_or_default() } else { &[] };
        expand_event(e, master, skip, &zones, (ws, we), &mut out);
    }
    out.sort_by(|a, b| (a.start, !a.all_day, &a.title).cmp(&(b.start, !b.all_day, &b.title)));
    out.dedup();
    out.truncate(MAX_OCCS);
    Ok(Parsed { events: feed.vevents, name: clip(&feed.name, 60), occs: out })
}

/// Výskyty jedné události v okně `win` (UTC). `recurring` = brát RRULE/RDATE (přepis výskytu ne);
/// `skip` = původní termíny přepsané jinou událostí (RECURRENCE-ID), vynechají se jako EXDATE.
fn expand_event(e: &Event, recurring: bool, skip: &[&Dt], zones: &Zones, (ws, we): (NaiveDateTime, NaiveDateTime), out: &mut Vec<Occ>) {
    let Some(start) = &e.start else { return };
    let rule = e.rrule.as_deref().filter(|_| recurring).and_then(|r| expand::parse_rule(r).ok());
    let title = clip(&e.summary, MAX_TITLE);
    let location = clip(e.location.lines().find(|l| !l.trim().is_empty()).unwrap_or(""), MAX_LOCATION);
    let join = e.join.as_ref().map(|(_, j)| j.clone());
    let mut keep = None;
    let (tz, local_start) = zones.resolve(start, &mut keep);
    let until = |u: &expand::Until| match u {
        expand::Until::Date(d) => d.and_hms_opt(23, 59, 59).unwrap_or_default(),
        expand::Until::Local(t) => *t,
        expand::Until::Utc(t) => zones.local_from_utc(tz, *t),
    };
    let excluded: Vec<&Dt> = e.exdates.iter().chain(skip.iter().copied()).collect();
    if start.is_date() {
        // celodenní: počítá se v datech, bez pásma
        let days = match (&e.end, e.duration) {
            (Some(end), _) => (end.naive().date() - local_start.date()).num_days(),
            (None, Some(s)) => s / 86_400,
            _ => 1,
        }
        .max(1);
        let dur = Duration::days(days);
        let mut starts = match &rule {
            Some(r) => expand::occurrences(r, local_start, r.until.as_ref().map(until), ws - dur, we),
            None => vec![local_start],
        };
        if recurring {
            starts.extend(e.rdates.iter().map(|d| d.naive().date().and_time(chrono::NaiveTime::MIN)));
        }
        let gone: Vec<chrono::NaiveDate> = excluded.iter().map(|d| d.naive().date()).collect();
        starts.sort_unstable();
        starts.dedup();
        for s in starts.into_iter().filter(|s| !gone.contains(&s.date())) {
            let en = s + dur;
            if en > ws && s < we {
                out.push(Occ { start: ms(s), end: ms(en), all_day: true, title: title.clone(), location: location.clone(), tz_guess: false, join: join.clone() });
            }
        }
        return;
    }
    let start_utc = zones.to_utc(tz, local_start);
    let dur_s = match (&e.end, e.duration) {
        (Some(end), _) if !end.is_date() => (zones.utc_of(end) - start_utc).num_seconds(),
        (_, Some(d)) => d,
        _ => 0,
    }
    .clamp(0, 400 * 86_400);
    let dur = Duration::seconds(dur_s);
    let mut starts: Vec<NaiveDateTime> = match &rule {
        Some(r) => {
            let from = zones.local_from_utc(tz, ws - dur) - Duration::hours(2);
            let to = zones.local_from_utc(tz, we) + Duration::hours(2);
            expand::occurrences(r, local_start, r.until.as_ref().map(until), from, to).into_iter().map(|t| zones.to_utc(tz, t)).collect()
        }
        None => vec![start_utc],
    };
    if recurring {
        starts.extend(e.rdates.iter().map(|d| zones.utc_of(d)));
    }
    starts.sort_unstable();
    starts.dedup();
    // EXDATE / RECURRENCE-ID: časem (UTC), datem jen u hodnoty VALUE=DATE (místní datum výskytu)
    let gone_at: Vec<NaiveDateTime> = excluded.iter().filter(|d| !d.is_date()).map(|d| zones.utc_of(d)).collect();
    let gone_day: Vec<chrono::NaiveDate> = excluded.iter().filter(|d| d.is_date()).map(|d| d.naive().date()).collect();
    for s in starts {
        if gone_at.contains(&s) || (!gone_day.is_empty() && gone_day.contains(&zones.local_from_utc(tz, s).date())) {
            continue;
        }
        let en = s + dur;
        if en.max(s + Duration::seconds(1)) > ws && s < we {
            out.push(Occ { start: ms(s), end: ms(en), all_day: false, title: title.clone(), location: location.clone(), tz_guess: tz.guess(), join: join.clone() });
        }
    }
}

#[cfg(test)]
mod tests;
