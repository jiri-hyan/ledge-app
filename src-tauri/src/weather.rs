//! Počasí pro záložku Osobní — Open-Meteo (`api.open-meteo.com/v1/forecast`, bez klíče).
//!
//! Žádný poll: stahuje se jen při otevření šuplíku na Osobní, když jsou data starší než 30 min
//! (nebo se změnilo místo). Síť volá jen Rust — CSP šuplíku pouští jen IPC. Místo je vždy z configu,
//! nikdy z polohy podle IP. Parsování a mapování kódů jsou čisté funkce (testy na uloženém vzorku).

use crate::i18n::tr;
use serde::Serialize;
use serde_json::Value;
use std::time::Duration;

pub const API: &str = "https://api.open-meteo.com/v1/forecast";
/// Data mladší než tohle se při otevření znovu nestahují.
pub const MAX_AGE_MS: i64 = 30 * 60 * 1000;
/// Po pokusu (i nepovedeném) aspoň tolik pauza — opakované otevírání šuplíku nebombarduje API.
pub const RETRY_MS: i64 = 2 * 60 * 1000;
pub const TIMEOUT: Duration = Duration::from_secs(5);
/// Chyba pokusu, který neodpověděl do 2× `TIMEOUT` (watchdog).
pub fn stuck() -> &'static str {
    tr("Open-Meteo neodpověděl včas", "Open-Meteo didn't answer in time")
}

/// Aktuální stav + dnešek z jedné odpovědi Open-Meteo.
#[derive(Serialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Weather {
    /// °C (`current.temperature_2m`)
    pub temp: f64,
    /// WMO kód (`current.weather_code`)
    pub code: i64,
    pub desc: &'static str,
    pub icon: &'static str,
    pub is_day: bool,
    /// dnešní minimum / maximum °C (`daily.temperature_2m_min/max`)
    pub min: Option<f64>,
    pub max: Option<f64>,
    /// dnešní srážky celkem v mm (`daily.precipitation_sum`)
    pub precip_sum: Option<f64>,
    /// dnešní nejvyšší pravděpodobnost srážek v % (`daily.precipitation_probability_max`)
    pub precip_prob: Option<f64>,
    /// čas měření v místním čase místa (`current.time`, např. "2026-10-16T07:00")
    pub observed: String,
}

/// URL dotazu — souřadnice na 4 desetinná místa (~11 m), časová zóna podle místa.
pub fn url(lat: f64, lon: f64) -> String {
    format!(
        "{API}?latitude={lat:.4}&longitude={lon:.4}\
         &current=temperature_2m,weather_code,precipitation,is_day\
         &daily=temperature_2m_max,temperature_2m_min,precipitation_sum,precipitation_probability_max\
         &timezone=auto&forecast_days=1"
    )
}

/// WMO kód → (popis v jazyce UI, ikona). Snapshot popis přepočítá, aby platil aktuální jazyk. Tabulka kódů podle dokumentace Open-Meteo.
pub fn wmo(code: i64, is_day: bool) -> (&'static str, &'static str) {
    match code {
        0 => (tr("Jasno", "Clear"), if is_day { "☀️" } else { "🌙" }),
        1 => (tr("Skoro jasno", "Mainly clear"), if is_day { "🌤️" } else { "🌙" }),
        2 => (tr("Polojasno", "Partly cloudy"), if is_day { "⛅" } else { "☁️" }),
        3 => (tr("Zataženo", "Overcast"), "☁️"),
        45 => (tr("Mlha", "Fog"), "🌫️"),
        48 => (tr("Mlha s námrazou", "Freezing fog"), "🌫️"),
        51 => (tr("Slabé mrholení", "Light drizzle"), "🌦️"),
        53 => (tr("Mrholení", "Drizzle"), "🌦️"),
        55 => (tr("Silné mrholení", "Heavy drizzle"), "🌧️"),
        56 => (tr("Slabé mrznoucí mrholení", "Light freezing drizzle"), "🌧️"),
        57 => (tr("Silné mrznoucí mrholení", "Heavy freezing drizzle"), "🌧️"),
        61 => (tr("Slabý déšť", "Light rain"), "🌧️"),
        63 => (tr("Déšť", "Rain"), "🌧️"),
        65 => (tr("Silný déšť", "Heavy rain"), "🌧️"),
        66 => (tr("Slabý mrznoucí déšť", "Light freezing rain"), "🌧️"),
        67 => (tr("Silný mrznoucí déšť", "Heavy freezing rain"), "🌧️"),
        71 => (tr("Slabé sněžení", "Light snow"), "🌨️"),
        73 => (tr("Sněžení", "Snow"), "🌨️"),
        75 => (tr("Silné sněžení", "Heavy snow"), "🌨️"),
        77 => (tr("Sněhová zrna", "Snow grains"), "🌨️"),
        80 => (tr("Slabé přeháňky", "Light showers"), "🌦️"),
        81 => (tr("Přeháňky", "Showers"), "🌧️"),
        82 => (tr("Prudké přeháňky", "Violent showers"), "🌧️"),
        85 => (tr("Slabé sněhové přeháňky", "Light snow showers"), "🌨️"),
        86 => (tr("Silné sněhové přeháňky", "Heavy snow showers"), "🌨️"),
        95 => (tr("Bouřka", "Thunderstorm"), "⛈️"),
        96 => (tr("Bouřka s kroupami", "Thunderstorm with hail"), "⛈️"),
        97 => (tr("Silná bouřka", "Severe thunderstorm"), "⛈️"),
        99 => (tr("Bouřka se silnými kroupami", "Thunderstorm with heavy hail"), "⛈️"),
        _ => (tr("Neznámý stav", "Unknown"), "❔"),
    }
}

