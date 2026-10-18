//! Zdroj kalendáře jako abstrakce. Každý zdroj vrací **stejný typ událostí** (`Event` = `calendar::Occ`:
//! UTC ms, celodenní, název, místo, odkaz Připojit) pro okno kolem teď (`Window`), takže stav, cache,
//! karta (`view.rs`) i připomínky (`remind.rs`) zdroj nerozlišují.
//!
//! | zdroj (`personal.calendars[].source`) | odkud | modul |
//! |---|---|---|
//! | `ics` (výchozí) | tajný odkaz ICS (Google, Outlook na webu) | `live.rs` `fetch_parse` |
//! | `outlook-local` | klasický Outlook v tomto PC přes COM, bez přihlášení | `outlook.rs` |
//! | `graph` / `google` | účet Microsoft / Google (OAuth, přihlášení v Nastavení → Účty) | `graph.rs` / `google.rs` přes `adapter` |
//!
//! Kalendář účtu jde do karty i bez řádku v `personal.calendars` — přepínač v Nastavení → Účty
//! (`accounts::calsync`, trait `Source`); má-li `personal.calendars` zapnutý zdroj téhož druhu, vyhraje
//! ten (připomínky, barva) a `calsync` ho vynechá.
//!
//! **Kontrakt pro další zdroje** (účty Microsoft/Google): funkce typu `Adapter` =
//! `fn(&CalendarFeed, Window) -> Result<Vec<Event>, String>` — běží na blokujícím vlákně (smí čekat na
//! síť s vlastním timeoutem), vrací výskyty v okně (opakování už rozbalená), chyba = lidská věta bez
//! tajemství. Id zdroje = `key` (otisk, ne token), název = `CalendarFeed.name` (prázdný → `Parsed.name`),
//! barva = `CalendarFeed.color`. Zapojení = jedna větev v `adapter`, nic jiného.

use super::conf::{self, CalendarFeed};
use super::{AHEAD_DAYS, MAX_OCCS, Occ, PAST_DAYS, Parsed};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Událost zdroje — sdílený tvar všech zdrojů (výskyt v okně).
pub type Event = Occ;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema, PartialEq, Eq, Default)]
#[serde(rename_all = "kebab-case")]
pub enum SourceKind {
    /// tajný odkaz ICS (`url`)
    #[default]
    Ics,
    /// klasický Outlook v tomto PC (COM, jen čtení; nový Outlook COM nemá)
    OutlookLocal,
    /// účet Microsoft (Graph) — `account`
    Graph,
    /// účet Google — `account`
    Google,
}

impl SourceKind {
    pub fn id(self) -> &'static str {
        match self {
            SourceKind::Ics => "ics",
            SourceKind::OutlookLocal => "outlook-local",
            SourceKind::Graph => "graph",
            SourceKind::Google => "google",
        }
    }
}

/// Okno výskytů (ms epochy UTC): −`PAST_DAYS` / +`AHEAD_DAYS` dní kolem `now`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Window {
    pub now: i64,
    pub from: i64,
    pub to: i64,
}

impl Window {
    pub fn around(now: i64) -> Self {
        Window { now, from: now - PAST_DAYS * 86_400_000, to: now + AHEAD_DAYS * 86_400_000 }
    }
}

/// Zdroj z jiného modulu (účty Microsoft/Google) — viz kontrakt v hlavičce.
pub type Adapter = fn(&CalendarFeed, Window) -> Result<Vec<Event>, String>;

/// Adaptér zdroje účtu (Microsoft Graph, Google Calendar).
pub fn adapter(kind: SourceKind) -> Option<Adapter> {
    match kind {
        SourceKind::Graph => Some(super::graph::adapter),
        SourceKind::Google => Some(super::google::adapter),
        SourceKind::Ics | SourceKind::OutlookLocal => None,
    }
}

