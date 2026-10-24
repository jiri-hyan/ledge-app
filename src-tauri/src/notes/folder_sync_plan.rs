//! Propojení se složkou — ČISTÉ rozhodování (bez I/O): ze tří stavů (poslední dohoda `Entry`, aktuální
//! soubory Ledge, aktuální soubory ve složce) udělá seznam kroků `Step`. Tabulkové testy jsou v
//! `folder_sync_tests.rs`.
//!
//! Pravidla: soubor se kopíruje bajtově (frontmatter, `[[vazby]]` i cizí klíče zůstanou), změnila-li se
//! jen jedna strana od poslední dohody; změnily-li se obě a liší se, nic se nepřepíše potichu — vznikne
//! kopie `název (konflikt Ledge YYYY-MM-DD).md` (viz [`Op::Conflict`]). Mazání se ve výchozím stavu
//! nepřenáší: na druhé straně zůstane soubor (náhrobek [`Gone`]) a nevrací se, dokud se nezmění.
//! Přejmenování se pozná podle shodného obsahu; shodné soubory na obou stranách se spárují bez zápisu.

use crate::config::LinkDirection as Dir;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

/// Složka Ledge pro poznámky bez projektu — sem se importuje.
pub const IMPORT_DIR: &str = "_obecne";
/// Název na Windows, když z titulku nezbylo nic.
const FALLBACK_NAME: &str = "Poznámka";
const MAX_NAME_CHARS: usize = 100;

/// Která strana je smazaná (a smazání se nepřeneslo).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Gone {
    #[default]
    None,
    Ledge,
    Ext,
}

impl Gone {
    pub fn is_none(&self) -> bool {
        *self == Gone::None
    }
}

/// Dvojice souborů a obsah (`hash`), na kterém se obě strany naposledy shodly. `lm`/`ls` a `em`/`es`
/// (mtime v µs, velikost) umožní nehashovat nezměněný soubor znovu.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Entry {
    /// cesta v Ledge od kořene poznámek, vždy `/`
    pub ledge: String,
    /// cesta ve složce, vždy `/`
    pub ext: String,
    pub hash: String,
    #[serde(default)]
    pub lm: i64,
    #[serde(default)]
    pub ls: u64,
    #[serde(default)]
    pub em: i64,
    #[serde(default)]
    pub es: u64,
    #[serde(default, skip_serializing_if = "Gone::is_none")]
    pub gone: Gone,
}

impl Entry {
    pub fn new(ledge: &str, ext: &str, hash: &str) -> Self {
        Self { ledge: ledge.into(), ext: ext.into(), hash: hash.into(), lm: 0, ls: 0, em: 0, es: 0, gone: Gone::None }
    }

    /// Stejný obsah na obou stranách (`l`, `x` mají hash entry) → osvěžit otisky souborů.
    fn fresh(&self, l: &Snap, x: &Snap) -> Self {
        Self { lm: l.mtime, ls: l.size, em: x.mtime, es: x.size, gone: Gone::None, ..self.clone() }
    }
}

