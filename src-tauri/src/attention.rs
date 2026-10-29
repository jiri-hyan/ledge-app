//! Pozornost Clauda pro proužek: kolik session čeká na člověka (segment `strip.items` „waiting“),
//! přechody „čeká na tebe“ (nové schválení / otázka) a „hotovo“ (konec turnu, hook `Stop`) →
//! krátký puls proužku a volitelný toast Windows (`claude.toast`, výchozí vypnuto).
//! Třetí událost je **schůzka za chvíli** (`calendar::remind`): stejný signál (`signal`), priorita
//! **mezi** „čeká na tebe“ a „hotovo“ (`Event::rank`): čekající Claude > schůzka > spadlé CI > PR k revizi >
//! hotovo. Čtvrtá a pátá událost jsou z GitHubu (`suggest::pulse`: CI na mém PR nově spadlo, nové PR čeká na
//! mou revizi) — jen puls a bliknutí, bez toastu. V proužku
//! totéž pořadí ambientní barvy (`stripview`: čeká/připomínka poznámky > schůzka > pád serveru).
//!
//! Všechno je čistá logika (tabulkové testy) kromě `on_change`, které volá `refresh::on_claude_change`
//! — žádné vlastní vlákno ani časovač: puls má termín (`PULSE_MS`, animuje ho vlákno `strip` jen po
//! tu dobu), toast je jednorázové krátké vlákno (`toast.rs`).
//!
//! Otázka Clauda čeká a šuplík je zavřený (proužek je vidět): puls se opakuje po `REPULSE_MS`
//! (`repulse_due`, termín pro vlákno `emit` z `next_repulse`) — jen dokud otázka čeká, v klidu nic.
//! Přišla-li otázka / schválení za „mrtvého“ Ledge (hra přes celou obrazovku, herní režim), peek po
//! návratu dožene `catch_up` (`revive_peek`, jednou při přechodu mrtvý → živý).

use crate::claude::{Ask, Pending, Session, SessionState};
use crate::i18n::tr;
use crate::strip::Rgba;
use parking_lot::Mutex;
use std::collections::{HashMap, HashSet};
use std::time::Instant;

/// Délka pulsu proužku (dvě vlny).
pub(crate) const PULSE_MS: u64 = 1200;
/// Odstup opakovaného pulsu, dokud otázka Clauda čeká se zavřeným šuplíkem.
pub(crate) const REPULSE_MS: u64 = 10_000;
/// Stejná session + stejná událost nejvýš jednou za tuto dobu (puls i toast).
pub(crate) const DEDUP_MS: u64 = 30_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum Event {
    /// nové schválení nebo otázka Clauda
    NeedsInput,
    /// session dokončila turn (Working → Idle, ne ukončením session)
    Done,
    /// schůzka z kalendáře začne za `remindMin` minut (`calendar::remind`)
    Meeting,
    /// CI na mém PR nově spadlo (GitHub, `suggest::pulse`)
    GithubCi,
    /// nové PR čeká na mou revizi (GitHub, `suggest::pulse`)
    GithubReview,
}

impl Event {
    pub(crate) fn color(self) -> Rgba {
        match self {
            Event::NeedsInput => Rgba::AMBER,
            Event::Meeting => Rgba::BLUE,
            Event::GithubCi => Rgba::RED,
            Event::GithubReview => Rgba::VIOLET,
            Event::Done => Rgba::TEAL,
        }
    }
    /// Priorita při souběhu (nižší = důležitější): čeká na tebe · schůzka · spadlé CI · PR k revizi · hotovo.
    pub(crate) fn rank(self) -> u8 {
        match self {
            Event::NeedsInput => 0,
            Event::Meeting => 1,
            Event::GithubCi => 2,
            Event::GithubReview => 3,
            Event::Done => 4,
        }
    }
}

// ---------- počet čekajících ----------

/// Kolik různých session čeká na člověka (schválení nebo otázka; jedna session s oběma = 1).
pub(crate) fn waiting_sessions<'a>(sids: impl IntoIterator<Item = &'a str>) -> usize {
    sids.into_iter().collect::<HashSet<_>>().len()
}

/// Text segmentu proužku; 0 = žádný segment.
pub(crate) fn waiting_text(n: usize) -> Option<String> {
    match n {
        0 => None,
        1 => Some(tr("1 čeká", "1 waiting").into()),
        2..=4 => Some(trf!("{n} čekají", "{n} waiting")),
        _ => Some(trf!("{n} čeká", "{n} waiting")),
    }
}

