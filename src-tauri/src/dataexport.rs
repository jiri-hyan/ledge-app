//! Export útrat a limitů Claude do CSV nebo Markdownu (karta Claude → ⋯ → „Exportovat…“).
//!
//! Jen data, která Ledge MÁ z reálných zdrojů a už je drží v paměti: přehled útrat z transcriptů
//! (`costslive::view`, odhad v cenách API, ne fakturace) a poslední vzorek limitů 5h / týden
//! (`limitslive::view`, z wrapperu stavového řádku). Co chybí (neznámá cena modelu, okno bez dat), zůstane
//! prázdné — nic se nedopočítává ani nevymýšlí. Žádná síť, žádný nový sken: export čte jen paměť; cíl vybírá
//! člověk v dialogu „Uložit jako“ (`commands::smart::export_usage`).
//!
//! Čisté: `render` (CSV / Markdown) a `csv_cell` (uvozovky + ochrana před vzorci v Excelu) — tabulkové testy.

use crate::costslive::CostsView;
use crate::limits;
use crate::limitslive::LimitsView;
use crate::usage::view::Line;
use std::fmt::Write as _;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Format {
    Csv,
    Markdown,
}

impl Format {
    pub(crate) fn parse(s: &str) -> Option<Self> {
        match s {
            "csv" => Some(Format::Csv),
            "md" | "markdown" => Some(Format::Markdown),
            _ => None,
        }
    }
    pub(crate) fn ext(self) -> &'static str {
        match self {
            Format::Csv => "csv",
            Format::Markdown => "md",
        }
    }
}

/// Buňka CSV: uvozovky a zdvojení, když je třeba; text začínající `= + - @` (i po úvodních mezerách) nebo
/// tabulátorem / CR dostane apostrof, aby ho Excel nebral jako vzorec (názvy projektů jsou cizí vstup z cest na disku).
pub(crate) fn csv_cell(s: &str) -> String {
    let mut v = s.to_string();
    // Excel i LibreOffice vzorec poznají i po úvodních mezerách: znak se kontroluje PO `trim_start`
    if v.starts_with(['\t', '\r']) || v.trim_start().starts_with(['=', '+', '-', '@']) {
        v.insert(0, '\'');
    }
    if v.contains([',', '"', '\n', '\r']) {
        format!("\"{}\"", v.replace('"', "\"\""))
    } else {
        v
    }
}

fn iso(ms: i64) -> String {
    chrono::DateTime::from_timestamp_millis(ms).map(|d| d.to_rfc3339_opts(chrono::SecondsFormat::Secs, true)).unwrap_or_default()
}

fn usd(c: Option<f64>) -> String {
    c.map(|c| format!("{c:.4}")).unwrap_or_default()
}

fn pct(p: f64) -> String {
    format!("{p:.1}")
}

fn line_name(l: &Line) -> String {
    if l.other { crate::i18n::tr("ostatní", "other").to_string() } else { l.name.clone() }
}

/// Je co exportovat? (přehled útrat nebo aspoň jedno okno limitů)
pub(crate) fn has_data(costs: &CostsView, limits: &LimitsView) -> bool {
    costs.summary.is_some() || limits.five_hour.is_some() || limits.seven_day.is_some()
}

fn limit_windows(l: &LimitsView) -> [(&'static str, &'static str, Option<limits::Window>); 2] {
    [("five_hour", "5h", l.five_hour), ("seven_day", "7d", l.seven_day)]
}

/// Obsah souboru. `now_ms` jen do hlavičky Markdownu (a testy).
pub(crate) fn render(f: Format, costs: &CostsView, limits: &LimitsView, now_ms: i64) -> String {
    match f {
        Format::Csv => csv(costs, limits),
        Format::Markdown => markdown(costs, limits, now_ms),
    }
}

fn csv(costs: &CostsView, limits: &LimitsView) -> String {
    // BOM: Excel pozná UTF-8 (diakritika v názvech projektů)
    let mut out = String::from("\u{feff}kind,name,period,tokens,cost_usd_api_estimate,pct,resets_at\r\n");
    let mut row = |cells: [String; 7]| {
        let line: Vec<String> = cells.iter().map(|c| csv_cell(c)).collect();
        out.push_str(&line.join(","));
        out.push_str("\r\n");
    };
    for (key, _, w) in limit_windows(limits) {
        if let Some(w) = w {
            row(["limit".into(), key.into(), String::new(), String::new(), String::new(), pct(w.pct), w.resets_at.map(iso).unwrap_or_default()]);
        }
    }
    if let Some(s) = &costs.summary {
        for r in &s.ranges {
            let period = format!("{}d", r.days);
            row(["total".into(), String::new(), period.clone(), r.total.tokens.to_string(), usd(r.total.cost), String::new(), String::new()]);
            for (kind, ls) in [("project", &r.projects), ("model", &r.models)] {
                for l in ls.iter() {
                    row([kind.into(), line_name(l), period.clone(), l.tokens.to_string(), usd(l.cost), String::new(), String::new()]);
                }
            }
        }
        for d in &s.days {
            row(["day".into(), d.date.clone(), String::new(), d.tokens.to_string(), usd(d.cost), String::new(), String::new()]);
        }
    }
    out
}

