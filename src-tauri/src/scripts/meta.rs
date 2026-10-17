//! Hlavička skriptu → `Meta`. Metadata jsou řádky `@ledge.<klíč> <hodnota>` v komentáři na začátku
//! souboru (čte se jen prvních `HEADER_BYTES`). Čisté: dostane text a jazyk, na disk nesahá.
//! Soubor bez jediného `@ledge.` řádku není skript Ledge (pomocný modul) → `Ok(None)`.

use crate::i18n::tr;
use serde::Serialize;

/// Kolik bajtů ze začátku souboru se čte pro hlavičku.
pub const HEADER_BYTES: usize = 4096;
pub const MAX_ARGS: usize = 32;
const MAX_ARG: usize = 1024;
pub const MAX_TITLE: usize = 80;
const MAX_DESC: usize = 160;
const MAX_ICON: usize = 8;
const MAX_GROUP: usize = 40;
pub const DEFAULT_TIMEOUT_S: u32 = 60;

/// Jazyk podle přípony — určuje komentáře v hlavičce a interpret.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Lang {
    Powershell,
    Cmd,
    Python,
    Node,
}

impl Lang {
    pub fn from_ext(ext: &str) -> Option<Lang> {
        match ext.to_ascii_lowercase().as_str() {
            "ps1" => Some(Lang::Powershell),
            "cmd" | "bat" => Some(Lang::Cmd),
            "py" => Some(Lang::Python),
            "js" | "mjs" => Some(Lang::Node),
            _ => None,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Lang::Powershell => "PowerShell",
            Lang::Cmd => "cmd",
            Lang::Python => "Python",
            Lang::Node => "Node.js",
        }
    }
}

/// Jak se skript spustí: `hidden` = bez okna, jen hláška hotovo/chyba; `output` (výchozí) = bez
/// okna, výstup v šuplíku; `terminal` = viditelné okno konzole, které zůstane otevřené.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Mode {
    Hidden,
    #[default]
    Output,
    Terminal,
}