/// Počet čekajících session z reálného stavu mostu + session, které podle Agent View čekají i bez hooků Ledge
/// (`sessions::waiting_ids`: `status: waiting` / `waitingFor` / `state: blocked`). Session, kterou už
/// hlásí hook (stejné `sessionId`), se počítá jednou.
pub(crate) fn waiting_count_all(pending: &[Pending], asks: &[Ask], live: &[&str]) -> usize {
    waiting_sessions(pending.iter().map(|p| p.session_id.as_str()).chain(asks.iter().map(|a| a.session_id.as_str())).chain(live.iter().copied()))
}

/// Revize výchozích položek proužku (`strip.itemsRev`): 1 = přibyla `waiting`.
pub(crate) const ITEMS_REV: u32 = 1;

/// Starý config (`itemsRev` < 1): doplní `waiting` hned za `claude` — jen když proužek Clauda
/// ukazuje (kdo `claude` odebral, nechce ani tohle). Jednou: s `itemsRev` 1 zůstane odebraná odebraná.
pub(crate) fn upgrade_items(strip: &mut crate::config::StripConfig) {
    use crate::config::StripItem;
    if strip.items_rev >= ITEMS_REV {
        return;
    }
    strip.items_rev = ITEMS_REV;
    if strip.items.contains(&StripItem::Waiting) {
        return;
    }
    if let Some(i) = strip.items.iter().position(|x| *x == StripItem::Claude) {
        strip.items.insert(i + 1, StripItem::Waiting);
    }
}

// ---------- přechody ----------

/// Snímek stavu pro porovnání: id čekajících žádostí/otázek (sdílený čítač mostu → unikátní)
/// s jejich session a stav každé session.
#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct Seen {
    pub(crate) waiting: HashMap<u64, String>,
    /// session → (stav, ukončená)
    pub(crate) states: HashMap<String, (SessionState, bool)>,
}

impl Seen {
    pub(crate) fn of(pending: &[Pending], asks: &[Ask], sessions: &[Session]) -> Self {
        let waiting = pending
            .iter()
            .map(|p| (p.id, p.session_id.clone()))
            .chain(asks.iter().map(|a| (a.id, a.session_id.clone())))
            .collect();
        let states = sessions.iter().map(|s| (s.id.clone(), (s.state, s.ended))).collect();
        Self { waiting, states }
    }
}

/// Nové události mezi dvěma snímky (seřazené podle session, deterministicky). Session, kterou
/// předchozí snímek neznal, „hotovo“ nehlásí (Ledge se připojil uprostřed turnu).
pub(crate) fn transitions(prev: &Seen, now: &Seen) -> Vec<(String, Event)> {
    let mut out: Vec<(String, Event)> = Vec::new();
    for (id, sid) in &now.waiting {
        if !prev.waiting.contains_key(id) && !out.iter().any(|(s, e)| s == sid && *e == Event::NeedsInput) {
            out.push((sid.clone(), Event::NeedsInput));
        }
    }
    for (sid, (state, ended)) in &now.states {
        let was_working = prev.states.get(sid).is_some_and(|(s, _)| *s == SessionState::Working);
        if was_working && *state == SessionState::Idle && !ended {
            out.push((sid.clone(), Event::Done));
        }
    }
    out.sort_by(|a, b| (&a.0, a.1 as u8).cmp(&(&b.0, b.1 as u8)));
    out
}

/// Deduplikace: (session, událost) nejvýš jednou za `DEDUP_MS`.
#[derive(Debug, Default)]
pub(crate) struct Dedup {
    last: HashMap<(String, Event), u64>,
}

impl Dedup {
    pub(crate) fn allow(&mut self, sid: &str, ev: Event, now_ms: u64) -> bool {
        self.last.retain(|_, t| now_ms.saturating_sub(*t) < DEDUP_MS);
        let key = (sid.to_string(), ev);
        if self.last.contains_key(&key) {
            return false;
        }
        self.last.insert(key, now_ms);
        true
    }
}

// ---------- reakce ----------

/// Vstupy rozhodnutí, co s událostí udělat.
#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct Gate {
    /// Ledge je mrtvý (fullscreen hra nebo herní režim — `gamemode::is_dead`)
    pub(crate) dead: bool,
    /// `appearance.motion.style` = none nebo systémové „animace vypnuté“
    pub(crate) no_motion: bool,
    /// `claude.toast`
    pub(crate) toast_on: bool,
    /// Windows právě oznámení nepřijímá (QUNS ≠ ACCEPTS_NOTIFICATIONS: hra, prezentace, nerušit)
    pub(crate) quiet: bool,
    /// šuplík je otevřený naplno — šéf se na Ledge dívá, toast by byl navíc
    pub(crate) drawer_full: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) struct Reaction {
    /// barevné bliknutí linky (`Core.flash`, statické, s termínem `FLASH`)
    pub(crate) flash: bool,
    /// animovaný puls (`PULSE_MS`)
    pub(crate) pulse: bool,
    pub(crate) toast: bool,
}