/// Soubor tak, jak ho viděl sken.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Snap {
    pub rel: String,
    pub size: u64,
    pub mtime: i64,
    pub hash: String,
    /// titulek (první řádek těla) — jen u souborů Ledge, které se četly (nové / změněné)
    pub title: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Side {
    Ledge,
    Ext,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Op {
    /// nic se nezapisuje, jen se uloží `keep`
    Keep,
    /// složka → Ledge; `over` = hash, který musí mít přepisovaný soubor Ledge (jinak se změnil → přeskočit)
    Import { ext: String, ledge: String, ext_hash: String, over: Option<String> },
    /// Ledge → složka (+ obrázky vedle)
    Export { ledge: String, ext: String, ledge_hash: String, over: Option<String> },
    /// Obě strany se změnily. `winner` zůstane na místě, obsah druhé strany se uloží jako kopie
    /// (`copy_ledge` / `copy_ext`, `None` = tam se nezapisuje).
    Conflict { ledge: String, ext: String, ledge_hash: String, ext_hash: String, winner: Side, copy_ledge: Option<String>, copy_ext: Option<String> },
    /// smazáno ve složce → poznámka Ledge do Archivu (jen když je nezměněná)
    DeleteLedge { ledge: String, hash: String },
    /// smazáno v Ledge → soubor ve složce do `.trash` (jen když je nezměněný)
    DeleteExt { ext: String, hash: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Step {
    pub op: Op,
    /// záznamy po úspěchu (otisky souborů doplní provedení)
    pub keep: Vec<Entry>,
    /// záznamy, když krok selže / přeskočí se
    pub fallback: Vec<Entry>,
}

#[derive(Debug, Clone)]
pub struct Opts {
    pub dir: Dir,
    pub deletes: bool,
    /// `YYYY-MM-DD` do názvu kopie konfliktu
    pub today: String,
}

#[derive(Debug, Default)]
pub struct Planned {
    pub steps: Vec<Step>,
    /// kolik mazání se nepřeneslo, protože jich bylo najednou podezřele moc
    pub held_deletes: usize,
}

pub fn allows_import(d: Dir) -> bool {
    matches!(d, Dir::Import | Dir::Both)
}

pub fn allows_export(d: Dir) -> bool {
    matches!(d, Dir::Export | Dir::Both)
}

// ---------- jména ----------

/// Název souboru bezpečný pro Windows a Obsidian: bez `<>:"/\|?*` a řídicích znaků (+ `#^[]`, které
/// Obsidian ve jménech nechce), bez teček a mezer na krajích, bez rezervovaných jmen, nejvýš 100 znaků.
pub fn sanitize_name(raw: &str) -> String {
    let spaced: String = raw
        .chars()
        .map(|c| if c.is_control() || matches!(c, '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*' | '#' | '^' | '[' | ']') { ' ' } else { c })
        .collect();
    let joined = spaced.split_whitespace().collect::<Vec<_>>().join(" ");
    let cut: String = joined.chars().take(MAX_NAME_CHARS).collect();
    let mut s = cut.trim_matches(|c| c == '.' || c == ' ').to_string();
    if s.is_empty() {
        s = FALLBACK_NAME.into();
    }
    if is_reserved_name(s.split('.').next().unwrap_or("")) {
        s.push('_');
    }
    s
}

/// Rezervované jméno zařízení ve Windows (bez přípony, na velikosti písmen nezáleží): CON PRN AUX NUL,
/// COM0–9 a LPT0–9 včetně horních indexů `¹ ² ³`, CONIN$ a CONOUT$. Čistá funkce.
pub fn is_reserved_name(head: &str) -> bool {
    let head = head.trim().to_uppercase();
    if matches!(head.as_str(), "CON" | "PRN" | "AUX" | "NUL" | "CONIN$" | "CONOUT$") {
        return true;
    }
    let mut chars = head.chars();
    let prefix: String = chars.by_ref().take(3).collect();
    matches!(prefix.as_str(), "COM" | "LPT") && matches!((chars.next(), chars.next()), (Some('0'..='9' | '¹' | '²' | '³'), None))
}

/// Cesta z relativní cesty ke složkám a souboru: `a/b/c.md` → (`a/b`, `c.md`).
pub fn split_rel(rel: &str) -> (&str, &str) {
    rel.rsplit_once('/').unwrap_or(("", rel))
}

pub fn join_rel(dir: &str, file: &str) -> String {
    if dir.is_empty() { file.to_string() } else { format!("{dir}/{file}") }
}

/// Jméno souboru bez přípony (poslední tečka).
pub fn stem_of(file: &str) -> &str {
    file.rsplit_once('.').map_or(file, |(s, _)| s)
}

fn lc(s: &str) -> String {
    s.to_lowercase()
}

/// Volná cesta `dir/stem.md`, jinak `dir/stem (2).md`, `(3)`… (velikost písmen se neliší — Windows); obsadí ji.
pub fn unique(taken: &mut HashSet<String>, dir: &str, stem: &str) -> String {
    let mut n = 1;
    loop {
        let name = if n == 1 { format!("{stem}.md") } else { format!("{stem} ({n}).md") };
        let rel = join_rel(dir, &name);
        if taken.insert(lc(&rel)) {
            return rel;
        }
        n += 1;
    }
}

/// Kopie při konfliktu: `název (konflikt Ledge 2026-10-24).md` ve stejné složce jako `rel`.
pub fn conflict_rel(taken: &mut HashSet<String>, rel: &str, today: &str) -> String {
    let (dir, file) = split_rel(rel);
    unique(taken, dir, &format!("{} (konflikt Ledge {today})", stem_of(file)))
}

/// Bezpečná relativní cesta ze stavu / plánu: neprázdné části bez `.`/`..`, `:`, `\`, řídicích znaků.
pub fn rel_ok(rel: &str) -> bool {
    !rel.is_empty()
        && !rel.starts_with('/')
        && rel.split('/').all(|p| {
            !p.is_empty() && p != "." && p != ".." && !p.chars().any(|c| c.is_control() || matches!(c, ':' | '\\' | '<' | '>' | '"' | '|' | '?' | '*')) && !p.ends_with(['.', ' '])
        })
}

// ---------- plán ----------

struct P<'a> {
    o: &'a Opts,
    imp: bool,
    exp: bool,
    free_l: Vec<&'a Snap>,
    free_e: Vec<&'a Snap>,
    taken_l: HashSet<String>,
    taken_e: HashSet<String>,
}

fn keep_step(e: Entry) -> Step {
    Step { op: Op::Keep, keep: vec![e.clone()], fallback: vec![e] }
}

impl<'a> P<'a> {
    fn take_free(pool: &mut Vec<&'a Snap>, hash: &str) -> Option<&'a Snap> {
        let i = pool.iter().position(|s| s.hash == hash)?;
        Some(pool.remove(i))
    }

    /// Obě strany existují.
    fn both(&mut self, e: &Entry, l: &Snap, x: &Snap) -> Step {
        let (lch, ech) = (l.hash != e.hash, x.hash != e.hash);
        if !lch && !ech {
            return keep_step(e.fresh(l, x));
        }
        if l.hash == x.hash {
            // stejná úprava na obou stranách (nebo ruční sjednocení) — nová dohoda
            let agreed = Entry { hash: l.hash.clone(), ..e.clone() };
            return Step { op: Op::Keep, keep: vec![agreed.fresh(l, x)], fallback: vec![e.clone()] };
        }
        let op = match (lch, ech) {
            (true, false) if self.exp => Op::Export { ledge: e.ledge.clone(), ext: e.ext.clone(), ledge_hash: l.hash.clone(), over: Some(x.hash.clone()) },
            (false, true) if self.imp => Op::Import { ext: e.ext.clone(), ledge: e.ledge.clone(), ext_hash: x.hash.clone(), over: Some(l.hash.clone()) },
            // změna na straně, kterou směr nepřenáší: nechat být (otisky zůstávají staré, ať se změna nepřehlédne)
            (true, false) | (false, true) => return keep_step(e.clone()),
            _ => return self.conflict(e, l, x),
        };
        let hash = if matches!(op, Op::Export { .. }) { &l.hash } else { &x.hash };
        Step { op, keep: vec![Entry { hash: hash.clone(), gone: Gone::None, ..e.clone() }], fallback: vec![e.clone()] }
    }

    fn conflict(&mut self, e: &Entry, l: &Snap, x: &Snap) -> Step {
        // ve složce vyhrává složka, jen u „jen export“ vyhrává Ledge (je zdrojem)
        let winner = if self.exp && !self.imp { Side::Ledge } else { Side::Ext };
        let today = &self.o.today;
        let (copy_ledge, copy_ext) = match winner {
            Side::Ext => (Some(conflict_rel(&mut self.taken_l, &e.ledge, today)), self.exp.then(|| conflict_rel(&mut self.taken_e, &e.ext, today))),
            Side::Ledge => (None, Some(conflict_rel(&mut self.taken_e, &e.ext, today))),
        };
        let (win_hash, lose_hash) = if winner == Side::Ext { (&x.hash, &l.hash) } else { (&l.hash, &x.hash) };
        let mut keep = vec![Entry { hash: win_hash.clone(), gone: Gone::None, ..e.clone() }];
        if let (Some(cl), Some(ce)) = (&copy_ledge, &copy_ext) {
            keep.push(Entry::new(cl, ce, lose_hash));
        }
        Step {
            op: Op::Conflict { ledge: e.ledge.clone(), ext: e.ext.clone(), ledge_hash: l.hash.clone(), ext_hash: x.hash.clone(), winner, copy_ledge, copy_ext },
            keep,
            fallback: vec![e.clone()],
        }
    }

    /// Ve složce soubor chybí, v Ledge je.
    fn only_ledge(&mut self, e: &Entry, l: &Snap) -> Step {
        let changed = l.hash != e.hash;
        if e.gone == Gone::None
            && let Some(s) = Self::take_free(&mut self.free_e, &e.hash)
        {
            // přejmenováno / přesunuto ve složce — stejný obsah
            let moved = Entry { ext: s.rel.clone(), em: s.mtime, es: s.size, ..e.clone() };
            return keep_step(moved);
        }
        if changed && self.exp {
            // po smazání ve složce se poznámka v Ledge upravila → vrátit do složky (nic se neztratí)
            // cesta zůstává obsazená v `taken_e` záznamem, nikdo jiný si ji nevezme
            return Step {
                op: Op::Export { ledge: e.ledge.clone(), ext: e.ext.clone(), ledge_hash: l.hash.clone(), over: None },
                keep: vec![Entry::new(&e.ledge, &e.ext, &l.hash)],
                fallback: vec![Entry { gone: Gone::Ext, ..e.clone() }],
            };
        }
        if e.gone == Gone::None && !changed && self.o.deletes && self.imp {
            return Step { op: Op::DeleteLedge { ledge: e.ledge.clone(), hash: l.hash.clone() }, keep: vec![], fallback: vec![e.clone()] };
        }
        keep_step(Entry { gone: Gone::Ext, ..e.clone() })
    }

    /// V Ledge soubor chybí, ve složce je.
    fn only_ext(&mut self, e: &Entry, x: &Snap) -> Step {
        let changed = x.hash != e.hash;
        if e.gone == Gone::None
            && let Some(s) = Self::take_free(&mut self.free_l, &e.hash)
        {
            let moved = Entry { ledge: s.rel.clone(), lm: s.mtime, ls: s.size, ..e.clone() };
            return keep_step(moved);
        }
        if changed && self.imp {
            return Step {
                op: Op::Import { ext: e.ext.clone(), ledge: e.ledge.clone(), ext_hash: x.hash.clone(), over: None },
                keep: vec![Entry::new(&e.ledge, &e.ext, &x.hash)],
                fallback: vec![Entry { gone: Gone::Ledge, ..e.clone() }],
            };
        }
        if e.gone == Gone::None && !changed && self.o.deletes && self.exp {
            return Step { op: Op::DeleteExt { ext: e.ext.clone(), hash: x.hash.clone() }, keep: vec![], fallback: vec![e.clone()] };
        }
        keep_step(Entry { gone: Gone::Ledge, ..e.clone() })
    }

    /// Nový název souboru ve složce pro poznámku Ledge `ledge_rel`: titulek, jinak jméno souboru.
    fn fresh_ext(&mut self, title: &Option<String>, ledge_rel: &str) -> String {
        let (ldir, lfile) = split_rel(ledge_rel);
        let dir = if ldir == IMPORT_DIR { "" } else { ldir };
        let base = title.as_deref().filter(|t| !t.trim().is_empty()).unwrap_or_else(|| stem_of(lfile));
        unique(&mut self.taken_e, dir, &sanitize_name(base))
    }
}

/// Plán synchronizace. `entries` = poslední dohoda, `ledge` / `ext` = aktuální soubory (se hashi).
pub fn plan(entries: &[Entry], ledge: &[Snap], ext: &[Snap], o: &Opts) -> Planned {
    let index = |v: &'_ [Snap]| -> HashMap<String, usize> { v.iter().enumerate().map(|(i, s)| (lc(&s.rel), i)).collect() };
    let (li, xi) = (index(ledge), index(ext));
    let paired_l: HashSet<String> = entries.iter().map(|e| lc(&e.ledge)).collect();
    let paired_e: HashSet<String> = entries.iter().map(|e| lc(&e.ext)).collect();
    let mut sorted_l: Vec<&Snap> = ledge.iter().filter(|s| !paired_l.contains(&lc(&s.rel))).collect();
    let mut sorted_e: Vec<&Snap> = ext.iter().filter(|s| !paired_e.contains(&lc(&s.rel))).collect();
    sorted_l.sort_by(|a, b| a.rel.cmp(&b.rel));
    sorted_e.sort_by(|a, b| a.rel.cmp(&b.rel));
    let mut p = P {
        o,
        imp: allows_import(o.dir),
        exp: allows_export(o.dir),
        free_l: sorted_l,
        free_e: sorted_e,
        taken_l: paired_l.iter().cloned().chain(ledge.iter().map(|s| lc(&s.rel))).collect(),
        taken_e: paired_e.iter().cloned().chain(ext.iter().map(|s| lc(&s.rel))).collect(),
    };
    let mut steps = Vec::new();
    for e in entries {
        let l = li.get(&lc(&e.ledge)).map(|&i| &ledge[i]);
        let x = xi.get(&lc(&e.ext)).map(|&i| &ext[i]);
        match (l, x) {
            (Some(l), Some(x)) => steps.push(p.both(e, l, x)),
            (Some(l), None) => steps.push(p.only_ledge(e, l)),
            (None, Some(x)) => steps.push(p.only_ext(e, x)),
            (None, None) => {} // zmizelo na obou stranách → záznam končí
        }
    }
    // shodný obsah na obou stranách bez záznamu (ztracený stav, nebo už zkopírováno ručně) → jen spárovat
    let mut free_e_left: Vec<&Snap> = std::mem::take(&mut p.free_e);
    let mut free_l_left: Vec<&Snap> = Vec::new();
    for l in std::mem::take(&mut p.free_l) {
        let best = free_e_left
            .iter()
            .enumerate()
            .filter(|(_, x)| x.hash == l.hash)
            .max_by_key(|(_, x)| stem_of(split_rel(&x.rel).1).to_lowercase() == stem_of(split_rel(&l.rel).1).to_lowercase())
            .map(|(i, _)| i);
        match best {
            Some(i) => {
                let x = free_e_left.remove(i);
                let e = Entry { lm: l.mtime, ls: l.size, em: x.mtime, es: x.size, ..Entry::new(&l.rel, &x.rel, &l.hash) };
                steps.push(keep_step(e));
            }
            None => free_l_left.push(l),
        }
    }
    if p.imp {
        for x in free_e_left {
            let ledge_rel = unique(&mut p.taken_l, IMPORT_DIR, &sanitize_name(stem_of(split_rel(&x.rel).1)));
            steps.push(Step {
                op: Op::Import { ext: x.rel.clone(), ledge: ledge_rel.clone(), ext_hash: x.hash.clone(), over: None },
                keep: vec![Entry::new(&ledge_rel, &x.rel, &x.hash)],
                fallback: vec![],
            });
        }
    }
    if p.exp {
        for l in free_l_left {
            let ext_rel = p.fresh_ext(&l.title, &l.rel);
            steps.push(Step {
                op: Op::Export { ledge: l.rel.clone(), ext: ext_rel.clone(), ledge_hash: l.hash.clone(), over: None },
                keep: vec![Entry::new(&l.rel, &ext_rel, &l.hash)],
                fallback: vec![],
            });
        }
    }
    hold_mass_deletes(steps, entries.len())
}

/// Najednou podezřele mnoho mazání (odpojený disk, přesunutá složka, omylem vyprázdněná) se nepřenese —
/// z každého bude náhrobek. Zastaví se: smazání VŠEHO (100 % záznamů, i u malého vaultu), u malých počtů
/// víc než polovina záznamů při aspoň 3 smazáních, a víc než 10 smazání, která jsou víc než čtvrtina záznamů.
pub fn mass_delete(n: usize, entries: usize) -> bool {
    n > 0 && (n >= entries || (n >= 3 && n * 2 > entries) || (n > 10 && n * 4 > entries))
}

fn hold_mass_deletes(steps: Vec<Step>, entries: usize) -> Planned {
    let is_del = |s: &Step| matches!(s.op, Op::DeleteLedge { .. } | Op::DeleteExt { .. });
    let n = steps.iter().filter(|s| is_del(s)).count();
    if !mass_delete(n, entries) {
        return Planned { steps, held_deletes: 0 };
    }
    let held = |s: Step| {
        let gone = if matches!(s.op, Op::DeleteLedge { .. }) { Gone::Ext } else { Gone::Ledge };
        let tomb: Vec<Entry> = s.fallback.iter().map(|e| Entry { gone, ..e.clone() }).collect();
        Step { op: Op::Keep, keep: tomb.clone(), fallback: tomb }
    };
    let steps = steps.into_iter().map(|s| if is_del(&s) { held(s) } else { s }).collect();
    Planned { steps, held_deletes: n }
}
