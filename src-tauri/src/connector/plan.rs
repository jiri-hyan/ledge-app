//! Čistá logika „Poslat do propojené aplikace“: poznámka → plán odeslání. Bez I/O, bez sítě.
//!
//! - Poznámka bez `- [ ]` → 1 poznámka (`Mode::Note`); s ním → úkol na každý otevřený bod (`Mode::Tasks`;
//!   hotové `[x]` a navržené `[~]` se vynechají). Režim jde vynutit.
//! - Řešitel: „řešitel Tomáš Hora“, „assignee tomas“, „@tomas“ v textu bodu (jinak v ostatním textu poznámky
//!   jako výchozí pro všechny body). Proti seznamu členů se porovnává bez diakritiky a bez ohledu na
//!   velikost písmen, celými slovy. Nejednoznačné / nenalezené se NIKDY nehádá: `Ambiguous` / `Unknown`
//!   a UI to ukáže; fráze zůstane v názvu úkolu.
//! - Priorita: samostatné `!` (4), `!!`/`!!!` (5), `!1`…`!5` (1–2 nízká, 3–4 střední, 5 vysoká).
//!   „Pozor!“ ani `!` uprostřed slova priorita není.
//! - Idempotence: `request_id(path, nth, text)` jde jako `clientRequestId`, opakovaný klik úkol nezdvojí;
//!   bod s řádkem „↳ [název]…“ pod sebou je `sent` a neodešle se znovu.

use crate::notes::brief::{self, Item as Bullet};
use crate::notes::md::{TaskState, is_fence};
use crate::notes::search::fold_text;
use crate::notes::taskline;
#[cfg(test)]
pub use crate::notes::taskline::Mention;
pub use crate::notes::taskline::{LineParts, SENT_MARK, parse_line};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::path::Path;

/// Strop úkolů v jednom odeslání (strop jedné dávky).
pub const MAX_ITEMS: usize = 20;