pub(crate) fn react(g: Gate) -> Reaction {
    if g.dead {
        return Reaction::default();
    }
    Reaction { flash: true, pulse: !g.no_motion, toast: g.toast_on && !g.quiet && !g.drawer_full }
}

/// Jas pulsu 0–1 v čase od začátku (dvě vlny sin², plynule z nuly do nuly); `None` = puls skončil.
pub(crate) fn pulse_level(elapsed_ms: u64) -> Option<f32> {
    if elapsed_ms >= PULSE_MS {
        return None;
    }
    let wave = (PULSE_MS / 2) as f32;
    let x = std::f32::consts::PI * elapsed_ms as f32 / wave;
    Some(x.sin().powi(2))
}

/// Texty toastu (titulek, tělo); `where_` = projekt / název session, prázdné = bez těla.
pub(crate) fn toast_text(ev: Event, asks: bool, where_: &str) -> (String, String) {
    let title = match (ev, asks) {
        (Event::NeedsInput, false) => tr("Claude čeká na schválení", "Claude is waiting for approval"),
        (Event::NeedsInput, true) => tr("Claude se ptá", "Claude has a question"),
        (Event::Done, _) => tr("Claude je hotový", "Claude is done"),
        (Event::Meeting, _) => tr("Schůzka za chvíli", "Meeting soon"),
        (Event::GithubCi, _) => tr("Spadlo CI na tvém PR", "CI failed on your PR"),
        (Event::GithubReview, _) => tr("PR čeká na tvou revizi", "A PR is waiting for your review"),
    };
    (title.to_string(), where_.to_string())
}

// ---------- otázka čeká: opakovaný puls, peek po hře ----------

/// Zopakovat puls teď? Jen když čeká otázka Clauda, proužek je vidět (šuplík zavřený, Ledge živý) a od
/// posledního pulsu (`last_ago`, ms; `None` = žádný) uběhlo `REPULSE_MS`.
pub(crate) fn repulse_due(asking: bool, strip_shown: bool, last_ago: Option<u64>) -> bool {
    asking && strip_shown && last_ago.is_none_or(|t| t >= REPULSE_MS)
}

/// Za kolik ms se má vlákno `emit` probudit kvůli dalšímu pulsu; `None` = nečeká se (žádný časovač).
pub(crate) fn next_repulse(asking: bool, strip_shown: bool, last_ago: Option<u64>) -> Option<u64> {
    (asking && strip_shown).then(|| REPULSE_MS.saturating_sub(last_ago.unwrap_or(REPULSE_MS)))
}

/// Dohnat peek po konci hry / herního režimu? Jen při přechodu mrtvý → živý, se zavřeným šuplíkem a
/// něčím, co na člověka čeká (ruční zavření peeku za živého Ledge se nepřebije).
pub(crate) fn revive_peek(was_dead: bool, dead: bool, hidden: bool, attention: bool) -> bool {
    was_dead && !dead && hidden && attention
}

// ---------- napojení na jádro ----------

struct Watch {
    seen: Seen,
    dedup: Dedup,
    pulse: Option<(Rgba, Instant)>,
}

static WATCH: Mutex<Option<Watch>> = Mutex::new(None);

/// Běžící puls pro `refresh_strip` (barva, začátek); po `PULSE_MS` `None`.
pub(crate) fn pulse() -> Option<(Rgba, Instant)> {
    WATCH.lock().as_ref().and_then(|w| w.pulse).filter(|(_, t)| (t.elapsed().as_millis() as u64) < PULSE_MS)
}

/// Systémové „Efekty animace“ vypnuté (Nastavení Windows → Usnadnění → Vizuální efekty). Bere i
/// vysunutí šuplíku (`appearance::motion::anim_ms_now`).
pub(crate) fn os_animations_off() -> bool {
    use windows::Win32::Foundation::TRUE;
    use windows::Win32::UI::WindowsAndMessaging::{SPI_GETCLIENTAREAANIMATION, SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS, SystemParametersInfoW};
    let mut on = TRUE;
    let ok = unsafe { SystemParametersInfoW(SPI_GETCLIENTAREAANIMATION, 0, Some(&mut on as *mut _ as *mut _), SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS(0)) };
    ok.is_ok() && !on.as_bool()
}

