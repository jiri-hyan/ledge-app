//! Připomínka schůzky: `remindMin` minut před začátkem (výchozí 5, 0 = vypnuto, per kalendář) se
//! v proužku objeví „📅 Standup za 5 min“ (modře, ambientně — i s vypnutým proužkem, jako připomínka
//! poznámky), proužek blikne / pulsne (`attention::signal`, událost `Meeting` — priorita mezi „čeká na
//! tebe“ a „hotovo“) a volitelně (`remindToast`) přijde toast Windows s tlačítkem **Připojit**.
//!
//! Žádný polling: emit vlákno dostane jen termín (`next_deadline` → `next_alarm`: začátek okna
//! připomínky, začátek schůzky, konec „začíná“), mezitím se proužek přepočítá s minutovými hodinami.
//! Data jsou ta, co má kalendář v paměti (poslední stažení / načtení z Outlooku, po startu cache na disku
//! — `live::ensure_cache`); kvůli připomínce se nic nestahuje. Mrtvý Ledge (fullscreen hra, herní režim)
//! nic neukáže ani nepošle (`attention::react`, proužek je skrytý); toast navíc ne při „nerušit“ /
//! prezentaci / otevřeném šuplíku. Fokus nikdy.

use super::live;
use super::meeting::{self, Join};
use super::tz::{LocalTz, SystemLocal};
use super::{Occ, naive};
use crate::attention::Event;
use crate::config::Config;
use crate::i18n::tr;
use crate::state::{core, mark_dirty, now_ms};
use chrono::{Duration, Timelike};
use parking_lot::Mutex;
use std::collections::HashSet;

/// Po začátku ještě tolik „začíná“ (pak připomínka zmizí).
pub const GRACE_MS: i64 = 2 * 60_000;
/// Nejdelší název v proužku (znaky).
const STRIP_TITLE: usize = 28;

/// Schůzka v okně připomínky.
#[derive(Debug, Clone, PartialEq)]
pub struct Soon {
    /// index v `personal.calendars`
    pub cal: usize,
    pub start: i64,
    pub title: String,
    pub join: Option<Join>,
}

/// Výskyty s předstihem: (kalendář, výskyt, předstih ms). Celodenní se nepřipomínají.
type Cands<'a> = [(usize, &'a Occ, i64)];

/// Schůzky, jejichž okno připomínky [začátek − předstih, začátek + `GRACE_MS`) obsahuje `now`,
/// podle začátku; stejná schůzka ve dvou kalendářích jednou.
pub fn active(occs: &Cands, now: i64) -> Vec<Soon> {
    let mut v: Vec<Soon> = occs
        .iter()
        .filter(|(_, o, lead)| !o.all_day && *lead > 0 && o.start - lead <= now && now < o.start + GRACE_MS)
        .map(|&(cal, o, _)| Soon { cal, start: o.start, title: o.title.clone(), join: o.join.clone() })
        .collect();
    v.sort_by(|a, b| (a.start, &a.title, a.cal).cmp(&(b.start, &b.title, b.cal)));
    v.dedup_by(|b, a| a.start == b.start && a.title == b.title);
    v
}

/// Nejbližší budoucí okamžik, kdy se připomínky změní (začátek okna, začátek schůzky, konec „začíná“).
pub fn next_alarm(occs: &Cands, now: i64) -> Option<i64> {
    occs.iter()
        .filter(|(_, o, lead)| !o.all_day && *lead > 0)
        .flat_map(|(_, o, lead)| [o.start - lead, o.start, o.start + GRACE_MS])
        .filter(|&t| t > now)
        .min()
}

/// Celé minuty do začátku (zaokrouhleno nahoru); 0 = začíná / začala.
pub fn mins_left(start: i64, now: i64) -> i64 {
    if start <= now { 0 } else { (start - now + 59_999) / 60_000 }
}

fn short(title: &str) -> String {
    let t = if title.trim().is_empty() { tr("Schůzka", "Meeting").to_string() } else { title.to_string() };
    if t.chars().count() <= STRIP_TITLE {
        return t;
    }
    let mut s: String = t.chars().take(STRIP_TITLE - 1).collect();
    s.push('…');
    s
}

/// Text proužku: „📅 Standup za 4 min“ / „📅 Standup začíná“.
pub fn strip_text(s: &Soon, now: i64) -> String {
    let title = short(&s.title);
    match mins_left(s.start, now) {
        0 => trf!("📅 {title} začíná", "📅 {title} starting"),
        m => trf!("📅 {title} za {m} min", "📅 {title} in {m} min"),
    }
}

/// Místní „09:15“.
pub fn hm(ms: i64, local: &dyn LocalTz) -> String {
    let t = naive(ms);
    let l = t + Duration::seconds(local.offset_utc(t) as i64);
    format!("{:02}:{:02}", l.hour(), l.minute())
}

