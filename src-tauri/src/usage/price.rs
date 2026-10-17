//! Ceník API Claude (USD za 1M tokenů) jako data — jediné místo s cenami v Ledge.
//! Je to **odhad v cenách API, ne účet**: předplatné Pro/Max se po tokenech neúčtuje.
//! Neznámý model = cena `None` (UI „—“), nikdy odhad podle podobného jména.
//!
//! Zdroj: <https://platform.claude.com/docs/en/about-claude/pricing> (dřív docs.claude.com),
//! ověřeno `VERIFIED`. Modifikátory ze stejné stránky: Haiku 5.5 nad 100k tokenů promptu = vyšší pásmo
//! (každý požadavek zvlášť, prompt = vstup + zápis + čtení cache), fast mode = 2× (Opus 5.5 8/40,
//! Opus 5 a 4.8 10/50; cache multiplikátory platí navrch), `inference_geo: "us"` = 1,1× u modelů 4.6+.
//! Batch sleva se netýká (Claude Code batch nepoužívá).

use super::parse::Tokens;

pub const VERIFIED: &str = "2026-10-17";
pub const SOURCE: &str = "https://platform.claude.com/docs/en/about-claude/pricing";

/// Ceny za 1M tokenů.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Price {
    pub input: f64,
    pub cw5: f64,
    pub cw1: f64,
    pub cr: f64,
    pub output: f64,
}

const fn p(input: f64, cw5: f64, cw1: f64, cr: f64, output: f64) -> Price {
    Price { input, cw5, cw1, cr, output }
}

pub struct Model {
    /// kanonické id bez data a sufixů (`claude-opus-4-5`, ne `claude-opus-4-5-20251101`)
    pub id: &'static str,
    pub name: &'static str,
    pub price: Price,
    /// vyšší pásmo pro prompt nad `LONG_PROMPT` (jen Haiku 5.5)
    pub long: Option<Price>,
    /// násobek ceny ve fast mode (jen modely s fast mode)
    pub fast: Option<f64>,
    /// podporuje `inference_geo` (4.6 a novější) → „us“ = ×1,1
    pub geo: bool,
}

/// Hranice promptu pro vyšší pásmo Haiku 5.5.
pub const LONG_PROMPT: u64 = 100_000;
const GEO_US: f64 = 1.1;

const OPUS5: Price = p(5.0, 6.25, 10.0, 0.50, 25.0);
const OPUS41: Price = p(15.0, 18.75, 30.0, 1.50, 75.0);
const SONNET4: Price = p(3.0, 3.75, 6.0, 0.30, 15.0);
const FABLE: Price = p(10.0, 12.50, 20.0, 1.0, 50.0);
const FABLE51: Price = p(10.0, 12.50, 20.0, 0.25, 50.0);

const fn m(id: &'static str, name: &'static str, price: Price, geo: bool) -> Model {
    Model { id, name, price, long: None, fast: None, geo }
}

pub static MODELS: &[Model] = &[
    m("claude-fable-5-1", "Fable 5.1", FABLE51, true),
    m("claude-mythos-5-1", "Mythos 5.1", FABLE51, true),
    m("claude-fable-5", "Fable 5", FABLE, true),
    m("claude-mythos-5", "Mythos 5", FABLE, true),
    Model { fast: Some(2.0), ..m("claude-opus-5-5", "Opus 5.5", p(4.0, 5.0, 8.0, 0.20, 20.0), true) },
    Model { fast: Some(2.0), ..m("claude-opus-5", "Opus 5", OPUS5, true) },
    Model { fast: Some(2.0), ..m("claude-opus-4-8", "Opus 4.8", OPUS5, true) },
    m("claude-opus-4-7", "Opus 4.7", OPUS5, true),
    m("claude-opus-4-6", "Opus 4.6", OPUS5, true),
    m("claude-opus-4-5", "Opus 4.5", OPUS5, false),
    m("claude-opus-4-1", "Opus 4.1", OPUS41, false),
    m("claude-opus-4", "Opus 4", OPUS41, false),
    m("claude-sonnet-5-5", "Sonnet 5.5", p(2.0, 2.50, 4.0, 0.10, 10.0), true),
    m("claude-sonnet-5", "Sonnet 5", p(2.0, 2.50, 4.0, 0.20, 10.0), true),
    m("claude-sonnet-4-6", "Sonnet 4.6", SONNET4, true),
    m("claude-sonnet-4-5", "Sonnet 4.5", SONNET4, false),
    m("claude-sonnet-4", "Sonnet 4", SONNET4, false),
    Model { long: Some(p(0.50, 0.625, 1.0, 0.05, 2.50)), ..m("claude-haiku-5-5", "Haiku 5.5", p(0.10, 0.125, 0.20, 0.01, 0.50), true) },
    m("claude-haiku-4-5", "Haiku 4.5", p(1.0, 1.25, 2.0, 0.10, 5.0), false),
    m("claude-3-5-haiku", "Haiku 3.5", p(0.80, 1.0, 1.60, 0.08, 4.0), false),
];

