//! Kontextové návrhy: pruh nahoře v šuplíku (max 3 chipy, `src/suggest/`) — VÝHRADNĚ z reálných událostí,
//! nic se neukáže bez zdroje. Výběr dělá čistá funkce `pick` v UI nad snapshotem; Rust tu drží jen
//! (1) config `suggest` (zapnuté zdroje) a (2) paměť GitHubu, kterou UI jinak nevidí (`GithubRefs`:
//! PR čekající na tvoji revizi a tvoje PR se spadlým CI — čtení cache `github::cached_prs`, nikdy dotaz na
//! síť). Žádné vlastní vlákno ani časovač: snapshot se přepočítá při změně (`on_github_refreshed` →
//! `mark_dirty` po každém načtení GitHubu, které už dělá karta / plánovač proužku).
//!
//! Zdroje a co je spouští:
//! - schránka: UI čte `clip_list` (historie schránky musí být zapnutá) při události `cliphist` — žádný polling,
//! - dev server: `Snapshot.procs` (běží z Ledge, má port), jen krátce po naběhnutí,
//! - GitHub: tady, jen když je integrace zapnutá a má čerstvá data,
//! - připomínky: `Snapshot.reminders` (po termínu).

pub mod conf;
pub mod pulse;

pub use conf::SuggestConfig;

use crate::github::parse::{Ci, Pr};
use serde::Serialize;

/// Odkaz na PR pro chip (jen adresy z `gh` na `https://github.com/`, jiné se nepouští).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PrRef {
    pub repo: String,
    pub number: u64,
    pub title: String,
    pub url: String,
}

/// PR, ke kterým se nabízí akce (z paměti GitHubu).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GithubRefs {
    /// čekají na tvoji revizi (bez konceptů)
    pub review: Vec<PrRef>,
    /// tvoje otevřené PR se spadlým CI (bez konceptů)
    pub failing: Vec<PrRef>,
    /// kdy se data naposled úspěšně načetla (UI starší než hodinu nenabídne)
    pub fetched_ms: Option<i64>,
}

/// Snapshot `suggest`: zapnuté zdroje + GitHub.
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SuggestView {
    pub on: SuggestConfig,
    pub github: GithubRefs,
}

fn pr_ref(p: &Pr) -> Option<PrRef> {
    (!p.draft && !p.url.is_empty()).then(|| PrRef { repo: p.repo.clone(), number: p.number, title: p.title.clone(), url: p.url.clone() })
}

/// Z PR k revizi a vlastních PR: co čeká na revizi a co má spadlé CI. Koncepty a PR bez bezpečné adresy
/// (`Pr.url` prázdná = cizí adresa) se vynechají. Čistá funkce.
pub(crate) fn refs(review: &[Pr], mine: &[Pr], fetched_ms: Option<i64>) -> GithubRefs {
    GithubRefs {
        review: review.iter().filter_map(pr_ref).collect(),
        failing: mine.iter().filter(|p| p.ci == Ci::Failure).filter_map(pr_ref).collect(),
        fetched_ms,
    }
}

/// Pohled pro snapshot: GitHub jen se zapnutým zdrojem a zapnutou integrací s daty ve stavu `Ok`.
pub(crate) fn view(cfg: &crate::config::Config) -> SuggestView {
    let github = if cfg.suggest.github {
        crate::github::cached_prs(&cfg.github).map(|(r, m, at)| refs(&r, &m, at)).unwrap_or_default()
    } else {
        GithubRefs::default()
    };
    SuggestView { on: cfg.suggest.clone(), github }
}

/// GitHub se právě načetl (volá `github::refresh` po uvolnění zámku): snapshot musí znát nová data.
pub fn on_github_refreshed() {
    // nejdřív puls (nastaví `attention::signal`), pak překreslení — proužek si puls vyzvedne v `refresh_strip`
    pulse::on_refreshed();
    let (ours, enabled) = {
        let c = crate::state::core().cfg.read();
        (c.suggest.github, c.github.any())
    };
    if ours && enabled {
        crate::state::mark_dirty();
    }
}

/// GitHub se vypnul: pozorování PR se zahodí (po novém zapnutí se nejdřív jen načte, bez pulsu).
pub fn reset_github() {
    pulse::reset();
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pr(repo: &str, n: u64, ci: Ci, draft: bool, url: &str) -> Pr {
        Pr { repo: repo.into(), number: n, title: format!("t{n}"), url: url.into(), author: String::new(), draft, updated: String::new(), ci, decision: String::new() }
    }
    const U: &str = "https://github.com/a/b/pull/1";

    #[test]
    fn refs_table() {
        // (název, k revizi, moje, očekávaná čísla review, očekávaná čísla failing)
        type Case<'a> = (&'a str, Vec<Pr>, Vec<Pr>, Vec<u64>, Vec<u64>);
        let cases: Vec<Case> = vec![
            ("nic", vec![], vec![], vec![], vec![]),
            ("k revizi", vec![pr("a/b", 1, Ci::None, false, U), pr("a/b", 2, Ci::None, false, U)], vec![], vec![1, 2], vec![]),
            ("koncept se nenabízí", vec![pr("a/b", 1, Ci::None, true, U), pr("a/b", 2, Ci::None, false, U)], vec![], vec![2], vec![]),
            ("cizí adresa se nepustí", vec![pr("a/b", 1, Ci::None, false, "")], vec![pr("a/b", 3, Ci::Failure, false, "")], vec![], vec![]),
            ("jen spadlé CI", vec![], vec![pr("a/b", 3, Ci::Failure, false, U), pr("a/b", 4, Ci::Success, false, U), pr("a/b", 5, Ci::Pending, false, U), pr("a/b", 6, Ci::None, false, U)], vec![], vec![3]),
            ("koncept se spadlým CI ne", vec![], vec![pr("a/b", 3, Ci::Failure, true, U)], vec![], vec![]),
        ];
        for (name, review, mine, want_r, want_f) in cases {
            let r = refs(&review, &mine, Some(5));
            assert_eq!(r.review.iter().map(|p| p.number).collect::<Vec<_>>(), want_r, "{name}: review");
            assert_eq!(r.failing.iter().map(|p| p.number).collect::<Vec<_>>(), want_f, "{name}: failing");
            assert_eq!(r.fetched_ms, Some(5));
        }
    }

    #[test]
    fn view_is_empty_when_github_is_off() {
        let cfg = crate::config::Config::default();
        let v = view(&cfg);
        assert_eq!(v.github, GithubRefs::default(), "výchozí GitHub je vypnutý");
        assert!(v.on.dev_server && v.on.github && v.on.reminders && !v.on.clipboard, "výchozí zdroje");
    }
}
