//! Sekce zadání v těle poznámky: `## Otázky` (otázka `- ❓ text`, odpověď odsazeně `  - ↳ text`) a
//! `## Log` (agent jen připisuje `- <ISO čas> text`). Čisté funkce nad tělem; mění se jen řádky sekce,
//! zbytek zůstává bajtově stejný (CRLF se zachová, koncové zalomení také).

use super::brief::one_line;
use super::md::{heading, is_fence};
use super::md_list::indent_width;
use crate::i18n::tr;
use serde::Serialize;

pub const QUESTIONS: &str = "Otázky";
pub const LOG: &str = "Log";

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Question {
    /// pořadí otázky od 0
    pub index: usize,
    /// řádek otázky v těle od 0
    pub line: usize,
    pub text: String,
    pub answer: Option<String>,
}

fn is_name(title: &str, name: &str) -> bool {
    let fold = |s: &str| s.trim().to_lowercase().replace('á', "a");
    let t = fold(title);
    t == fold(name) || (name == QUESTIONS && t == "questions")
}

/// Sekce `## název`: (řádek nadpisu, první řádek za sekcí). Konec = další nadpis úrovně 1–2.
fn section(lines: &[&str], name: &str) -> Option<(usize, usize)> {
    let mut code = false;
    let mut start = None;
    for (i, l) in lines.iter().enumerate() {
        if is_fence(l) {
            code = !code;
            continue;
        }
        if code {
            continue;
        }
        if let Some((lv, title)) = heading(l.trim_end_matches('\r')) {
            match start {
                Some(s) if lv <= 2 => return Some((s, i)),
                None if lv == 2 && is_name(title, name) => start = Some(i),
                _ => {}
            }
        }
    }
    start.map(|s| (s, lines.len()))
}

fn cr_of(body: &str) -> &'static str {
    if body.contains("\r\n") { "\r" } else { "" }
}

fn bullet<'a>(line: &'a str, mark: &str) -> Option<&'a str> {
    line.trim_start().strip_prefix("- ")?.strip_prefix(mark).map(str::trim)
}

/// Otázky ze sekce `## Otázky` s případnou odpovědí (`  - ↳ …`; víc řádků odpovědi se spojí mezerou).
pub fn questions(body: &str) -> Vec<Question> {
    let lines: Vec<&str> = body.split('\n').map(|l| l.trim_end_matches('\r')).collect();
    let Some((s, e)) = section(&lines, QUESTIONS) else { return Vec::new() };
    let mut out: Vec<Question> = Vec::new();
    for (i, l) in lines.iter().enumerate().take(e).skip(s + 1) {
        if indent_width(l) == 0 {
            if let Some(t) = bullet(l, "❓") {
                out.push(Question { index: out.len(), line: i, text: t.to_string(), answer: None });
            }
        } else if let (Some(q), Some(a)) = (out.last_mut(), bullet(l, "↳")) {
            q.answer = Some(match q.answer.take() {
                Some(prev) => format!("{prev} {a}"),
                None => a.to_string(),
            });
        }
    }
    out
}

/// Připojí řádky na konec sekce `name` (založí ji na konci těla, když chybí).
fn push_lines(body: &str, name: &str, new: &[String]) -> String {
    let cr = cr_of(body);
    let mut lines: Vec<String> = body.split('\n').map(str::to_string).collect();
    let refs: Vec<&str> = lines.iter().map(String::as_str).collect();
    match section(&refs, name) {
        Some((s, e)) => {
            // za poslední neprázdný řádek sekce
            let mut at = e;
            while at > s + 1 && lines[at - 1].trim().is_empty() {
                at -= 1;
            }
            for (k, n) in new.iter().enumerate() {
                lines.insert(at + k, format!("{n}{cr}"));
            }
            lines.join("\n")
        }
        None => {
            let trimmed = body.trim_end_matches(['\n', '\r']);
            let tail = &body[trimmed.len()..];
            let sep = if trimmed.is_empty() { String::new() } else { format!("{cr}\n{cr}\n") };
            let mut out = format!("{trimmed}{sep}## {name}{cr}\n{cr}\n");
            out.push_str(&new.iter().map(|n| format!("{n}{cr}")).collect::<Vec<_>>().join("\n"));
            out.push_str(if tail.is_empty() { "" } else { "\n" });
            out
        }
    }
}

