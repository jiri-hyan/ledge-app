//! Cena jedné session (kolo 34, H6): součty tokenů po modelu a pásmu držené v `FileEntry.ses`, plněné při
//! běžném čtení souboru (`index::read_file`, žádný druhý průchod ani polling). Session = `<sid>.jsonl`, soubory
//! subagentů `<sid>/subagents/…/agent-*.jsonl` se přičtou k rodiči. Čisté, bez I/O.
//!
//! Definice: odhad v cenách API od startu session, jen z odpovědí uvnitř okna indexu (`WINDOW_DAYS`); starší
//! odpovědi index zahazuje → `partial` (částka je dolní odhad). Odpověď zkopírovaná do jiného souboru (resume)
//! se počítá jednou, u souboru, který index přečetl první (dedup `seen`).

use super::index::Index;
use super::parse::{Rec, Tokens};
use super::price;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Nejvíc session v přehledu (nejnovější podle poslední odpovědi) — strop paměti snapshotu.
pub const CAP: usize = 40;

/// Součet jednoho souboru pro jeden model a pásmo.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SesSum {
    pub model: String,
    pub tier: u8,
    pub t: Tokens,
    pub n: u64,
}

/// Přičte odpověď (už prošla dedupem) k součtům souboru; `span` = čas první a poslední započtené odpovědi.
pub fn note(ses: &mut Vec<SesSum>, span: &mut (i64, i64), model: &str, tier: u8, r: &Rec) {
    match ses.iter_mut().find(|s| s.model == model && s.tier == tier) {
        Some(s) => {
            s.t.add(&r.tokens);
            s.n += 1;
        }
        None => ses.push(SesSum { model: model.to_string(), tier, t: r.tokens, n: 1 }),
    }
    span.0 = if span.0 == 0 { r.ts_ms } else { span.0.min(r.ts_ms) };
    span.1 = span.1.max(r.ts_ms);
}

/// id session z cesty transcriptu: `…/<sid>.jsonl` → `sid`; subagent `…/<sid>/subagents/…` → `sid`.
pub fn session_id(path: &str) -> Option<String> {
    let p = path.replace('\\', "/");
    let parts: Vec<&str> = p.split('/').collect();
    let id = match parts.iter().position(|c| *c == "subagents") {
        Some(i) if i > 0 => parts[i - 1],
        Some(_) => return None,
        None => parts.last()?.strip_suffix(".jsonl")?,
    };
    (!id.is_empty()).then(|| id.to_string())
}

/// Cena jedné session v přehledu.
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SessCost {
    pub id: String,
    pub tokens: u64,
    /// `None` = žádný token nemá cenu v ceníku; jinak součet jen NACENĚNÝCH tokenů
    pub cost: Option<f64>,
    /// tokeny bez ceny (neznámý model) — částka je pak dolní odhad
    pub unpriced: u64,
    /// odpovědí API
    pub requests: u64,
    /// první a poslední započtená odpověď (ms)
    pub first_ms: i64,
    pub last_ms: i64,
    /// soubor má i odpovědi starší než okno (zahozené) → dolní odhad
    pub partial: bool,
}

/// Index → ceny session (nejnovější první, nejvýš `CAP`). Soubory bez započtené odpovědi se vynechají.
pub fn build(ix: &Index) -> Vec<SessCost> {
    let mut by: HashMap<String, SessCost> = HashMap::new();
    for (path, e) in &ix.files {
        if e.ses.is_empty() {
            continue;
        }
        let Some(id) = session_id(path) else { continue };
        let c = by.entry(id.clone()).or_insert_with(|| SessCost { id, ..SessCost::default() });
        for s in &e.ses {
            let tokens = s.t.total();
            c.tokens = c.tokens.saturating_add(tokens);
            c.requests = c.requests.saturating_add(s.n);
            match price::cost(&s.model, s.tier, &s.t) {
                Some(v) => c.cost = Some(c.cost.unwrap_or(0.0) + v),
                None => c.unpriced = c.unpriced.saturating_add(tokens),
            }
        }
        c.first_ms = if c.first_ms == 0 { e.span.0 } else if e.span.0 == 0 { c.first_ms } else { c.first_ms.min(e.span.0) };
        c.last_ms = c.last_ms.max(e.span.1);
        c.partial |= e.partial;
    }
    let mut v: Vec<SessCost> = by.into_values().collect();
    v.sort_by(|a, b| b.last_ms.cmp(&a.last_ms).then(a.id.cmp(&b.id)));
    v.truncate(CAP);
    v
}

