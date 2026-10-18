//! Adaptér kalendáře účtu Microsoft (Outlook, Microsoft 365) přes Microsoft Graph: `/me/calendarView`
//! v okně karty (opakování rozbalí Graph sám), časy v UTC (`Prefer: outlook.timezone="UTC"`), odkaz
//! na schůzku z `onlineMeeting.joinUrl` (ověřený jako u ICS — `meeting::classify`). Přihlášení a token:
//! `accounts::session`. Zapojení: `source::adapter` (`personal.calendars[].source = "graph"`) a
//! `accounts::calsync` (kalendář zapnutý v Nastavení → Účty).

use super::conf::CalendarFeed;
use super::meeting::{self, Join};
use super::source::{Event, Source, SourceInfo, Window};
use super::tz::{LocalTz, SystemLocal};
use super::{MAX_OCCS, clip, ms};
use crate::accounts::{GRAPH, Provider, session};
use chrono::{DateTime, Duration, NaiveDateTime, Timelike};
use serde::Deserialize;

/// Nejvíc stránek odpovědi (po 100 událostech).
const MAX_PAGES: usize = 5;
pub const DEFAULT_COLOR: &str = "#3b82f6";

#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct GDate {
    #[serde(default)]
    date_time: String,
    #[serde(default)]
    time_zone: String,
}

#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct GLocation {
    #[serde(default)]
    display_name: Option<String>,
}

#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct GOnline {
    #[serde(default)]
    join_url: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct GEvent {
    #[serde(default)]
    subject: Option<String>,
    #[serde(default)]
    is_all_day: bool,
    #[serde(default)]
    is_cancelled: bool,
    #[serde(default)]
    start: GDate,
    #[serde(default)]
    end: GDate,
    #[serde(default)]
    location: Option<GLocation>,
    #[serde(default)]
    online_meeting: Option<GOnline>,
    #[serde(default)]
    online_meeting_url: Option<String>,
}

#[derive(Deserialize)]
struct Page {
    #[serde(default)]
    value: Vec<GEvent>,
    #[serde(rename = "@odata.nextLink", default)]
    next: Option<String>,
}

fn iso(ms: i64) -> String {
    DateTime::from_timestamp_millis(ms).unwrap_or_default().format("%Y-%m-%dT%H:%M:%SZ").to_string()
}

/// První stránka `/me/calendarView` pro okno.
pub fn view_url(w: Window) -> String {
    format!(
        "{GRAPH}/me/calendarView?startDateTime={}&endDateTime={}&$top=100&$orderby=start/dateTime&$select=subject,start,end,isAllDay,isCancelled,location,onlineMeeting,onlineMeetingUrl",
        iso(w.from),
        iso(w.to)
    )
}

/// Jen `https://` odkaz (do prohlížeče se nic jiného neposílá).
pub fn https_only(u: Option<String>) -> Option<String> {
    u.map(|s| s.trim().to_string()).filter(|s| s.len() <= 2048 && s.to_ascii_lowercase().starts_with("https://") && !s.chars().any(char::is_whitespace))
}

/// Odkaz Připojit z účtu: jen `https://` a jen ověřený tvar schůzky (Teams/Meet/Zoom/Webex/Jitsi, stejně
/// jako u ICS — `meeting::classify`); bez odkazu zkusí místo (`meeting::find`).
pub fn join_of(url: Option<String>, location: &str) -> Option<Join> {
    https_only(url).and_then(|u| meeting::classify(&u)).or_else(|| meeting::find(location))
}

fn parse_dt(s: &str) -> Option<NaiveDateTime> {
    NaiveDateTime::parse_from_str(s.trim(), "%Y-%m-%dT%H:%M:%S%.f").ok()
}