/// Pásmo ceny jednoho požadavku (bity; ukládá se v indexu vedle tokenů).
pub const TIER_LONG: u8 = 1;
pub const TIER_FAST: u8 = 2;
pub const TIER_US: u8 = 4;

/// Id modelu z transcriptu → kanonické id (malá písmena; bez `us.anthropic.` / `anthropic.` prefixu,
/// `[1m]`, `@datum`, `:verze`, `-v1`, `-YYYYMMDD`). Neznámý model zůstane, jak je (jen oříznutý).
pub fn canonical(model: &str) -> String {
    let mut s = model.trim().to_lowercase();
    if let Some(i) = s.rfind("anthropic.") {
        s = s[i + "anthropic.".len()..].to_string();
    }
    if let Some(i) = s.find(['[', '@', ':']) {
        s.truncate(i);
    }
    if let Some(i) = s.rfind("-v")
        && s[i + 2..].chars().all(|c| c.is_ascii_digit())
        && i + 2 < s.len()
    {
        s.truncate(i);
    }
    if let Some(i) = s.rfind('-')
        && s.len() - i == 9
        && s[i + 1..].chars().all(|c| c.is_ascii_digit())
    {
        s.truncate(i);
    }
    s
}

pub fn lookup(canonical_id: &str) -> Option<&'static Model> {
    MODELS.iter().find(|m| m.id == canonical_id)
}

/// Pásmo požadavku podle modelu a jeho usage.
pub fn tier(canonical_id: &str, t: &Tokens, fast: bool, geo_us: bool) -> u8 {
    let mut tier = 0;
    if lookup(canonical_id).is_some_and(|m| m.long.is_some()) && t.prompt() > LONG_PROMPT {
        tier |= TIER_LONG;
    }
    if fast {
        tier |= TIER_FAST;
    }
    if geo_us {
        tier |= TIER_US;
    }
    tier
}

/// Cena v USD; `None` = model (nebo jeho pásmo, třeba fast mode u modelu bez fast mode) ceník nezná.
pub fn cost(canonical_id: &str, tier: u8, t: &Tokens) -> Option<f64> {
    let m = lookup(canonical_id)?;
    let pr = if tier & TIER_LONG != 0 { m.long? } else { m.price };
    let mut mult = 1.0;
    if tier & TIER_FAST != 0 {
        mult *= m.fast?;
    }
    if tier & TIER_US != 0 && m.geo {
        mult *= GEO_US;
    }
    let usd = t.input as f64 * pr.input + t.cw5 as f64 * pr.cw5 + t.cw1 as f64 * pr.cw1 + t.cr as f64 * pr.cr + t.output as f64 * pr.output;
    Some(usd * mult / 1e6)
}