#[cfg(test)]
mod tests {
    use super::super::index::FileEntry;
    use super::*;

    const M: u64 = 1_000_000;

    fn rec(ts: i64, out: u64) -> Rec {
        Rec { key: 1, ts_ms: ts, model: "claude-opus-5-5".into(), cwd: None, tokens: Tokens { output: out, ..Tokens::default() }, fast: false, geo_us: false }
    }

    fn entry(model: &str, out: u64, span: (i64, i64), partial: bool) -> FileEntry {
        let mut e = FileEntry { partial, ..FileEntry::default() };
        note(&mut e.ses, &mut e.span, model, 0, &rec(span.0, out));
        e.span = span;
        e
    }

    #[test]
    fn session_id_table() {
        let rows = [
            (r"C:\u\.claude\projects\C--x\abc-123.jsonl", Some("abc-123")),
            (r"C:\u\.claude\projects\C--x\abc-123\subagents\agent-9.jsonl", Some("abc-123")),
            (r"C:\u\.claude\projects\C--x\abc-123\subagents\workflows\w\agent-9.jsonl", Some("abc-123")),
            ("/home/u/.claude/projects/x/s1.jsonl", Some("s1")),
            ("subagents/agent-1.jsonl", None),
            (r"C:\x\notes.txt", None),
        ];
        for (p, want) in rows {
            assert_eq!(session_id(p).as_deref(), want, "{p}");
        }
    }

    #[test]
    fn note_merges_by_model_tier_and_tracks_span() {
        let (mut ses, mut span) = (Vec::new(), (0, 0));
        note(&mut ses, &mut span, "claude-opus-5-5", 0, &rec(500, 10));
        note(&mut ses, &mut span, "claude-opus-5-5", 0, &rec(200, 5));
        note(&mut ses, &mut span, "claude-sonnet-5-5", 0, &rec(900, 1));
        assert_eq!(ses.len(), 2);
        assert_eq!((ses[0].t.output, ses[0].n), (15, 2));
        assert_eq!(span, (200, 900));
    }

    #[test]
    fn build_sums_subagents_into_parent_and_prices() {
        let mut ix = Index::default();
        ix.files.insert(r"C:\p\s1.jsonl".into(), entry("claude-opus-5-5", M, (100, 300), false)); // 20 $
        ix.files.insert(r"C:\p\s1\subagents\agent-a.jsonl".into(), entry("claude-sonnet-5-5", M, (150, 400), true)); // 10 $
        ix.files.insert(r"C:\p\s2.jsonl".into(), entry("neznamy-model", 7, (50, 60), false));
        ix.files.insert(r"C:\p\empty.jsonl".into(), FileEntry::default());
        let v = build(&ix);
        assert_eq!(v.len(), 2, "soubor bez odpovědí se vynechá");
        let s1 = &v[0];
        assert_eq!(s1.id, "s1", "nejnovější první");
        assert_eq!((s1.cost, s1.unpriced, s1.requests, s1.tokens), (Some(30.0), 0, 2, 2 * M));
        assert_eq!((s1.first_ms, s1.last_ms, s1.partial), (100, 400, true));
        let s2 = &v[1];
        assert_eq!((s2.cost, s2.unpriced, s2.tokens), (None, 7, 7), "neznámý model: tokeny ano, cena ne");
        assert!(!s2.partial);
    }

    #[test]
    fn build_caps_to_newest() {
        let mut ix = Index::default();
        for i in 0..(CAP as i64 + 10) {
            ix.files.insert(format!("C:\\p\\s{i}.jsonl"), entry("claude-opus-5-5", 1, (i + 1, i + 1), false));
        }
        let v = build(&ix);
        assert_eq!(v.len(), CAP);
        assert_eq!(v[0].id, format!("s{}", CAP + 9));
        assert!(v.iter().all(|s| s.last_ms > 10), "nejstarší vypadly");
    }
}