fn markdown(costs: &CostsView, limits: &LimitsView, now_ms: i64) -> String {
    use crate::i18n::tr;
    let mut o = String::new();
    let _ = writeln!(o, "# {}\n", tr("Ledge — útrata a limity Claude", "Ledge — Claude usage and limits"));
    let _ = writeln!(o, "{}: {}\n", tr("Vytvořeno", "Created"), iso(now_ms));
    if limit_windows(limits).iter().any(|(_, _, w)| w.is_some()) {
        let _ = writeln!(o, "## {}\n", tr("Limity", "Limits"));
        if let Some(at) = limits.at {
            let _ = writeln!(o, "{}: {}\n", tr("Vzorek z", "Sample from"), iso(at));
        }
        let _ = writeln!(o, "| {} | {} | {} |\n|---|---:|---|", tr("Okno", "Window"), tr("Spotřeba", "Used"), tr("Reset", "Resets"));
        for (_, label, w) in limit_windows(limits) {
            if let Some(w) = w {
                let _ = writeln!(o, "| {label} | {} % | {} |", pct(w.pct), w.resets_at.map(iso).unwrap_or_else(|| "—".into()));
            }
        }
        o.push('\n');
    }
    if let Some(s) = &costs.summary {
        let _ = writeln!(o, "## {}\n", tr("Útrata (odhad)", "Usage (estimate)"));
        let _ = writeln!(
            o,
            "{}\n",
            tr(
                "Odhad v cenách API podle lokálních transcriptů Claude Code — není to fakturace; předplatné Pro/Max se po tokenech neúčtuje. Prázdná cena = model bez ceny v ceníku.",
                "An estimate at API prices from local Claude Code transcripts — not a bill; Pro/Max subscriptions are not charged per token. An empty price means the model has no price in the table."
            )
        );
        for r in &s.ranges {
            let _ = writeln!(o, "### {} {}\n", r.days, tr("d", "d"));
            let _ = writeln!(o, "{}: {} · {}: {} · USD: {}\n", tr("Tokeny", "Tokens"), r.total.tokens, tr("Odpovědi", "Requests"), r.requests, r.total.cost.map(|c| format!("{c:.2}")).unwrap_or_else(|| "—".into()));
            for (title, ls) in [(tr("Projekty", "Projects"), &r.projects), (tr("Modely", "Models"), &r.models)] {
                if ls.is_empty() {
                    continue;
                }
                let _ = writeln!(o, "| {title} | {} | USD |\n|---|---:|---:|", tr("Tokeny", "Tokens"));
                for l in ls {
                    let _ = writeln!(o, "| {} | {} | {} |", md_cell(&line_name(l)), l.tokens, l.cost.map(|c| format!("{c:.2}")).unwrap_or_else(|| "—".into()));
                }
                o.push('\n');
            }
        }
        let _ = writeln!(o, "### {}\n\n| {} | {} | USD |\n|---|---:|---:|", tr("Po dnech", "By day"), tr("Den", "Day"), tr("Tokeny", "Tokens"));
        for d in &s.days {
            let _ = writeln!(o, "| {} | {} | {} |", d.date, d.tokens, d.cost.map(|c| format!("{c:.2}")).unwrap_or_else(|| "—".into()));
        }
    }
    o
}

