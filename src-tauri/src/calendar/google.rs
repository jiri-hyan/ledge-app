//! Adaptér kalendáře účtu Google (Calendar API v3, scope `calendar.readonly`): události vybraných
//! kalendářů (`accounts.google.calendars`, prázdné = hlavní) v okně karty se `singleEvents=true`
//! (opakování rozbalí Google), odkaz na schůzku z `hangoutLink` / `conferenceData`. Seznam kalendářů
//! pro výběr v Nastavení: `calendar_list`. Přihlášení a token: `accounts::session` (worker Q).

use super::conf::CalendarFeed;
use super::graph::{https_only, join_of};
use super::source::{Event, Source, SourceInfo, Window};
use super::{MAX_OCCS, clip, ms};
use crate::accounts::{GCAL, Provider, oauth::enc, session};
use chrono::{DateTime, NaiveDate};
use serde::{Deserialize, Serialize};

const MAX_PAGES: usize = 4;
pub const DEFAULT_COLOR: &str = "#22c55e";

#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct GTime {
    #[serde(default)]
    date: Option<String>,
    #[serde(default)]
    date_time: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Entry {
    #[serde(default)]
    entry_point_type: String,
    #[serde(default)]
    uri: Option<String>,
}

#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct Conf {
    #[serde(default)]
    entry_points: Vec<Entry>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct GEvent {
    #[serde(default)]
    status: String,
    #[serde(default)]
    summary: Option<String>,
    #[serde(default)]
    location: Option<String>,
    #[serde(default)]
    start: GTime,
    #[serde(default)]
    end: GTime,
    #[serde(default)]
    hangout_link: Option<String>,
    #[serde(default)]
    conference_data: Option<Conf>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Page {
    #[serde(default)]
    items: Vec<GEvent>,
    #[serde(default)]
    next_page_token: Option<String>,
}

fn rfc3339(ms: i64) -> String {
    DateTime::from_timestamp_millis(ms).unwrap_or_default().format("%Y-%m-%dT%H:%M:%SZ").to_string()
}

/// Stránka událostí jednoho kalendáře.
pub fn events_url(cal: &str, w: Window, page: Option<&str>) -> String {
    let mut u = format!(
        "{GCAL}/calendars/{}/events?timeMin={}&timeMax={}&singleEvents=true&orderBy=startTime&maxResults=250&fields={}",
        enc(cal),
        enc(&rfc3339(w.from)),
        enc(&rfc3339(w.to)),
        enc("items(status,summary,location,start,end,hangoutLink,conferenceData/entryPoints),nextPageToken")
    );
    if let Some(p) = page {
        u.push_str(&format!("&pageToken={}", enc(p)));
    }
    u
}

fn date_ms(s: &str) -> Option<i64> {
    NaiveDate::parse_from_str(s.trim(), "%Y-%m-%d").ok().map(|d| ms(d.and_time(chrono::NaiveTime::MIN)))
}

/// Jedna stránka → události a token další stránky.
pub fn parse(body: &[u8]) -> Result<(Vec<Event>, Option<String>), String> {
    let p: Page = serde_json::from_slice(body).map_err(|_| crate::i18n::tr("Kalendář Google vrátil nečitelná data.", "The Google calendar returned unreadable data.").to_string())?;
    let mut out = Vec::new();
    for e in p.items.into_iter().filter(|e| e.status != "cancelled") {
        let title = clip(e.summary.as_deref().unwrap_or(""), 120);
        let location = clip(e.location.as_deref().unwrap_or("").lines().find(|l| !l.trim().is_empty()).unwrap_or(""), 120);
        let video = e.conference_data.unwrap_or_default().entry_points.into_iter().find(|x| x.entry_point_type == "video").and_then(|x| x.uri);
        let join = join_of(https_only(e.hangout_link).or_else(|| https_only(video)), &location);
        let ev = match (&e.start, &e.end) {
            (GTime { date: Some(s), .. }, GTime { date: Some(en), .. }) => {
                let (Some(s), Some(en)) = (date_ms(s), date_ms(en)) else { continue };
                Event { start: s, end: en.max(s + 86_400_000), all_day: true, title, location, join, tz_guess: false }
            }
            (GTime { date_time: Some(s), .. }, GTime { date_time: Some(en), .. }) => {
                let (Ok(s), Ok(en)) = (DateTime::parse_from_rfc3339(s.trim()), DateTime::parse_from_rfc3339(en.trim())) else { continue };
                let (s, en) = (s.timestamp_millis(), en.timestamp_millis());
                Event { start: s, end: en.max(s), all_day: false, title, location, join, tz_guess: false }
            }
            _ => continue,
        };
        out.push(ev);
    }
    Ok((out, p.next_page_token.filter(|t| !t.is_empty())))
}

/// Události vybraných kalendářů (prázdný výběr = hlavní) v okně, seřazené podle začátku.
pub async fn fetch(calendars: &[String], w: Window) -> Result<Vec<Event>, String> {
    let primary = ["primary".to_string()];
    let list: &[String] = if calendars.is_empty() { &primary } else { calendars };
    let mut all = Vec::new();
    for cal in list {
        let mut page: Option<String> = None;
        for _ in 0..MAX_PAGES {
            let r = session::api_get(Provider::Google, &events_url(cal, w, page.as_deref()), &[]).await?;
            let (mut evs, next) = parse(&r.body)?;
            all.append(&mut evs);
            match next {
                Some(n) if all.len() < MAX_OCCS => page = Some(n),
                _ => break,
            }
        }
    }
    all.sort_by(|a, b| (a.start, !a.all_day, &a.title).cmp(&(b.start, !b.all_day, &b.title)));
    all.dedup();
    all.truncate(MAX_OCCS);
    Ok(all)
}

/// Adaptér pro `source::adapter` (`source = "google"`): vybrané kalendáře z `accounts.google.calendars`.
pub fn adapter(_feed: &CalendarFeed, w: Window) -> Result<Vec<Event>, String> {
    let calendars = crate::state::core().cfg.read().accounts.google.calendars.clone();
    crate::accounts::calsync::block_on(fetch(&calendars, w))
}

/// Kalendář účtu pro výběr v Nastavení.
#[derive(Serialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CalendarEntry {
    pub id: String,
    pub name: String,
    pub color: Option<String>,
    pub primary: bool,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ListItem {
    id: String,
    #[serde(default)]
    summary: String,
    #[serde(default)]
    summary_override: Option<String>,
    #[serde(default)]
    background_color: Option<String>,
    #[serde(default)]
    primary: bool,
}

#[derive(Deserialize)]
struct List {
    #[serde(default)]
    items: Vec<ListItem>,
}

pub fn parse_list(body: &[u8]) -> Result<Vec<CalendarEntry>, String> {
    let l: List = serde_json::from_slice(body).map_err(|_| crate::i18n::tr("Seznam kalendářů Google je nečitelný.", "The Google calendar list is unreadable.").to_string())?;
    let mut v: Vec<CalendarEntry> = l
        .items
        .into_iter()
        .filter(|i| !i.id.is_empty())
        .map(|i| CalendarEntry {
            name: clip(i.summary_override.filter(|s| !s.is_empty()).as_deref().unwrap_or(&i.summary), 60),
            color: i.background_color.filter(|c| c.len() == 7 && c.starts_with('#')),
            primary: i.primary,
            id: i.id,
        })
        .collect();
    v.sort_by_key(|c| (!c.primary, c.name.to_lowercase()));
    Ok(v)
}

pub async fn calendar_list() -> Result<Vec<CalendarEntry>, String> {
    let url = format!("{GCAL}/users/me/calendarList?maxResults=100&fields={}", enc("items(id,summary,summaryOverride,backgroundColor,primary)"));
    let r = session::api_get(Provider::Google, &url, &[]).await?;
    parse_list(&r.body)
}

/// Zdroj „kalendář účtu Google“ pro kontrakt `source::Source`.
pub struct GoogleSource {
    pub name: String,
    pub color: Option<String>,
    pub calendars: Vec<String>,
}

impl Source for GoogleSource {
    fn info(&self) -> SourceInfo {
        SourceInfo { id: "google".into(), name: self.name.clone(), color: self.color.clone().or_else(|| Some(DEFAULT_COLOR.into())) }
    }
    fn events(&self, window: Window) -> Result<Vec<Event>, String> {
        crate::accounts::calsync::block_on(fetch(&self.calendars, window))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn utc(d: u32, h: u32, mi: u32) -> i64 {
        ms(NaiveDate::from_ymd_opt(2026, 10, d).unwrap().and_hms_opt(h, mi, 0).unwrap())
    }

    #[test]
    fn parses_saved_sample() {
        let (evs, next) = parse(include_bytes!("../fixtures/google-events.json")).unwrap();
        let t: Vec<(&str, bool, Option<&str>)> = evs.iter().map(|e| (e.title.as_str(), e.all_day, e.join.as_ref().map(|j| j.url.as_str()))).collect();
        assert_eq!(
            t,
            vec![
                ("Plánování", false, Some("https://meet.google.com/abc-defg-hij")),
                ("Narozeniny", true, None),
                ("Zoom hovor", false, Some("https://us02web.zoom.us/j/999")),
                ("Večerní běh", false, None),
            ]
        );
        // +02:00 → UTC; celodenní výlučný konec
        assert_eq!((evs[0].start, evs[0].end, evs[0].location.as_str()), (utc(19, 8, 0), utc(19, 9, 0), "Zasedačka 2"));
        assert_eq!((evs[1].start, evs[1].end), (utc(23, 0, 0), utc(24, 0, 0)));
        assert_eq!(evs[3].start, utc(22, 15, 15) + 30_000);
        assert_eq!(next.as_deref(), Some("CkkKOwoN"));
        assert!(parse(b"nesmysl").is_err());
        assert_eq!(parse(br#"{"items":[],"nextPageToken":""}"#).unwrap().1, None);
    }

    #[test]
    fn calendar_list_primary_first() {
        let b = r##"{"items":[{"id":"x@group.calendar.google.com","summary":"Rodina","backgroundColor":"#9fe1e7"},{"id":"jana@example.com","summary":"jana@example.com","summaryOverride":"Já","backgroundColor":"#4986e7","primary":true},{"id":"","summary":"bez id"},{"id":"cz.czech#holiday@group.v.calendar.google.com","summary":"Státní svátky","backgroundColor":"nesmysl"}]}"##.as_bytes();
        let l = parse_list(b).unwrap();
        let names: Vec<(&str, bool, Option<&str>)> = l.iter().map(|c| (c.name.as_str(), c.primary, c.color.as_deref())).collect();
        assert_eq!(names, vec![("Já", true, Some("#4986e7")), ("Rodina", false, Some("#9fe1e7")), ("Státní svátky", false, None)]);
    }

    #[test]
    fn url_encodes_calendar_and_window() {
        let u = events_url("cz.czech#holiday@group.v.calendar.google.com", Window { now: utc(17, 10, 0), from: utc(17, 10, 0), to: utc(31, 10, 0) }, Some("tok/1"));
        assert!(u.starts_with("https://www.googleapis.com/calendar/v3/calendars/cz.czech%23holiday%40group.v.calendar.google.com/events?timeMin=2026-10-17T10%3A00%3A00Z&timeMax=2026-10-31T10%3A00%3A00Z&singleEvents=true&orderBy=startTime"), "{u}");
        assert!(u.ends_with("&pageToken=tok%2F1"));
        assert!(!u.contains("description"), "popis události Ledge nečte");
    }
}
