//! Cache kalendářů na disku (`%LOCALAPPDATA%\Ledge\calendar-cache.json`): výskyty v okně klíčované
//! otiskem zdroje (`source::key` — nikdy adresa ani token). Čte a zapisuje `live.rs` (po startu,
//! po úspěšném načtení); tady jen formát a stáří.

use super::Occ;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Starší cache se po startu nepoužije (okno výskytů už by nesedělo).
pub const CACHE_MAX_AGE_MS: i64 = 7 * 86_400_000;

#[derive(Serialize, Deserialize, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CacheFeed {
    pub key: String,
    pub fetched_ms: i64,
    #[serde(default)]
    pub events: usize,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub occs: Vec<Occ>,
}

#[derive(Serialize, Deserialize)]
struct CacheFile {
    v: u32,
    feeds: Vec<CacheFeed>,
}

pub fn cache_path() -> PathBuf {
    crate::actionrun::local_dir().join("calendar-cache.json")
}

pub fn encode_cache(feeds: Vec<CacheFeed>) -> String {
    serde_json::to_string(&CacheFile { v: 1, feeds }).unwrap_or_default()
}

/// Čitelné, verze 1, ne starší než `CACHE_MAX_AGE_MS`. Cokoli jiného = prázdno (nikdy chyba).
pub fn decode_cache(text: &str, now: i64) -> Vec<CacheFeed> {
    match serde_json::from_str::<CacheFile>(text) {
        Ok(f) if f.v == 1 => f.feeds.into_iter().filter(|c| now - c.fetched_ms < CACHE_MAX_AGE_MS && c.fetched_ms <= now).collect(),
        _ => Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cache_roundtrip_and_age() {
        let now = 1_800_000_000_000;
        let occ = Occ { start: now, end: now + 1, title: "A".into(), ..Occ::default() };
        let feeds = vec![
            CacheFeed { key: "k1".into(), fetched_ms: now - 1000, events: 3, name: "Rodina".into(), occs: vec![occ.clone()] },
            CacheFeed { key: "old".into(), fetched_ms: now - CACHE_MAX_AGE_MS, events: 1, name: String::new(), occs: vec![] },
            CacheFeed { key: "budoucí".into(), fetched_ms: now + 60_000, events: 1, name: String::new(), occs: vec![] },
        ];
        let text = encode_cache(feeds);
        assert!(!text.contains("https://"), "cache nesmí nést adresu");
        let back = decode_cache(&text, now);
        assert_eq!(back.len(), 1);
        assert_eq!((back[0].key.as_str(), back[0].events, back[0].name.as_str(), &back[0].occs[0]), ("k1", 3, "Rodina", &occ));
        for bad in ["", "{", r#"{"v":2,"feeds":[]}"#, "[]"] {
            assert!(decode_cache(bad, now).is_empty(), "{bad}");
        }
    }

    #[test]
    fn join_roundtrip_and_old_cache_without_it() {
        use crate::calendar::meeting::{Join, Provider};
        let now = 1_800_000_000_000;
        let occ = Occ { start: now, end: now + 1, title: "Standup".into(), join: Some(Join { provider: Provider::Teams, url: "https://teams.microsoft.com/meet/1?p=a".into() }), ..Occ::default() };
        let text = encode_cache(vec![CacheFeed { key: "k".into(), fetched_ms: now, events: 1, name: String::new(), occs: vec![occ.clone()] }]);
        assert!(text.contains(r#""provider":"teams""#), "{text}");
        assert_eq!(decode_cache(&text, now)[0].occs[0], occ);
        // cache z vlny 5 (bez `join`) se načte beze ztráty
        let old = format!(r#"{{"v":1,"feeds":[{{"key":"k","fetchedMs":{now},"events":1,"name":"","occs":[{{"start":{now},"end":{},"allDay":false,"title":"A","location":"","tzGuess":false}}]}}]}}"#, now + 1);
        let back = decode_cache(&old, now);
        assert_eq!((back[0].occs[0].title.as_str(), back[0].occs[0].join.is_none()), ("A", true));
    }
}
