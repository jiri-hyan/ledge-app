//! Předpověď z téhož dotazu Open-Meteo jako aktuální počasí (`weather::url`): hodiny dnešek + zítřek,
//! 5 dní dopředu, vítr, pocitová teplota, východ / západ slunce. Parsování je obranné — chybějící
//! nebo nečíselné pole = `None`, hodina / den bez jediného údaje se vynechá. Žádná síť, čistá funkce.

use serde::Serialize;
use serde_json::Value;

/// Kolik hodin dopředu od půlnoci dneška se posílá (dnešek + zítřek).
pub const HOURS: usize = 48;
/// Kolik dalších dnů po dnešku.
pub const DAYS_AHEAD: usize = 5;

/// Jedna hodina předpovědi; `time` je místní čas místa („2026-10-16T07:00“).
#[derive(Serialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Hour {
    pub time: String,
    pub temp: Option<f64>,
    /// pravděpodobnost srážek v %
    pub precip_prob: Option<f64>,
    pub code: Option<i64>,
}

/// Jeden den předpovědi; `date` je „2026-10-17“.
#[derive(Serialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Day {
    pub date: String,
    pub min: Option<f64>,
    pub max: Option<f64>,
    pub code: Option<i64>,
}

/// Vše navíc k aktuálnímu stavu.
#[derive(Serialize, Clone, Debug, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct Extra {
    /// pocitová teplota °C
    pub feels: Option<f64>,
    /// vítr km/h
    pub wind: Option<f64>,
    /// směr, odkud vane, ve stupních (0 = sever)
    pub wind_dir: Option<f64>,
    /// dnešní východ / západ slunce v místním čase („2026-10-16T06:58“)
    pub sunrise: Option<String>,
    pub sunset: Option<String>,
    pub hours: Vec<Hour>,
    /// dnešek je index 0 už v `Weather.min/max`; tady jsou jen další dny
    pub days: Vec<Day>,
}

fn arr<'a>(v: &'a Value, sect: &str, key: &str) -> &'a [Value] {
    v.get(sect).and_then(|d| d.get(key)).and_then(Value::as_array).map_or(&[][..], Vec::as_slice)
}

pub fn parse_extra(v: &Value) -> Extra {
    let cur = |k: &str| v.get("current").and_then(|c| c.get(k)).and_then(Value::as_f64);
    let times = arr(v, "hourly", "time");
    let (t, p, c) = (arr(v, "hourly", "temperature_2m"), arr(v, "hourly", "precipitation_probability"), arr(v, "hourly", "weather_code"));
    let hours = times
        .iter()
        .enumerate()
        .take(HOURS)
        .filter_map(|(i, tm)| {
            let h = Hour { time: tm.as_str()?.to_string(), temp: t.get(i).and_then(Value::as_f64), precip_prob: p.get(i).and_then(Value::as_f64), code: c.get(i).and_then(Value::as_i64) };
            (h.temp.is_some() || h.precip_prob.is_some() || h.code.is_some()).then_some(h)
        })
        .collect();
    let dates = arr(v, "daily", "time");
    let (mn, mx, dc) = (arr(v, "daily", "temperature_2m_min"), arr(v, "daily", "temperature_2m_max"), arr(v, "daily", "weather_code"));
    let days = dates
        .iter()
        .enumerate()
        .skip(1)
        .take(DAYS_AHEAD)
        .filter_map(|(i, d)| {
            let day = Day { date: d.as_str()?.to_string(), min: mn.get(i).and_then(Value::as_f64), max: mx.get(i).and_then(Value::as_f64), code: dc.get(i).and_then(Value::as_i64) };
            (day.min.is_some() || day.max.is_some() || day.code.is_some()).then_some(day)
        })
        .collect();
    let today_str = |k: &str| arr(v, "daily", k).first().and_then(Value::as_str).map(str::to_string);
    Extra { feels: cur("apparent_temperature"), wind: cur("wind_speed_10m"), wind_dir: cur("wind_direction_10m"), sunrise: today_str("sunrise"), sunset: today_str("sunset"), hours, days }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(s: &str) -> Extra {
        parse_extra(&serde_json::from_str(s).unwrap())
    }

    #[test]
    fn full_sample() {
        let e = p(include_str!("fixtures/open-meteo-praha.json"));
        assert_eq!((e.feels, e.wind, e.wind_dir), (Some(3.5), Some(11.2), Some(250.0)));
        assert_eq!(e.sunrise.as_deref(), Some("2026-10-16T06:58"));
        assert_eq!(e.sunset.as_deref(), Some("2026-10-16T17:31"));
        assert_eq!(e.hours.len(), 4);
        assert_eq!(e.hours[1], Hour { time: "2026-10-16T01:00".into(), temp: Some(5.5), precip_prob: Some(10.0), code: Some(3) });
        assert_eq!(e.days.len(), 5);
        assert_eq!(e.days[0], Day { date: "2026-10-17".into(), min: Some(4.0), max: Some(14.5), code: Some(61) });
    }

    #[test]
    fn missing_is_none_and_garbage_skipped() {
        assert_eq!(p("{}"), Extra::default());
        let e = p(r#"{"hourly":{"time":["a","b",3,"d"],"temperature_2m":[1.0,null,2.0]},"daily":{"time":["x","y","z"],"temperature_2m_max":[1,"?",3]}}"#);
        // "b" nemá nic → pryč; 3 není řetězec → pryč; "d" nemá teplotu → pryč
        assert_eq!(e.hours.iter().map(|h| h.time.as_str()).collect::<Vec<_>>(), ["a"]);
        // den 0 se přeskakuje; "y" má max nečíselný → pryč; "z" max 3
        assert_eq!(e.days, vec![Day { date: "z".into(), min: None, max: Some(3.0), code: None }]);
    }

    #[test]
    fn caps_lengths() {
        let times: Vec<String> = (0..100).map(|i| format!("\"t{i}\"")).collect();
        let temps = vec!["1.0"; 100].join(",");
        let e = p(&format!(r#"{{"hourly":{{"time":[{}],"temperature_2m":[{temps}]}}}}"#, times.join(",")));
        assert_eq!(e.hours.len(), HOURS);
    }
}
