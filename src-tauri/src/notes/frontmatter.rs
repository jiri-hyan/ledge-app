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

/// Nastaví / odebere víc klíčů najednou (`Some` = nastav, `None` = odeber). Pořadí a neznámé klíče
/// zůstanou, nový klíč jde na konec. Hodnotu volající dodá už bezpečnou (`yaml_value`).
pub fn set_meta(front: &mut Vec<String>, pairs: &[(&str, Option<&str>)]) {
    for (k, v) in pairs {
        match v {
            Some(v) => set(front, k, v),
            None => unset(front, k),
        }
    }
}

/// Hodnoty `status:` zadání.
pub const STATUSES: [&str; 6] = ["draft", "ready", "running", "waiting", "review", "done"];
/// Hodnoty `model:` zadání.
pub const MODELS: [&str; 3] = ["sonnet", "opus", "haiku"];

/// Peníze z frontmatteru: konečné číslo ≥ 0.
pub(crate) fn money(v: Option<String>) -> Option<f64> {
    v?.trim().parse::<f64>().ok().filter(|x| x.is_finite() && *x >= 0.0)
}

/// Hodnota klíče, jen když je v povoleném seznamu (bez ohledu na velikost písmen).
pub(crate) fn one_of(v: Option<String>, allowed: &[&str]) -> Option<String> {
    let v = v?.trim().to_ascii_lowercase();
    allowed.contains(&v.as_str()).then_some(v)
}

/// Id session: jen `[A-Za-z0-9-]`, nejvýš 64 znaků (jde na příkazový řádek).
pub(crate) fn session_id(v: Option<String>) -> Option<String> {
    let v = v?.trim().to_string();
    (!v.is_empty() && v.len() <= 64 && v.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')).then_some(v)
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

/// Jako `with_done` pro `set_meta`: text → text, tělo bajtově stejné.
fn with_meta(text: &str, pairs: &[(&str, Option<&str>)]) -> String {
    let (front, body) = split(text);
    let mut front = front.unwrap_or_default();
    set_meta(&mut front, pairs);
    render(&front, body)
}

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

    fn texts(seed: u64, count: usize, pieces: &[&str]) -> Vec<String> {
        let mut rng = crate::testrng::Rng(seed);
        (0..count).map(|_| rng.soup(pieces, 13)).collect()
    }

    #[test]
    fn property_split_render_roundtrip_and_with_done_keeps_body() {
        let pieces = ["---\n", "---", "a: 1\n", "kind: \"x y\"\n", "# c\n", "  - x\n", "\n", "\r\n", "é", "\u{feff}", "text ", "--- \n", ":\n", "\t", "done: true\n"];
        for t in texts(0x9E37_79B9_7F4A_7C15, 4000, &pieces) {
            let (front, body) = split(&t);
            let bomless = t.trim_start_matches('\u{feff}');
            match &front {
                // bez CRLF se text složí zpět přesně (tělo je bajtově beze změny)
                // (uzavírací `---` na posledním řádku bez konce řádku dostane při složení `\n`)
                Some(f) if !t.contains('\r') => {
                    let r = render(f, body);
                    assert!(r == bomless || (body.is_empty() && r == format!("{bomless}\n")), "{t:?}");
                }
                Some(_) => assert!(bomless.ends_with(body), "{t:?}"),
                None => assert_eq!(body, bomless, "bez hlavičky je tělo celý text: {t:?}"),
            }
            for done in [true, false] {
                let out = with_done(&t, done);
                let (f2, body2) = split(&out);
                assert_eq!(body2, body, "with_done nesmí sáhnout na tělo: {t:?}");
                assert_eq!(get(&f2.expect("hlavička"), "done").as_deref(), Some(if done { "true" } else { "false" }), "{t:?}");
                assert_eq!(with_done(&out, done), out, "idempotence: {t:?}");
            }
        }
    }

    #[test]
    fn hostile_input_does_not_panic() {
        let huge_line = "k: ".to_string() + &"é".repeat(1_000_000);
        let cases = [String::new(), "---".into(), "---\n".into(), "\u{feff}".into(), "---\n---".into(), format!("---\n{huge_line}\n---\nx"), "\n\n\n".into(), "---\r".into(), "\0\0---\n".into()];
        for t in &cases {
            let (f, body) = split(t);
            assert!(f.is_some() || body.len() <= t.len());
            let _ = with_done(t, true);
        }
        let mut f: Vec<String> = vec![];
        set(&mut f, "", "x"); // prázdný klíč se jen připojí, nespadne
        unset(&mut f, "");
        assert_eq!(get(&f, "kind"), None);
        for v in ["", "\"", "''", "\"x'", "'", "\u{e9}\"\u{e9}"] {
            let q = yaml_value(v);
            assert!(!q.contains(['\n', '\r']), "{v:?}");
            let _ = get(&[format!("k: {q}")], "k");
        }
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

    #[test]
    fn set_meta_keeps_order_unknown_keys_and_body() {
        let text = "---\ncreated: 2026-11-11T10:00:00+01:00\nx-vlastni: \"a: b\"\nstatus: draft\nlist:\n  - 1\ndone: false\n---\n\nTělo\r\n- [ ] a\n";
        let out = with_meta(text, &[("status", Some("ready")), ("cost", Some("0.5")), ("budget", None)]);
        assert_eq!(
            out,
            "---\ncreated: 2026-11-11T10:00:00+01:00\nx-vlastni: \"a: b\"\nstatus: ready\nlist:\n  - 1\ndone: false\ncost: 0.5\n---\n\nTělo\r\n- [ ] a\n"
        );
        // odebrání klíčů a prázdný seznam změn
        let gone = with_meta(&out, &[("status", None), ("cost", None)]);
        assert_eq!(gone, text.replace("status: draft\n", ""));
        assert_eq!(with_meta(text, &[]), text);
        // bez frontmatteru vznikne nový, tělo zůstane
        assert_eq!(with_meta("jen text\n", &[("status", Some("draft"))]), "---\nstatus: draft\n---\njen text\n");
    }
}
