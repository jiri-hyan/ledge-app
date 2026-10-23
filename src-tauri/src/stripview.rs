//! Obsah nativního proužku (`refresh_strip`): segmenty podle `strip.items` (+ časovač, když běží;
//! právě hraje z paměti `media.rs`),
//! „v klidu skryté“ položky a barvy (`stripconf::compose`), ambientní barva,
//! viditelnost (fullscreen, otevřený šuplík, lištové okno), vzhled a osa z `appearance`. Kreslí `strip.rs`.

use crate::appearance::geom;
use crate::config::{self, Config};
use crate::i18n::tr;
use crate::drawer::View;
use crate::state::{FLASH, core};
use crate::strip::{Rgba, Segment};
use crate::{attention, claude, procman, stripconf};
use std::sync::atomic::Ordering;

// ---------- proužek ----------

pub(crate) fn seg(text: impl Into<String>, color: Rgba, dot: Option<Rgba>, strong: bool, pill: bool) -> Segment {
    Segment { text: text.into(), color, dot, strong, pill }
}

pub(crate) fn refresh_strip() {
    let c = core();
    // monitor dřív než zámek configu: `monitor()` čte config sám (rekurzivní read lock parking_lot
    // se zasekne, když mezitím čeká zápis z `apply_config`)
    let mon = c.monitor();
    let cfg = c.cfg.read();
    let pending = c.claude.pending();
    let asks = c.claude.questions();
    let procs = c.procs.snapshot();
    let failed = procs.iter().any(|p| p.phase == procman::Phase::Failed);
    let flash = c.flash.lock().and_then(|(col, t)| (t.elapsed() < FLASH).then_some(col));
    // připomínky po termínu (titulky) — ambientní událost jako čekající schválení
    let (due_first, due_count) = {
        let st = c.notes.lock();
        (st.due.first().map(|n| n.title.clone()), st.due.len())
    };
    // schůzka za chvíli (`calendar::remind`): priorita mezi „čeká na tebe“ a pádem / bliknutím „hotovo“
    let meeting = crate::calendar::remind::strip_now(&cfg, crate::state::now_ms());
    let ambient = if !pending.is_empty() || !asks.is_empty() || due_count > 0 {
        Some(Rgba::AMBER)
    } else if meeting.is_some() {
        Some(Rgba::BLUE)
    } else if failed {
        Some(Rgba::RED)
    } else {
        flash
    };
    let name_of = |id: &str| cfg.projects.iter().find(|p| p.id == id).map(|p| p.name.clone()).unwrap_or_else(|| id.to_string());
    let mode_title = crate::layout::page(&cfg.layout, &c.mode.lock()).map(crate::layout::title).unwrap_or_default();
    let timer = crate::timer::state();
    let now_ms = crate::state::now_ms();
    // vzhled „Jen v proužku“ doplní položku `media` (`mediaconf::with_strip_item`)
    let items = crate::mediaconf::with_strip_item(stripconf::effective_items(&cfg.strip.items, crate::timer::active(&timer), cfg.timer.strip), &cfg.media);
    // segmenty po položkách; „v klidu skryté“ a barvy řeší čisté `stripconf::compose`
    let mut parts: Vec<(config::StripItem, Vec<Segment>)> = Vec::with_capacity(items.len());
    for item in &items {
        let mut segs = Vec::new();
        match item {
            config::StripItem::Mode => {
                segs.push(seg(mode_title.clone(), Rgba::TEXT, None, true, false));
            }
            config::StripItem::Clock => segs.push(seg(local_clock(), Rgba::DIM, None, false, false)),
            config::StripItem::Running => {
                let shown: Vec<_> = procs.iter().filter(|p| p.phase != procman::Phase::Exited).collect();
                for p in shown.iter().take(3) {
                    let (dot, text) = match p.phase {
                        procman::Phase::Failed => (Rgba::RED, trf!("{} pád", "{} crashed", name_of(&p.id))),
                        procman::Phase::Starting | procman::Phase::Preparing => (Rgba::AMBER, trf!("{} startuje", "{} starting", name_of(&p.id))),
                        _ => (Rgba::GREEN, match p.port {
                            Some(port) if p.url.is_some() => format!("{} :{port}", name_of(&p.id)),
                            _ => name_of(&p.id),
                        }),
                    };
                    segs.push(seg(text, Rgba::TEXT, Some(dot), false, true));
                }
                if shown.len() > 3 {
                    segs.push(seg(format!("+{}", shown.len() - 3), Rgba::DIM, None, false, false));
                }
            }
            config::StripItem::Claude => {
                let sessions = c.claude.sessions();
                // schválení má přednost, pak otázka Clauda (AskUserQuestion)
                let first = pending
                    .first()
                    .map(|p| (p.cwd.as_str(), tr("čeká", "waiting")))
                    .or_else(|| asks.first().map(|a| (a.cwd.as_str(), tr("se ptá", "asks"))));
                if let Some((cwd, verb)) = first {
                    let where_ = project_for_cwd(&cfg, cwd).unwrap_or_else(|| {
                        cwd.rsplit(['/', '\\']).find(|s| !s.is_empty()).unwrap_or("").to_string()
                    });
                    let n = pending.len() + asks.len();
                    let more = if n > 1 { format!(" +{}", n - 1) } else { String::new() };
                    segs.push(seg(format!("Claude {verb} · {where_}{more}"), Rgba::AMBER, Some(Rgba::AMBER), true, true));
                } else {
                    let working = sessions.iter().filter(|s| s.state == claude::SessionState::Working).count();
                    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_millis() as u64).unwrap_or(0);
                    let done = sessions
                        .iter()
                        .filter(|s| s.state == claude::SessionState::Idle && !s.unfinished() && now.saturating_sub(s.updated_at) < 15 * 60 * 1000)
                        .count();
                    if working > 0 {
                        segs.push(seg(trf!("Claude {working}× pracuje", "Claude {working}× working"), Rgba::TEXT, Some(Rgba::VIOLET), false, true));
                    }
                    if done > 0 {
                        segs.push(seg(trf!("{done}× hotovo", "{done}× done"), Rgba::DIM, Some(Rgba::TEAL), false, false));
                    }
                }
                // hlídač: session s nezacommitovanými / nepushnutými vlastními změnami
                let unfinished = sessions.iter().filter(|s| s.unfinished()).count();
                if unfinished > 0 {
                    segs.push(seg(trf!("{unfinished}× nedokončeno", "{unfinished}× unfinished"), Rgba::AMBER, Some(Rgba::AMBER), false, true));
                }
            }
            config::StripItem::Limits => crate::limitslive::strip_segments(&mut segs),
            config::StripItem::Waiting => {
                // kolik různých session čeká na tebe (schválení / otázka); 0 = žádný segment
                // + čekající podle Agent View (i bez hooků Ledge; seznam z posledního otevření šuplíku + registr)
                let live: Vec<String> = crate::sessionlive::with_cache(|c| {
                    let live = crate::sessions::Live { agents: c.agents.as_deref(), fetched_at: c.fetched_at, registry: &c.registry };
                    crate::sessions::waiting_ids(&live).into_iter().map(str::to_string).collect()
                });
                let live: Vec<&str> = live.iter().map(String::as_str).collect();
                if let Some(text) = attention::waiting_text(attention::waiting_count_all(&pending, &asks, &live)) {
                    segs.push(seg(text, Rgba::AMBER, Some(Rgba::AMBER), true, true));
                }
            }
            config::StripItem::Repos => {
                let git = c.git.lock();
                let mut dirty = 0;
                let mut unpushed = 0;
                for p in cfg.projects.iter().filter(|p| p.enabled) {
                    if let Some((_, Some(g))) = git.get(&p.id) {
                        if g.dirty > 0 {
                            dirty += 1;
                        }
                        if g.ahead > 0 {
                            unpushed += 1;
                        }
                    }
                }
                let mut words = Vec::new();
                if dirty > 0 {
                    words.push(trf!("{dirty}× neuloženo", "{dirty}× uncommitted"));
                }
                if unpushed > 0 {
                    words.push(trf!("{unpushed}× nepushnuto", "{unpushed}× unpushed"));
                }
                if !words.is_empty() {
                    segs.push(seg(words.join(" · "), Rgba::DIM, Some(Rgba::AMBER), false, false));
                }
            }
            config::StripItem::Custom => segs.extend(stripconf::custom_segment(&cfg.strip.custom)),
            config::StripItem::Timer => segs.extend(crate::timer::strip_segments(&timer, now_ms)),
            // jen paměť (`media.rs` ji přepíše při události přehrávače → `mark_dirty`); nic nehraje = nic
            config::StripItem::Media => segs.extend(crate::media::strip_now(&cfg, now_ms)),
            // jen paměť (`github/` ji plní karta nebo plánovač `integrations.rs`); bez dat nebo bez revizí nic
            config::StripItem::Github => segs.extend(crate::github::strip_now(&cfg.github)),
            config::StripItem::System => segs.extend(crate::sysmon::strip_now()),
            // widget ze skriptu: jen paměť (`widgets::live`; běhy plánuje vlákno `widgets`)
            config::StripItem::Widget(_) if cfg.widgets.enabled => segs.extend(crate::widgets::live::strip_segments(&item.key())),
            config::StripItem::Widget(_) => {}
            // jen paměť (`accounts/mail.rs`); 0 nepřečtených / nepřihlášeno = nic
            config::StripItem::Mail => segs.extend(crate::accounts::mail::strip_segment(&cfg)),
        }
        parts.push((item.clone(), segs));
    }
    let look = &cfg.appearance;
    let palette = crate::appearance::pages::palette_for(look);
    let accent = geom::parse_hex(&look.accent).unwrap_or(palette.accent[0]);
    let mut segs = stripconf::compose(parts, &cfg.strip, ambient.is_some() || due_count > 0, |c| geom::readable(c.base(accent), &palette));
    if let Some(first) = &due_first {
        let mut text: String = if first.is_empty() { tr("Připomínka", "Reminder").into() } else { first.chars().take(28).collect() };
        if first.chars().count() > 28 {
            text.push('…');
        }
        let more = if due_count > 1 { format!(" +{}", due_count - 1) } else { String::new() };
        segs.insert(0, seg(format!("🔔 {text}{more}"), Rgba::AMBER, Some(Rgba::AMBER), true, true));
    }
    if let Some(text) = &meeting {
        let at = usize::from(due_count > 0);
        segs.insert(at, seg(text.clone(), Rgba::BLUE, Some(Rgba::BLUE), true, true));
    }
    // mrtvý Ledge (fullscreen hra nebo herní režim) = proužek pryč
    let dead = c.dead();
    let drawer_open = c.drawer.view() != View::Hidden;
    // nad taby prohlížeče proužek ustoupí na tenkou linku, dokud nepřijde událost
    let retreat = c.bar_window.load(Ordering::Relaxed) && ambient.is_none();
    // styl zavřeného Ledge (`strip.look`): obsah podle stavu (`looks::show`); vypnutý proužek a ústup
    // nad taby (u stylů, které taby překrývají) = klasická linka
    let closed = cfg.strip.look;
    let looked = closed != crate::looks::Look::Classic && cfg.strip.enabled && !(retreat && closed.retreats());
    let show = looked.then(|| {
        let f = look_facts(&cfg, &pending, &asks, &procs, due_first.as_deref(), due_count, meeting.as_deref(), &mode_title, accent);
        crate::looks::show(closed, &crate::looks::without(f, &cfg.strip.look_hide))
    });
    let hidden = show.as_ref().is_some_and(|s| s.hidden);
    let line_only = !cfg.strip.enabled || (retreat && !looked);
    let visible = !dead && !drawer_open && (cfg.strip.enabled || ambient.is_some()) && !hidden;
    let position = cfg.appearance.strip.position;
    // plynulá šířka ostrůvku jen se zapnutými animacemi (Ledge i Windows „omezit pohyb“)
    let animate = closed == crate::looks::Look::Island
        && cfg.appearance.motion.style != crate::appearance::MotionStyle::None
        && !attention::os_animations_off();
    let opacity = cfg.strip.opacity;
    let (style, metrics, line) = (look.strip.style, geom::strip_metrics(look.strip.style, look.strip.size), geom::line_stops(look, &palette));
    let axis2 = geom::axis2_for(&cfg, mon.rect.left, mon.rect.right, mon.dpi);
    // puls „čeká / hotovo“ (`attention::pulse`, s termínem; animuje vlákno `strip`)
    let pulse = attention::pulse();
    drop(cfg);
    c.strip.set(|m| {
        m.visible = visible;
        m.segments = segs;
        m.ambient = ambient;
        m.opacity = opacity;
        m.line_only = line_only;
        m.monitor = mon.rect;
        m.dpi = mon.dpi;
        m.style = style;
        m.metrics = metrics;
        m.palette = palette;
        m.line = line;
        m.axis2 = Some(axis2);
        m.pulse = pulse;
        m.look = closed;
        m.show = show;
        m.animate = animate;
        m.position = position;
    });
}

