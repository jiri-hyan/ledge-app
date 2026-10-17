//! Subagenti jedné Claude session z hooků `SubagentStart` / `SubagentStop` — čistá logika bez I/O.
//! Drží je `claude::Session::subagents` (plní `Claude::handle_event`); UI je ukazuje pod session
//! (`sessions::Row`). Běžící: typ + od kdy; dokončení: posledních `DONE_KEEP` s trváním a zkrácenou
//! poslední zprávou (`last_assistant_message`).

use serde::Serialize;

/// Kolik dokončených subagentů si session pamatuje (nejnovější první).
pub const DONE_KEEP: usize = 5;
/// Zkrácení `last_assistant_message` pro UI.
pub const MSG_CHARS: usize = 140;
/// Pojistka proti nekonečnému růstu, kdyby `SubagentStop` nikdy nepřišel.
const RUNNING_MAX: usize = 32;

#[derive(Debug, Clone, Default, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Subagents {
    pub running: Vec<Running>,
    pub done: Vec<Done>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Running {
    pub id: String,
    pub agent_type: String,
    /// ms od epochy
    pub since: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Done {
    pub agent_type: String,
    /// `None` = start jsme neviděli (Ledge naběhl až během běhu) → trvání nevíme
    pub duration_ms: Option<u64>,
    pub ended_at: u64,
    pub message: Option<String>,
}

fn kind(t: &str) -> String {
    if t.trim().is_empty() { "agent".into() } else { t.trim().to_string() }
}

impl Subagents {
    pub fn start(&mut self, id: &str, agent_type: &str, now: u64) {
        if id.is_empty() {
            return;
        }
        self.running.retain(|r| r.id != id);
        self.running.push(Running { id: id.to_string(), agent_type: kind(agent_type), since: now });
        if self.running.len() > RUNNING_MAX {
            self.running.remove(0);
        }
    }

    pub fn stop(&mut self, id: &str, agent_type: &str, message: &str, now: u64) {
        let run = self.running.iter().position(|r| r.id == id && !id.is_empty()).map(|i| self.running.remove(i));
        let agent_type = match (&run, agent_type.trim().is_empty()) {
            (Some(r), true) => r.agent_type.clone(),
            _ => kind(agent_type),
        };
        let duration_ms = run.map(|r| now.saturating_sub(r.since));
        self.done.insert(0, Done { agent_type, duration_ms, ended_at: now, message: shorten(message, MSG_CHARS) });
        self.done.truncate(DONE_KEEP);
    }

    /// Konec session: běžící subagenti s ní skončili (dokončené zůstávají jako historie).
    pub fn clear_running(&mut self) {
        self.running.clear();
    }
}

/// Jeden řádek, nejvýš `max` znaků (+ „…“); prázdné → `None`.
pub fn shorten(text: &str, max: usize) -> Option<String> {
    let one = text.split_whitespace().collect::<Vec<_>>().join(" ");
    if one.is_empty() {
        return None;
    }
    Some(if one.chars().count() > max { one.chars().take(max).collect::<String>() + "…" } else { one })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn start_stop_lifecycle() {
        let mut s = Subagents::default();
        s.start("a1", "Explore", 1000);
        s.start("a2", "", 1500);
        s.start("a1", "Explore", 1200); // opakovaný start = jeden záznam, nový čas
        assert_eq!(s.running.iter().map(|r| (r.id.as_str(), r.agent_type.as_str(), r.since)).collect::<Vec<_>>(), [("a2", "agent", 1500), ("a1", "Explore", 1200)]);
        // stop bez typu převezme typ ze startu
        s.stop("a1", "", "  hotovo\n\nnašel jsem 3 soubory ", 4200);
        assert_eq!(s.running.len(), 1);
        assert_eq!(s.done[0], Done { agent_type: "Explore".into(), duration_ms: Some(3000), ended_at: 4200, message: Some("hotovo našel jsem 3 soubory".into()) });
        // stop bez startu (Ledge naběhl během běhu) → trvání neznámé
        s.stop("x", "Plan", "", 5000);
        assert_eq!(s.done[0], Done { agent_type: "Plan".into(), duration_ms: None, ended_at: 5000, message: None });
        s.clear_running();
        assert!(s.running.is_empty() && s.done.len() == 2);
        // prázdné id se nestartuje
        s.start("", "Explore", 1);
        assert!(s.running.is_empty());
    }

    #[test]
    fn caps() {
        let mut s = Subagents::default();
        for i in 0..(RUNNING_MAX + 3) {
            s.start(&format!("a{i}"), "t", i as u64);
        }
        assert_eq!(s.running.len(), RUNNING_MAX);
        assert_eq!(s.running[0].id, "a3", "nejstarší vypadly");
        for i in 0..10 {
            s.stop(&format!("a{}", i + 3), "", "", 100);
        }
        assert_eq!(s.done.len(), DONE_KEEP);
    }

    #[test]
    fn shorten_table() {
        let long = "x".repeat(200);
        let cases: &[(&str, usize, Option<String>)] = &[
            ("", 10, None),
            ("  \n ", 10, None),
            ("a  b\nc", 10, Some("a b c".into())),
            ("příliš dlouhé", 6, Some("příliš…".into())),
            (&long, MSG_CHARS, Some("x".repeat(MSG_CHARS) + "…")),
        ];
        for (i, max, want) in cases {
            assert_eq!(shorten(i, *max), *want, "{i:?}");
        }
    }
}