/// Popis zdroje účtu pro kartu (patička, barva teček) — `accounts::calsync`.
#[derive(Debug, Clone, PartialEq)]
pub struct SourceInfo {
    /// stabilní id (`microsoft`, `google`)
    pub id: String,
    pub name: String,
    /// `#rrggbb`
    pub color: Option<String>,
}

/// Zdroj účtu pro `accounts::calsync` (kalendář zapnutý v Nastavení → Účty). `events` blokuje (síť) —
/// volat z vlákna mimo async runtime nebo ze `spawn_blocking`.
pub trait Source: Send + Sync {
    fn info(&self) -> SourceInfo;
    fn events(&self, window: Window) -> Result<Vec<Event>, String>;
}

/// Co stáhnout / načíst.
#[derive(Clone)]
pub enum Target {
    Ics(String),
    OutlookLocal,
    Adapter(Adapter, CalendarFeed),
}

/// Klíč stavu a cache (nikdy tajemství): ICS = otisk adresy, Outlook = `outlook-local`, účet = otisk
/// `druh:účet`. Chyba = kalendář nejde použít (lidská věta bez adresy).
pub fn key(f: &CalendarFeed) -> Result<String, String> {
    match f.source {
        SourceKind::Ics => conf::normalize_url(&f.url).map(|u| conf::key(&u)),
        SourceKind::OutlookLocal => Ok("outlook-local".into()),
        SourceKind::Graph | SourceKind::Google => Ok(conf::key(&format!("{}:{}", f.source.id(), f.account.trim()))),
    }
}

/// Cíl načtení kalendáře; `None` = nejde (špatná adresa, nezapojený účet).
pub fn target(f: &CalendarFeed) -> Option<Target> {
    match f.source {
        SourceKind::Ics => conf::normalize_url(&f.url).ok().map(Target::Ics),
        SourceKind::OutlookLocal => Some(Target::OutlookLocal),
        SourceKind::Graph | SourceKind::Google => adapter(f.source).map(|a| Target::Adapter(a, f.clone())),
    }
}

/// (klíč, cíl) zapnutých kalendářů, které jde načíst.
pub fn targets(list: &[CalendarFeed]) -> Vec<(String, Target)> {
    list.iter().filter(|f| f.enabled).filter_map(|f| Some((key(f).ok()?, target(f)?))).collect()
}

/// Co UI smí o zdroji ukázat: u ICS doména adresy, jinde nic (název zdroje si UI řekne podle `source`).
pub fn domain(f: &CalendarFeed) -> String {
    match f.source {
        SourceKind::Ics => conf::domain(&f.url),
        _ => String::new(),
    }
}

/// Název bez názvu z configu i ze zdroje: u ICS doména, jinak název služby.
pub fn fallback_name(f: &CalendarFeed) -> String {
    match f.source {
        SourceKind::Ics => conf::domain(&f.url),
        SourceKind::OutlookLocal => "Outlook".into(),
        SourceKind::Graph => "Microsoft".into(),
        SourceKind::Google => "Google".into(),
    }
}

/// Proč kalendář nejde načíst (nezapojený účet) — pro stav karty.
pub fn unavailable(f: &CalendarFeed) -> Option<String> {
    match f.source {
        SourceKind::Graph | SourceKind::Google if adapter(f.source).is_none() => {
            Some(crate::i18n::tr("Účty zatím tahle verze Ledge neumí — přidej kalendář odkazem ICS.", "This Ledge version can't use accounts yet — add the calendar with an ICS link.").into())
        }
        _ => None,
    }
}

/// Výskyty ze zdroje → `Parsed`: jen v okně, seřazené jako u ICS, nejvýš `MAX_OCCS`.
pub fn finish(mut evs: Vec<Event>, w: Window, name: &str) -> Parsed {
    let total = evs.len();
    evs.retain(|o| o.end.max(o.start + 1) > w.from && o.start < w.to);
    evs.sort_by(|a, b| (a.start, !a.all_day, &a.title).cmp(&(b.start, !b.all_day, &b.title)));
    evs.dedup();
    evs.truncate(MAX_OCCS);
    Parsed { events: total, name: super::clip(name, 60), occs: evs }
}

