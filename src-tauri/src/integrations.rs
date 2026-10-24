//! Plánovač položek proužku `github` a `system` (karty GitHub a Systém mají vlastní obnovu, dokud jsou
//! vidět — viz `github/`, `sysmon/`). Se zavřeným šuplíkem se nic nepočítá, KROMĚ zapnuté položky proužku:
//! `system` = vzorek 1× za 10 s, `github` = dotaz `gh` podle `github.refreshMin` (aspoň 2 min).
//! Vlákno vzniká jen, když je nějaká položka zapnutá, spí na podmínce do nejbližšího termínu a po vypnutí
//! posledního zájmu samo skončí — v klidu (nic zapnuto) neběží žádné. Ve fullscreen hře (Ledge „mrtvý“)
//! se nevzorkuje ani nevolá `gh`.

use crate::config::{Config, StripItem};
use crate::github::{self, GithubConfig, Mode};
use crate::state::{core, mark_dirty, now_ms};
use crate::sysmon::{self, SystemConfig};
use parking_lot::{Condvar, Mutex};
use std::time::Duration;

/// Odstup vzorků karty Systém v proužku (karta sama vzorkuje po 2 s).
pub const SYSTEM_EVERY_MS: i64 = 10_000;
/// Jak dlouho vlákno čeká, když je Ledge „mrtvý“ (fullscreen hra), než se znovu podívá.
const DEAD_WAIT_MS: u64 = 10_000;

/// Které položky proužku teď něco vyžadují.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Want {
    pub github: bool,
    pub system: bool,
}

impl Want {
    pub fn any(self) -> bool {
        self.github || self.system
    }
}

/// Co si proužek žádá: položka v `strip.items` při zapnutém proužku; `github` navíc jen se zapnutým
/// GitHubem a sekcí „čeká na revizi“ (jinak by nebylo co ukázat). Systém chce i styl zavřeného Ledge, který
/// CPU a paměť ukazuje (`looks::Look::shows_system`, kapsle „Trojice“), pokud ho uživatel v Pokročilých neskryl.
pub fn wants(cfg: &Config) -> Want {
    let on = |i: StripItem| cfg.strip.enabled && cfg.strip.items.contains(&i);
    let by_look = cfg.strip.enabled && cfg.strip.look.shows_system() && !cfg.strip.look_hide.contains(&StripItem::System);
    Want { github: on(StripItem::Github) && cfg.github.enabled && cfg.github.review, system: on(StripItem::System) || by_look }
}

/// Co plánovač udělá teď a za jak dlouho se má znovu podívat.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Plan {
    pub github: bool,
    pub system: bool,
    /// nejbližší budoucí termín (ms od teď); `None` = žádný
    pub wait_ms: Option<u64>,
}

/// Čistá funkce: `*_due` = kdy je položka na řadě (`None` = nezapnutá). Splatné (≤ teď) se spustí,
/// čekání je do nejbližšího budoucího termínu.
pub fn plan(now: i64, github_due: Option<i64>, system_due: Option<i64>) -> Plan {
    let due = |d: Option<i64>| d.is_some_and(|d| d <= now);
    let wait_ms = [github_due, system_due].into_iter().flatten().filter(|d| *d > now).map(|d| (d - now) as u64).min();
    Plan { github: due(github_due), system: due(system_due), wait_ms }
}

struct Sched {
    running: bool,
}

static SCHED: Mutex<Sched> = Mutex::new(Sched { running: false });
static WAKE: Condvar = Condvar::new();

/// Po startu a po každé změně configu proužku / GitHubu / Systému: spustí vlákno, když je co plánovat,
/// a probudí ho, ať přepočítá termíny. Levné (zámek + notify); vlákno samo skončí, když už nic neplánuje.
pub fn sync(cfg: &Config) {
    let mut s = SCHED.lock();
    if wants(cfg).any() && !s.running {
        s.running = true;
        let spawned = std::thread::Builder::new().name("integrations".into()).spawn(worker);
        if spawned.is_err() {
            s.running = false;
        }
    }
    WAKE.notify_all();
}