/// Co se děje — pro styly zavřeného Ledge (`looks::Facts`). Jen paměť jádra, jako zbytek `refresh_strip`.
#[allow(clippy::too_many_arguments)]
fn look_facts(
    cfg: &Config,
    pending: &[claude::Pending],
    asks: &[claude::Ask],
    procs: &[procman::ProcView],
    due_first: Option<&str>,
    due_count: usize,
    meeting: Option<&str>,
    mode_title: &str,
    accent: Rgba,
) -> crate::looks::Facts {
    use crate::looks::{DONE_MS, Facts, facts};
    let c = core();
    let now_ms = crate::state::now_ms();
    let where_of = |cwd: &str| project_for_cwd(cfg, cwd).unwrap_or_else(|| cwd.rsplit(['/', '\\']).find(|s| !s.is_empty()).unwrap_or("").to_string());
    // čekající: jako položka `waiting` (schválení, otázky, Agent View)
    let live: Vec<String> = crate::sessionlive::with_cache(|c| {
        let live = crate::sessions::Live { agents: c.agents.as_deref(), fetched_at: c.fetched_at, registry: &c.registry };
        crate::sessions::waiting_ids(&live).into_iter().map(str::to_string).collect()
    });
    let live: Vec<&str> = live.iter().map(String::as_str).collect();
    let waiting = attention::waiting_count_all(pending, asks, &live);
    let waiting_where = pending.first().map(|p| p.cwd.as_str()).or_else(|| asks.first().map(|a| a.cwd.as_str())).map(where_of).unwrap_or_default();
    let sessions = c.claude.sessions();
    let now = now_ms.max(0) as u64;
    let working: Vec<_> = sessions.iter().filter(|s| s.state == claude::SessionState::Working).collect();
    let mut done: Vec<_> = sessions
        .iter()
        .filter(|s| s.state == claude::SessionState::Idle && !s.ended && !s.unfinished() && now.saturating_sub(s.updated_at) < DONE_MS)
        .collect();
    done.sort_by_key(|s| std::cmp::Reverse(s.updated_at));
    let (mut repos, git) = (0, c.git.lock());
    for p in cfg.projects.iter().filter(|p| p.enabled) {
        if let Some((_, Some(g))) = git.get(&p.id)
            && (g.dirty > 0 || g.ahead > 0)
        {
            repos += 1;
        }
    }
    drop(git);
    let reminder = due_first.map(|t| {
        let mut s: String = if t.is_empty() { tr("Připomínka", "Reminder").into() } else { t.chars().take(28).collect() };
        if t.chars().count() > 28 {
            s.push('…');
        }
        (s, due_count)
    });
    // připomínka schůzky (`calendar::remind`) i ve stylech zavřeného Ledge (ostrůvek…): jako připomínka
    // poznámky (stav „čeká na tebe“), jen když žádná poznámka po termínu není
    let reminder = reminder.or_else(|| meeting.map(|m| (m.to_string(), 1)));
    use windows::Win32::System::SystemInformation::GetLocalTime;
    let lt = unsafe { GetLocalTime() };
    Facts {
        waiting,
        waiting_where,
        reminder,
        working: working.len(),
        working_where: working.first().map(|s| where_of(&s.cwd)).unwrap_or_default(),
        done: done.len(),
        done_where: done.first().map(|s| where_of(&s.cwd)).unwrap_or_default(),
        timer: facts::timer_fact(&crate::timer::state(), now_ms),
        music: facts::music_fact(crate::media::view().as_ref(), now_ms),
        servers: procs.iter().filter(|p| p.phase != procman::Phase::Exited).count(),
        server_failed: procs.iter().any(|p| p.phase == procman::Phase::Failed),
        repos,
        limit5h: facts::limit_fact(&crate::limitslive::view(), now_ms),
        clock: crate::looks::hm(lt.wHour, lt.wMinute),
        date: clock_text(lt.wHour, lt.wMinute, lt.wDayOfWeek, lt.wDay, lt.wMonth),
        mode: mode_title.to_string(),
        // akcent čitelný na tmavém těle stylů
        accent: geom::readable(accent, &geom::palette(crate::appearance::Theme::Dark)),
    }
}