/// Odpověď Open-Meteo → `Weather`. Chybová odpověď (`{"error":true,"reason":…}`) → Err s důvodem.
pub fn parse(text: &str) -> Result<Weather, String> {
    let v: Value = serde_json::from_str(text).map_err(|e| trf!("Open-Meteo: nečitelná odpověď ({e})", "Open-Meteo: unreadable answer ({e})"))?;
    if v.get("error").and_then(Value::as_bool) == Some(true) {
        let reason = v.get("reason").and_then(Value::as_str).unwrap_or(tr("bez důvodu", "no reason given"));
        return Err(format!("Open-Meteo: {reason}"));
    }
    let cur = v.get("current").ok_or_else(|| tr("Open-Meteo: chybí `current`", "Open-Meteo: `current` is missing"))?;
    let temp = cur.get("temperature_2m").and_then(Value::as_f64).ok_or_else(|| tr("Open-Meteo: chybí teplota", "Open-Meteo: temperature is missing"))?;
    let code = cur.get("weather_code").and_then(Value::as_i64).ok_or_else(|| tr("Open-Meteo: chybí kód počasí", "Open-Meteo: weather code is missing"))?;
    // bez údaje o dni/noci radši denní ikona než noční
    let is_day = cur.get("is_day").and_then(Value::as_i64).is_none_or(|d| d != 0);
    let today = |key: &str| v.get("daily").and_then(|d| d.get(key)).and_then(|a| a.get(0)).and_then(Value::as_f64);
    let (desc, icon) = wmo(code, is_day);
    Ok(Weather {
        temp,
        code,
        desc,
        icon,
        is_day,
        min: today("temperature_2m_min"),
        max: today("temperature_2m_max"),
        precip_sum: today("precipitation_sum"),
        precip_prob: today("precipitation_probability_max"),
        observed: cur.get("time").and_then(Value::as_str).unwrap_or("").to_string(),
    })
}

/// Běží pokus doopravdy? `loading` bez odpovědi déle než 2× timeout (zaseklý / spadlý task) se
/// bere jako neúspěch — jinak by karta navždy „načítala“ a další pokus by nikdy nepřišel.
pub fn loading_live(loading: bool, attempt_ms: Option<i64>, now: i64) -> bool {
    loading && attempt_ms.is_some_and(|a| now - a < 2 * TIMEOUT.as_millis() as i64)
}

/// Data starší než `MAX_AGE_MS` (nebo bez času stažení) jsou „stará“.
pub fn is_old(fetched_ms: Option<i64>, now: i64) -> bool {
    fetched_ms.is_none_or(|f| now - f >= MAX_AGE_MS)
}

/// Stáhnout? Ne během běžícího pokusu, ne dřív než `RETRY_MS` po posledním pokusu, jinak když
/// data chybí nebo jsou starší než `MAX_AGE_MS`.
pub fn needs_fetch(now: i64, fetched_ms: Option<i64>, attempt_ms: Option<i64>, loading: bool) -> bool {
    if loading || attempt_ms.is_some_and(|a| now - a < RETRY_MS) {
        return false;
    }
    fetched_ms.is_none_or(|f| now - f >= MAX_AGE_MS)
}

