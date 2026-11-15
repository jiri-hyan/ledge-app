//! Zápis odkazu na založený úkol zpět do poznámky. Čisté funkce nad tělem (I/O dělá příkaz přes
//! `notes::ops::rewrite` — zámek cesty, kontrola změny na disku, atomický zápis).
//!
//! Úkoly: pod bodem (a jeho snímky) vznikne odsazený řádek `↳ [{název}](https://…/?tab=tasks&task=ID)`.
//! Je to běžný odstavec pokračující položku seznamu (platný Markdown, v náhledu klikatelný), přežije
//! přesun bodu a je vidět přímo v editoru; podle něj se pozná, že bod už odeslaný je (`plan::Item::sent`).
//! Poznámka jako celek (1 : 1) má `connector: <id poznámky>` ve frontmatteru.

use super::client::Outcome;
use super::plan::SENT_MARK;
use crate::notes::{brief, taskline};
use std::path::Path;

/// Štítek v řádku: název z configu bez hranatých závorek a zalomení; prázdný = „Úkol“.
fn mark_label(name: &str) -> String {
    let n: String = name.chars().filter(|c| !matches!(c, '[' | ']') && !c.is_control()).collect();
    let n = n.trim();
    if n.is_empty() { "Úkol".to_string() } else { n.to_string() }
}

/// Řádek odeslaného bodu: `↳ [{název}](url)`; bez adresy `↳ [{název}] id`; bez obojího jen `↳ [{název}]`
/// (nejde o odkaz, ale o platný text). Začíná `SENT_MARK`, takže ho čtečky rozpoznají pro libovolný název.
pub fn link_line(o: &Outcome, name: &str) -> String {
    let lab = mark_label(name);
    match (&o.url, &o.id) {
        (Some(u), _) => format!("{SENT_MARK}{lab}]({u})"),
        (None, Some(id)) => format!("{SENT_MARK}{lab}] {id}"),
        (None, None) => format!("{SENT_MARK}{lab}]"),
    }
}

/// Který bod odpovídá výsledku: podle textu (bez `⏰`), `line` je jen nápověda pro stejné texty. Bez
/// textu (starý tvar) rozhoduje řádek. Víc shod bez sedícího řádku nebo žádná = `None` (nehádá se).
fn locate(items: &[brief::Item], o: &Outcome) -> Option<usize> {
    if o.text.trim().is_empty() {
        return brief::find_item(items, Some(o.line), None).ok();
    }
    let hits: Vec<usize> = (0..items.len()).filter(|&k| taskline::set_due(&items[k].text, None).trim() == o.text.trim()).collect();
    hits.iter().copied().find(|&k| items[k].line == o.line).or(if let [k] = hits.as_slice() { Some(*k) } else { None })
}

/// Doplní odkazy pod body (štítek = `name`, viz `link_line`). `done` = výsledky (`Outcome::text` + řádek jako nápověda); bod, který v těle
/// už není (nebo ho nejde určit), se NEzapíše na jiné místo, ale vrátí se jeho řádek ze starého plánu.
/// Vrací nové tělo, počet doplněných řádků a nenalezené body. Existující odkaz pod bodem se nepřidává podruhé.
pub fn add_links(body: &str, done: &[Outcome], name: &str) -> (String, usize, Vec<usize>) {
    let sc = brief::scan_body(body, Path::new(""));
    let mut lines: Vec<String> = body.split('\n').map(str::to_string).collect();
    let mut at: Vec<(usize, String)> = Vec::new();
    let mut missed: Vec<usize> = Vec::new();
    for o in done.iter().filter(|o| o.ok) {
        let Some(k) = locate(&sc.items, o) else {
            missed.push(o.line);
            continue;
        };
        let it = &sc.items[k];
        let after = it.line + 1 + it.images.len();
        if lines.get(after).is_some_and(|l| l.trim_start().starts_with(SENT_MARK)) {
            continue;
        }
        let b = &lines[it.line];
        let lead = &b[..b.len() - b.trim_start().len()];
        let numbered = b.trim_start().starts_with(|c: char| c.is_ascii_digit());
        let cr = if b.ends_with('\r') { "\r" } else { "" };
        at.push((after, format!("{lead}{}{}{cr}", " ".repeat(if numbered { 3 } else { 2 }), link_line(o, name))));
    }
    // odspodu nahoru, ať se čísla řádků neposouvají
    at.sort_by_key(|a| std::cmp::Reverse(a.0));
    let n = at.len();
    for (i, l) in at {
        lines.insert(i.min(lines.len()), l);
    }
    (lines.join("\n"), n, missed)
}

/// `connector:` z frontmatteru poznámky (celý text souboru) = už odeslaná jako poznámka.
pub fn connector_id(file_text: &str) -> Option<String> {
    let (front, _) = crate::notes::split(file_text);
    crate::notes::get(&front?, "connector").filter(|v| !v.is_empty())
}
