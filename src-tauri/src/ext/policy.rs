//! Rozhodovací logika supervisoru rozšíření — čisté funkce bez I/O a bez hodin (čas přichází parametrem),
//! aby šla tabulkově testovat: restart backoff, nečinnost, pauza při hře, životní fáze, koalescence dat,
//! whitelist požadavků `req`. Dráty a vlákna jsou v `sup.rs`.

use std::collections::HashMap;
use std::time::{Duration, Instant};

// ---------- manifest ----------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Start {
    /// běží, dokud je v configu (po startu Ledge s odstupem, po konci hry znovu)
    Always,
    /// jen na `ext_wake` (klik, událost); po pádu se sám nerestartuje
    OnDemand,
}

/// Neměnný popis rozšíření (konstanta v Rustu, ne config).
#[derive(Debug, Clone, Copy)]
pub struct Manifest {
    pub id: &'static str,
    pub start: Start,
    /// konec po tolika minutách bez `active`; `None` = nikdy
    pub idle_quit_min: Option<u16>,
    /// konec po tolika minutách pauzy (hra); `None` = nikdy
    pub pause_quit_min: Option<u16>,
    /// témata dat, která smí rozšíření odebírat
    pub wants: &'static [&'static str],
}

/// Témata, která smí ext odebírat: průnik jeho `hello.wants` s manifestem (ext si cizí téma nevynutí).
pub fn granted_topics(m: &Manifest, wants: &[String]) -> Vec<String> {
    m.wants.iter().filter(|t| wants.iter().any(|w| w == **t)).map(|t| t.to_string()).collect()
}

// ---------- restart ----------

/// Okno pro počítání pádů.
pub const CRASH_WINDOW: Duration = Duration::from_secs(600);
/// Po tak dlouhém běhu od `ready` se počítadlo pádů nuluje.
pub const STABLE_RUN: Duration = Duration::from_secs(60);

/// Prodleva před dalším spuštěním po `crashes`-tém pádu v okně: 1. → 2 s, 2. → 10 s, 3. → 60 s, 4. → `None` = „selhalo“.
pub fn restart_delay(crashes: u8) -> Option<Duration> {
    match crashes {
        0 => Some(Duration::ZERO),
        1 => Some(Duration::from_secs(2)),
        2 => Some(Duration::from_secs(10)),
        3 => Some(Duration::from_secs(60)),
        _ => None,
    }
}

/// Zapíše pád do historie a vrátí počet pádů v okně. Pád po `STABLE_RUN` běhu od `ready` historii nejdřív smaže.
pub fn record_crash(history: &mut Vec<Instant>, ready_at: Option<Instant>, now: Instant) -> u8 {
    if ready_at.is_some_and(|r| now.saturating_duration_since(r) >= STABLE_RUN) {
        history.clear();
    }
    history.retain(|t| now.saturating_duration_since(*t) < CRASH_WINDOW);
    history.push(now);
    history.len().min(u8::MAX as usize) as u8
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExitKind {
    Clean,
    Crash,
}

/// Jak skončil proces: čistě = kód 0 a (řekl `bye` nebo jsme ho o konec požádali). Jinak pád
/// (nenulový kód, EOF bez `bye`, zabití po vypršení handshake).
pub fn exit_kind(code_zero: bool, said_bye: bool, asked_quit: bool, killed_late: bool) -> ExitKind {
    if code_zero && (said_bye || asked_quit) && !killed_late { ExitKind::Clean } else { ExitKind::Crash }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum After {
    /// nic: čistý konec, nebo pád OnDemand rozšíření (znovu až při dalším `wake`)
    Idle,
    Restart(Duration),
    Failed,
}

pub fn after_exit(start: Start, kind: ExitKind, crashes_in_window: u8) -> After {
    match (kind, start) {
        (ExitKind::Clean, _) | (ExitKind::Crash, Start::OnDemand) => After::Idle,
        (ExitKind::Crash, Start::Always) => restart_delay(crashes_in_window).map_or(After::Failed, After::Restart),
    }
}

// ---------- nečinnost a pauza ----------

/// Termín konce pro nečinnost: poslední `active` + limit. `None` = rozšíření nečinností nekončí.
pub fn idle_deadline(last_active: Instant, idle_min: Option<u16>) -> Option<Instant> {
    idle_min.map(|m| last_active + Duration::from_secs(m as u64 * 60))
}

/// Termín konce pro dlouhou pauzu (hra): začátek pauzy + limit.
pub fn pause_deadline(paused_since: Option<Instant>, pause_min: Option<u16>) -> Option<Instant> {
    Some(paused_since? + Duration::from_secs(pause_min? as u64 * 60))
}

/// Co Ledge ví o stavu stroje.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Facts {
    /// fullscreen hra nebo herní režim (`Core::dead`)
    pub dead: bool,
}

/// Důvod pauzy rozšíření (`state.why`), `None` = běžet. Další důvody (`lock`…) přibudou jen s reálným zdrojem.
pub fn pause_reason(f: Facts) -> Option<&'static str> {
    f.dead.then_some("game")
}

