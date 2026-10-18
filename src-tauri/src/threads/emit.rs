//! Vlákno `emit`: posílá stav do šuplíku (jen když je vidět, max ~8×/s), překresluje proužek,
//! hlídá termíny připomínek a git stav pro proužek. Spí na `Core.wake` (budí ho `mark_dirty`) a sám
//! se probudí jen v nejbližším termínu z `next_wake` — v klidu ~1×/min (hodiny), ne 1×/s.

use super::notes_watch::check_reminders;
use crate::drawer::{self, View};
use crate::refresh::{maybe_refresh_weather, refresh_git_async};
use crate::snapshot::snapshot;
use crate::state::{FLASH, core, now_ms};
use crate::stripview::refresh_strip;
use crate::weather;
use std::time::{Duration, Instant};
use tauri::Emitter;

/// Git stav pro proužek: co 3 minuty (8 projektů ≈ desítky ms).
const GIT_EVERY: Duration = Duration::from_secs(180);
/// Rezerva za začátkem minuty, aby hodiny v proužku už ukázaly novou minutu.
const MINUTE_MARGIN_MS: i64 = 50;

/// Za jak dlouho se má vlákno probudit samo (bez `mark_dirty`): začátek další minuty (hodiny
/// v proužku, 15min okno „hotovo“ u session) nebo nejbližší budoucí termín z `deadlines` (ms epochy:
/// připomínka, konec bliknutí, git, watchdog počasí), cokoli dřív. Termíny v minulosti se ignorují —
/// už je obsloužil průchod smyčkou, a kdyby ne, točila by se smyčka naprázdno.
pub(crate) fn next_wake(now_ms: i64, deadlines: impl IntoIterator<Item = Option<i64>>) -> Duration {
    let minute = now_ms - now_ms.rem_euclid(60_000) + 60_000 + MINUTE_MARGIN_MS;
    let at = deadlines.into_iter().flatten().filter(|&d| d > now_ms).fold(minute, i64::min);
    Duration::from_millis((at - now_ms) as u64)
}

/// Termíny pro `next_wake` ze stavu jádra (jen čtení paměti).
fn deadlines(last_git: Instant) -> [Option<i64>; 11] {
    let c = core();
    let now = now_ms();
    let in_ms = |left: Duration| now + left.as_millis() as i64 + 1;
    let reminder = c.notes.lock().next_due;
    let flash = c.flash.lock().and_then(|(_, t)| FLASH.checked_sub(t.elapsed())).map(in_ms);
    let git = Some(in_ms(GIT_EVERY.saturating_sub(last_git.elapsed())));
    let weather = {
        let w = c.weather.lock();
        // `weather::loading_live`: pokus bez odpovědi déle než 2× timeout → watchdog v `maybe_refresh_weather`
        w.attempt_ms.filter(|_| w.loading).map(|a| a + 2 * weather::TIMEOUT.as_millis() as i64 + 1)
    };
    // časovač: konec odpočtu a změny minuty v proužku jen, když běží (`timer::next_deadline`);
    // kalendář jen s viditelnou kartou (`calendar::live::next_deadline`); čas skladby v proužku po minutách
    // jen když hraje a proužek ho ukazuje (`media::next_deadline_live`); připomínka schůzky jen na čas
    // okna připomínky / začátku (`calendar::remind::next_deadline`, bez kalendáře s připomínkou nic)
    [reminder, flash, git, weather, crate::limitslive::next_deadline(), crate::timer::next_deadline_live(), crate::calendar::live::next_deadline(), crate::media::next_deadline_live(), crate::calendar::remind::next_deadline(), crate::accounts::calsync::next_deadline(), crate::accounts::mail::next_deadline()]
}

pub(crate) fn spawn_emit_thread() {
    std::thread::Builder::new()
        .name("emit".into())
        .spawn(|| {
            let mut last_emit = Instant::now() - Duration::from_secs(1);
            // git hned po startu (za 10 s), pak co `GIT_EVERY`
            let mut last_git = Instant::now() - GIT_EVERY + Duration::from_secs(10);
            loop {
                let c = core();
                let wait = next_wake(now_ms(), deadlines(last_git));
                {
                    let mut d = c.dirty.lock();
                    if !*d {
                        c.wake.wait_for(&mut d, wait);
                    }
                }
                check_reminders();
                crate::limitslive::tick(now_ms());
                crate::timer::tick_live(now_ms());
                crate::calendar::live::tick(now_ms());
                crate::calendar::remind::tick(now_ms());
                // počasí na Osobní: jen porovnání času, stahuje až backoff/30 min (žádný nový interval)
                maybe_refresh_weather(false);
                crate::calendar::live::maybe_refresh(false);
                // účty: kalendář jen s viditelnou kartou, pošta s kartou nebo položkou proužku `mail`
                crate::accounts::calsync::maybe_refresh(false);
                crate::accounts::mail::maybe_refresh(false);
                let since = last_emit.elapsed();
                if since < Duration::from_millis(120) {
                    std::thread::sleep(Duration::from_millis(120) - since);
                }
                let dirty = std::mem::replace(&mut *c.dirty.lock(), false);
                if dirty && c.drawer.view() != View::Hidden {
                    let _ = c.app.emit_to(drawer::LABEL, "state", snapshot());
                    last_emit = Instant::now();
                } else if dirty {
                    // šuplík zavřený → poslat až při otevření (UiCmd::Show posílá vždy čerstvý)
                }
                if last_git.elapsed() >= GIT_EVERY {
                    last_git = Instant::now();
                    refresh_git_async();
                }
                refresh_strip();
            }
        })
        .ok();
}

#[cfg(test)]
mod tests {
    use super::*;

    const T: i64 = 1_759_999_980_000; // celá minuta (T % 60 000 == 0)

    #[test]
    fn next_wake_picks_earliest_future_deadline() {
        let ms = |d: Duration| d.as_millis() as i64;
        let cases: [(&str, i64, Vec<Option<i64>>, i64); 7] = [
            ("nic → začátek minuty", T + 10_000, vec![], 50_050),
            ("přesně na minutě → celá další", T, vec![None], 60_050),
            ("připomínka dřív než minuta", T + 10_000, vec![Some(T + 12_000)], 2_000),
            ("připomínka až po minutě", T + 10_000, vec![Some(T + 90_000)], 50_050),
            ("minulý termín se ignoruje", T + 10_000, vec![Some(T + 9_000), Some(T + 10_000)], 50_050),
            ("nejbližší z více", T + 1, vec![Some(T + 4_001), Some(T + 3_001), None], 3_000),
            ("1 ms do termínu", T + 5, vec![Some(T + 6)], 1),
        ];
        for (name, now, d, want) in cases {
            assert_eq!(ms(next_wake(now, d)), want, "{name}");
        }
    }

    /// Hodina klidu (šuplík zavřený, žádné události): dřív 3 600 probuzení (1×/s), teď minuty + git.
    #[test]
    fn idle_hour_wakes_about_once_per_minute() {
        let mut now = T + 7_000;
        let mut next_git = now + 180_000;
        let mut wakes = 0;
        while now < T + 3_600_000 {
            now += next_wake(now, [Some(next_git)]).as_millis() as i64;
            if now >= next_git {
                next_git = now + 180_000;
            }
            wakes += 1;
        }
        assert!((60..=85).contains(&wakes), "{wakes} probuzení za hodinu");
    }
}