/// Čas Graphu → ms UTC. Pásmo `UTC` (vyžádané) = přímo; jiné = bráno jako místní (`tz_guess`).
fn to_utc(d: &GDate, local: &dyn LocalTz) -> Option<(i64, bool)> {
    let t = parse_dt(&d.date_time)?;
    if d.time_zone.is_empty() || d.time_zone.eq_ignore_ascii_case("UTC") || d.time_zone.eq_ignore_ascii_case("Etc/UTC") {
        Some((ms(t), false))
    } else {
        Some((ms(t - Duration::seconds(local.offset_local(t) as i64)), true))
    }
}

/// Datum celodenní události: půlnoc = datum samo (Graph ho nepřevedl); jinak UTC převedené na místní
/// datum (Graph převedl půlnoc pásma události do UTC, např. 22:00 předchozího dne).
fn all_day_date(d: &GDate, local: &dyn LocalTz) -> Option<NaiveDateTime> {
    let t = parse_dt(&d.date_time)?;
    let day = if t.hour() == 0 && t.minute() == 0 { t.date() } else { (t + Duration::seconds(local.offset_utc(t) as i64)).date() };
    Some(day.and_time(chrono::NaiveTime::MIN))
}

/// Jedna stránka odpovědi → události a odkaz na další stránku (jen na Graph — token jinam nejde).
pub fn parse(body: &[u8], local: &dyn LocalTz) -> Result<(Vec<Event>, Option<String>), String> {
    let p: Page = serde_json::from_slice(body).map_err(|_| crate::i18n::tr("Kalendář Microsoft vrátil nečitelná data.", "The Microsoft calendar returned unreadable data.").to_string())?;
    let mut out = Vec::new();
    for e in p.value.into_iter().filter(|e| !e.is_cancelled) {
        let title = clip(e.subject.as_deref().unwrap_or(""), 120);
        let location = clip(e.location.and_then(|l| l.display_name).as_deref().unwrap_or(""), 120);
        let url = https_only(e.online_meeting.and_then(|o| o.join_url)).or_else(|| https_only(e.online_meeting_url));
        let join = join_of(url, &location);
        let ev = if e.is_all_day {
            let (Some(s), Some(en)) = (all_day_date(&e.start, local), all_day_date(&e.end, local)) else { continue };
            let en = en.max(s + Duration::days(1));
            Event { start: ms(s), end: ms(en), all_day: true, title, location, join, tz_guess: false }
        } else {
            let (Some((s, g1)), Some((en, g2))) = (to_utc(&e.start, local), to_utc(&e.end, local)) else { continue };
            Event { start: s, end: en.max(s), all_day: false, title, location, join, tz_guess: g1 || g2 }
        };
        out.push(ev);
    }
    let next = p.next.filter(|n| n.starts_with(&format!("{GRAPH}/")));
    Ok((out, next))
}

/// Události kalendáře účtu Microsoft v okně (síť; chyba = lidská věta bez adresy a tokenu).
pub async fn fetch(w: Window) -> Result<Vec<Event>, String> {
    let mut url = view_url(w);
    let mut all = Vec::new();
    for _ in 0..MAX_PAGES {
        let r = session::api_get(Provider::Microsoft, &url, &[("Prefer", "outlook.timezone=\"UTC\"")]).await?;
        let (mut evs, next) = parse(&r.body, &SystemLocal)?;
        all.append(&mut evs);
        match next {
            Some(n) if all.len() < MAX_OCCS => url = n,
            _ => break,
        }
    }
    all.truncate(MAX_OCCS);
    Ok(all)
}

/// Adaptér pro `source::adapter` (`source = "graph"`): účet je jeden (přihlášený v Nastavení → Účty).
pub fn adapter(_feed: &CalendarFeed, w: Window) -> Result<Vec<Event>, String> {
    crate::accounts::calsync::block_on(fetch(w))
}

/// Zdroj „kalendář účtu Microsoft“ pro kontrakt `source::Source`.
pub struct GraphSource {
    pub name: String,
    pub color: Option<String>,
}