/// Stav karty: `unset` (místo nenastavené) · `ok` · `stale` (poslední dobrá data: poslední pokus
/// selhal, nebo jsou starší 30 min) · `loading` (první stahování) · `unavailable` (nic dobrého
/// nemáme) · `idle` (nic nestaženo).
pub fn status(place_set: bool, has_data: bool, error: bool, loading: bool, old: bool) -> &'static str {
    match (place_set, has_data, error || old, loading) {
        (false, ..) => "unset",
        (true, true, true, _) => "stale",
        (true, true, false, _) => "ok",
        (true, false, _, true) => "loading",
        (true, false, true, false) if error => "unavailable",
        (true, false, ..) => "idle",
    }
}

/// Jedno stažení (async, na runtime Tauri), timeout 5 s. Tělo se parsuje i u HTTP chyby — Open-Meteo
/// v něm posílá důvod.
pub async fn fetch(lat: f64, lon: f64) -> Result<Weather, String> {
    let client = reqwest::Client::builder().timeout(TIMEOUT).build().map_err(|e| trf!("HTTP klient: {e}", "HTTP client: {e}"))?;
    let net = |e: reqwest::Error| {
        if e.is_timeout() { tr("Open-Meteo neodpověděl do 5 s", "Open-Meteo didn't answer within 5 s").to_string() } else { trf!("Open-Meteo nedostupné: {e}", "Open-Meteo unavailable: {e}") }
    };
    let resp = client.get(url(lat, lon)).send().await.map_err(net)?;
    let status = resp.status();
    let text = resp.text().await.map_err(net)?;
    parse(&text).map_err(|e| if status.is_success() { e } else { format!("{e} (HTTP {})", status.as_u16()) })
}

#[cfg(test)]
mod tests {
    use super::*;

    const PRAHA: &str = include_str!("fixtures/open-meteo-praha.json");
    const ERROR: &str = include_str!("fixtures/open-meteo-error.json");

    #[test]
    fn parses_saved_sample() {
        let w = parse(PRAHA).unwrap();
        assert_eq!(w.temp, 6.0);
        assert_eq!(w.code, 45);
        assert_eq!((w.desc, w.icon), ("Mlha", "🌫️"));
        assert!(!w.is_day);
        assert_eq!(w.min, Some(5.0));
        assert_eq!(w.max, Some(16.8));
        assert_eq!(w.precip_sum, Some(0.0));
        assert_eq!(w.precip_prob, Some(0.0));
        assert_eq!(w.observed, "2026-10-16T07:00");
    }