/// Toast: (titulek, tělo).
pub fn toast_text(s: &Soon, now: i64, local: &dyn LocalTz) -> (String, String) {
    let title = match mins_left(s.start, now) {
        0 => tr("Schůzka začíná", "Meeting is starting").to_string(),
        m => trf!("Schůzka za {m} min", "Meeting in {m} min"),
    };
    let name = if s.title.trim().is_empty() { tr("Schůzka", "Meeting").to_string() } else { s.title.clone() };
    (title, format!("{name} · {}", hm(s.start, local)))
}

/// Klíč ohlášené připomínky (kalendář, začátek, název).
type Key = (usize, i64, String);

/// Nově ohlásit: schůzka ještě nezačala a v `fired` není (vloží ji tam).
pub fn fresh(act: Vec<Soon>, fired: &mut HashSet<Key>, now: i64) -> Vec<Soon> {
    fired.retain(|k| k.1 > now - 3_600_000);
    act.into_iter().filter(|s| s.start > now && fired.insert((s.cal, s.start, s.title.clone()))).collect()
}

// ---------- napojení na jádro ----------

static FIRED: Mutex<Option<HashSet<Key>>> = Mutex::new(None);

/// Termín pro `next_wake` emit vlákna; bez kalendáře s připomínkou `None` (nic se nebudí).
pub(crate) fn next_deadline() -> Option<i64> {
    let cfg = core().cfg.read();
    let feeds = live::reminder_feeds(&cfg);
    if feeds.is_empty() {
        return None;
    }
    live::ensure_cache(&cfg);
    let now = now_ms();
    live::with_reminders(&feeds, |occs| next_alarm(occs, now))
}

/// Emit vlákno po probuzení: nové připomínky → signál proužku (+ toast, když je zapnutý).
pub(crate) fn tick(now: i64) {
    let c = core();
    let fresh: Vec<(Soon, bool)> = {
        let cfg = c.cfg.read();
        let feeds = live::reminder_feeds(&cfg);
        if feeds.is_empty() {
            return;
        }
        let act = live::with_reminders(&feeds, |occs| active(occs, now));
        let mut g = FIRED.lock();
        fresh(act, g.get_or_insert_with(HashSet::new), now).into_iter().map(|s| {
            let toast = cfg.personal.calendars.get(s.cal).is_some_and(|f| f.remind_toast);
            (s, toast)
        }).collect()
    };
    if fresh.is_empty() {
        return;
    }
    mark_dirty();
    if !crate::attention::signal(Event::Meeting.color(), fresh.iter().any(|(_, t)| *t)) {
        return;
    }
    let app_id = c.app.config().identifier.clone();
    for (s, _) in fresh.iter().filter(|(_, t)| *t) {
        let (title, body) = toast_text(s, now, &SystemLocal);
        let actions: Vec<(String, String)> = s.join.iter().map(|j| (tr("Připojit", "Join").to_string(), meeting::launch_uri(j))).collect();
        crate::toast::show_xml(&app_id, crate::toast::toast_xml_actions(&title, &body, &actions));
    }
}

