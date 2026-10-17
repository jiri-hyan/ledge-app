//! Frontmatter poznámky: ruční tolerantní parser (`split`, `get`), úpravy řádků (`set`, `unset`),
//! zpětné složení (`render`) a přepnutí `done` (`with_done`). Neznámé klíče, pořadí i tělo zůstávají
//! bajtově stejné. Čisté funkce bez I/O.

/// Nejvýš tolik řádků smí mít frontmatter — dál už je to text, ne hlavička.
const MAX_FRONT_LINES: usize = 30;

/// Řádek, který do frontmatteru patří: `klíč: hodnota` (klíč `[A-Za-z0-9_-]`), odsazené
/// pokračování (YAML seznam/objekt), prázdný řádek nebo komentář `#`.
fn front_line_ok(line: &str) -> bool {
    if line.trim().is_empty() || line.starts_with([' ', '\t']) || line.starts_with('#') {
        return true;
    }
    line.split_once(':').is_some_and(|(k, _)| !k.is_empty() && k.chars().all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-'))
}

/// Rozdělí text na řádky frontmatteru (bez `---`) a tělo (vše za uzavírací `---`, bajtově beze změny).
/// Frontmatter je jen tehdy, když první řádek je přesně `---`, uzavírací `---` přijde do 30 řádků a
/// mezi nimi jsou jen řádky `klíč: hodnota` (viz `front_line_ok`). Jinak `(None, celý text)` —
/// tělo začínající oddělovačem `---` se tak za hlavičku nevydává. Snese BOM a CRLF.
pub fn split(text: &str) -> (Option<Vec<String>>, &str) {
    let t = text.trim_start_matches('\u{feff}');
    let mut lines = t.split_inclusive('\n');
    let Some(first) = lines.next() else { return (None, t) };
    if first.trim_end_matches(['\n', '\r']) != "---" {
        return (None, t);
    }
    let mut off = first.len();
    let mut front = Vec::new();
    for l in lines.take(MAX_FRONT_LINES + 1) {
        off += l.len();
        let line = l.trim_end_matches(['\n', '\r']);
        if line == "---" {
            return (Some(front), &t[off..]);
        }
        if !front_line_ok(line) {
            return (None, t);
        }
        front.push(line.to_string());
    }
    // neuzavřený / moc dlouhý frontmatter → nebrat jako frontmatter
    (None, t)
}

fn unquote(v: &str) -> String {
    let v = v.trim();
    let q = v.len() >= 2 && ((v.starts_with('"') && v.ends_with('"')) || (v.starts_with('\'') && v.ends_with('\'')));
    if q { v[1..v.len() - 1].to_string() } else { v.to_string() }
}

/// Hodnota klíče z řádků frontmatteru (`klíč: hodnota`, uvozovky volitelné).
pub fn get(front: &[String], key: &str) -> Option<String> {
    front.iter().find_map(|l| {
        let (k, v) = l.split_once(':')?;
        (k.trim() == key && !l.starts_with([' ', '\t'])).then(|| unquote(v))
    })
}

fn is_key(l: &str, key: &str) -> bool {
    !l.starts_with([' ', '\t']) && l.split_once(':').is_some_and(|(k, _)| k.trim() == key)
}

/// Odebere klíč (jen řádek nejvyšší úrovně; odsazená pokračování cizích klíčů zůstanou).
pub fn unset(front: &mut Vec<String>, key: &str) {
    front.retain(|l| !is_key(l, key));
}

/// Nastaví klíč: přepíše existující řádek, jinak připojí na konec. Ostatní řádky beze změny.
pub fn set(front: &mut Vec<String>, key: &str, value: &str) {
    let line = format!("{key}: {value}");
    match front.iter_mut().find(|l| is_key(l, key)) {
        Some(l) => *l = line,
        None => front.push(line),
    }
}

pub fn render(front: &[String], body: &str) -> String {
    let mut s = String::from("---\n");
    for l in front {
        s.push_str(l);
        s.push('\n');
    }
    s.push_str("---\n");
    s.push_str(body);
    s
}

/// Přepne `done` a nic jiného (neznámé klíče, pořadí i tělo zůstanou).
pub fn with_done(text: &str, done: bool) -> String {
    let (front, body) = split(text);
    let mut front = front.unwrap_or_default();
    set(&mut front, "done", if done { "true" } else { "false" });
    render(&front, body)
}

/// Hodnota do frontmatteru: bezpečné id jak je, jinak v uvozovkách (`get` je zase sundá).
pub(crate) fn yaml_value(v: &str) -> String {
    if !v.is_empty() && v.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.')) {
        v.to_string()
    } else {
        format!("\"{}\"", v.replace(['"', '\n', '\r'], ""))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frontmatter_split_tolerates_bom_crlf_and_missing() {
        let (f, body) = split("\u{feff}---\r\nproject: a\r\ndone: true\r\n---\r\nahoj\r\n");
        assert_eq!(f.unwrap(), vec!["project: a", "done: true"]);
        assert_eq!(body, "ahoj\r\n");
        let (f, body) = split("jen text\n---\n");
        assert!(f.is_none());
        assert_eq!(body, "jen text\n---\n");
        // neuzavřený frontmatter = žádný frontmatter
        let (f, _) = split("---\nproject: a\n");
        assert!(f.is_none());
        let (f, body) = split("---\n---\n");
        assert_eq!(f.unwrap(), Vec::<String>::new());
        assert_eq!(body, "");
    }

    #[test]
    fn frontmatter_get_unquotes_and_ignores_nested() {
        let f: Vec<String> = ["project: \"atlas\"", "remind: '2026-10-17T09:00:00+02:00'", "tags:", "  done: true", "done: false"]
            .map(String::from)
            .to_vec();
        assert_eq!(get(&f, "project").as_deref(), Some("atlas"));
        assert_eq!(get(&f, "remind").as_deref(), Some("2026-10-17T09:00:00+02:00"));
        assert_eq!(get(&f, "done").as_deref(), Some("false"));
        assert_eq!(get(&f, "nic"), None);
    }

    #[test]
    fn set_done_preserves_unknown_keys_order_and_body() {
        let src = "---\nproject: a\nautor: claude\ntags: [x, y]\ndone: false\nzz: 1\n---\n\nTělo\n\n![](./a-1.png)\n";
        let out = with_done(src, true);
        assert_eq!(out, "---\nproject: a\nautor: claude\ntags: [x, y]\ndone: true\nzz: 1\n---\n\nTělo\n\n![](./a-1.png)\n");
        assert_eq!(with_done(&out, false), src);
        // bez done → připojí; bez frontmatteru → vytvoří
        assert_eq!(with_done("---\nx: 1\n---\nb", true), "---\nx: 1\ndone: true\n---\nb");
        assert_eq!(with_done("jen text\n", true), "---\ndone: true\n---\njen text\n");
    }

    #[test]
    fn frontmatter_needs_key_value_lines_and_close_within_30() {
        // tělo začínající oddělovačem není hlavička
        let t = "---\nTohle je odstavec bez dvojtečky\n---\n";
        assert!(split(t).0.is_none());
        assert!(split("--- \nproject: a\n---\n").0.is_none());
        let long = format!("---\n{}---\n", "a: 1\n".repeat(31));
        assert!(split(&long).0.is_none());
        let ok = format!("---\n{}---\nb", "a: 1\n".repeat(30));
        assert_eq!(split(&ok).0.unwrap().len(), 30);
        // YAML seznam, komentář a prázdný řádek jsou v pořádku
        let (f, body) = split("---\ntags:\n  - x\n# pozn\n\nproject: a\n---\ntělo");
        assert_eq!(f.unwrap().len(), 5);
        assert_eq!(body, "tělo");
        // set_done na „falešné“ hlavičce nic nerozbije — přidá skutečnou nad ni
        assert_eq!(with_done(t, true), format!("---\ndone: true\n---\n{t}"));
    }
}