/// Brána reakce ze stavu jádra (systémová volání jen tehdy, když na nich něco závisí).
fn live_gate(toast_on: bool) -> Gate {
    use crate::drawer::View;
    let c = crate::state::core();
    let no_motion = c.cfg.read().appearance.motion.style == crate::appearance::MotionStyle::None;
    let mut gate = Gate { dead: c.dead(), no_motion, toast_on, quiet: false, drawer_full: c.drawer.view() == View::Full };
    if !gate.dead && !gate.no_motion {
        gate.no_motion = os_animations_off();
    }
    if gate.toast_on && !gate.dead {
        gate.quiet = !crate::toast::accepts_notifications();
    }
    gate
}

/// Upozornění proužkem barvou `color` podle `react`: bliknutí linky (`Core.flash`) a puls (`PULSE_MS`);
/// mrtvý Ledge (fullscreen hra / herní režim) nic. Vrací, jestli smí volající ukázat toast (`toast_on`,
/// Windows oznámení přijímá, šuplík není otevřený naplno). Fokus nikdy. Volá `on_change` i konec
/// časovače (`timer::tick_live`).
pub(crate) fn signal(color: Rgba, toast_on: bool) -> bool {
    let r = react(live_gate(toast_on));
    if r.flash {
        *crate::state::core().flash.lock() = Some((color, Instant::now()));
    }
    if r.pulse {
        let mut g = WATCH.lock();
        let w = g.get_or_insert_with(|| Watch { seen: Seen::default(), dedup: Dedup::default(), pulse: None });
        w.pulse = Some((color, Instant::now()));
    }
    r.toast
}

/// Reakce na změnu stavu mostu (volá `refresh::on_claude_change`, ten pak `mark_dirty`).
pub(crate) fn on_change() {
    use crate::state::core;
    let c = core();
    let pending = c.claude.pending();
    let asks = c.claude.questions();
    let sessions = c.claude.sessions();
    let now_seen = Seen::of(&pending, &asks, &sessions);
    let now = crate::state::now_ms().max(0) as u64;
    let events = {
        let mut g = WATCH.lock();
        let w = g.get_or_insert_with(|| Watch { seen: Seen::default(), dedup: Dedup::default(), pulse: None });
        let evs = transitions(&w.seen, &now_seen);
        w.seen = now_seen;
        evs.into_iter().filter(|(sid, ev)| w.dedup.allow(sid, *ev, now)).collect::<Vec<_>>()
    };
    if events.is_empty() {
        return;
    }
    // zvuk (volitelný, výchozí vypnuto; `approvalsound.rs` si hlídá brány sám) jen u nového schválení / otázky
    if events.iter().any(|(_, e)| *e == Event::NeedsInput) {
        crate::approvalsound::on_needs_input();
    }
    let (wait_on, done_on) = {
        let cl = &c.cfg.read().claude;
        (cl.toast, done_toast_on(cl.toast, cl.toast_done))
    };
    let toast_on = wait_on || done_on;
    // „čeká“ má přednost (barva pulsu), pak hotovo (`Event::rank`)
    let first = events.iter().min_by_key(|(_, e)| e.rank()).cloned();
    let Some((_, ev)) = first else { return };
    if signal(ev.color(), toast_on) {
        // oznámení (Notch / toast podle `notify`); config se pustí dřív, než `notify::emit` čte svůj
        let mut notices = Vec::new();
        let cfg = c.cfg.read();
        for (sid, ev) in &events {
            if !event_toast_on(*ev, wait_on, done_on) {
                continue;
            }
            let s = sessions.iter().find(|s| &s.id == sid);
            let cwd = s.map(|s| s.cwd.as_str()).or_else(|| pending.iter().find(|p| &p.session_id == sid).map(|p| p.cwd.as_str())).or_else(|| asks.iter().find(|a| &a.session_id == sid).map(|a| a.cwd.as_str())).unwrap_or("");
            let project = crate::stripview::project_for_cwd(&cfg, cwd).unwrap_or_else(|| claude_dir_name(cwd));
            let where_ = match s.and_then(|s| s.title.as_deref()) {
                Some(t) if !project.is_empty() => format!("{project} · {t}"),
                Some(t) => t.to_string(),
                None => project,
            };
            let is_ask = *ev == Event::NeedsInput && !pending.iter().any(|p| &p.session_id == sid);
            let (title, body) = toast_text(*ev, is_ask, &where_);
            notices.push(notice_of(*ev, sid, title, body, pending.iter().find(|p| &p.session_id == sid).map(|p| p.tool.as_str())));
        }
        drop(cfg);
        for n in notices {
            crate::notify::emit(n);
        }
    }
}