/// Text proužku pro nejbližší schůzku v okně připomínky (jen paměť; volá `stripview` pod zámkem configu).
pub(crate) fn strip_now(cfg: &Config, now: i64) -> Option<String> {
    let feeds = live::reminder_feeds(cfg);
    if feeds.is_empty() {
        return None;
    }
    live::with_reminders(&feeds, |occs| active(occs, now).first().map(|s| strip_text(s, now)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::calendar::meeting::Provider;
    use crate::calendar::tz::builtin;
    use crate::i18n::with_lang;
    use crate::uiconf::Lang;

    const MIN: i64 = 60_000;
    const T: i64 = 1_792_310_400_000; // 2026-10-18 08:00 UTC = 10:00 v Praze

    fn occ(title: &str, start: i64) -> Occ {
        Occ { start, end: start + 30 * MIN, title: title.into(), ..Occ::default() }
    }

    #[test]
    fn active_window_table() {
        let a = occ("Standup", T + 5 * MIN);
        let b = occ("Oběd", T + 20 * MIN);
        let day = Occ { all_day: true, ..occ("Svátek", T + MIN) };
        let started = occ("Začala", T - MIN);
        let old = occ("Dávno", T - 10 * MIN);
        type Case<'a> = (&'a str, Vec<(usize, &'a Occ, i64)>, Vec<&'a str>);
        let cases: Vec<Case> = vec![
            ("přesně na začátku okna", vec![(0, &a, 5 * MIN)], vec!["Standup"]),
            ("ještě ne (předstih 4 min)", vec![(0, &a, 4 * MIN)], vec![]),
            ("vypnuto (0)", vec![(0, &a, 0)], vec![]),
            ("celodenní nikdy", vec![(0, &day, 5 * MIN)], vec![]),
            ("po začátku ještě 2 min „začíná“", vec![(0, &started, 5 * MIN)], vec!["Začala"]),
            ("po 10 min pryč", vec![(0, &old, 15 * MIN)], vec![]),
            ("seřazeno podle začátku, různé předstihy", vec![(1, &b, 30 * MIN), (0, &a, 5 * MIN)], vec!["Standup", "Oběd"]),
            ("stejná schůzka ve dvou kalendářích jednou", vec![(0, &a, 5 * MIN), (1, &a, 10 * MIN)], vec!["Standup"]),
        ];
        for (name, occs, want) in cases {
            let got: Vec<String> = active(&occs, T).into_iter().map(|s| s.title).collect();
            assert_eq!(got, want, "{name}");
        }
    }

    #[test]
    fn next_alarm_table() {
        let a = occ("A", T + 30 * MIN);
        let b = occ("B", T + 3 * MIN);
        type Case<'a> = (&'a str, Vec<(usize, &'a Occ, i64)>, Option<i64>);
        let cases: Vec<Case> = vec![
            ("nic", vec![], None),
            ("začátek okna připomínky", vec![(0, &a, 5 * MIN)], Some(T + 25 * MIN)),
            ("v okně: začátek schůzky", vec![(0, &b, 5 * MIN)], Some(T + 3 * MIN)),
            ("vypnutá připomínka nebudí", vec![(0, &a, 0)], None),
            ("nejbližší z více", vec![(0, &a, 5 * MIN), (1, &b, 5 * MIN)], Some(T + 3 * MIN)),
        ];
        for (name, occs, want) in cases {
            assert_eq!(next_alarm(&occs, T), want, "{name}");
        }
        // po začátku: konec „začíná“
        assert_eq!(next_alarm(&[(0, &b, 5 * MIN)], T + 3 * MIN), Some(T + 3 * MIN + GRACE_MS));
        assert_eq!(next_alarm(&[(0, &b, 5 * MIN)], T + 3 * MIN + GRACE_MS), None);
    }

    #[test]
    fn texts_both_languages() {
        let tz = builtin("Europe/Prague").unwrap();
        let s = Soon { cal: 0, start: T + 5 * MIN, title: "Standup".into(), join: None };
        let cases = [(T, "📅 Standup za 5 min", "📅 Standup in 5 min"), (T + 1, "📅 Standup za 5 min", "📅 Standup in 5 min"), (T + 4 * MIN + 1, "📅 Standup za 1 min", "📅 Standup in 1 min"), (T + 5 * MIN, "📅 Standup začíná", "📅 Standup starting")];
        for (now, cs, en) in cases {
            assert_eq!(strip_text(&s, now), cs);
            assert_eq!(with_lang(Lang::En, || strip_text(&s, now)), en);
        }
        let long = Soon { title: "Velmi dlouhý název porady celého oddělení".into(), ..s.clone() };
        assert_eq!(strip_text(&long, T), "📅 Velmi dlouhý název porady c… za 5 min");
        assert_eq!(strip_text(&Soon { title: " ".into(), ..s.clone() }, T), "📅 Schůzka za 5 min");
        assert_eq!(toast_text(&s, T, &tz), ("Schůzka za 5 min".into(), "Standup · 10:05".into()));
        assert_eq!(with_lang(Lang::En, || toast_text(&s, T + 5 * MIN, &tz)).0, "Meeting is starting");
    }

    #[test]
    fn fresh_fires_once_and_not_after_start() {
        let mut fired = HashSet::new();
        let j = Some(Join { provider: Provider::Meet, url: "https://meet.google.com/abc-defg-hij".into() });
        let s = |start: i64| Soon { cal: 0, start, title: "A".into(), join: j.clone() };
        assert_eq!(fresh(vec![s(T + MIN)], &mut fired, T).len(), 1);
        assert!(fresh(vec![s(T + MIN)], &mut fired, T + 1000).is_empty(), "podruhé ne");
        assert!(fresh(vec![s(T - 1)], &mut fired, T).is_empty(), "po začátku se už neohlašuje");
        assert_eq!(fresh(vec![s(T + 2 * MIN)], &mut fired, T).len(), 1, "jiná schůzka ano");
        // staré klíče se po hodině zahodí
        fresh(vec![], &mut fired, T + 2 * 3_600_000);
        assert!(fired.is_empty());
        assert_eq!(mins_left(T, T), 0);
        assert_eq!(mins_left(T + 61_000, T), 2);
    }
}
