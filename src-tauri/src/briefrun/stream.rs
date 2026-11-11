//! Čisté čtení výstupu `claude -p --output-format stream-json`: řádek → událost, náklad po běhu,
//! verze CLI, výsledný stav zadání a začištění textu do logu (klíč nikdy).

/// Zdroj klíče, který smí běh ze zadání použít (pole `apiKeySource` v `init`).
pub const OUR_SOURCE: &str = "ANTHROPIC_API_KEY";

#[derive(Debug, Clone, PartialEq)]
pub enum Event {
    /// `{"type":"system","subtype":"init",…}`; `api_key_source` = pole `apiKeySource`, chybí-li → `None`
    Init { api_key_source: Option<String> },
    /// poslední řádek tahu
    Result { cost: Option<f64>, is_error: bool, subtype: String },
    /// cokoli jiného (zprávy asistenta, nástroje, neplatný JSON)
    Other,
}

pub fn parse_line(line: &str) -> Event {
    let Ok(v) = serde_json::from_str::<serde_json::Value>(line.trim()) else { return Event::Other };
    let s = |k: &str| v.get(k).and_then(|x| x.as_str());
    match (s("type"), s("subtype")) {
        (Some("system"), Some("init")) => Event::Init { api_key_source: s("apiKeySource").map(str::to_string) },
        (Some("result"), sub) => Event::Result {
            cost: v.get("total_cost_usd").and_then(|x| x.as_f64()).filter(|c| c.is_finite() && *c >= 0.0),
            is_error: v.get("is_error").and_then(|x| x.as_bool()).unwrap_or(true),
            subtype: sub.unwrap_or("").to_string(),
        },
        _ => Event::Other,
    }
}

/// Pojistka: `init` hlásí jiný zdroj klíče než náš env → běh by jel z předplatného. Chybějící pole
/// (`None`) pojistku nespouští — formát není ověřený, náklad pak hlídá `--max-budget-usd`.
pub fn wrong_source(api_key_source: Option<&str>) -> bool {
    api_key_source.is_some_and(|s| s != OUR_SOURCE)
}

/// Verze z `claude --version` („2.1.268 (Claude Code)“).
pub fn parse_version(out: &str) -> Option<(u32, u32, u32)> {
    let w = out.split_whitespace().next()?;
    let mut it = w.split('.').map(|p| p.parse::<u32>().ok());
    Some((it.next()??, it.next()??, it.next()??))
}

/// Od této verze `total_cost_usd` s `--resume` hlásí součet celé session (dřív jen běh).
pub const CUMULATIVE_FROM: (u32, u32, u32) = (2, 1, 277);

/// Náklad zadání po běhu. `prev` = náklad v poznámce před během, `reported` = `total_cost_usd`.
/// Novější CLI při `--resume` hlásí celou session → nepřičítat, jen nikdy neklesnout. Neznámá verze
/// = starší chování (přičíst) — v nejhorším nadhodnotí, nikdy nepodhodnotí rozpočet.
pub fn next_cost(prev: f64, reported: f64, cli_version: Option<(u32, u32, u32)>, resumed: bool) -> f64 {
    let cumulative = resumed && cli_version.is_some_and(|v| v >= CUMULATIVE_FROM);
    if cumulative { prev.max(reported) } else { prev + reported }
}

/// Jak běh skončil (sběr ze čtecího vlákna).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Ended {
    pub result: Option<(Option<f64>, bool, String)>,
    /// pojistka zdroje klíče
    pub wrong_source: Option<String>,
    /// `brief_stop`
    pub stopped: bool,
    /// první řádek stderr (už začištěný)
    pub stderr: String,
}

/// Výsledný stav poznámky + řádek do `## Log` (je-li).
#[derive(Debug, Clone, PartialEq)]
pub struct Finish {
    pub status: &'static str,
    pub log: Option<String>,
}

pub fn finish(e: &Ended, open_questions: bool) -> Finish {
    let draft = |log: String| Finish { status: "draft", log: Some(log) };
    if let Some(src) = &e.wrong_source {
        return draft(format!("Ledge běh zastavil: běh by jel z předplatného (zdroj klíče {src}), ne z API klíče."));
    }
    if e.stopped {
        return draft("Běh zastaven ručně v Ledge.".into());
    }
    match &e.result {
        Some((_, false, _)) => Finish { status: if open_questions { "waiting" } else { "review" }, log: None },
        Some((_, true, sub)) if sub == "error_max_budget_usd" => draft("Běh skončil: vyčerpaný rozpočet zadání.".into()),
        Some((_, true, sub)) => draft(format!("Běh skončil chybou ({}).", if sub.is_empty() { "?" } else { sub })),
        None if e.stderr.is_empty() => draft("Běh skončil bez výsledku.".into()),
        None => draft(format!("Běh skončil bez výsledku: {}", e.stderr)),
    }
}

/// Text z výstupu procesu do logu: jeden řádek, nejvýš 300 znaků, výskyty klíče nahrazené.
pub fn redact(text: &str, key: &str) -> String {
    let one: String = text.split_whitespace().collect::<Vec<_>>().join(" ");
    let clean = if key.len() >= 8 { one.replace(key, "***") } else { one };
    // i kdyby klíč byl useknutý, prefix `sk-ant-` s pokračováním nepustit
    let clean = mask_sk(&clean);
    clean.chars().take(300).collect()
}

