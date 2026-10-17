//! Index → přehled pro kartu: dnes / 7 dní / okno, top projekty, modely, sloupce po dnech.
//! Čisté: projekt podle `cwd` dodá volající (`project_of`). Cena jen z ceníku (`price::cost`);
//! tokeny modelů bez ceny se sčítají zvlášť (`unpriced`) a cena se za ně nevymýšlí.

use super::index::Index;
use super::price;
use serde::Serialize;
use std::collections::HashMap;

/// Kolik projektů / modelů ukázat; zbytek se sečte do řádku `other`.
pub const TOP: usize = 5;

#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Split {
    pub input: u64,
    pub output: u64,
    pub cache_write: u64,
    pub cache_read: u64,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Line {
    pub name: String,
    pub tokens: u64,
    /// `None` = žádný token řádku nemá cenu v ceníku
    pub cost: Option<f64>,
    /// tokeny bez ceny (neznámý model / pásmo)
    pub unpriced: u64,
    /// součet zbytku za `TOP`
    pub other: bool,
}

impl Line {
    fn add(&mut self, tokens: u64, cost: Option<f64>) {
        self.tokens += tokens;
        match cost {
            Some(c) => self.cost = Some(self.cost.unwrap_or(0.0) + c),
            None => self.unpriced += tokens,
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Range {
    pub days: u32,
    pub split: Split,
    pub total: Line,
    pub requests: u64,
    pub projects: Vec<Line>,
    pub models: Vec<Line>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Day {
    /// `YYYY-MM-DD` (místní den)
    pub date: String,
    pub tokens: u64,
    pub cost: Option<f64>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Summary {
    /// dnes, 7 dní, celé okno
    pub ranges: Vec<Range>,
    /// sloupce po dnech od nejstaršího po dnešek (dny bez odpovědí = 0)
    pub days: Vec<Day>,
}

/// Den (dny od 1970-01-01) → `YYYY-MM-DD`.
pub fn date_of(day: i32) -> String {
    chrono::NaiveDate::from_num_days_from_ce_opt(day + 719_163).map(|d| d.format("%Y-%m-%d").to_string()).unwrap_or_default()
}

fn top(map: HashMap<String, Line>) -> Vec<Line> {
    let mut v: Vec<Line> = map.into_values().collect();
    v.sort_by(|a, b| b.cost.unwrap_or(0.0).total_cmp(&a.cost.unwrap_or(0.0)).then(b.tokens.cmp(&a.tokens)).then(a.name.cmp(&b.name)));
    if v.len() > TOP + 1 {
        let rest = v.split_off(TOP);
        let mut o = Line { other: true, ..Line::default() };
        for l in rest {
            o.tokens += l.tokens;
            o.unpriced += l.unpriced;
            if let Some(c) = l.cost {
                o.cost = Some(o.cost.unwrap_or(0.0) + c);
            }
        }
        v.push(o);
    }
    v
}

/// `today` = dnešní místní den, `window` = počet dní okna (≥ 7).
pub fn build(ix: &Index, today: i32, window: u32, project_of: &dyn Fn(&str) -> String) -> Summary {
    let mut names: HashMap<&str, String> = HashMap::new();
    let mut ranges = Vec::new();
    for days in [1, 7.min(window), window] {
        let from = today - days as i32 + 1;
        let mut r = Range { days, ..Range::default() };
        let mut projects: HashMap<String, Line> = HashMap::new();
        let mut models: HashMap<String, Line> = HashMap::new();
        for (k, s) in ix.agg.iter().filter(|(k, _)| k.day >= from && k.day <= today) {
            let tokens = s.t.total();
            let cost = price::cost(&k.model, k.tier, &s.t);
            r.split.input += s.t.input;
            r.split.output += s.t.output;
            r.split.cache_write += s.t.cw5 + s.t.cw1;
            r.split.cache_read += s.t.cr;
            r.requests += s.n;
            r.total.add(tokens, cost);
            let name = names.entry(k.cwd.as_str()).or_insert_with(|| project_of(&k.cwd)).clone();
            projects.entry(name.clone()).or_insert_with(|| Line { name, ..Line::default() }).add(tokens, cost);
            let m = price::display(&k.model);
            models.entry(m.clone()).or_insert_with(|| Line { name: m, ..Line::default() }).add(tokens, cost);
        }
        r.projects = top(projects);
        r.models = top(models);
        ranges.push(r);
    }
    let from = today - window as i32 + 1;
    let mut per_day: HashMap<i32, Line> = HashMap::new();
    for (k, s) in ix.agg.iter().filter(|(k, _)| k.day >= from && k.day <= today) {
        per_day.entry(k.day).or_default().add(s.t.total(), price::cost(&k.model, k.tier, &s.t));
    }
    let days = (from..=today)
        .map(|d| {
            let l = per_day.remove(&d).unwrap_or_default();
            Day { date: date_of(d), tokens: l.tokens, cost: l.cost }
        })
        .collect();
    Summary { ranges, days }
}

#[cfg(test)]
mod tests {
    use super::super::index::{Key, Sum};
    use super::super::parse::Tokens;
    use super::*;

    const M: u64 = 1_000_000;

    fn ix(rows: &[(i32, &str, &str, u8, Tokens)]) -> Index {
        let mut ix = Index::default();
        for (day, cwd, model, tier, t) in rows {
            let s = ix.agg.entry(Key { day: *day, cwd: (*cwd).into(), model: (*model).into(), tier: *tier }).or_insert(Sum::default());
            s.t.add(t);
            s.n += 1;
        }
        ix
    }

    fn out(n: u64) -> Tokens {
        Tokens { output: n, ..Tokens::default() }
    }

    fn proj(cwd: &str) -> String {
        if cwd.starts_with("C:\\ledge") { "Ledge".into() } else { cwd.rsplit('\\').next().unwrap_or("").into() }
    }

    #[test]
    fn ranges_projects_models_days() {
        let t = 20_000;
        let ix = ix(&[
            (t, "C:\\ledge", "claude-opus-5-5", 0, out(M)),           // 20 $
            (t, "C:\\ledge\\wt", "claude-opus-5-5", 0, out(M)),       // stejný projekt
            (t - 1, "C:\\x\\ws", "claude-sonnet-5-5", 0, out(M)),     // 10 $
            (t - 6, "C:\\x\\ws", "neznamy-model", 0, out(5)),
            (t - 7, "C:\\x\\ws", "claude-opus-5-5", 0, out(M)),       // jen v 30 dnech
            (t - 30, "C:\\x\\ws", "claude-opus-5-5", 0, out(M)),      // mimo okno
        ]);
        let s = build(&ix, t, 30, &proj);
        assert_eq!(s.ranges.iter().map(|r| r.days).collect::<Vec<_>>(), [1, 7, 30]);
        let [d1, d7, d30] = [&s.ranges[0], &s.ranges[1], &s.ranges[2]];
        assert_eq!(d1.total.cost, Some(40.0));
        assert_eq!(d1.projects, vec![Line { name: "Ledge".into(), tokens: 2 * M, cost: Some(40.0), ..Line::default() }]);
        assert_eq!(d7.total.tokens, 3 * M + 5);
        assert_eq!(d7.total.cost, Some(50.0));
        assert_eq!(d7.total.unpriced, 5, "neznámý model: tokeny ano, cena ne");
        let unk = d7.models.iter().find(|l| l.name == "neznamy-model").unwrap();
        assert_eq!((unk.cost, unk.unpriced), (None, 5));
        assert_eq!(d7.models[0].name, "Opus 5.5");
        assert_eq!(d30.total.cost, Some(70.0));
        assert_eq!(d30.requests, 5);
        assert_eq!(s.days.len(), 30);
        assert_eq!(s.days.last().unwrap().cost, Some(40.0));
        assert_eq!(s.days[0].tokens, 0, "den bez odpovědí = 0, ne vynechaný");
        assert_eq!(s.days[0].cost, None);
        assert_eq!(s.days.last().unwrap().date, date_of(t));
    }

    #[test]
    fn top_merges_rest_into_other() {
        let rows: Vec<(i32, String)> = (0..8).map(|i| (0, format!("C:\\p{i}"))).collect();
        let mut ix = Index::default();
        for (i, (d, cwd)) in rows.iter().enumerate() {
            ix.agg.insert(Key { day: *d, cwd: cwd.clone(), model: "claude-opus-5-5".into(), tier: 0 }, Sum { t: out((i as u64 + 1) * M), n: 1 });
        }
        let s = build(&ix, 0, 30, &proj);
        let p = &s.ranges[0].projects;
        assert_eq!(p.len(), TOP + 1);
        assert_eq!(p[0].name, "p7");
        assert!(p[TOP].other);
        assert_eq!(p[TOP].tokens, (1 + 2 + 3) * M);
    }

    #[test]
    fn split_and_dates() {
        let t = Tokens { input: 1, output: 2, cw5: 3, cw1: 4, cr: 5 };
        let s = build(&ix(&[(0, "C:\\a", "claude-opus-5-5", 0, t)]), 0, 30, &proj);
        assert_eq!(s.ranges[0].split, Split { input: 1, output: 2, cache_write: 7, cache_read: 5 });
        let sp = s.ranges[0].split;
        assert_eq!(sp.input + sp.output + sp.cache_write + sp.cache_read, t.total());
        assert_eq!(date_of(0), "1970-01-01");
        assert_eq!(date_of(20_743), "2026-10-17");
    }
}