/// Buňka tabulky Markdownu: svislítko a zalomení pryč (cizí text z cest).
fn md_cell(s: &str) -> String {
    s.replace('|', "\\|").replace(['\n', '\r'], " ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::limits::{Source, Window};
    use crate::usage::view::{Day, Range, Split, Summary};

    fn line(name: &str, tokens: u64, cost: Option<f64>) -> Line {
        Line { name: name.into(), tokens, cost, unpriced: 0, other: false }
    }

    fn costs(summary: Option<Summary>) -> CostsView {
        CostsView { state: "ok", progress: None, computed_at: Some(1), error: None, window_days: 30, summary, prices_verified: "x", prices_source: "y" }
    }

    fn limits(five: Option<Window>, seven: Option<Window>) -> LimitsView {
        LimitsView { at: Some(1_790_000_000_000), source: Some(Source::StatusLine), five_hour: five, seven_day: seven, stale_ms: 1, status_line: "full", notice: None, pace: false, history5h: Vec::new() }
    }

    fn sample() -> (CostsView, LimitsView) {
        let range = Range {
            days: 7,
            split: Split::default(),
            total: line("", 3000, Some(1.5)),
            requests: 4,
            projects: vec![line("app", 2000, Some(1.0)), line("=evil, \"x\"", 1000, None)],
            models: vec![line("Model A", 3000, Some(1.5))],
        };
        let s = Summary { ranges: vec![range], days: vec![Day { date: "2026-10-22".into(), tokens: 10, cost: Some(0.25) }, Day { date: "2026-10-23".into(), tokens: 0, cost: None }], hours: vec![], floor: Default::default() };
        (costs(Some(s)), limits(Some(Window { pct: 42.5, resets_at: Some(1_790_003_600_000) }), None))
    }

    #[test]
    fn csv_cell_quotes_and_neutralises_formulas() {
        let cases = [("app", "app"), ("a,b", "\"a,b\""), ("say \"hi\"", "\"say \"\"hi\"\"\""), ("=SUM(A1)", "'=SUM(A1)"), ("+1", "'+1"), ("@x", "'@x"), ("-2", "'-2"), (" =HYPERLINK(\"http://example.com\",\"x\")", "\"' =HYPERLINK(\"\"http://example.com\"\",\"\"x\"\")\""), ("  +1", "'  +1"), ("\t=1", "'\t=1"), (" ok - fine", " ok - fine"), ("a=b", "a=b"), ("řádek\nnový", "\"řádek\nnový\""), ("", "")];
        for (input, want) in cases {
            assert_eq!(csv_cell(input), want, "{input:?}");
        }
    }

    #[test]
    fn csv_has_header_limits_ranges_days_and_blanks_for_unknown() {
        let (c, l) = sample();
        let out = render(Format::Csv, &c, &l, 0);
        let lines: Vec<&str> = out.trim_end().split("\r\n").collect();
        assert_eq!(lines[0], "\u{feff}kind,name,period,tokens,cost_usd_api_estimate,pct,resets_at");
        assert!(lines.contains(&format!("limit,five_hour,,,,42.5,{}", iso(1_790_003_600_000)).as_str()));
        assert!(lines.contains(&"total,,7d,3000,1.5000,,"));
        assert!(lines.contains(&"project,app,7d,2000,1.0000,,"));
        // neznámá cena = prázdná buňka, jméno s vzorcem a uvozovkami bezpečně
        assert!(lines.contains(&"project,\"'=evil, \"\"x\"\"\",7d,1000,,,"), "{lines:?}");
        assert!(lines.contains(&"day,2026-10-22,,10,0.2500,,"));
        assert!(lines.contains(&"day,2026-10-23,,0,,,"));
        // okno týdne nemá data → žádný řádek, nic vymyšleného
        assert!(!out.contains("seven_day"));
    }

    #[test]
    fn markdown_states_estimate_and_uses_dashes_for_unknown() {
        let (c, l) = sample();
        let out = with_lang_cs(|| render(Format::Markdown, &c, &l, 1_790_000_000_000));
        assert!(out.starts_with("# Ledge — útrata a limity Claude"));
        assert!(out.contains("není to fakturace"));
        assert!(out.contains("| 5h | 42.5 % |"));
        assert!(!out.contains("| 7d |"), "okno bez dat se nevypisuje");
        assert!(out.contains("| app | 2000 | 1.00 |"));
        assert!(out.contains("| =evil, \"x\" | 1000 | — |"));
        assert!(out.contains("| 2026-10-23 | 0 | — |"));
    }

    fn with_lang_cs<R>(f: impl FnOnce() -> R) -> R {
        crate::i18n::with_lang(crate::uiconf::Lang::Cs, f)
    }

    #[test]
    fn markdown_cell_is_safe() {
        assert_eq!(md_cell("a|b\nc"), "a\\|b c");
    }

    #[test]
    fn nothing_to_export_is_detected() {
        assert!(!has_data(&costs(None), &limits(None, None)));
        assert!(has_data(&costs(None), &limits(Some(Window { pct: 1.0, resets_at: None }), None)));
        assert!(has_data(&sample().0, &limits(None, None)));
    }

    #[test]
    fn format_parse() {
        assert_eq!(Format::parse("csv"), Some(Format::Csv));
        assert_eq!(Format::parse("md"), Some(Format::Markdown));
        assert_eq!(Format::parse("pdf"), None);
        assert_eq!((Format::Csv.ext(), Format::Markdown.ext()), ("csv", "md"));
    }
}