impl Mode {
    pub fn key(self) -> &'static str {
        match self {
            Mode::Hidden => "hidden",
            Mode::Output => "output",
            Mode::Terminal => "terminal",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Meta {
    pub title: String,
    pub description: Option<String>,
    /// emoji / krátký text místo monogramu (nejvýš 8 znaků)
    pub icon: Option<String>,
    pub group: Option<String>,
    pub mode: Mode,
    /// ptát se před každým spuštěním (jinak jen poprvé a po změně souboru)
    pub confirm: bool,
    /// pevné argumenty předané skriptu (každý zvlášť)
    pub args: Vec<String>,
    /// termín bez okna (s), pak se strom procesu zabije
    pub timeout_s: u32,
}

/// Text hlavičky z prvních bajtů souboru: BOM pryč, a když se čtení useklo (`truncated`), zahodí se
/// poslední neúplný řádek.
pub fn header_text(bytes: &[u8], truncated: bool) -> String {
    let b = bytes.strip_prefix(&[0xEF, 0xBB, 0xBF]).unwrap_or(bytes);
    let b = if truncated { b.iter().rposition(|&c| c == b'\n').map_or(&b[..0], |i| &b[..i]) } else { b };
    String::from_utf8_lossy(b).into_owned()
}

/// Řádek bez značky komentáře, pokud je to místo, kde smí být metadata. `cmd` je musí mít
/// v `::` / `rem` (jinak by se řádek provedl); ostatní je berou i uvnitř blokového komentáře
/// (`<# … #>`, docstring, `/* … */`).
fn strip_comment(line: &str, lang: Lang) -> Option<&str> {
    let l = line.trim_start();
    match lang {
        Lang::Cmd => {
            if let Some(r) = l.strip_prefix("::") {
                return Some(r);
            }
            let low = l.get(..4).map(str::to_ascii_lowercase).unwrap_or_default();
            let rest = if low.starts_with("rem") { &l[3..] } else if low == "@rem" { &l[4..] } else { return None };
            (rest.is_empty() || rest.starts_with([' ', '\t'])).then_some(rest)
        }
        Lang::Powershell | Lang::Python => Some(l.strip_prefix("<#").or_else(|| l.strip_prefix('#')).unwrap_or(l)),
        Lang::Node => Some(l.strip_prefix("//").or_else(|| l.strip_prefix("/*")).or_else(|| l.strip_prefix('*')).unwrap_or(l)),
    }
}

/// Argumenty: oddělené mezerou, `"…"` drží mezery pohromadě. Uvozovky uvnitř ani nový řádek ne.
pub fn split_args(s: &str) -> Result<Vec<String>, String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut quoted = false;
    let mut any = false;
    for c in s.chars() {
        match c {
            '"' => {
                quoted = !quoted;
                any = true;
            }
            c if c.is_whitespace() && !quoted => {
                if any {
                    out.push(std::mem::take(&mut cur));
                    any = false;
                }
            }
            '\0' => return Err(tr("argument obsahuje nulový znak", "an argument contains a null character").into()),
            c => {
                cur.push(c);
                any = true;
            }
        }
    }
    if quoted {
        return Err(tr("neuzavřené uvozovky v @ledge.args", "unclosed quotes in @ledge.args").into());
    }
    if any {
        out.push(cur);
    }
    if out.len() > MAX_ARGS || out.iter().any(|a| a.len() > MAX_ARG) {
        return Err(trf!("@ledge.args: nejvýš {MAX_ARGS} argumentů po {MAX_ARG} znacích", "@ledge.args: at most {MAX_ARGS} arguments of {MAX_ARG} characters"));
    }
    Ok(out)
}

fn flag(v: &str) -> Option<bool> {
    match v.to_ascii_lowercase().as_str() {
        "" | "true" | "yes" | "ano" | "1" => Some(true),
        "false" | "no" | "ne" | "0" => Some(false),
        _ => None,
    }
}

fn cut(s: &str, max: usize) -> String {
    if s.chars().count() <= max { s.to_string() } else { format!("{}…", s.chars().take(max - 1).collect::<String>()) }
}

/// Metadata z textu hlavičky. `Ok(None)` = žádné `@ledge.` (není to skript Ledge), `Err` = hlavička
/// je, ale chybná (skript se ukáže s problémem a nespustí se). Neznámé klíče se přeskočí (novější
/// verze), u opakovaného klíče platí první.
pub fn parse(text: &str, lang: Lang) -> Result<Option<Meta>, String> {
    let mut seen: Vec<(String, String)> = Vec::new();
    for line in text.lines() {
        let Some(rest) = strip_comment(line, lang) else { continue };
        let Some(kv) = rest.trim().strip_prefix("@ledge.") else { continue };
        let (k, v) = kv.split_once(char::is_whitespace).unwrap_or((kv, ""));
        let k = k.to_ascii_lowercase();
        let v = v.trim().trim_end_matches("#>").trim_end_matches("*/").trim();
        if !seen.iter().any(|(sk, _)| *sk == k) {
            seen.push((k, v.to_string()));
        }
    }
    if seen.is_empty() {
        return Ok(None);
    }
    let get = |k: &str| seen.iter().find(|(sk, _)| sk == k).map(|(_, v)| v.as_str());
    let title = get("title").unwrap_or("").trim();
    if title.is_empty() {
        return Err(tr("chybí @ledge.title", "@ledge.title is missing").into());
    }
    if title.chars().count() > MAX_TITLE {
        return Err(trf!("@ledge.title má nejvýš {MAX_TITLE} znaků", "@ledge.title has at most {MAX_TITLE} characters"));
    }
    let mode = match get("mode").map(str::to_ascii_lowercase).as_deref() {
        None | Some("") | Some("output") => Mode::Output,
        Some("hidden") => Mode::Hidden,
        Some("terminal") => Mode::Terminal,
        Some(m) => return Err(trf!("neznámý @ledge.mode „{m}“ (hidden, output, terminal)", "unknown @ledge.mode “{m}” (hidden, output, terminal)")),
    };
    let confirm = match get("confirm") {
        None => false,
        Some(v) => flag(v).ok_or_else(|| trf!("@ledge.confirm „{v}“ — čekám true/false", "@ledge.confirm “{v}” — expected true/false"))?,
    };
    let icon = match get("icon").filter(|v| !v.is_empty()) {
        None => None,
        Some(v) if v.chars().count() <= MAX_ICON && !v.contains(char::is_whitespace) => Some(v.to_string()),
        Some(_) => return Err(trf!("@ledge.icon = emoji nebo nejvýš {MAX_ICON} znaků bez mezer", "@ledge.icon = an emoji or at most {MAX_ICON} characters without spaces")),
    };
    let group = match get("group").filter(|v| !v.is_empty()) {
        Some(g) if g.chars().count() > MAX_GROUP => return Err(trf!("@ledge.group má nejvýš {MAX_GROUP} znaků", "@ledge.group has at most {MAX_GROUP} characters")),
        g => g.map(str::to_string),
    };
    let timeout_s = match get("timeout") {
        None => DEFAULT_TIMEOUT_S,
        Some(v) => v.parse::<u32>().ok().filter(|n| (1..=3600).contains(n)).ok_or_else(|| tr("@ledge.timeout musí být 1–3600 s", "@ledge.timeout must be 1–3600 s"))?,
    };
    Ok(Some(Meta {
        title: title.to_string(),
        description: get("description").filter(|v| !v.is_empty()).map(|v| cut(v, MAX_DESC)),
        icon,
        group,
        mode,
        confirm,
        args: split_args(get("args").unwrap_or(""))?,
        timeout_s,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ok(text: &str, lang: Lang) -> Meta {
        parse(text, lang).unwrap().unwrap()
    }

    /// (jazyk, hlavička, očekávaný titulek / None = není skript Ledge)
    #[test]
    fn comment_styles_table() {
        let cases: &[(Lang, &str, Option<&str>)] = &[
            (Lang::Powershell, "# @ledge.title Vyčistit temp\nRemove-Item $env:TEMP\\* -Recurse", Some("Vyčistit temp")),
            (Lang::Powershell, "<#\n@ledge.title Blok\n#>\nWrite-Output 1", Some("Blok")),
            (Lang::Powershell, "<# @ledge.title Jeden řádek #>", Some("Jeden řádek")),
            (Lang::Python, "#!/usr/bin/env python3\n# @ledge.title Py\nprint(1)", Some("Py")),
            (Lang::Python, "\"\"\"\n@ledge.title Docstring\n\"\"\"", Some("Docstring")),
            (Lang::Cmd, "@echo off\n:: @ledge.title Dvojtečky\n", Some("Dvojtečky")),
            (Lang::Cmd, "REM @ledge.title Rem\n", Some("Rem")),
            (Lang::Cmd, "@rem @ledge.title Zavináč rem\n", Some("Zavináč rem")),
            // cmd bez komentáře = příkaz, ne metadata
            (Lang::Cmd, "@ledge.title Ne\n", None),
            (Lang::Cmd, "remove @ledge.title Ne\n", None),
            (Lang::Node, "// @ledge.title Node\nconsole.log(1)", Some("Node")),
            (Lang::Node, "/**\n * @ledge.title Hvězdička\n */", Some("Hvězdička")),
            (Lang::Node, "/* @ledge.title Blok */", Some("Blok")),
            (Lang::Python, "import os\nprint('bez hlavičky')", None),
            (Lang::Python, "", None),
        ];
        for (lang, text, want) in cases {
            let got = parse(text, *lang).unwrap_or_else(|e| panic!("{text:?}: {e}"));
            assert_eq!(got.map(|m| m.title).as_deref(), *want, "{lang:?} {text:?}");
        }
    }

    #[test]
    fn all_fields() {
        let m = ok(
            "# @ledge.title Záloha\n# @ledge.description Zkopíruje dokumenty na NAS\n# @ledge.icon 💾\n# @ledge.mode terminal\n# @ledge.confirm\n# @ledge.args --cil \"D:\\Záloha dat\" -v\n# @ledge.group Údržba\n# @ledge.timeout 300\n# @ledge.budoucí cokoli\n# @ledge.title Druhý se ignoruje",
            Lang::Powershell,
        );
        assert_eq!(m.title, "Záloha");
        assert_eq!(m.description.as_deref(), Some("Zkopíruje dokumenty na NAS"));
        assert_eq!(m.icon.as_deref(), Some("💾"));
        assert_eq!(m.mode, Mode::Terminal);
        assert!(m.confirm);
        assert_eq!(m.args, ["--cil", r"D:\Záloha dat", "-v"]);
        assert_eq!(m.group.as_deref(), Some("Údržba"));
        assert_eq!(m.timeout_s, 300);
        let d = ok("# @ledge.title X", Lang::Python);
        assert_eq!((d.mode, d.confirm, d.args.len(), d.timeout_s, d.icon), (Mode::Output, false, 0, DEFAULT_TIMEOUT_S, None));
    }

    /// (hlavička, část chyby)
    #[test]
    fn errors_table() {
        let cases: &[(&str, &str)] = &[
            ("# @ledge.icon 💾", "@ledge.title"),
            ("# @ledge.title   ", "@ledge.title"),
            ("# @ledge.title X\n# @ledge.mode loud", "loud"),
            ("# @ledge.title X\n# @ledge.confirm možná", "možná"),
            ("# @ledge.title X\n# @ledge.icon dlouhá ikona", "@ledge.icon"),
            ("# @ledge.title X\n# @ledge.args \"neuzavřené", "uvozovky"),
            ("# @ledge.title X\n# @ledge.timeout 0", "timeout"),
            ("# @ledge.title X\n# @ledge.timeout 9999", "timeout"),
        ];
        for (text, want) in cases {
            let e = parse(text, Lang::Python).unwrap_err();
            assert!(e.contains(want), "{text:?} → {e}");
        }
        let long = format!("# @ledge.title {}", "x".repeat(MAX_TITLE + 1));
        assert!(parse(&long, Lang::Python).is_err());
    }

    #[test]
    fn flags_and_cuts() {
        assert!(!ok("# @ledge.title X\n# @ledge.confirm false", Lang::Python).confirm);
        assert!(ok("# @ledge.title X\n# @ledge.confirm ano", Lang::Python).confirm);
        assert_eq!(ok("# @ledge.title X\n# @ledge.mode Hidden", Lang::Python).mode, Mode::Hidden);
        let d = ok(&format!("# @ledge.title X\n# @ledge.description {}", "d".repeat(500)), Lang::Python).description.unwrap();
        assert_eq!(d.chars().count(), MAX_DESC);
        assert!(d.ends_with('…'));
    }

    #[test]
    fn split_args_table() {
        let cases: &[(&str, &[&str])] = &[("", &[]), ("  a  b ", &["a", "b"]), ("\"a b\" c", &["a b", "c"]), ("x\"y z\"", &["xy z"]), ("\"\"", &[""])];
        for (s, want) in cases {
            assert_eq!(split_args(s).unwrap(), *want, "{s:?}");
        }
        assert!(split_args(&"a ".repeat(MAX_ARGS + 1)).is_err());
    }

    #[test]
    fn header_text_drops_bom_and_partial_line() {
        let mut b = vec![0xEF, 0xBB, 0xBF];
        b.extend_from_slice("# @ledge.title A\n# @ledge.desc".as_bytes());
        assert_eq!(header_text(&b, false), "# @ledge.title A\n# @ledge.desc");
        assert_eq!(header_text(&b, true), "# @ledge.title A");
        assert_eq!(header_text(b"bez konce", true), "");
        assert_eq!(Lang::from_ext("PS1"), Some(Lang::Powershell));
        assert_eq!(Lang::from_ext("bat"), Some(Lang::Cmd));
        assert_eq!(Lang::from_ext("mjs"), Some(Lang::Node));
        assert_eq!(Lang::from_ext("sh"), None);
    }
}
