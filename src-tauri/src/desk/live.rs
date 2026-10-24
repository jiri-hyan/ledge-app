//! Data pro okna widgetů (událost `desk-data`). Nic nového se nestahuje ani nepolluje: volá to vlákno
//! `emit` při každém svém probuzení (`mark_dirty` z událostí, které Ledge už má — limity ze stavového
//! řádku, most Clauda, procesy, kalendář z cache, widgety ze skriptů; jinak jednou za minutu kvůli
//! hodinám). Počítá se jen, když je nějaký widget VIDĚT, jen pro druhy, které jsou vidět, a posílá se jen
//! při změně (otisk JSON). Ve hře / se schovanými widgety = nic.

use super::Kind;
use crate::state::{core, now_ms};
use serde::Serialize;
use std::sync::atomic::{AtomicU64, Ordering};
use tauri::Emitter;

/// Otisk naposledy poslaných dat (0 = poslat znovu: nové / znovu ukázané okno).
static LAST: AtomicU64 = AtomicU64::new(0);

/// Jedna věc, která čeká na člověka (jen ukázat — schvaluje se v šuplíku).
#[derive(Serialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct WaitRow {
    pub id: u64,
    /// `approve` (schválení akce) · `question` (otázka Clauda)
    pub kind: &'static str,
    /// projekt podle složky session, jinak jméno složky
    pub project: String,
    /// co: „Bash: npm test“ / první otázka
    pub text: String,
    /// kdy přišlo (ms)
    pub since: u64,
}

#[derive(Serialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct NameRef {
    pub id: String,
    pub name: String,
}

/// Co okna widgetů dostanou. Pole jen pro druhy, které jsou vidět (`None` = nikdo ho nepotřebuje).
#[derive(Serialize, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct DeskData {
    /// čas zaokrouhlený na minutu (UI z něj počítá odpočty) — data se tak mění nejvýš 1×/min
    pub now: i64,
    pub cfg_error: Option<String>,
    pub claude_enabled: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limits: Option<crate::limitslive::LimitsView>,
    /// pracuje aspoň jedna session (zvýraznění resetu 5h okna)
    pub working: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub waiting: Option<Vec<WaitRow>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub procs: Option<Vec<crate::procman::ProcView>>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub projects: Vec<NameRef>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub calendar: Option<crate::calendar::live::CalendarView>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub widgets: Option<Vec<crate::widgets::live::WidgetView>>,
    /// kolo 22: útrata Claude (widget Claude dnes) — jen paměť; přepočet nejvýš 1× / čtvrthodinu
    #[serde(skip_serializing_if = "Option::is_none")]
    pub costs: Option<crate::costslive::CostsView>,
}

/// Nejvýš tolik čekajících řádků (widget je malý; zbytek je v šuplíku).
pub const WAIT_MAX: usize = 6;
const TEXT_MAX: usize = 120;

/// Zkrátit na `max` znaků (ne bajtů) s „…“.
pub fn clip(s: &str, max: usize) -> String {
    let s = s.split_whitespace().collect::<Vec<_>>().join(" ");
    if s.chars().count() <= max { s } else { format!("{}…", s.chars().take(max.saturating_sub(1)).collect::<String>()) }
}

/// Jméno složky z cesty (`C:\x\app` → `app`).
pub fn folder_name(cwd: &str) -> String {
    cwd.trim_end_matches(['\\', '/']).rsplit(['\\', '/']).next().unwrap_or(cwd).to_string()
}