/// Efektivní vypínač oznámení „Hotovo“: vlastní `claude.toastDone`, jinak `claude.toast`.
pub(crate) fn done_toast_on(toast: bool, toast_done: Option<bool>) -> bool {
    toast_done.unwrap_or(toast)
}

/// Smí tato událost do oznámení? Čeká (a dotaz GitHubu apod.) řídí `claude.toast`, Hotovo svůj vypínač.
pub(crate) fn event_toast_on(ev: Event, wait_on: bool, done_on: bool) -> bool {
    if ev == Event::Done { done_on } else { wait_on }
}

/// Oznámení pro `notify`: čeká → tlačítko Otevřít (přivede okno session) a nástroj v textu; hotovo bez
/// tlačítka. Povolit nikdy — to smí jen člověk v okně Claude.
pub(crate) fn notice_of(ev: Event, sid: &str, title: String, body: String, tool: Option<&str>) -> crate::notify::Notice {
    use crate::notify::{Action, Kind, Notice};
    let waiting = ev == Event::NeedsInput;
    let body = match tool.filter(|t| waiting && !t.is_empty()) {
        Some(t) if body.is_empty() => t.to_string(),
        Some(t) => format!("{body} · {t}"),
        None => body,
    };
    Notice {
        kind: if waiting { Kind::Waiting } else { Kind::Done },
        title,
        body,
        button: waiting.then(|| tr("Otevřít", "Open").to_string()),
        action: (waiting && !sid.is_empty()).then(|| Action::Session(sid.to_string())),
        count: 1,
        ..Notice::default()
    }
}

/// Stav pro `catch_up`: byl Ledge při minulém průchodu mrtvý?
static WAS_DEAD: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
/// Kdy `catch_up` naposledy zopakoval puls čekající otázky.
static LAST_REPULSE: Mutex<Option<Instant>> = Mutex::new(None);

/// (čeká otázka Clauda, proužek je vidět, ms od posledního pulsu) ze stavu jádra — jen paměť.
fn asking_now() -> (bool, bool, Option<u64>) {
    use crate::drawer::View;
    let c = crate::state::core();
    let asking = !c.claude.questions().is_empty();
    // čekající otázka = jantarová linka, proužek je vidět i vypnutý (`stripview`: ambientní barva)
    let shown = asking && c.drawer.view() == View::Hidden && !c.dead();
    // poslední puls i poslední pokus o něj (bez animací se puls nenastaví — jinak by se `emit` točil)
    let pulse = WATCH.lock().as_ref().and_then(|w| w.pulse).map(|(_, t)| t);
    let ago = [pulse, *LAST_REPULSE.lock()].into_iter().flatten().map(|t| t.elapsed().as_millis() as u64).min();
    (asking, shown, ago)
}

/// Vlákno `emit` (po každém probuzení): peek po hře (`revive_peek`) a opakovaný puls čekající otázky
/// (`repulse_due`). Bez otázky a bez přechodu ze hry nic.
pub(crate) fn catch_up() {
    use crate::drawer::View;
    use std::sync::atomic::Ordering;
    let c = crate::state::core();
    let dead = c.dead();
    let was = WAS_DEAD.swap(dead, Ordering::Relaxed);
    if revive_peek(was, dead, c.drawer.view() == View::Hidden, c.claude.attention()) {
        c.send_ui(crate::state::UiCmd::Show(View::Peek, crate::drawer::Opener::Hover));
    }
    let (asking, shown, ago) = asking_now();
    if repulse_due(asking, shown, ago) {
        *LAST_REPULSE.lock() = Some(Instant::now());
        // proužek si puls vyzvedne v `refresh_strip` na konci téhož průchodu `emit`
        signal(Event::NeedsInput.color(), false);
    }
}

/// Termín pro `next_wake` vlákna `emit` (ms epochy): další puls čekající otázky; jinak `None`.
pub(crate) fn next_deadline() -> Option<i64> {
    let (asking, shown, ago) = asking_now();
    next_repulse(asking, shown, ago).map(|ms| crate::state::now_ms() + ms as i64 + 1)
}