/// Přidá otázky (každá jednořádková) do sekce `## Otázky`; prázdné se vynechají.
pub fn add_questions(body: &str, qs: &[String]) -> Result<String, String> {
    let new: Vec<String> = qs.iter().map(|q| one_line(q)).filter(|q| !q.is_empty()).map(|q| format!("- ❓ {q}")).collect();
    if new.is_empty() {
        return Err(tr("Chybí text otázky.", "Missing the question text.").into());
    }
    Ok(push_lines(body, QUESTIONS, &new))
}

/// Zapíše odpověď k otázce `index` (nahradí případnou starou; prázdná odpověď ji odebere).
pub fn answer(body: &str, index: usize, text: &str) -> Result<String, String> {
    let qs = questions(body);
    let q = qs.get(index).ok_or_else(|| trf!("Otázka {index} neexistuje (je jich {}).", "There's no question {index} ({} in total).", qs.len()))?;
    let cr = cr_of(body);
    let mut lines: Vec<String> = body.split('\n').map(str::to_string).collect();
    // stará odpověď = odsazené `- ↳` řádky hned za otázkou
    let mut end = q.line + 1;
    while end < lines.len() && indent_width(&lines[end]) > 0 && bullet(lines[end].trim_end_matches('\r'), "↳").is_some() {
        end += 1;
    }
    lines.drain(q.line + 1..end);
    let a = one_line(text);
    if !a.is_empty() {
        lines.insert(q.line + 1, format!("  - ↳ {a}{cr}"));
    }
    Ok(lines.join("\n"))
}