/// Čitelný název modelu (`Opus 5.5`); neznámý = id.
pub fn display(canonical_id: &str) -> String {
    lookup(canonical_id).map_or_else(|| canonical_id.to_string(), |m| m.name.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    const M: u64 = 1_000_000;

    #[test]
    fn canonical_ids() {
        for (raw, want) in [
            ("claude-opus-5-5", "claude-opus-5-5"),
            ("claude-opus-4-5-20251101", "claude-opus-4-5"),
            ("claude-opus-4-20250514", "claude-opus-4"),
            ("claude-sonnet-4-5-20250929[1m]", "claude-sonnet-4-5"),
            ("us.anthropic.claude-opus-4-1-20250805-v1:0", "claude-opus-4-1"),
            ("claude-opus-4-5@20251101", "claude-opus-4-5"),
            ("Claude-Fable-5-1", "claude-fable-5-1"),
            ("claude-3-5-haiku-20241022", "claude-3-5-haiku"),
            ("<synthetic>", "<synthetic>"),
            ("gpt-5", "gpt-5"),
        ] {
            assert_eq!(canonical(raw), want, "{raw}");
        }
        assert!(lookup("claude-opus-5-5").is_some() && lookup("claude-opus-5-5-x").is_none(), "jen přesná shoda");
    }

    fn close(a: Option<f64>, b: f64) -> bool {
        a.is_some_and(|a| (a - b).abs() < 1e-9)
    }

    #[test]
    fn costs_table() {
        let t = |input, output, cw5, cw1, cr| Tokens { input, output, cw5, cw1, cr };
        let cases: &[(&str, u8, Tokens, Option<f64>)] = &[
            ("claude-opus-5-5", 0, t(M, M, 0, 0, 0), Some(4.0 + 20.0)),
            ("claude-opus-5-5", 0, t(0, 0, M, M, M), Some(5.0 + 8.0 + 0.20)),
            ("claude-sonnet-5-5", 0, t(0, 0, 0, 0, 10 * M), Some(1.0)),
            ("claude-sonnet-5", 0, t(0, 0, 0, 0, 10 * M), Some(2.0)),
            ("claude-fable-5-1", 0, t(M, M, M, M, M), Some(10.0 + 50.0 + 12.5 + 20.0 + 0.25)),
            ("claude-opus-4-1", 0, t(M, 0, 0, 0, 0), Some(15.0)),
            ("claude-haiku-5-5", 0, t(M, 0, 0, 0, 0), Some(0.10)),
            ("claude-haiku-5-5", TIER_LONG, t(M, 0, 0, 0, 0), Some(0.50)),
            ("claude-opus-5-5", TIER_FAST, t(M, M, 0, 0, M), Some(8.0 + 40.0 + 0.40)),
            ("claude-opus-5-5", TIER_US, t(M, 0, 0, 0, 0), Some(4.4)),
            ("claude-opus-4-5", TIER_US, t(M, 0, 0, 0, 0), Some(5.0)),
            ("claude-sonnet-5-5", TIER_FAST, t(M, 0, 0, 0, 0), None),
            ("claude-opus-4-5", TIER_LONG, t(M, 0, 0, 0, 0), None),
            ("gpt-5", 0, t(M, 0, 0, 0, 0), None),
            ("<synthetic>", 0, t(1, 0, 0, 0, 0), None),
        ];
        for (model, tier, tok, want) in cases {
            let got = cost(model, *tier, tok);
            match want {
                Some(w) => assert!(close(got, *w), "{model} {tier}: {got:?} ≠ {w}"),
                None => assert_eq!(got, None, "{model} {tier}"),
            }
        }
    }

    #[test]
    fn tiers() {
        let small = Tokens { input: 10, cr: LONG_PROMPT - 10, ..Tokens::default() };
        let big = Tokens { input: 10, cw1: 5, cr: LONG_PROMPT, ..Tokens::default() };
        assert_eq!(tier("claude-haiku-5-5", &small, false, false), 0, "přesně 100k = nižší pásmo");
        assert_eq!(tier("claude-haiku-5-5", &big, false, false), TIER_LONG, "prompt počítá i cache");
        assert_eq!(tier("claude-opus-5-5", &big, false, false), 0, "Opus pásma nemá");
        assert_eq!(tier("claude-opus-5-5", &small, true, true), TIER_FAST | TIER_US);
    }

    #[test]
    fn every_model_has_unique_id_and_name() {
        let mut ids: Vec<_> = MODELS.iter().map(|m| m.id).collect();
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), MODELS.len());
        assert!(MODELS.iter().all(|m| canonical(m.id) == m.id));
        assert_eq!(display("claude-fable-5-1"), "Fable 5.1");
        assert_eq!(display("neznamy"), "neznamy");
    }
}