/// Název projektu, ve kterém leží cwd (nejdelší shoda cesty).
pub(crate) fn project_for_cwd(cfg: &Config, cwd: &str) -> Option<String> {
    let c = core();
    let cwd = cwd.to_lowercase().replace('/', "\\");
    cfg.projects
        .iter()
        .filter_map(|p| c.project_dir(p).map(|d| (p, d.to_string_lossy().to_lowercase())))
        .filter(|(_, d)| cwd.starts_with(d.as_str()))
        .max_by_key(|(_, d)| d.len())
        .map(|(p, _)| p.name.clone())
}

/// Hodiny proužku z místního času.
pub(crate) fn local_clock() -> String {
    use windows::Win32::System::SystemInformation::GetLocalTime;
    let t = unsafe { GetLocalTime() };
    clock_text(t.wHour, t.wMinute, t.wDayOfWeek, t.wDay, t.wMonth)
}

/// „08:31 · st 15. 10.“ česky, „08:31 · Wed 10/15“ anglicky (`dow` 0 = neděle).
pub(crate) fn clock_text(hour: u16, minute: u16, dow: u16, day: u16, month: u16) -> String {
    const DNY: [&str; 7] = ["ne", "po", "út", "st", "čt", "pá", "so"];
    const DAYS: [&str; 7] = ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"];
    let d = dow as usize % 7;
    trf!("{hour:02}:{minute:02} · {} {day}. {month}.", "{hour:02}:{minute:02} · {} {month}/{day}", tr(DNY[d], DAYS[d]))
}

#[cfg(test)]
mod tests {
    use super::clock_text;
    use crate::i18n::with_lang;
    use crate::uiconf::Lang;

    #[test]
    fn clock_in_both_languages() {
        assert_eq!(clock_text(8, 31, 3, 15, 10), "08:31 · st 15. 10.");
        assert_eq!(with_lang(Lang::En, || clock_text(8, 31, 3, 15, 10)), "08:31 · Wed 10/15");
        assert_eq!(clock_text(23, 5, 7, 1, 1), "23:05 · ne 1. 1.", "dow mimo rozsah = modulo");
    }
}