fn mask_sk(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut rest = s;
    while let Some(i) = rest.find("sk-ant-") {
        out.push_str(&rest[..i]);
        out.push_str("sk-ant-***");
        let after = &rest[i + 7..];
        let end = after.find(|c: char| !(c.is_ascii_alphanumeric() || c == '-' || c == '_')).unwrap_or(after.len());
        rest = &after[end..];
    }
    out.push_str(rest);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_line_table() {
        let cases: &[(&str, Event)] = &[
            (r#"{"type":"system","subtype":"init","session_id":"s","apiKeySource":"ANTHROPIC_API_KEY"}"#, Event::Init { api_key_source: Some("ANTHROPIC_API_KEY".into()) }),
            (r#"{"type":"system","subtype":"init","session_id":"s"}"#, Event::Init { api_key_source: None }),
            (r#"{"type":"result","subtype":"success","is_error":false,"total_cost_usd":0.1234}"#, Event::Result { cost: Some(0.1234), is_error: false, subtype: "success".into() }),
            (r#"{"type":"result","subtype":"error_max_budget_usd","is_error":true,"total_cost_usd":2.01}"#, Event::Result { cost: Some(2.01), is_error: true, subtype: "error_max_budget_usd".into() }),
            (r#"{"type":"result"}"#, Event::Result { cost: None, is_error: true, subtype: String::new() }),
            (r#"{"type":"result","is_error":false,"total_cost_usd":-1}"#, Event::Result { cost: None, is_error: false, subtype: String::new() }),
            (r#"{"type":"assistant","message":{}}"#, Event::Other),
            (r#"{"type":"system","subtype":"compact_boundary"}"#, Event::Other),
            ("není json", Event::Other),
            ("", Event::Other),
        ];
        for (l, want) in cases {
            assert_eq!(&parse_line(l), want, "{l}");
        }
    }

    #[test]
    fn source_guard_table() {
        for (s, bad) in [(Some("ANTHROPIC_API_KEY"), false), (None, false), (Some("none"), true), (Some("/login managed key"), true), (Some("ANTHROPIC_AUTH_TOKEN"), true)] {
            assert_eq!(wrong_source(s), bad, "{s:?}");
        }
    }

    #[test]
    fn version_and_cost_table() {
        assert_eq!(parse_version("2.1.268 (Claude Code)\n"), Some((2, 1, 268)));
        assert_eq!(parse_version("2.1.277"), Some((2, 1, 277)));
        assert_eq!(parse_version("claude"), None);
        assert_eq!(parse_version(""), None);
        let old = Some((2, 1, 268));
        let new = Some((2, 1, 277));
        let cases = [
            (0.0, 0.5, old, false, 0.5),
            (0.5, 0.25, old, true, 0.75),
            (0.5, 0.75, new, true, 0.75),
            (0.9, 0.75, new, true, 0.9),
            (0.5, 0.25, new, false, 0.75),
            (0.5, 0.25, None, true, 0.75),
        ];
        for (prev, rep, v, res, want) in cases {
            assert!((next_cost(prev, rep, v, res) - want).abs() < 1e-9, "{prev} {rep} {v:?} {res}");
        }
    }

    #[test]
    fn finish_table() {
        let ok = Ended { result: Some((Some(0.1), false, "success".into())), ..Default::default() };
        assert_eq!(finish(&ok, true), Finish { status: "waiting", log: None });
        assert_eq!(finish(&ok, false), Finish { status: "review", log: None });
        let e = |sub: &str| Ended { result: Some((None, true, sub.into())), ..Default::default() };
        assert_eq!(finish(&e("error_max_budget_usd"), true).status, "draft");
        assert!(finish(&e("error_max_budget_usd"), false).log.unwrap().contains("rozpočet"));
        assert!(finish(&e("error_during_execution"), false).log.unwrap().contains("error_during_execution"));
        let src = Ended { wrong_source: Some("none".into()), result: Some((None, false, String::new())), ..Default::default() };
        assert!(finish(&src, false).log.unwrap().contains("předplatného"));
        let stop = Ended { stopped: true, ..Default::default() };
        assert_eq!(finish(&stop, false).status, "draft");
        let crash = Ended { stderr: "error: unknown option".into(), ..Default::default() };
        assert!(finish(&crash, false).log.unwrap().ends_with("error: unknown option"));
        assert_eq!(finish(&Ended::default(), false).log.as_deref(), Some("Běh skončil bez výsledku."));
    }

    #[test]
    fn redact_never_leaks_key() {
        let k = "sk-ant-api03-TESTKLIC_uzivatel-123";
        for t in [format!("Invalid API key {k}\nretry"), "x sk-ant-api03-TESTK".to_string(), format!("{k}{k}")] {
            let r = redact(&t, k);
            assert!(!r.contains("TESTK"), "{r}");
            assert!(!r.contains('\n'));
        }
        assert_eq!(redact("a\n b", k), "a b");
        assert_eq!(redact(&"x".repeat(400), k).chars().count(), 300);
    }
}