fn worker() {
    loop {
        let mut s = SCHED.lock();
        // config se čte POD zámkem plánovače: `sync` zapíše config dřív, než zámek vezme, takže
        // vlákno mezi čtením a čekáním nepřehlédne změnu
        let (w, gh, sys): (Want, GithubConfig, SystemConfig) = {
            let c = core().cfg.read();
            (wants(&c), c.github.clone(), c.system.clone())
        };
        if !w.any() {
            s.running = false;
            return;
        }
        if core().dead() {
            WAKE.wait_for(&mut s, Duration::from_millis(DEAD_WAIT_MS));
            continue;
        }
        let now = now_ms();
        let p = plan(now, if w.github { github::next_due(&gh, now) } else { None }, w.system.then(|| sysmon::next_strip_due(now, SYSTEM_EVERY_MS)));
        if !(p.github || p.system) {
            // `None` jen když zapnutá položka nemá termín (GitHub se mezitím vypnul) — krátce počkat a přepočítat
            WAKE.wait_for(&mut s, Duration::from_millis(p.wait_ms.unwrap_or(DEAD_WAIT_MS)));
            continue;
        }
        drop(s);
        if p.system {
            let _ = sysmon::poll(now, false, &sys);
        }
        if p.github {
            let _ = github::refresh(&gh, Mode::Auto);
        }
        mark_dirty();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plan_table() {
        let cases = [
            ("nic zapnuto", 1000, None, None, (false, false, None)),
            ("jen systém, ještě ne", 1000, None, Some(6000), (false, false, Some(5000))),
            ("jen systém, splatný", 6000, None, Some(6000), (false, true, None)),
            ("jen GitHub za 3 minuty", 0, Some(180_000), None, (false, false, Some(180_000))),
            ("GitHub splatný, systém za 4 s → po běhu čeká", 10_000, Some(10_000), Some(14_000), (true, false, Some(4000))),
            ("oba splatné", 50, Some(10), Some(50), (true, true, None)),
            ("termín v minulosti = hned", 5000, Some(1000), None, (true, false, None)),
            ("nejbližší z budoucích", 100, Some(900), Some(400), (false, false, Some(300))),
        ];
        for (name, now, g, s, (rg, rs, wait)) in cases {
            let p = plan(now, g, s);
            assert_eq!((p.github, p.system, p.wait_ms), (rg, rs, wait), "{name}");
        }
    }

    #[test]
    fn wants_follow_strip_items_and_switches() {
        let mut c = Config::default();
        assert_eq!(wants(&c), Want::default(), "výchozí config nic neplánuje");
        c.strip.items.push(StripItem::System);
        assert_eq!(wants(&c), Want { github: false, system: true });
        c.strip.items.push(StripItem::Github);
        assert!(!wants(&c).github, "položka bez zapnutého GitHubu nic nečte");
        c.github.enabled = true;
        assert!(wants(&c).github);
        c.github.review = false;
        assert!(!wants(&c).github, "bez sekce „na revizi“ není co ukázat");
        c.github.review = true;
        c.strip.enabled = false;
        assert_eq!(wants(&c), Want::default(), "vypnutý proužek nic nepotřebuje");
        // kapsle Trojice (kolo 22) ukazuje CPU a paměť → vzorek jako položka „Systém“; skrytá v Pokročilých = nic
        let mut c = Config::default();
        c.strip.look = crate::looks::Look::Trio;
        assert_eq!(wants(&c), Want { github: false, system: true });
        c.strip.look_hide = vec![StripItem::System];
        assert_eq!(wants(&c), Want::default());
        c.strip.look = crate::looks::Look::Minimum;
        c.strip.look_hide.clear();
        assert_eq!(wants(&c), Want::default(), "minimum systém neukazuje");
    }

    #[test]
    fn idle_hour_with_system_item_is_360_samples() {
        // 1× za 10 s = 360 vzorků za hodinu (ne víc), bez položky 0 (vlákno neexistuje)
        let (mut now, mut n) = (0i64, 0);
        let mut due = Some(0i64);
        while now < 3_600_000 {
            let p = plan(now, None, due);
            if p.system {
                n += 1;
                due = Some(now + SYSTEM_EVERY_MS);
            } else {
                now += p.wait_ms.unwrap() as i64;
            }
        }
        assert_eq!(n, 360);
    }
}