/// Načtení jednoho cíle (volá `live.rs`): ICS stáhne, Outlook a účty běží na blokujícím vlákně.
/// `allow_start` = uživatel výslovně klikl „Načíst z Outlooku“ (smí Outlook spustit na pozadí).
pub async fn fetch(t: Target, now: i64, allow_start: bool) -> Result<Parsed, String> {
    let w = Window::around(now);
    match t {
        Target::Ics(url) => super::live::fetch_parse(&url, now).await,
        Target::OutlookLocal => tauri::async_runtime::spawn_blocking(move || super::outlook::events(w, allow_start).map(|evs| finish(evs, w, "Outlook")))
            .await
            .map_err(|e| trf!("Outlook: {e}", "Outlook: {e}"))?,
        Target::Adapter(f, feed) => tauri::async_runtime::spawn_blocking(move || f(&feed, w).map(|evs| finish(evs, w, "")))
            .await
            .map_err(|e| trf!("kalendář: {e}", "calendar: {e}"))?,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keys_targets_and_domain_table() {
        let ics = CalendarFeed { url: "webcal://x.cz/a.ics".into(), ..CalendarFeed::default() };
        let bad = CalendarFeed { url: "http://x.cz/a.ics".into(), ..CalendarFeed::default() };
        let ol = CalendarFeed { source: SourceKind::OutlookLocal, url: "ignorováno".into(), ..CalendarFeed::default() };
        let g = CalendarFeed { source: SourceKind::Google, account: "jana@example.com".into(), ..CalendarFeed::default() };
        let off = CalendarFeed { enabled: false, ..ics.clone() };
        assert_eq!(key(&ics).unwrap(), conf::key("https://x.cz/a.ics"));
        assert!(key(&bad).is_err());
        assert_eq!(key(&ol).unwrap(), "outlook-local");
        let kg = key(&g).unwrap();
        assert_eq!(kg.len(), 16);
        assert!(!kg.contains("jana"), "klíč nenese účet");
        assert_ne!(kg, key(&CalendarFeed { source: SourceKind::Graph, ..g.clone() }).unwrap());
        assert_eq!((domain(&ics).as_str(), domain(&ol).as_str()), ("x.cz", ""));
        assert_eq!([fallback_name(&ics), fallback_name(&ol), fallback_name(&g)], ["x.cz", "Outlook", "Google"]);
        // účet má adaptér (graph.rs / google.rs) → cíl, žádná hláška „neumí“
        let t = targets(&[ics.clone(), bad, ol, g.clone(), off]);
        let kinds: Vec<&str> = t.iter().map(|(_, t)| match t {
            Target::Ics(_) => "ics",
            Target::OutlookLocal => "outlook",
            Target::Adapter(..) => "adapter",
        }).collect();
        assert_eq!(kinds, ["ics", "outlook", "adapter"]);
        assert!(unavailable(&g).is_none() && unavailable(&ics).is_none());
        assert!(adapter(SourceKind::Graph).is_some() && adapter(SourceKind::Ics).is_none());
    }

    #[test]
    fn finish_window_sort_and_cap() {
        let w = Window::around(1_000 * 86_400_000);
        let o = |start: i64, title: &str| Occ { start, end: start + 60_000, title: title.into(), ..Occ::default() };
        let evs = vec![o(w.now + 3_600_000, "B"), o(w.now, "A"), o(w.from - 120_000, "staré"), o(w.to + 1, "pozdě"), o(w.now, "A")];
        let p = finish(evs, w, "Outlook");
        assert_eq!(p.occs.iter().map(|o| o.title.as_str()).collect::<Vec<_>>(), ["A", "B"]);
        assert_eq!((p.events, p.name.as_str()), (5, "Outlook"));
        let many: Vec<Occ> = (0..MAX_OCCS as i64 + 10).map(|i| o(w.now + i * 60_000, "x")).collect();
        assert_eq!(finish(many, w, "").occs.len(), MAX_OCCS);
    }
}
