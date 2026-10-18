//! Snapshot pro UI (`get_state`, událost `state`): projekty, procesy, Claude, poznámky, Osobní.
//! Jen čte paměť `Core` (cache gitu, log, odkazů, počasí) — žádné I/O na emit/UI vlákně.

use crate::config::{self, Config, ProjectKind};
use crate::drawer::View;
use crate::gitinfo::GitInfo;
use crate::state::{core, now_ms};
use crate::{claude, cloning, hooksetup, notes, notesync, personal, procman, resolve, sessionlive, sessions, update, weather};
use serde::Serialize;

// ---------- snapshot pro UI ----------

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ProjectView {
    id: String,
    name: String,
    kind: ProjectKind,
    accent: String,
    monogram: String,
    /// klíč obsahu loga (hash); data URL přes `project_logo` — base64 nejde s každým snapshotem
    logo_key: Option<String>,
    path: Option<String>,
    path_ok: bool,
    /// složka chybí a jde naklonovat (`cloning.rs`); `cloning` = klon právě běží
    clonable: bool,
    cloning: bool,
    github: Option<String>,
    links: Vec<config::Link>,
    git: Option<GitInfo>,
    run: Option<String>,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Snapshot {
    mode: String,
    personal_enabled: bool,
    view: View,
    cfg_error: Option<String>,
    sections: config::Sections,
    /// záložky a karty na nich (`layout.rs`); `mode` = id záložky, která je vidět
    layout: crate::layout::LayoutConfig,
    claude_enabled: bool,
    projects: Vec<ProjectView>,
    procs: Vec<procman::ProcView>,
    pending: Vec<claude::Pending>,
    /// otázky Clauda (AskUserQuestion) čekající na odpověď v Ledge / jen ukázané
    questions: Vec<claude::Ask>,
    /// všechny Claude session na PC (`sessions.rs`), seřazené pro kartu
    sessions: Vec<sessions::Row>,
    /// hooky Ledge v `~/.claude/settings.json`
    hooks: hooksetup::HooksStatus,
    /// limity Claude 5h / týden (`limitslive::view`, jen paměť)
    limits: crate::limitslive::LimitsView,
    /// časovač: odpočet a stopky jako časy (sekundy si UI počítá samo) + předvolby (`timer::view`)
    timer: crate::timer::TimerView,
    /// útrata Claude — odhad v cenách API (`costslive::view`, jen paměť)
    costs: crate::costslive::CostsView,
    notes_enabled: bool,
    /// kompaktní paleta (`palette.compact`, `src/palette/compact.ts`)
    palette_compact: bool,
    /// hledání stránek Nastavení Windows v paletě (`palette.winSettings`, `src/palette/winsettings.ts`)
    palette_win_settings: bool,
    /// skripty jako dlaždice karty Rychlé akce (`scripts.inActions`; jinak prázdné — paleta si je načte sama)
    #[serde(skip_serializing_if = "Vec::is_empty")]
    scripts: Vec<crate::scripts::ScriptView>,
    /// karta Poznámky: nehotové připomínky do konce dneška, pak nejnovější — nejvýš `CARD_ROWS`
    notes: Vec<notes::NoteView>,
    /// kolik poznámek se do karty nevešlo (načtou se přes `note_list`)
    notes_more: usize,
    /// první připomínky po termínu (peek), celkový počet zvlášť
    reminders: Vec<notes::NoteView>,
    reminders_count: usize,
    /// kategorie poznámek z configu (záložky, výběr) a počty nehotových podle `kind`
    note_kinds: Vec<config::NoteKind>,
    notes_kind_counts: std::collections::BTreeMap<String, usize>,
    /// roste s každým přenačtením poznámek — UI podle něj obnoví výběr záložky (`note_query`)
    notes_rev: u64,
    personal: PersonalView,
    /// synchronizace poznámek mezi PC (štítek v kartě Poznámky)
    notes_sync: notesync::Status,
    /// nová verze Ledge: stav poslední kontroly (chip v hlavičce jen při `dostupna`)
    update: update::UpdateView,
    /// rychlé akce (karta, paleta) — ikony jen jako `iconKey` (`action_icon`)
    actions: Vec<crate::actionstate::ActionView>,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PersonalView {
    weather: WeatherView,
    links: Vec<LinkView>,
    /// obecné poznámky (bez projektu): nehotové připomínky do konce dneška, pak nejnovější — nejvýš `CARD_ROWS`
    notes: Vec<notes::NoteView>,
    /// kolik obecných poznámek se nevešlo
    notes_more: usize,
    /// právě hraje (`media::view`, jen paměť); `None` = nic nehraje / karta vypnutá / ještě nespuštěno
    media: Option<crate::media::MediaView>,
    /// vzhled karty Právě hraje (`media` v configu, `mediaconf.rs`) — i bez skladby (karta Zvuk, přepínač)
    media_look: crate::mediaconf::MediaConfig,
    /// hlasitost výchozího výstupu (`volume::view`); `None` = bez zařízení / vypnuto
    volume: Option<crate::volume::VolumeView>,
    /// baterie (`battery::view`); `None` = stroj bez baterie / vypnuto
    battery: Option<crate::battery::BatteryView>,
    /// karta Kalendář (`calendar::live::view`, jen paměť)
    calendar: crate::calendar::live::CalendarView,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub(crate) struct WeatherView {
    /// unset · loading · ok · stale · unavailable · idle (viz `weather::status`)
    status: &'static str,
    label: String,
    data: Option<weather::Weather>,
    fetched_ms: Option<i64>,
    error: Option<String>,
    loading: bool,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub(crate) struct LinkView {
    name: String,
    target: String,
    kind: personal::LinkKind,
    monogram: String,
    /// proč nejde otevřít (jen u kind=invalid)
    problem: Option<String>,
}

pub(crate) fn github_url(remote: &str) -> Option<String> {
    resolve::owner_repo_raw(remote).map(|(o, r)| format!("https://github.com/{o}/{r}"))
}

pub(crate) fn monogram(name: &str) -> String {
    let words: Vec<&str> = name.split(|c: char| !c.is_alphanumeric()).filter(|w| !w.is_empty()).collect();
    match words.as_slice() {
        [] => "?".into(),
        [one] => one.chars().take(1).collect::<String>().to_uppercase(),
        [a, b, ..] => format!("{}{}", a.chars().next().unwrap(), b.chars().next().unwrap()).to_uppercase(),
    }
}

pub(crate) fn snapshot() -> Snapshot {
    let c = core();
    let cfg = c.cfg.read().clone();
    let git = c.git.lock().clone();
    let mut projects = Vec::new();
    for p in cfg.projects.iter().filter(|p| p.enabled) {
        let dir = c.project_dir(p);
        let path_ok = dir.as_ref().is_some_and(|d| d.is_dir());
        let clonable = !path_ok && cloning::clonable(p, path_ok, p.remote.as_deref().map_or(0, |r| c.index.lock().clones(r)));
        let logo_key = c.logos.lock().key(&p.id);
        projects.push(ProjectView {
            id: p.id.clone(),
            name: p.name.clone(),
            kind: p.kind,
            accent: p.accent.clone().unwrap_or_else(|| "#64748b".into()),
            monogram: p.monogram.clone().unwrap_or_else(|| monogram(&p.name)),
            logo_key,
            path_ok,
            clonable,
            cloning: cloning::is_busy(&p.id),
            path: dir.map(|d| d.to_string_lossy().into_owned()),
            github: p.remote.as_deref().and_then(github_url),
            links: p.links.clone(),
            git: git.get(&p.id).and_then(|(_, g)| g.clone()),
            run: p.run.clone(),
        });
    }
    let sessions = {
        let dirs: Vec<(String, String)> = projects.iter().filter_map(|p| Some((p.id.clone(), p.path.clone()?))).collect();
        session_rows(&dirs, &|id| projects.iter().find(|p| p.id == id).map(|p| p.name.clone()))
    };
    let mode = c.mode.lock().clone();
    // snapshot má pevný strop — neroste s počtem poznámek (zbytek přes note_list); platí i pro Osobní
    let (notes_kind_counts, notes_rev) = {
        let st = c.notes.lock();
        (st.kind_counts.clone(), st.rev)
    };
    let ((notes_card, notes_more), reminders, reminders_count, personal_notes) = if cfg.notes.enabled {
        let st = c.notes.lock();
        let eod = notes::end_of_day_ms(&notes::now_local());
        let general: Vec<notes::NoteView> =
            st.all.iter().filter(|n| n.project.is_none()).cloned().collect();
        (
            notes::card(&st.all, eod, notes::CARD_ROWS),
            st.due.iter().take(3).cloned().collect(),
            st.due.len(),
            notes::card(&general, eod, notes::CARD_ROWS),
        )
    } else {
        ((Vec::new(), 0), Vec::new(), 0, (Vec::new(), 0))
    };
    let personal = personal_view(&cfg, personal_notes);
    Snapshot {
        mode: crate::layout::page_id(&cfg.layout, &mode),
        personal_enabled: cfg.modes.personal_enabled,
        view: c.drawer.view(),
        cfg_error: c.cfg_error.lock().clone(),
        sections: cfg.drawer.sections.clone(),
        layout: cfg.layout.clone(),
        claude_enabled: cfg.claude.enabled,
        projects,
        procs: c.procs.snapshot(),
        pending: c.claude.pending(),
        questions: c.claude.questions(),
        sessions,
        hooks: hooksetup::status_cached(cfg.claude.port),
        limits: crate::limitslive::view(),
        timer: crate::timer::view(&cfg.timer),
        costs: crate::costslive::view(),
        notes_enabled: cfg.notes.enabled,
        palette_compact: cfg.palette.compact,
        palette_win_settings: cfg.palette.win_settings,
        scripts: crate::scripts::snapshot_views(&cfg.scripts),
        notes: notes_card,
        notes_more,
        reminders,
        reminders_count,
        note_kinds: cfg.notes.kinds.clone(),
        notes_kind_counts,
        notes_rev,
        personal,
        notes_sync: c.notes_sync.lock().clone(),
        update: c.updater.view(),
        actions: crate::actionstate::views(&cfg.actions),
    }
}

/// Karta Claude: živý seznam (`sessionlive`) + hook stav; `dirs` = (id projektu, složka).
pub(crate) fn session_rows(dirs: &[(String, String)], name_of: &dyn Fn(&str) -> Option<String>) -> Vec<sessions::Row> {
    let hooked = core().claude.sessions();
    sessionlive::with_cache(|c| {
        let live = sessions::Live { agents: c.agents.as_deref(), fetched_at: c.fetched_at, registry: &c.registry };
        sessions::merge(&live, hooked, dirs, name_of, now_ms() as u64)
    })
}

pub(crate) fn personal_view(cfg: &Config, (notes, notes_more): (Vec<notes::NoteView>, usize)) -> PersonalView {
    let place = cfg.personal.weather.coords();
    let now = now_ms();
    let weather = {
        let st = core().weather.lock();
        let here = st.place == place;
        // data k jinému místu (změna configu, stažení ještě neproběhlo) se neukazují
        // popis podle aktuálního jazyka (přepnutí jazyka nečeká na další stažení)
        let data = st.data.clone().filter(|_| here).map(|mut w| {
            w.desc = weather::wmo(w.code, w.is_day).0;
            w
        });
        let live = here && weather::loading_live(st.loading, st.attempt_ms, now);
        // zaseklý pokus (watchdog ho při dalším `maybe_refresh_weather` uzavře) = neúspěch
        let stuck = (here && st.loading && !live).then(|| weather::stuck().to_string());
        let error = st.error.clone().filter(|_| here).or(stuck);
        let fetched_ms = data.as_ref().and(st.fetched_ms);
        WeatherView {
            status: weather::status(place.is_some(), data.is_some(), error.is_some(), live, weather::is_old(fetched_ms, now)),
            label: cfg.personal.weather.label.clone(),
            fetched_ms,
            data,
            error,
            loading: live,
        }
    };
    // jen čtení cache — stat disku tu nikdy (emit/UI vlákno)
    let cache = core().links.lock();
    let links = cfg
        .personal
        .links
        .iter()
        .map(|l| {
            let t = cache.1.get(&l.target);
            LinkView {
                name: l.name.clone(),
                target: l.target.clone(),
                kind: t.map_or(personal::LinkKind::Pending, |t| t.kind()),
                monogram: monogram(if l.name.trim().is_empty() { &l.target } else { &l.name }),
                problem: match t {
                    Some(personal::LinkTarget::Invalid(why)) => Some(why.clone()),
                    _ => None,
                },
            }
        })
        .collect();
    drop(cache);
    let sec = &cfg.drawer.sections;
    PersonalView {
        weather,
        links,
        notes,
        notes_more,
        media: crate::media::view().filter(|_| sec.now_playing),
        media_look: cfg.media.clone(),
        volume: crate::volume::view().filter(|_| sec.sound),
        battery: crate::battery::view().filter(|_| sec.battery),
        calendar: crate::calendar::live::view(cfg, now),
    }
}