    #[test]
    fn descriptions_and_errors_follow_language() {
        assert_eq!(wmo(45, true).0, "Mlha");
        crate::i18n::with_lang(crate::uiconf::Lang::En, || {
            assert_eq!(wmo(45, true), ("Fog", "🌫️"));
            assert_eq!(wmo(1234, true).0, "Unknown");
            assert_eq!(parse(PRAHA).unwrap().desc, "Fog");
            assert!(parse("<html>502</html>").unwrap_err().contains("unreadable"));
            assert!(parse(r#"{"current":{"weather_code":1}}"#).unwrap_err().contains("temperature"));
            assert_eq!(stuck(), "Open-Meteo didn't answer in time");
        });
        // každý kód z tabulky má anglický popis (ne český zbytek)
        for code in [0, 1, 2, 3, 45, 48, 51, 53, 55, 56, 57, 61, 63, 65, 66, 67, 71, 73, 75, 77, 80, 81, 82, 85, 86, 95, 96, 97, 99] {
            let (cs, en) = (wmo(code, true).0, crate::i18n::with_lang(crate::uiconf::Lang::En, || wmo(code, true).0));
            assert!(en.is_ascii() && en != cs, "{code}: {cs} / {en}");
        }
    }

    #[test]
    fn error_response_and_garbage() {
        let e = parse(ERROR).unwrap_err();
        assert!(e.contains("Latitude must be in range"), "{e}");
        assert!(parse("<html>502</html>").unwrap_err().contains("nečitelná"));
        assert!(parse(r#"{"current":{"weather_code":1}}"#).unwrap_err().contains("teplota"));
        assert!(parse(r#"{"daily":{}}"#).unwrap_err().contains("current"));
    }

    #[test]
    fn missing_daily_is_none_not_zero() {
        let w = parse(r#"{"current":{"temperature_2m":-3.5,"weather_code":71,"is_day":1,"time":"2026-01-02T08:15"}}"#).unwrap();
        assert_eq!(w.temp, -3.5);
        assert_eq!((w.min, w.max, w.precip_sum, w.precip_prob), (None, None, None, None));
        assert_eq!(w.desc, "Slabé sněžení");
        // bez is_day → denní ikona
        let w = parse(r#"{"current":{"temperature_2m":10,"weather_code":0}}"#).unwrap();
        assert!(w.is_day);
        assert_eq!(w.icon, "☀️");
    }

    #[test]
    fn wmo_table() {
        let cases: &[(i64, bool, &str, &str)] = &[
            (0, true, "Jasno", "☀️"),
            (0, false, "Jasno", "🌙"),
            (1, true, "Skoro jasno", "🌤️"),
            (2, true, "Polojasno", "⛅"),
            (2, false, "Polojasno", "☁️"),
            (3, true, "Zataženo", "☁️"),
            (45, true, "Mlha", "🌫️"),
            (48, true, "Mlha s námrazou", "🌫️"),
            (51, true, "Slabé mrholení", "🌦️"),
            (55, true, "Silné mrholení", "🌧️"),
            (57, true, "Silné mrznoucí mrholení", "🌧️"),
            (61, true, "Slabý déšť", "🌧️"),
            (63, true, "Déšť", "🌧️"),
            (65, true, "Silný déšť", "🌧️"),
            (67, true, "Silný mrznoucí déšť", "🌧️"),
            (71, true, "Slabé sněžení", "🌨️"),
            (75, true, "Silné sněžení", "🌨️"),
            (77, true, "Sněhová zrna", "🌨️"),
            (80, true, "Slabé přeháňky", "🌦️"),
            (82, true, "Prudké přeháňky", "🌧️"),
            (86, true, "Silné sněhové přeháňky", "🌨️"),
            (95, true, "Bouřka", "⛈️"),
            (96, true, "Bouřka s kroupami", "⛈️"),
            (99, false, "Bouřka se silnými kroupami", "⛈️"),
            (4, true, "Neznámý stav", "❔"),
            (-1, true, "Neznámý stav", "❔"),
        ];
        for &(code, day, desc, icon) in cases {
            assert_eq!(wmo(code, day), (desc, icon), "kód {code}, den {day}");
        }
    }

    #[test]
    fn url_has_documented_params() {
        let u = url(50.07752, 14.427236);
        assert!(u.starts_with("https://api.open-meteo.com/v1/forecast?latitude=50.0775&longitude=14.4272"));
        for p in [
            "current=temperature_2m,weather_code,precipitation,is_day",
            "daily=temperature_2m_max,temperature_2m_min,precipitation_sum,precipitation_probability_max",
            "timezone=auto",
            "forecast_days=1",
        ] {
            assert!(u.contains(p), "{p} v {u}");
        }
        assert!(!u.contains(' '));
    }

    #[test]
    fn fetch_policy() {
        let now = 10_000_000_000;
        // nic nestaženo → stáhnout
        assert!(needs_fetch(now, None, None, false));
        // čerstvá data → ne; po 30 min → ano
        assert!(!needs_fetch(now, Some(now - MAX_AGE_MS + 1), Some(now - MAX_AGE_MS + 1), false));
        assert!(needs_fetch(now, Some(now - MAX_AGE_MS), Some(now - MAX_AGE_MS), false));
        // běží pokus → ne
        assert!(!needs_fetch(now, None, None, true));
        // chyba před chvílí → počkat RETRY_MS, pak znovu
        assert!(!needs_fetch(now, None, Some(now - 1000), false));
        assert!(needs_fetch(now, None, Some(now - RETRY_MS), false));
        assert!(!needs_fetch(now, Some(now - 3_600_000), Some(now - 1000), false));
    }

    #[test]
    fn card_status() {
        assert_eq!(status(false, true, false, false, false), "unset");
        assert_eq!(status(true, true, false, true, false), "ok");
        assert_eq!(status(true, true, true, false, false), "stale");
        // ok, ale starší 30 min → stará data (i když se zrovna obnovuje)
        assert_eq!(status(true, true, false, false, true), "stale");
        assert_eq!(status(true, true, false, true, true), "stale");
        assert_eq!(status(true, false, false, true, true), "loading");
        assert_eq!(status(true, false, true, false, true), "unavailable");
        assert_eq!(status(true, false, false, false, true), "idle");
    }

    #[test]
    fn loading_watchdog_and_age() {
        let now = 10_000_000_000;
        assert!(loading_live(true, Some(now - 9_999), now));
        assert!(!loading_live(true, Some(now - 10_000), now), "2× timeout bez odpovědi = neúspěch");
        assert!(!loading_live(true, None, now));
        assert!(!loading_live(false, Some(now), now));
        assert!(!is_old(Some(now - MAX_AGE_MS + 1), now));
        assert!(is_old(Some(now - MAX_AGE_MS), now));
        assert!(is_old(None, now));
    }
}
