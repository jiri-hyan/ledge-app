//! Vrátit změnu přes MCP (`undo_last`): zásobník posledních ~20 zápisů v paměti. Každý záznam drží jen
//! dotčené ČÁSTI configu (JSON pointer → stav před / po), ne celý config — vrácení tak nepřepíše nic,
//! co uživatel mezitím změnil jinde. Když dotčenou část od té doby změnil někdo jiný (Nastavení, ruční
//! úprava), vrácení se odmítne. Čisté funkce.

use crate::config::Config;
use serde_json::Value;

/// Nejvíc záznamů (starší vypadnou).
pub const MAX: usize = 20;

/// Jeden zápis přes MCP.
#[derive(Debug, Clone, PartialEq)]
pub struct Entry {
    /// co se stalo (věta pro model / člověka)
    pub label: String,
    /// (pointer, před, po)
    pub parts: Vec<(String, Value, Value)>,
}

#[derive(Debug, Default)]
pub struct Undo {
    stack: Vec<Entry>,
}

#[allow(clippy::len_without_is_empty)]
impl Undo {
    pub fn push(&mut self, e: Entry) {
        self.stack.push(e);
        if self.stack.len() > MAX {
            self.stack.remove(0);
        }
    }
    pub fn pop(&mut self) -> Option<Entry> {
        self.stack.pop()
    }
    pub fn len(&self) -> usize {
        self.stack.len()
    }
}

/// Hodnoty částí configu podle pointerů (`/projects`, `/drawer/glass`).
pub fn pick(cfg: &Config, ptrs: &[&str]) -> Vec<(String, Value)> {
    let v = serde_json::to_value(cfg).unwrap_or(Value::Null);
    ptrs.iter().map(|p| (p.to_string(), v.pointer(p).cloned().unwrap_or(Value::Null))).collect()
}

/// Config s vrácenými částmi; chyba = část se od zápisu změnila jinde (vrátit by ji přepsalo).
pub fn revert(cur: &Config, e: &Entry) -> Result<Config, String> {
    let mut v = serde_json::to_value(cur).map_err(|e| e.to_string())?;
    for (ptr, _, after) in &e.parts {
        if v.pointer(ptr) != Some(after) {
            let what = ptr.trim_start_matches('/').replace('/', ".");
            return Err(trf!(
                "Nejde vrátit „{}“: {what} od té doby změnil někdo jiný (Nastavení nebo ruční úprava). Nic jsem neměnil.",
                "Can't undo “{}”: {what} was changed elsewhere since (Settings or a manual edit). Nothing changed.",
                e.label
            ));
        }
    }
    for (ptr, before, _) in &e.parts {
        if let Some(slot) = v.pointer_mut(ptr) {
            *slot = before.clone();
        }
    }
    serde_json::from_value(v).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(n: usize) -> Entry {
        Entry { label: format!("z{n}"), parts: vec![] }
    }

    #[test]
    fn stack_is_bounded_lifo() {
        let mut u = Undo::default();
        for n in 0..MAX + 5 {
            u.push(entry(n));
        }
        assert_eq!(u.len(), MAX);
        assert_eq!(u.pop().unwrap().label, format!("z{}", MAX + 4));
        let mut last = None;
        while let Some(e) = u.pop() {
            last = Some(e.label);
        }
        assert_eq!(last.as_deref(), Some("z5"), "nejstarší vypadly");
    }

    #[test]
    fn revert_only_touched_parts_and_refuse_when_changed_elsewhere() {
        let before = Config::default();
        let mut after = before.clone();
        after.strip.custom = "MCP".into();
        let p = |c: &Config| pick(c, &["/strip"]).remove(0).1;
        let e = Entry { label: "proužek".into(), parts: vec![("/strip".into(), p(&before), p(&after))] };
        // mezitím jiná sekce změněná v Nastavení → vrátí se jen proužek, ta druhá zůstane
        let mut cur = after.clone();
        cur.autostart = !cur.autostart;
        let back = revert(&cur, &e).unwrap();
        assert_eq!(back.strip, before.strip);
        assert_eq!(back.autostart, cur.autostart);
        // tentýž proužek změněný jinde → odmítnout
        let mut other = after.clone();
        other.strip.custom = "ruka".into();
        assert!(revert(&other, &e).unwrap_err().contains("proužek"));
    }
}
