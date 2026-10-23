//! Puls proužku u GitHubu: nové PR čekající na tvoji revizi a PR, jehož CI nově spadlo → krátké bliknutí
//! linky + puls proužku (`attention::signal`, tentýž mechanismus jako „Claude čeká“: nic ve fullscreen hře /
//! herním režimu, žádný fokus, bez toastu; `appearance.motion.style = none` jen bliknutí bez pohybu).
//!
//! Nic nepolluje: porovnává se jen při načtení GitHubu (`github::refresh`), které už dělá karta nebo plánovač
//! proužku. **První pozorování po startu / zapnutí jen založí paměť** (PR, která tu byla už před Ledge, nepulsuje),
//! stejně jako „hotovo“ u Clauda nehlásí session, kterou předchozí snímek neznal. Stejná událost nejvýš 1× za
//! `DEDUP_MS`. Vypíná `suggest.githubPulse`. Čisté rozhodování (`transitions`, `Dedup`, `decide`) má tabulkové testy.

use super::GithubRefs;
use crate::attention::{self, Event};
use parking_lot::Mutex;
use std::collections::{BTreeSet, HashMap};

/// Stejná událost (PR + druh) nejvýš jednou za tuto dobu — CI, které padá a zase naskakuje, nepulsuje pořád.
pub(crate) const DEDUP_MS: i64 = 30 * 60_000;

/// Co jsme u GitHubu viděli: klíče `vlastník/repo#číslo` PR k revizi a PR se spadlým CI.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct GhSeen {
    pub(crate) review: BTreeSet<String>,
    pub(crate) failing: BTreeSet<String>,
}

fn key(repo: &str, number: u64) -> String {
    format!("{repo}#{number}")
}

impl GhSeen {
    pub(crate) fn of(r: &GithubRefs) -> Self {
        Self { review: r.review.iter().map(|p| key(&p.repo, p.number)).collect(), failing: r.failing.iter().map(|p| key(&p.repo, p.number)).collect() }
    }
}

/// Nové události mezi dvěma pozorováními (seřazené, deterministicky): PR, které před tím nečekalo na revizi /
/// nemělo spadlé CI. `prev = None` (první pozorování) nic nehlásí.
pub(crate) fn transitions(prev: Option<&GhSeen>, now: &GhSeen) -> Vec<(String, Event)> {
    let Some(prev) = prev else { return Vec::new() };
    let ci = now.failing.difference(&prev.failing).map(|k| (k.clone(), Event::GithubCi));
    let review = now.review.difference(&prev.review).map(|k| (k.clone(), Event::GithubReview));
    ci.chain(review).collect()
}

/// Deduplikace: (PR, událost) nejvýš jednou za `DEDUP_MS`.
#[derive(Debug, Default)]
pub(crate) struct Dedup {
    last: HashMap<(String, Event), i64>,
}

impl Dedup {
    pub(crate) fn allow(&mut self, key: &str, ev: Event, now_ms: i64) -> bool {
        self.last.retain(|_, t| now_ms.saturating_sub(*t) < DEDUP_MS && *t <= now_ms);
        let k = (key.to_string(), ev);
        if self.last.contains_key(&k) {
            return false;
        }
        self.last.insert(k, now_ms);
        true
    }
}

/// Která událost se ukáže (nejdůležitější podle `Event::rank`), nebo žádná: puls vypnutý / nic nového / právě
/// probíhá schůzka (klid; událost se nezapamatuje k pozdějšímu pulsu, chip v návrzích zůstává).
pub(crate) fn decide(events: &[(String, Event)], pulse_on: bool, in_meeting: bool) -> Option<Event> {
    if !pulse_on || in_meeting {
        return None;
    }
    events.iter().map(|(_, e)| *e).min_by_key(|e| e.rank())
}

struct Watch {
    seen: Option<GhSeen>,
    dedup: Option<Dedup>,
}

static WATCH: Mutex<Watch> = Mutex::new(Watch { seen: None, dedup: None });

/// GitHub se právě načetl: porovnat s minulým pozorováním a případně pulsovat. Bez dat v paměti (chyba sítě,
/// `gh` odhlášené) se paměť pozorování nemění — po návratu dat se porovnává s posledním, co jsme viděli.
pub(super) fn on_refreshed() {
    let c = crate::state::core();
    let now_ms = crate::state::now_ms();
    let (pulse_on, gh, in_meeting) = {
        let g = c.cfg.read();
        (g.suggest.github_pulse, g.github.clone(), crate::calendar::live::meeting_now(&g, now_ms))
    };
    let Some((review, mine, at)) = crate::github::cached_prs(&gh) else { return };
    let now = GhSeen::of(&super::refs(&review, &mine, at));
    let events: Vec<(String, Event)> = {
        let mut w = WATCH.lock();
        let evs = transitions(w.seen.as_ref(), &now);
        w.seen = Some(now);
        let dedup = w.dedup.get_or_insert_with(Dedup::default);
        evs.into_iter().filter(|(k, e)| dedup.allow(k, *e, now_ms)).collect()
    };
    if let Some(ev) = decide(&events, pulse_on, in_meeting) {
        // bez toastu (druhý parametr): GitHub nemá vlastní oznámení Windows
        attention::signal(ev.color(), false);
        crate::state::mark_dirty();
    }
}