/// Poslední část cesty (název složky) — když cwd neleží v žádném projektu.
fn claude_dir_name(cwd: &str) -> String {
    cwd.rsplit(['/', '\\']).find(|s| !s.is_empty()).unwrap_or("").to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::i18n::with_lang;
    use crate::uiconf::Lang;

    fn seen(waiting: &[(u64, &str)], states: &[(&str, SessionState, bool)]) -> Seen {
        Seen {
            waiting: waiting.iter().map(|(i, s)| (*i, s.to_string())).collect(),
            states: states.iter().map(|(s, st, e)| (s.to_string(), (*st, *e))).collect(),
        }
    }

    #[test]
    fn waiting_counts_distinct_sessions() {
        let cases: &[(&[&str], usize)] = &[(&[], 0), (&["a"], 1), (&["a", "a"], 1), (&["a", "b"], 2), (&["a", "b", "a", "c"], 3)];
        for (ids, want) in cases {
            assert_eq!(waiting_sessions(ids.iter().copied()), *want, "{ids:?}");
        }
        // Agent View bez hooků: jen živý seznam, duplicity jednou
        assert_eq!(waiting_count_all(&[], &[], &["bg", "bg", "cli"]), 2);
        assert_eq!(waiting_count_all(&[], &[], &[]), 0);
    }

    #[test]
    fn waiting_text_plural_both_languages() {
        let cases = [(0, None, None), (1, Some("1 čeká"), Some("1 waiting")), (2, Some("2 čekají"), Some("2 waiting")), (4, Some("4 čekají"), Some("4 waiting")), (5, Some("5 čeká"), Some("5 waiting")), (12, Some("12 čeká"), Some("12 waiting"))];
        for (n, cs, en) in cases {
            assert_eq!(waiting_text(n).as_deref(), cs, "cs {n}");
            assert_eq!(with_lang(Lang::En, || waiting_text(n)).as_deref(), en, "en {n}");
        }
    }

    #[test]
    fn upgrade_items_table() {
        use crate::config::{StripConfig, StripItem::*};
        let cases: Vec<(&str, Vec<crate::config::StripItem>, u32, Vec<crate::config::StripItem>)> = vec![
            ("starý config: za claude", vec![Mode, Claude, Repos], 0, vec![Mode, Claude, Waiting, Repos]),
            ("starý config bez claude: nic", vec![Mode, Clock], 0, vec![Mode, Clock]),
            ("už tam je: beze změny", vec![Waiting, Claude], 0, vec![Waiting, Claude]),
            ("rev 1, odebraná zůstane odebraná", vec![Mode, Claude], 1, vec![Mode, Claude]),
            ("prázdný proužek", vec![], 0, vec![]),
        ];
        for (name, items, rev, want) in cases {
            let mut s = StripConfig { items, items_rev: rev, ..StripConfig::default() };
            upgrade_items(&mut s);
            assert_eq!(s.items, want, "{name}");
            assert_eq!(s.items_rev, ITEMS_REV, "{name}");
            upgrade_items(&mut s);
            assert_eq!(s.items, want, "{name}: podruhé beze změny");
        }
        // nový uživatel: výchozí proužek už `waiting` má; starý soubor bez `itemsRev` ho dostane při načtení
        assert!(StripConfig::default().items.contains(&Waiting));
        let dir = tempfile::tempdir().unwrap();
        let load = |json: &str| {
            let p = dir.path().join("config.json");
            std::fs::write(&p, json).unwrap();
            crate::config::load(&p).unwrap().unwrap()
        };
        assert_eq!(load(r#"{"strip":{"items":["mode","claude"]}}"#).strip.items, vec![Mode, Claude, Waiting]);
        assert_eq!(load(r#"{"strip":{"items":["mode","claude"],"itemsRev":1}}"#).strip.items, vec![Mode, Claude]);
    }

    #[test]
    fn transitions_table() {
        use SessionState::*;
        type Case = (&'static str, Seen, Seen, Vec<(&'static str, Event)>);
        let cases: Vec<Case> = vec![
            ("nic se nezměnilo", seen(&[(1, "a")], &[("a", Waiting, false)]), seen(&[(1, "a")], &[("a", Waiting, false)]), vec![]),
            ("nové schválení", seen(&[], &[("a", Working, false)]), seen(&[(1, "a")], &[("a", Working, false)]), vec![("a", Event::NeedsInput)]),
            ("dvě nové žádosti jedné session = jedna událost", seen(&[], &[]), seen(&[(1, "a"), (2, "a")], &[]), vec![("a", Event::NeedsInput)]),
            ("vyřízená žádost nic nehlásí", seen(&[(1, "a")], &[]), seen(&[], &[]), vec![]),
            ("konec turnu", seen(&[], &[("a", Working, false)]), seen(&[], &[("a", Idle, false)]), vec![("a", Event::Done)]),
            ("ukončení session není hotovo", seen(&[], &[("a", Working, false)]), seen(&[], &[("a", Idle, true)]), vec![]),
            ("neznámá session v klidu (Ledge se připojil) nic", seen(&[], &[]), seen(&[], &[("a", Idle, false)]), vec![]),
            ("idle → idle nic", seen(&[], &[("a", Idle, false)]), seen(&[], &[("a", Idle, false)]), vec![]),
            (
                "obojí naráz, seřazeno",
                seen(&[], &[("b", Working, false), ("a", Working, false)]),
                seen(&[(7, "a")], &[("b", Idle, false), ("a", Working, false)]),
                vec![("a", Event::NeedsInput), ("b", Event::Done)],
            ),
        ];
        for (name, prev, now, want) in cases {
            let got = transitions(&prev, &now);
            let want: Vec<(String, Event)> = want.into_iter().map(|(s, e)| (s.to_string(), e)).collect();
            assert_eq!(got, want, "{name}");
        }
    }

    #[test]
    fn dedup_window() {
        let mut d = Dedup::default();
        assert!(d.allow("a", Event::Done, 0));
        assert!(!d.allow("a", Event::Done, 10_000), "do 30 s ne");
        assert!(d.allow("a", Event::NeedsInput, 10_000), "jiná událost ano");
        assert!(d.allow("b", Event::Done, 10_000), "jiná session ano");
        assert!(!d.allow("a", Event::Done, 29_999));
        assert!(d.allow("a", Event::Done, 30_000), "po 30 s znovu");
    }

    #[test]
    fn react_table() {
        let base = Gate::default();
        let cases = [
            ("výchozí: bliknutí + puls, toast vypnutý", base, Reaction { flash: true, pulse: true, toast: false }),
            ("mrtvý (fullscreen / herní režim) = nic", Gate { dead: true, toast_on: true, ..base }, Reaction::default()),
            ("bez animací = bez pulsu", Gate { no_motion: true, ..base }, Reaction { flash: true, pulse: false, toast: false }),
            ("toast zapnutý", Gate { toast_on: true, ..base }, Reaction { flash: true, pulse: true, toast: true }),
            ("toast při nerušit / hře (QUNS) ne", Gate { toast_on: true, quiet: true, ..base }, Reaction { flash: true, pulse: true, toast: false }),
            ("toast při otevřeném šuplíku ne", Gate { toast_on: true, drawer_full: true, ..base }, Reaction { flash: true, pulse: true, toast: false }),
        ];
        for (name, g, want) in cases {
            assert_eq!(react(g), want, "{name}");
        }
    }

    #[test]
    fn repulse_table() {
        /// (popis, čeká otázka, proužek vidět, ms od pulsu, zopakovat teď, za kolik ms další)
        type Case = (&'static str, bool, bool, Option<u64>, bool, Option<u64>);
        let cases: &[Case] = &[
            ("nic nečeká = nic", false, true, None, false, None),
            ("šuplík otevřený / hra = proužek není vidět", true, false, None, false, None),
            ("čeká, puls ještě nebyl", true, true, None, true, Some(0)),
            ("čeká, puls před chvílí", true, true, Some(2_000), false, Some(REPULSE_MS - 2_000)),
            ("čeká, puls před odstupem", true, true, Some(REPULSE_MS), true, Some(0)),
            ("čeká, puls dávno", true, true, Some(10 * REPULSE_MS), true, Some(0)),
        ];
        for (name, asking, shown, ago, due, next) in cases {
            assert_eq!(repulse_due(*asking, *shown, *ago), *due, "{name}");
            assert_eq!(next_repulse(*asking, *shown, *ago), *next, "{name}");
        }
    }

    /// Mezi opakovanými pulsy je klid (puls je kratší než odstup) — hlídá překladač.
    const _: () = assert!(REPULSE_MS > PULSE_MS);

    #[test]
    fn revive_peek_table() {
        // (byl mrtvý, je mrtvý, šuplík zavřený, něco čeká) → peek
        let cases = [
            ((true, false, true, true), true),
            ((true, false, true, false), false),
            ((true, false, false, true), false),
            ((true, true, true, true), false),
            ((false, false, true, true), false),
            ((false, true, true, true), false),
        ];
        for ((was, dead, hidden, att), want) in cases {
            assert_eq!(revive_peek(was, dead, hidden, att), want, "{was} {dead} {hidden} {att}");
        }
    }

    #[test]
    fn pulse_level_shape() {
        assert_eq!(pulse_level(0), Some(0.0));
        let peak = pulse_level(PULSE_MS / 4).unwrap();
        assert!((peak - 1.0).abs() < 1e-4, "vrchol první vlny: {peak}");
        assert!(pulse_level(PULSE_MS / 2).unwrap() < 1e-4, "mezi vlnami do nuly");
        assert!((pulse_level(3 * PULSE_MS / 4).unwrap() - 1.0).abs() < 1e-4, "druhá vlna");
        assert_eq!(pulse_level(PULSE_MS), None, "po termínu konec");
        for t in (0..PULSE_MS).step_by(37) {
            let l = pulse_level(t).unwrap();
            assert!((0.0..=1.0).contains(&l), "{t}: {l}");
        }
    }

    #[test]
    fn meeting_ranks_between_waiting_and_done() {
        let mut evs = [Event::Done, Event::Meeting, Event::NeedsInput];
        evs.sort_by_key(|e| e.rank());
        assert_eq!(evs, [Event::NeedsInput, Event::Meeting, Event::Done]);
        assert_eq!(Event::Meeting.color(), Rgba::BLUE);
        assert_ne!(Event::Meeting.color(), Event::NeedsInput.color());
    }

    #[test]
    fn github_events_rank_after_meeting_before_done() {
        let mut evs = [Event::Done, Event::GithubReview, Event::GithubCi, Event::Meeting, Event::NeedsInput];
        evs.sort_by_key(|e| e.rank());
        assert_eq!(evs, [Event::NeedsInput, Event::Meeting, Event::GithubCi, Event::GithubReview, Event::Done]);
        assert_eq!(Event::GithubCi.color(), Rgba::RED);
        assert_eq!(Event::GithubReview.color(), Rgba::VIOLET);
        // žádná z nových událostí nesdílí barvu s „čeká na tebe“ / „hotovo“
        for e in [Event::GithubCi, Event::GithubReview] {
            assert!(e.color() != Event::NeedsInput.color() && e.color() != Event::Done.color());
        }
        // souběh s Claudem: čekající Claude drží barvu
        let mixed = [(String::new(), Event::GithubCi), (String::new(), Event::NeedsInput)];
        assert_eq!(mixed.iter().min_by_key(|(_, e)| e.rank()).map(|(_, e)| *e), Some(Event::NeedsInput));
    }

    #[test]
    fn github_toast_texts() {
        assert_eq!(toast_text(Event::GithubCi, false, "").0, "Spadlo CI na tvém PR");
        assert_eq!(with_lang(Lang::En, || toast_text(Event::GithubReview, false, "").0), "A PR is waiting for your review");
    }

    #[test]
    fn notices_open_session_never_allow() {
        use crate::notify::{Action, Kind};
        let n = notice_of(Event::NeedsInput, "s1", "Claude čeká na schválení".into(), "app".into(), Some("Bash"));
        assert_eq!((n.kind, n.body.as_str()), (Kind::Waiting, "app · Bash"));
        assert_eq!(n.action, Some(Action::Session("s1".into())));
        assert_eq!(n.button.as_deref(), Some("Otevřít"));
        let n = notice_of(Event::NeedsInput, "s1", "t".into(), String::new(), Some("Edit"));
        assert_eq!(n.body, "Edit");
        let n = notice_of(Event::Done, "s1", "Claude je hotový".into(), "app".into(), Some("Bash"));
        assert_eq!((n.kind, n.body.as_str(), n.button, n.action), (Kind::Done, "app", None, None));
    }

    #[test]
    fn done_switch_follows_toast_unless_set() {
        // (toast, toastDone, Čeká, Hotovo)
        let rows = [(false, None, false, false), (true, None, true, true), (true, Some(false), true, false), (false, Some(true), false, true)];
        for (t, d, w, dn) in rows {
            let done_on = done_toast_on(t, d);
            assert_eq!(event_toast_on(Event::NeedsInput, t, done_on), w, "{t} {d:?}");
            assert_eq!(event_toast_on(Event::Done, t, done_on), dn, "{t} {d:?}");
        }
    }

    #[test]
    fn toast_texts() {
        assert_eq!(toast_text(Event::NeedsInput, false, "Ledge"), ("Claude čeká na schválení".into(), "Ledge".into()));
        assert_eq!(toast_text(Event::NeedsInput, true, "Ledge").0, "Claude se ptá");
        assert_eq!(with_lang(Lang::En, || toast_text(Event::Done, false, "")), ("Claude is done".into(), String::new()));
    }
}