// ---------- životní fáze jednoho běhu ----------

pub const HELLO_TIMEOUT: Duration = Duration::from_secs(5);
pub const READY_TIMEOUT: Duration = Duration::from_secs(15);
pub const QUIT_GRACE: Duration = Duration::from_secs(3);

/// Lhůty handshake a ukončení (v testech zkrácené).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Timing {
    pub hello: Duration,
    pub ready: Duration,
    pub grace: Duration,
}

impl Default for Timing {
    fn default() -> Self {
        Timing { hello: HELLO_TIMEOUT, ready: READY_TIMEOUT, grace: QUIT_GRACE }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Stage {
    /// proces spuštěn, čekáme na `hello`
    Spawned,
    /// `hello` přišlo, `init` poslán, čekáme na `ready`
    Greeted,
    Ready,
    /// poslali jsme `quit`, čeká se na konec
    Quitting,
}

#[derive(Debug, Clone, Copy)]
pub struct Life {
    pub stage: Stage,
    pub stage_at: Instant,
    pub last_active: Instant,
    pub paused_since: Option<Instant>,
    pub timing: Timing,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Due {
    HelloLate,
    ReadyLate,
    Idle,
    PauseLong,
    /// po `quit` neskončil v lhůtě → `job.kill()`
    GraceOver,
}

/// Co s termínem: požádat o konec (`quit`), nebo zabít.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Act {
    Quit,
    Kill,
}

pub fn act_for(d: Due) -> Act {
    match d {
        Due::Idle | Due::PauseLong => Act::Quit,
        Due::HelloLate | Due::ReadyLate | Due::GraceOver => Act::Kill,
    }
}

impl Life {
    #[allow(dead_code)] // jen testy životního cyklu
    pub fn spawned(now: Instant) -> Self {
        Self::with_timing(now, Timing::default())
    }
    pub fn with_timing(now: Instant, timing: Timing) -> Self {
        Life { stage: Stage::Spawned, stage_at: now, last_active: now, paused_since: None, timing }
    }
    pub fn greeted(&mut self, now: Instant) {
        if self.stage == Stage::Spawned {
            self.stage = Stage::Greeted;
            self.stage_at = now;
        }
    }
    pub fn ready(&mut self, now: Instant) {
        if self.stage == Stage::Greeted {
            self.stage = Stage::Ready;
            self.stage_at = now;
            self.last_active = now;
        }
    }
    pub fn active(&mut self, now: Instant) {
        self.last_active = now;
    }
    pub fn quitting(&mut self, now: Instant) {
        if self.stage != Stage::Quitting {
            self.stage = Stage::Quitting;
            self.stage_at = now;
        }
    }
    pub fn set_pause(&mut self, pause: bool, now: Instant) {
        match (pause, self.paused_since) {
            (true, None) => self.paused_since = Some(now),
            (false, Some(_)) => {
                self.paused_since = None;
                self.last_active = now; // konec hry není nečinnost
            }
            _ => {}
        }
    }
    /// Nejbližší termín a jeho druh. Žádný periodický tik — vlákno spí do tohoto okamžiku.
    pub fn next_due(&self, m: &Manifest) -> Option<(Instant, Due)> {
        match self.stage {
            Stage::Spawned => Some((self.stage_at + self.timing.hello, Due::HelloLate)),
            Stage::Greeted => Some((self.stage_at + self.timing.ready, Due::ReadyLate)),
            Stage::Quitting => Some((self.stage_at + self.timing.grace, Due::GraceOver)),
            Stage::Ready => match self.paused_since {
                // v pauze rozhoduje jen pauza; po hře se nečinnost počítá znovu od konce pauzy
                Some(_) => pause_deadline(self.paused_since, m.pause_quit_min).map(|t| (t, Due::PauseLong)),
                None => idle_deadline(self.last_active, m.idle_quit_min).map(|t| (t, Due::Idle)),
            },
        }
    }
}

// ---------- plán synchronizace s configem ----------

/// Veřejná fáze rozšíření (do `status()` a Nastavení).
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Phase {
    /// neběží (vypnuto, nebo OnDemand čeká, nebo skončilo)
    Idle,
    Starting,
    Running,
    /// hra / zámek: okno schované, data nejdou
    Paused,
    /// čeká na restart po pádu
    Backoff,
    /// 4. pád v okně — dál se nezkouší
    Failed,
    /// `hello` s jinou verzí kontraktu
    Incompatible,
}

impl Phase {
    /// Drží proces, vlákno nebo plánovaný restart?
    pub fn is_active(self) -> bool {
        matches!(self, Phase::Starting | Phase::Running | Phase::Paused | Phase::Backoff)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SyncAct {
    Start(String),
    /// požádat o konec a zapomenout slot (i `Failed` / `Incompatible`)
    Stop(String),
}

/// Co udělat po změně configu / konce hry: spustit povolená `Always` rozšíření, která neběží (ne ve hře, ne po
/// selhání), a zastavit běžící / selhaná, která už povolená nejsou. Neznámá id se ignorují.
pub fn plan_sync(enabled: &[String], known: &[Manifest], slots: &[(String, Phase)], dead: bool) -> Vec<SyncAct> {
    let mut out = Vec::new();
    for (id, phase) in slots {
        let on = enabled.iter().any(|e| e == id);
        if !on && (phase.is_active() || matches!(phase, Phase::Failed | Phase::Incompatible)) {
            out.push(SyncAct::Stop(id.clone()));
        }
    }
    if dead {
        return out;
    }
    for id in enabled {
        let Some(m) = known.iter().find(|m| m.id == id) else { continue };
        let phase = slots.iter().find(|(s, _)| s == id).map_or(Phase::Idle, |(_, p)| *p);
        if m.start == Start::Always && phase == Phase::Idle && !out.contains(&SyncAct::Start(id.clone())) {
            out.push(SyncAct::Start(id.clone()));
        }
    }
    out
}

// ---------- koalescence dat dolů ----------

/// Minimální odstup dvou odeslání téhož tématu.
pub const COALESCE: Duration = Duration::from_secs(1);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Offer {
    Send,
    /// stejný otisk jako naposledy odeslaný — nic neposílat
    Same,
    /// příliš brzy; odeslat poslední hodnotu v tomto okamžiku (`take_due`)
    Defer(Instant),
}

#[derive(Debug, Default)]
pub struct Coalescer {
    /// téma → (otisk, kdy odesláno)
    sent: HashMap<String, (u64, Instant)>,
    /// téma → otisk odložené hodnoty
    pending: HashMap<String, u64>,
}

impl Coalescer {
    pub fn offer(&mut self, topic: &str, fp: u64, now: Instant) -> Offer {
        match self.sent.get(topic) {
            Some((last_fp, _)) if *last_fp == fp => {
                self.pending.remove(topic); // hodnota se vrátila na odeslanou
                Offer::Same
            }
            Some((_, at)) if now < *at + COALESCE => {
                self.pending.insert(topic.to_string(), fp);
                Offer::Defer(*at + COALESCE)
            }
            _ => {
                self.mark_sent(topic, fp, now);
                Offer::Send
            }
        }
    }
    pub fn mark_sent(&mut self, topic: &str, fp: u64, now: Instant) {
        self.sent.insert(topic.to_string(), (fp, now));
        self.pending.remove(topic);
    }
    /// Nejbližší termín odložené hodnoty.
    pub fn next_due(&self) -> Option<Instant> {
        self.pending.keys().filter_map(|t| self.sent.get(t).map(|(_, at)| *at + COALESCE)).min()
    }
    /// Témata, jejichž odklad vypršel: označí je odeslaná a vrátí (volající pošle poslední hodnotu).
    pub fn take_due(&mut self, now: Instant) -> Vec<String> {
        let due: Vec<String> = self
            .pending
            .keys()
            .filter(|t| self.sent.get(*t).is_none_or(|(_, at)| now >= *at + COALESCE))
            .cloned()
            .collect();
        for t in &due {
            if let Some(fp) = self.pending.remove(t) {
                self.sent.insert(t.clone(), (fp, now));
            }
        }
        due
    }
    /// Pauza / nový běh: zapomenout vše (po `resume` jde čerstvý snapshot všech témat).
    pub fn reset(&mut self) {
        self.sent.clear();
        self.pending.clear();
    }
}

// ---------- požadavky ext → Ledge ----------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MediaAct {
    Toggle,
    Next,
    Prev,
}

/// Povolené požadavky rozšíření. Pevná whitelist; cokoli jiného (hlavně schvalování akcí Clauda, `/ctl/decide`)
/// se zahodí a zaloguje — povolit akci Clauda smí jen člověk.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Req {
    Cover(String),
    Media(MediaAct),
    OpenSettings,
}

pub fn parse_req(what: &str, act: Option<&str>, key: Option<&str>) -> Option<Req> {
    match (what, act) {
        ("cover", _) => key.filter(|k| !k.is_empty() && k.len() <= 256).map(|k| Req::Cover(k.to_string())),
        ("media", Some("toggle")) => Some(Req::Media(MediaAct::Toggle)),
        ("media", Some("next")) => Some(Req::Media(MediaAct::Next)),
        ("media", Some("prev")) => Some(Req::Media(MediaAct::Prev)),
        ("open_settings", _) => Some(Req::OpenSettings),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const WALL: Manifest = Manifest { id: "wall", start: Start::Always, idle_quit_min: None, pause_quit_min: Some(10), wants: &["media"] };
    const BROWSER: Manifest = Manifest { id: "browser", start: Start::OnDemand, idle_quit_min: Some(10), pause_quit_min: Some(5), wants: &[] };

    fn t0() -> Instant {
        Instant::now()
    }
    fn s(secs: u64) -> Duration {
        Duration::from_secs(secs)
    }

    #[test]
    fn restart_delay_table() {
        let cases = [(0, Some(0)), (1, Some(2)), (2, Some(10)), (3, Some(60)), (4, None), (200, None)];
        for (n, want) in cases {
            assert_eq!(restart_delay(n), want.map(Duration::from_secs), "pádů {n}");
        }
    }

    #[test]
    fn record_crash_window_and_stable_run() {
        let now = t0();
        let mut h = vec![];
        assert_eq!(record_crash(&mut h, None, now), 1);
        assert_eq!(record_crash(&mut h, None, now + s(5)), 2);
        assert_eq!(record_crash(&mut h, None, now + s(20)), 3);
        // pád po 11 minutách: starší než okno vypadnou
        assert_eq!(record_crash(&mut h, None, now + s(20 + 601)), 1);
        // dlouhý běh po ready vynuluje i čerstvé pády
        let mut h = vec![now, now + s(1)];
        assert_eq!(record_crash(&mut h, Some(now), now + s(90)), 1);
        // krátký běh nevynuluje
        let mut h = vec![now];
        assert_eq!(record_crash(&mut h, Some(now + s(10)), now + s(50)), 2);
    }

    #[test]
    fn exit_and_after_table() {
        // (kód 0, bye, požádáno, zabito pozdě) → druh
        let cases = [
            (true, true, false, false, ExitKind::Clean),
            (true, false, true, false, ExitKind::Clean),
            (true, false, false, false, ExitKind::Crash), // EOF bez bye
            (false, true, false, false, ExitKind::Crash),
            (false, false, true, false, ExitKind::Crash),
            (true, false, true, true, ExitKind::Crash),
        ];
        for (code, bye, asked, late, want) in cases {
            assert_eq!(exit_kind(code, bye, asked, late), want, "{code} {bye} {asked} {late}");
        }
        assert_eq!(after_exit(Start::Always, ExitKind::Clean, 0), After::Idle);
        assert_eq!(after_exit(Start::OnDemand, ExitKind::Crash, 1), After::Idle);
        assert_eq!(after_exit(Start::Always, ExitKind::Crash, 1), After::Restart(s(2)));
        assert_eq!(after_exit(Start::Always, ExitKind::Crash, 3), After::Restart(s(60)));
        assert_eq!(after_exit(Start::Always, ExitKind::Crash, 4), After::Failed);
    }

    #[test]
    fn idle_and_pause_deadline_table() {
        let t = t0();
        assert_eq!(idle_deadline(t, None), None);
        assert_eq!(idle_deadline(t, Some(10)), Some(t + s(600)));
        assert_eq!(pause_deadline(None, Some(10)), None);
        assert_eq!(pause_deadline(Some(t), None), None);
        assert_eq!(pause_deadline(Some(t), Some(5)), Some(t + s(300)));
    }

    #[test]
    fn pause_reason_table() {
        assert_eq!(pause_reason(Facts { dead: true }), Some("game"));
        assert_eq!(pause_reason(Facts { dead: false }), None);
        assert_eq!(pause_reason(Facts::default()), None);
    }

    #[test]
    fn life_deadlines_follow_stage() {
        let t = t0();
        let mut l = Life::spawned(t);
        assert_eq!(l.next_due(&WALL), Some((t + HELLO_TIMEOUT, Due::HelloLate)));
        l.greeted(t + s(1));
        assert_eq!(l.next_due(&WALL), Some((t + s(1) + READY_TIMEOUT, Due::ReadyLate)));
        l.ready(t + s(2));
        // wall: nečinnost nikdy, pauza po 10 min
        assert_eq!(l.next_due(&WALL), None);
        assert_eq!(l.next_due(&BROWSER), Some((t + s(2 + 600), Due::Idle)));
        l.active(t + s(100));
        assert_eq!(l.next_due(&BROWSER), Some((t + s(100 + 600), Due::Idle)));
        // pauza: rozhoduje jen pauza
        l.set_pause(true, t + s(200));
        assert_eq!(l.next_due(&WALL), Some((t + s(200 + 600), Due::PauseLong)));
        assert_eq!(l.next_due(&BROWSER), Some((t + s(200 + 300), Due::PauseLong)));
        // konec pauzy = nečinnost znovu od konce pauzy
        l.set_pause(false, t + s(250));
        assert_eq!(l.next_due(&BROWSER), Some((t + s(250 + 600), Due::Idle)));
        assert_eq!(l.next_due(&WALL), None);
        l.quitting(t + s(300));
        assert_eq!(l.next_due(&WALL), Some((t + s(300) + QUIT_GRACE, Due::GraceOver)));
    }

    #[test]
    fn life_stage_transitions_are_ordered() {
        let t = t0();
        let mut l = Life::spawned(t);
        l.ready(t + s(1)); // ready před hello se ignoruje
        assert_eq!(l.stage, Stage::Spawned);
        l.greeted(t + s(1));
        l.greeted(t + s(3)); // druhé hello nic nemění
        assert_eq!((l.stage, l.stage_at), (Stage::Greeted, t + s(1)));
        l.ready(t + s(2));
        assert_eq!(l.stage, Stage::Ready);
        l.quitting(t + s(5));
        l.quitting(t + s(9)); // druhý quit neprodlužuje lhůtu
        assert_eq!(l.stage_at, t + s(5));
    }

    #[test]
    fn act_for_table() {
        assert_eq!(act_for(Due::Idle), Act::Quit);
        assert_eq!(act_for(Due::PauseLong), Act::Quit);
        assert_eq!(act_for(Due::HelloLate), Act::Kill);
        assert_eq!(act_for(Due::ReadyLate), Act::Kill);
        assert_eq!(act_for(Due::GraceOver), Act::Kill);
    }

    fn v(xs: &[&str]) -> Vec<String> {
        xs.iter().map(|x| x.to_string()).collect()
    }

    #[test]
    fn plan_sync_table() {
        let known = [WALL, BROWSER];
        let st = |id: &str, p| (id.to_string(), p);
        // nic povoleno, nic neběží
        assert!(plan_sync(&[], &known, &[], false).is_empty());
        // povolené Always bez slotu → Start
        assert_eq!(plan_sync(&v(&["wall"]), &known, &[], false), vec![SyncAct::Start("wall".into())]);
        // ve hře se nestartuje
        assert!(plan_sync(&v(&["wall"]), &known, &[st("wall", Phase::Idle)], true).is_empty());
        // běží → nic
        assert!(plan_sync(&v(&["wall"]), &known, &[st("wall", Phase::Running)], false).is_empty());
        // selhalo zůstává selhané, dokud ho uživatel nevypne
        assert!(plan_sync(&v(&["wall"]), &known, &[st("wall", Phase::Failed)], false).is_empty());
        // OnDemand se nestartuje samo; neznámé id se ignoruje
        assert!(plan_sync(&v(&["browser", "z-budoucna"]), &known, &[], false).is_empty());
        // odebráno z configu: běžící i selhané → Stop (i ve hře)
        assert_eq!(plan_sync(&[], &known, &[st("wall", Phase::Paused)], true), vec![SyncAct::Stop("wall".into())]);
        assert_eq!(plan_sync(&[], &known, &[st("wall", Phase::Failed)], false), vec![SyncAct::Stop("wall".into())]);
        assert_eq!(plan_sync(&[], &known, &[st("browser", Phase::Backoff)], false), vec![SyncAct::Stop("browser".into())]);
        // vypnuté a nečinné → nic
        assert!(plan_sync(&[], &known, &[st("wall", Phase::Idle)], false).is_empty());
    }

    #[test]
    fn granted_topics_intersects_manifest() {
        assert_eq!(granted_topics(&WALL, &v(&["media", "clock", "agenda"])), v(&["media"]));
        assert!(granted_topics(&BROWSER, &v(&["media"])).is_empty());
        assert!(granted_topics(&WALL, &[]).is_empty());
    }

    #[test]
    fn coalescer_table() {
        let t = t0();
        let mut c = Coalescer::default();
        assert_eq!(c.offer("media", 1, t), Offer::Send);
        assert_eq!(c.offer("media", 1, t + s(5)), Offer::Same);
        // změna před uplynutím minima → odložit, poslední vyhrává
        assert_eq!(c.offer("media", 2, t + Duration::from_millis(300)), Offer::Defer(t + COALESCE));
        assert_eq!(c.offer("media", 3, t + Duration::from_millis(600)), Offer::Defer(t + COALESCE));
        assert_eq!(c.next_due(), Some(t + COALESCE));
        assert!(c.take_due(t + Duration::from_millis(900)).is_empty());
        assert_eq!(c.take_due(t + COALESCE), vec!["media".to_string()]);
        assert_eq!(c.next_due(), None);
        // po odeslání odloženého se měří odstup od něj
        assert_eq!(c.offer("media", 4, t + COALESCE + Duration::from_millis(10)), Offer::Defer(t + COALESCE * 2));
        // hodnota se vrátila na odeslanou → odklad zmizí
        assert_eq!(c.offer("media", 3, t + COALESCE + Duration::from_millis(20)), Offer::Same);
        assert_eq!(c.next_due(), None);
        // témata jsou nezávislá
        assert_eq!(c.offer("agenda", 9, t + COALESCE), Offer::Send);
        // po resetu (pauza) jde zase hned
        c.reset();
        assert_eq!(c.offer("media", 3, t + COALESCE), Offer::Send);
    }

    #[test]
    fn req_whitelist_table() {
        type Case<'a> = (&'a str, Option<&'a str>, Option<&'a str>, Option<Req>);
        let cases: &[Case] = &[
            ("cover", None, Some("abc"), Some(Req::Cover("abc".into()))),
            ("cover", None, None, None),
            ("cover", None, Some(""), None),
            ("media", Some("next"), None, Some(Req::Media(MediaAct::Next))),
            ("media", Some("prev"), None, Some(Req::Media(MediaAct::Prev))),
            ("media", Some("toggle"), None, Some(Req::Media(MediaAct::Toggle))),
            ("media", Some("seek"), None, None),
            ("media", None, None, None),
            ("open_settings", None, None, Some(Req::OpenSettings)),
            // nic z toho, co by schvalovalo / spouštělo
            ("decide", Some("allow"), None, None),
            ("ctl", Some("decide"), None, None),
            ("approve", None, None, None),
            ("exec", Some("cmd"), None, None),
            ("", None, None, None),
        ];
        for (what, act, key, want) in cases {
            assert_eq!(&parse_req(what, *act, *key), want, "{what} {act:?}");
        }
        assert_eq!(parse_req("cover", None, Some(&"k".repeat(300))), None);
    }
}