/// Člen týmu z `list_team_members`.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct Member {
    pub email: String,
    #[serde(default)]
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum Assignee {
    /// v textu řešitel není (úkol dostane řešitele podle propojené aplikace = vlastník klíče)
    Unset,
    Resolved { email: String, name: String },
    /// víc členů se stejným jménem — nehádá se
    Ambiguous { query: String, names: Vec<String> },
    /// jméno v týmu není
    Unknown { query: String },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Item {
    /// řádek bodu v těle poznámky od 0
    pub line: usize,
    pub title: String,
    pub assignee: Assignee,
    /// jen když ji poznámka výslovně říká (`!`); jinak ji nastaví propojená aplikace
    pub priority: Option<u8>,
    /// termín z `⏰ YYYY-MM-DD[ HH:MM]` (`taskline`, místní čas); do `create_task` se zatím neposílá
    #[serde(default)]
    pub deadline: Option<String>,
    /// pod bodem už je odkaz na úkol v propojené aplikaci
    pub sent: bool,
    pub request_id: String,
    /// text bodu bez `⏰` (identita bodu pro zápis odkazu zpět; do UI nejde)
    #[serde(skip)]
    pub text: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Mode {
    Tasks,
    Note,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Plan {
    pub mode: Mode,
    /// poznámka má `- [ ]` body (UI nabídne přepnout režim)
    pub has_tasks: bool,
    pub note_title: String,
    pub note_request_id: String,
    /// poznámka už byla do propojené aplikace odeslána (`connector:` ve frontmatteru) — doplní příkaz
    #[serde(default)]
    pub note_sent: bool,
    pub items: Vec<Item>,
    /// bodů bylo víc než `MAX_ITEMS`, zbytek se neodešle
    pub truncated: bool,
}

/// Cesta v jednotném tvaru pro identitu (bez `\\?\`, lomítka `/`, malá písmena — Windows).
fn norm_path(p: &str) -> String {
    p.strip_prefix(r"\\?\").unwrap_or(p).replace('\\', "/").to_lowercase()
}

/// Idempotenční klíč požadavku (`clientRequestId`): hash cesty, textu a pořadí stejného textu v poznámce
/// (`nth`, od 0; NE řádek — přesun bodu nesmí založit druhý úkol), 38 znaků.
pub fn request_id(path: &str, nth: usize, text: &str) -> String {
    let mut h = Sha256::new();
    h.update(norm_path(path).as_bytes());
    h.update([0]);
    h.update(nth.to_string().as_bytes());
    h.update([0]);
    h.update(text.trim().as_bytes());
    let d = h.finalize();
    format!("ledge-{}", d[..16].iter().map(|b| format!("{b:02x}")).collect::<String>())
}

/// Plán odeslání. `body` = tělo poznámky bez frontmatteru, `force` = režim vybraný uživatelem.
pub fn plan(path: &str, note_title: &str, body: &str, members: &[Member], force: Option<Mode>) -> Plan {
    let sc = brief::scan_body(body, Path::new(""));
    let lines: Vec<&str> = body.split('\n').map(|l| l.trim_end_matches('\r')).collect();
    let open: Vec<&Bullet> = sc.items.iter().filter(|i| i.state == TaskState::Open && !i.text.trim().is_empty()).collect();
    let has_tasks = !open.is_empty();
    let mode = force.unwrap_or(if has_tasks { Mode::Tasks } else { Mode::Note });
    let default_assignee = note_assignee(&lines, &sc.items, members);
    let mut items = Vec::new();
    let mut truncated = false;
    if mode == Mode::Tasks {
        for b in open {
            if items.len() >= MAX_ITEMS {
                truncated = true;
                break;
            }
            // token `⏰ …` čte `taskline`; do názvu úkolu nepatří (priorita `!` zůstává pro `parse_line`)
            let tl = taskline::parse_tail(&b.text);
            let clean = taskline::set_due(&b.text, None);
            let p = parse_and_resolve(&clean, members);
            // odkaz píše `link` hned za bod a jeho snímky
            let sent = lines.get(b.line + 1 + b.images.len()).is_some_and(|l| l.trim_start().starts_with(SENT_MARK));
            let assignee = if p.assignee == Assignee::Unset { default_assignee.clone() } else { p.assignee };
            let nth = sc.items.iter().take_while(|x| x.line < b.line).filter(|x| taskline::set_due(&x.text, None).trim() == clean.trim()).count();
            items.push(Item { line: b.line, request_id: request_id(path, nth, &clean), text: clean.trim().to_string(), title: p.title, assignee, priority: p.priority, deadline: tl.due.map(|d| taskline::due_text(d, tl.all_day)), sent });
        }
    }
    Plan { mode, has_tasks, note_title: note_title.trim().to_string(), note_request_id: request_id(path, 0, "note"), note_sent: false, items, truncated }
}

/// Řešitel zmíněný mimo body (první „řešitel X“ / `@x` v ostatním textu) — výchozí pro body bez vlastního.
fn note_assignee(lines: &[&str], bullets: &[Bullet], members: &[Member]) -> Assignee {
    let item_lines: BTreeSet<usize> = bullets.iter().map(|b| b.line).collect();
    let mut code = false;
    for (i, l) in lines.iter().enumerate() {
        if is_fence(l) {
            code = !code;
            continue;
        }
        if code || item_lines.contains(&i) || l.trim_start().starts_with(SENT_MARK) {
            continue;
        }
        let a = parse_and_resolve(l, members).assignee;
        if a != Assignee::Unset {
            return a;
        }
    }
    Assignee::Unset
}

/// Tokeny člena: slova jména, část před `@` e-mailu (celá i po částech) a jméno bez mezer (`@tomashora`).
fn member_tokens(m: &Member) -> BTreeSet<String> {
    let mut t = BTreeSet::new();
    let name = fold_text(&m.name);
    t.extend(name.split(|c: char| !c.is_alphanumeric()).filter(|s| !s.is_empty()).map(str::to_string));
    let squashed: String = name.chars().filter(|c| c.is_alphanumeric()).collect();
    if !squashed.is_empty() {
        t.insert(squashed);
    }
    let local = fold_text(m.email.split('@').next().unwrap_or(""));
    t.extend(local.split(|c: char| !c.is_alphanumeric()).filter(|s| !s.is_empty()).map(str::to_string));
    if !local.is_empty() {
        t.insert(local);
    }
    t
}

/// Členové, jejichž tokeny obsahují všechna slova `cand` (celá slova, ne předpony).
fn matching<'a>(cand: &[&str], members: &'a [Member]) -> Vec<&'a Member> {
    let want: Vec<String> = cand.iter().map(|c| fold_text(c)).collect();
    members
        .iter()
        .filter(|m| {
            if m.email.trim().is_empty() {
                return false;
            }
            let t = member_tokens(m);
            want.iter().all(|w| t.contains(w))
        })
        .collect()
}

fn shown(m: &Member) -> String {
    if m.name.trim().is_empty() { m.email.trim().to_string() } else { m.name.trim().to_string() }
}

fn verdict(cand: &[&str], hits: &[&Member]) -> Assignee {
    match hits {
        [] => Assignee::Unknown { query: cand.join(" ") },
        [m] => Assignee::Resolved { email: m.email.trim().to_string(), name: shown(m) },
        many => Assignee::Ambiguous { query: cand.join(" "), names: many.iter().map(|m| shown(m)).collect() },
    }
}

struct Parsed {
    title: String,
    assignee: Assignee,
    priority: Option<u8>,
}

/// Spáruje zmínku se členy týmu a složí název (bez značek priority a rozpoznané fráze řešitele).
fn resolve(text: &str, parts: &LineParts, members: &[Member]) -> Parsed {
    let mut gone = vec![false; parts.words.len()];
    parts.prio_words.iter().for_each(|&i| gone[i] = true);
    let mut assignee = Assignee::Unset;
    if let Some(m) = &parts.mention {
        let cand: Vec<&str> = m.cand.iter().map(String::as_str).collect();
        if m.at_sign {
            assignee = verdict(&cand, &matching(&cand, members));
            if matches!(assignee, Assignee::Resolved { .. }) {
                gone[m.at] = true;
            }
        } else {
            // nejdelší shoda vyhrává („Tomáš Hora“ před „Tomáš“)
            for k in (1..=cand.len()).rev() {
                let hits = matching(&cand[..k], members);
                if hits.is_empty() {
                    continue;
                }
                assignee = verdict(&cand[..k], &hits);
                if matches!(assignee, Assignee::Resolved { .. }) {
                    gone[m.at..=m.at + m.skip + k].iter_mut().for_each(|g| *g = true);
                }
                break;
            }
            if assignee == Assignee::Unset {
                let two = cand.len() > 1 && cand[1].chars().next().is_some_and(char::is_uppercase);
                assignee = Assignee::Unknown { query: cand[..if two { 2 } else { 1 }].join(" ") };
            }
        }
    }
    let kept: Vec<&str> = parts.words.iter().zip(&gone).filter(|(_, g)| !**g).map(|(w, _)| w.as_str()).collect();
    let title = kept.join(" ");
    let mut title = title.trim_matches(|c: char| matches!(c, ',' | ';' | ':' | '-' | '–') || c.is_whitespace()).to_string();
    if title.is_empty() {
        title = text.trim().to_string();
    }
    Parsed { title, assignee, priority: parts.priority }
}

/// Rozbor + párování v jednom (plán).
fn parse_and_resolve(text: &str, members: &[Member]) -> Parsed {
    resolve(text, &parse_line(text), members)
}