/// Řádky „čeká na tebe“: nejstarší první (čeká nejdéle), nejvýš `WAIT_MAX`. `name` = projekt ke složce.
pub fn wait_rows(pending: &[(u64, &str, &str, &str, u64)], questions: &[(u64, &str, &str, u64)], name: &dyn Fn(&str) -> Option<String>) -> Vec<WaitRow> {
    let project = |cwd: &str| name(cwd).unwrap_or_else(|| folder_name(cwd));
    let mut out: Vec<WaitRow> = pending
        .iter()
        .map(|(id, cwd, tool, summary, at)| {
            let text = if summary.trim().is_empty() { tool.to_string() } else { format!("{tool}: {summary}") };
            WaitRow { id: *id, kind: "approve", project: project(cwd), text: clip(&text, TEXT_MAX), since: *at }
        })
        .chain(questions.iter().map(|(id, cwd, q, at)| WaitRow { id: *id, kind: "question", project: project(cwd), text: clip(q, TEXT_MAX), since: *at }))
        .collect();
    out.sort_by_key(|r| (r.since, r.id));
    out.truncate(WAIT_MAX);
    out
}

/// Nové / znovu ukázané okno: příští průchod pošle data i beze změny.
pub(crate) fn invalidate() {
    LAST.store(0, Ordering::Relaxed);
}

fn fnv(s: &str) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in s.bytes() {
        h = (h ^ b as u64).wrapping_mul(0x100_0000_01b3);
    }
    h.max(1)
}

/// Sestaví data pro druhy `kinds` (jen paměť).
fn build(kinds: &[Kind]) -> DeskData {
    let c = core();
    let cfg = c.cfg.read().clone();
    let has = |k: &Kind| kinds.contains(k);
    let now = now_ms();
    let mut d = DeskData { now: now - now.rem_euclid(60_000), cfg_error: c.cfg_error.lock().clone(), claude_enabled: cfg.claude.enabled, ..Default::default() };
    if has(&Kind::Limits) {
        d.limits = Some(crate::limitslive::view());
        d.working = c.claude.sessions().iter().any(|s| s.state == crate::claude::SessionState::Working);
    }
    if has(&Kind::Waiting) {
        let pending = c.claude.pending();
        let questions = c.claude.questions();
        let p: Vec<(u64, &str, &str, &str, u64)> = pending.iter().map(|p| (p.id, p.cwd.as_str(), p.tool.as_str(), p.summary.as_str(), p.created_at)).collect();
        let q: Vec<(u64, &str, &str, u64)> = questions
            .iter()
            .map(|a| (a.id, a.cwd.as_str(), a.questions.first().map(|q| if q.question.trim().is_empty() { q.header.as_str() } else { q.question.as_str() }).unwrap_or(""), a.created_at))
            .collect();
        d.waiting = Some(wait_rows(&p, &q, &|cwd| crate::stripview::project_for_cwd(&cfg, cwd)));
    }
    if has(&Kind::Dev) {
        d.procs = Some(c.procs.snapshot());
        d.projects = cfg.projects.iter().map(|p| NameRef { id: p.id.clone(), name: p.name.clone() }).collect();
    }
    if has(&Kind::Agenda) || has(&Kind::Week) {
        d.calendar = Some(crate::calendar::live::view(&cfg, now));
    }
    if has(&Kind::Usage) {
        crate::costslive::request_desk();
        d.costs = Some(crate::costslive::desk_view());
    }
    if kinds.iter().any(|k| matches!(k, Kind::Script(_))) {
        let ids: Vec<String> = kinds.iter().filter_map(|k| if let Kind::Script(id) = k { Some(id.clone()) } else { None }).collect();
        d.widgets = Some(crate::widgets::live::snapshot_views(&cfg).into_iter().filter(|w| ids.contains(&w.id)).collect());
    }
    d
}

/// Data pro okno při startu (`desk_init`) — vždy čerstvá.
pub(crate) fn data_for(kind: &Kind) -> DeskData {
    build(std::slice::from_ref(kind))
}