/// GitHub se vypnul: zapomenout pozorování (po novém zapnutí se nejdřív jen načte).
pub(super) fn reset() {
    let mut w = WATCH.lock();
    w.seen = None;
    w.dedup = None;
}

#[cfg(test)]
mod tests {
    use super::*;

    fn seen(review: &[&str], failing: &[&str]) -> GhSeen {
        GhSeen { review: review.iter().map(|s| s.to_string()).collect(), failing: failing.iter().map(|s| s.to_string()).collect() }
    }

    #[test]
    fn transitions_table() {
        let a = seen(&["a/b#1"], &["a/b#9"]);
        // (název, předchozí, teď, očekávané události)
        type Case<'a> = (&'a str, Option<GhSeen>, GhSeen, Vec<(&'a str, Event)>);
        let cases: Vec<Case> = vec![
            ("první pozorování jen založí paměť", None, seen(&["a/b#1", "a/b#2"], &["a/b#9"]), vec![]),
            ("beze změny", Some(a.clone()), a.clone(), vec![]),
            ("nové PR k revizi", Some(a.clone()), seen(&["a/b#1", "a/b#2"], &["a/b#9"]), vec![("a/b#2", Event::GithubReview)]),
            ("nově spadlé CI", Some(a.clone()), seen(&["a/b#1"], &["a/b#9", "a/b#7"]), vec![("a/b#7", Event::GithubCi)]),
            ("obojí naráz, CI první", Some(a.clone()), seen(&["a/b#1", "a/b#2"], &["a/b#9", "a/b#7"]), vec![("a/b#7", Event::GithubCi), ("a/b#2", Event::GithubReview)]),
            ("PR zmizelo (hotovo / sloučeno)", Some(a.clone()), seen(&[], &[]), vec![]),
            ("CI opraveno a zase spadlo = nová událost", Some(seen(&[], &[])), seen(&[], &["a/b#9"]), vec![("a/b#9", Event::GithubCi)]),
            ("po nule se vše hlásí (paměť byla prázdná, ne neznámá)", Some(seen(&[], &[])), seen(&["x/y#1"], &[]), vec![("x/y#1", Event::GithubReview)]),
        ];
        for (name, prev, now, want) in cases {
            let got = transitions(prev.as_ref(), &now);
            let want: Vec<(String, Event)> = want.into_iter().map(|(k, e)| (k.to_string(), e)).collect();
            assert_eq!(got, want, "{name}");
        }
    }

    #[test]
    fn dedup_window() {
        let mut d = Dedup::default();
        assert!(d.allow("a/b#1", Event::GithubCi, 1_000));
        assert!(!d.allow("a/b#1", Event::GithubCi, 1_000 + DEDUP_MS - 1), "uvnitř okna");
        assert!(d.allow("a/b#1", Event::GithubReview, 2_000), "jiný druh události");
        assert!(d.allow("a/b#2", Event::GithubCi, 2_000), "jiné PR");
        assert!(d.allow("a/b#1", Event::GithubCi, 1_000 + DEDUP_MS), "po okně znovu");
        // hodiny šly zpět: starý záznam z budoucnosti nesmí blokovat navždy
        let mut d = Dedup::default();
        assert!(d.allow("a/b#1", Event::GithubCi, 10_000_000));
        assert!(d.allow("a/b#1", Event::GithubCi, 5));
    }

    #[test]
    fn decide_table() {
        let ci = ("a/b#1".to_string(), Event::GithubCi);
        let rev = ("a/b#2".to_string(), Event::GithubReview);
        type Case<'a> = (&'a str, Vec<(String, Event)>, bool, bool, Option<Event>);
        let cases: Vec<Case> = vec![
            ("nic nového", vec![], true, false, None),
            ("vypnuto", vec![ci.clone()], false, false, None),
            ("jen CI", vec![ci.clone()], true, false, Some(Event::GithubCi)),
            ("jen revize", vec![rev.clone()], true, false, Some(Event::GithubReview)),
            ("CI má přednost před revizí", vec![rev.clone(), ci.clone()], true, false, Some(Event::GithubCi)),
            ("během schůzky klid (CI)", vec![ci.clone()], true, true, None),
            ("během schůzky klid (revize)", vec![rev.clone()], true, true, None),
            ("během schůzky bez nové události nic", vec![], true, true, None),
        ];
        for (name, evs, on, meeting, want) in cases {
            assert_eq!(decide(&evs, on, meeting), want, "{name}");
        }
    }

    #[test]
    fn seen_from_refs_ignores_drafts_and_unsafe_urls() {
        use crate::github::parse::{Ci, Pr};
        let pr = |n: u64, ci: Ci, draft: bool, url: &str| Pr { repo: "a/b".into(), number: n, title: String::new(), url: url.into(), author: String::new(), draft, updated: String::new(), ci, decision: String::new() };
        let u = "https://github.com/a/b/pull/1";
        let refs = super::super::refs(&[pr(1, Ci::None, false, u), pr(2, Ci::None, true, u), pr(3, Ci::None, false, "")], &[pr(4, Ci::Failure, false, u), pr(5, Ci::Success, false, u)], None);
        assert_eq!(GhSeen::of(&refs), seen(&["a/b#1"], &["a/b#4"]));
    }
}