impl Source for GraphSource {
    fn info(&self) -> SourceInfo {
        SourceInfo { id: "microsoft".into(), name: self.name.clone(), color: self.color.clone().or_else(|| Some(DEFAULT_COLOR.into())) }
    }
    fn events(&self, window: Window) -> Result<Vec<Event>, String> {
        crate::accounts::calsync::block_on(fetch(window))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::calendar::tz::builtin;

    fn utc(d: u32, h: u32, mi: u32) -> i64 {
        ms(chrono::NaiveDate::from_ymd_opt(2026, 10, d).unwrap().and_hms_opt(h, mi, 0).unwrap())
    }

    #[test]
    fn parses_saved_sample() {
        let tz = builtin("Europe/Prague").unwrap();
        let body = include_bytes!("../fixtures/graph-calendarview.json");
        let (evs, next) = parse(body, &tz).unwrap();
        let t: Vec<(&str, bool, Option<&str>)> = evs.iter().map(|e| (e.title.as_str(), e.all_day, e.join.as_ref().map(|j| j.url.as_str()))).collect();
        assert_eq!(
            t,
            vec![
                ("Standup týmu", false, Some("https://teams.microsoft.com/l/meetup-join/19%3ameeting_abc%40thread.v2/0")),
                ("Dovolená", true, None),
                ("Svátek (převedený)", true, None),
                ("Oběd", false, None),
                ("Starý odkaz", false, Some("https://contoso.zoom.us/j/123")),
            ]
        );
        // časová v UTC beze změny, s místem
        assert_eq!((evs[0].start, evs[0].end, evs[0].location.as_str(), evs[0].tz_guess), (utc(19, 7, 0), utc(19, 7, 15), "Microsoft Teams Meeting", false));
        // celodenní: datum samo, konec výlučně; převedená půlnoc (22:00 UTC předchozího dne v Praze) = správné datum
        assert_eq!((evs[1].start, evs[1].end), (utc(20, 0, 0), utc(22, 0, 0)));
        assert_eq!((evs[2].start, evs[2].end), (utc(28, 0, 0), utc(29, 0, 0)));
        // jiné pásmo než UTC = místní čas s příznakem
        assert_eq!((evs[3].start, evs[3].tz_guess), (utc(19, 10, 0), true));
        // http odkaz se zahodí, http(s) z onlineMeetingUrl jako záloha
        assert_eq!(next.as_deref(), Some("https://graph.microsoft.com/v1.0/me/calendarView?$skiptoken=abc"));
    }

    #[test]
    fn next_link_only_to_graph_and_bad_data() {
        let b = br#"{"value":[],"@odata.nextLink":"https://evil.example.com/steal"}"#;
        assert_eq!(parse(b, &SystemLocal).unwrap().1, None);
        assert!(parse(b"<html>", &SystemLocal).is_err());
        // zrušená a nečitelná událost se přeskočí
        let b = r#"{"value":[{"subject":"Zrušeno","isCancelled":true,"start":{"dateTime":"2026-10-19T07:00:00","timeZone":"UTC"},"end":{"dateTime":"2026-10-19T08:00:00","timeZone":"UTC"}},{"subject":"Bez času"}]}"#.as_bytes();
        assert!(parse(b, &SystemLocal).unwrap().0.is_empty());
        assert_eq!(https_only(Some("http://x.cz".into())), None);
        assert_eq!(https_only(Some(" https://x.cz/a ".into())).as_deref(), Some("https://x.cz/a"));
    }

    #[test]
    fn url_has_window_and_minimal_fields() {
        let u = view_url(Window { now: utc(17, 10, 0), from: utc(17, 10, 0), to: utc(31, 10, 0) });
        assert!(u.starts_with("https://graph.microsoft.com/v1.0/me/calendarView?startDateTime=2026-10-17T10:00:00Z&endDateTime=2026-10-31T10:00:00Z"), "{u}");
        assert!(u.contains("$select=") && !u.contains("body"), "tělo události Ledge nečte");
    }
}