/// Vlákno `emit` po každém probuzení: poslat data viditelným widgetům, když se změnila.
pub(crate) fn tick() {
    let windows = super::win::shown_windows();
    if windows.is_empty() {
        return;
    }
    // Záměrně se staví vše najednou a porovnává otisk, ne jednotlivé zdroje: `build` jen čte paměť (žádný disk,
    // žádná síť), jen pro VIDITELNÁ okna a jen při probuzení vlákna `emit` (událostmi, jinak 1× za minutu).
    // Dílčí `invalidate()` po zdrojích by potřebovalo vědět, který zdroj probuzení způsobil (`mark_dirty` to
    // neříká) a při chybě by widget ukazoval zastaralá data; otisk je jediná pojistka bez takového rizika.
    let per = per_window(&windows, build);
    let mut json = String::new();
    for (label, data) in &per {
        let Ok(one) = serde_json::to_string(data) else { return };
        json.push_str(label);
        json.push('\n');
        json.push_str(&one);
        json.push('\n');
    }
    let h = fnv(&json);
    if LAST.swap(h, Ordering::Relaxed) == h {
        return;
    }
    let app = &core().app;
    for (l, data) in &per {
        let _ = app.emit_to(l.as_str(), "desk-data", data);
    }
}

/// Data pro každé viditelné okno jen jeho druhu: okno Hodin ani Týdne nedostane útratu Clauda, okno Útraty
/// kalendář apod. `build` se volá s právě jedním druhem okna (druhy ostatních oken se nesčítají).
pub(crate) fn per_window(windows: &[(String, Kind)], build: impl Fn(&[Kind]) -> DeskData) -> Vec<(String, DeskData)> {
    windows.iter().map(|(label, kind)| (label.clone(), build(std::slice::from_ref(kind)))).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wait_rows_order_cap_and_names() {
        let name = |cwd: &str| (cwd == r"C:\code\app").then(|| "App".to_string());
        let p = [(7, r"C:\code\app", "Bash", "npm test", 300), (3, r"C:\code\other\", "Edit", "", 100)];
        let q = [(9, r"C:\code\app", "Který port?", 200)];
        let rows = wait_rows(&p, &q, &name);
        let got: Vec<(u64, &str, &str, &str)> = rows.iter().map(|r| (r.id, r.kind, r.project.as_str(), r.text.as_str())).collect();
        assert_eq!(got, [(3, "approve", "other", "Edit"), (9, "question", "App", "Který port?"), (7, "approve", "App", "Bash: npm test")]);
        // strop
        let many: Vec<(u64, &str, &str, &str, u64)> = (0..20).map(|i| (i, "C:\\x", "Bash", "ls", i)).collect();
        assert_eq!(wait_rows(&many, &[], &name).len(), WAIT_MAX);
        assert!(wait_rows(&[], &[], &name).is_empty());
    }

    #[test]
    fn clip_and_folder() {
        assert_eq!(clip("  a \n b  ", 10), "a b");
        assert_eq!(clip("příliš dlouhý text", 7), "příliš…");
        assert_eq!(clip("abc", 3), "abc");
        assert_eq!(folder_name(r"C:\code\app\"), "app");
        assert_eq!(folder_name("C:/code/app"), "app");
        assert_eq!(folder_name("app"), "app");
    }

    #[test]
    fn each_window_gets_only_its_own_kind() {
        use std::cell::RefCell;
        // `build` dostane vždy jediný druh okna, pro které se staví
        let seen: RefCell<Vec<Vec<Kind>>> = RefCell::new(vec![]);
        let fake = |kinds: &[Kind]| {
            seen.borrow_mut().push(kinds.to_vec());
            DeskData { working: kinds.contains(&Kind::Usage), ..Default::default() }
        };
        let windows = [("desk-clock".to_string(), Kind::Clock), ("desk-week".to_string(), Kind::Week), ("desk-usage".to_string(), Kind::Usage)];
        let got = per_window(&windows, fake);
        assert_eq!(*seen.borrow(), [vec![Kind::Clock], vec![Kind::Week], vec![Kind::Usage]]);
        assert_eq!(got.iter().map(|(l, d)| (l.as_str(), d.working)).collect::<Vec<_>>(), [("desk-clock", false), ("desk-week", false), ("desk-usage", true)]);
        assert!(per_window(&[], fake).is_empty());
    }

    #[test]
    fn fingerprint_differs_and_never_zero() {
        assert_ne!(fnv("a"), fnv("b"));
        assert_ne!(fnv(""), 0);
    }
}