/// Připíše do `## Log` řádek `- <čas> text` (sekci založí).
pub fn log_append(body: &str, iso: &str, text: &str) -> Result<String, String> {
    let t = one_line(text);
    if t.is_empty() {
        return Err(tr("Chybí text záznamu.", "Missing the log text.").into());
    }
    Ok(push_lines(body, LOG, &[format!("- {iso} {t}")]))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_questions_and_answers() {
        let b = "# Cíl\n\n## Otázky\n\n- ❓ Který branch?\n  - ↳ main\n- ❓ Smím měnit schéma?\n- ❓ Třetí\n  - ↳ ano\n  - ↳ ale opatrně\n\n## Log\n- 2026-11-11T10:00:00+01:00 začal\n";
        let q = questions(b);
        assert_eq!(q.len(), 3);
        assert_eq!((q[0].index, q[0].line, q[0].text.as_str(), q[0].answer.as_deref()), (0, 4, "Který branch?", Some("main")));
        assert_eq!(q[1].answer, None);
        assert_eq!(q[2].answer.as_deref(), Some("ano ale opatrně"));
        // otázky mimo sekci, v kódu nebo bez sekce se nepočítají
        assert!(questions("- ❓ volná\n").is_empty());
        assert!(questions("```\n## Otázky\n- ❓ x\n```\n").is_empty());
        assert_eq!(questions("## Questions\n- ❓ a\n").len(), 1);
        assert_eq!(questions("## Otazky\n- ❓ a\n### pod\n- ❓ b\n# Jiné\n- ❓ c\n").len(), 2);
    }

    #[test]
    fn add_questions_table() {
        let qs = |v: &[&str]| v.iter().map(|s| s.to_string()).collect::<Vec<_>>();
        // sekce chybí: založí se na konci, koncové zalomení zůstane
        assert_eq!(add_questions("# Cíl\n- [ ] a\n", &qs(&["Proč?"])).unwrap(), "# Cíl\n- [ ] a\n\n## Otázky\n\n- ❓ Proč?\n");
        assert_eq!(add_questions("text", &qs(&["a", "b\nc"])).unwrap(), "text\n\n## Otázky\n\n- ❓ a\n- ❓ b c");
        assert_eq!(add_questions("", &qs(&["a"])).unwrap(), "## Otázky\n\n- ❓ a");
        // sekce je: za poslední otázku (i za odpověď), před další sekcí; prázdné řádky před další sekcí zůstanou
        let b = "## Otázky\n\n- ❓ a\n  - ↳ b\n\n## Log\n- x\n";
        assert_eq!(add_questions(b, &qs(&["c"])).unwrap(), "## Otázky\n\n- ❓ a\n  - ↳ b\n- ❓ c\n\n## Log\n- x\n");
        // prázdné otázky = chyba
        assert!(add_questions(b, &qs(&["  ", ""])).is_err());
        // CRLF
        assert_eq!(add_questions("a\r\n", &qs(&["q"])).unwrap(), "a\r\n\r\n## Otázky\r\n\r\n- ❓ q\r\n");
        assert_eq!(add_questions("## Otázky\r\n- ❓ a\r\n", &qs(&["q"])).unwrap(), "## Otázky\r\n- ❓ a\r\n- ❓ q\r\n");
    }

    #[test]
    fn answer_table() {
        let b = "## Otázky\n- ❓ a\n- ❓ b\n  - ↳ stará\n  - ↳ druhý řádek\n- ❓ c\n\ntělo\n";
        assert_eq!(answer(b, 0, "ano").unwrap(), "## Otázky\n- ❓ a\n  - ↳ ano\n- ❓ b\n  - ↳ stará\n  - ↳ druhý řádek\n- ❓ c\n\ntělo\n");
        assert_eq!(answer(b, 1, "nová\nvíc").unwrap(), "## Otázky\n- ❓ a\n- ❓ b\n  - ↳ nová víc\n- ❓ c\n\ntělo\n");
        assert_eq!(answer(b, 1, "").unwrap(), "## Otázky\n- ❓ a\n- ❓ b\n- ❓ c\n\ntělo\n");
        assert_eq!(answer(b, 2, "z").unwrap(), b.replace("- ❓ c\n", "- ❓ c\n  - ↳ z\n"));
        assert!(answer(b, 3, "x").is_err());
        assert_eq!(answer("## Otázky\r\n- ❓ a\r\n", 0, "ok").unwrap(), "## Otázky\r\n- ❓ a\r\n  - ↳ ok\r\n");
        // odpověď se po zápisu přečte zpět
        assert_eq!(questions(&answer(b, 0, "ano").unwrap())[0].answer.as_deref(), Some("ano"));
    }

    #[test]
    fn log_append_creates_and_appends() {
        let t = "2026-11-11T10:00:00+01:00";
        assert_eq!(log_append("# Cíl\n", t, "start").unwrap(), format!("# Cíl\n\n## Log\n\n- {t} start\n"));
        let once = log_append("# Cíl\n", t, "start").unwrap();
        assert_eq!(log_append(&once, t, "dál\nřádek").unwrap(), format!("# Cíl\n\n## Log\n\n- {t} start\n- {t} dál řádek\n"));
        // za Logem následuje jiná sekce → připíše se před ni
        assert_eq!(log_append("## Log\n- a\n## Poznámky\ntext\n", t, "b").unwrap(), format!("## Log\n- a\n- {t} b\n## Poznámky\ntext\n"));
        assert!(log_append("x", t, " \n ").is_err());
        // zbytek těla zůstane bajtově stejný
        let body = "úvod\r\n\r\n- [ ] a\r\n";
        assert!(log_append(body, t, "x").unwrap().starts_with(body.trim_end()));
    }
}
