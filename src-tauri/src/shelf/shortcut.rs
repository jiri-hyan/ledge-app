//! Přetažení na OTEVŘENÝ šuplík = nový zástupce (kolo 20, `smart.dropShortcut`). Tahle část je čistá:
//! co se z puštěných dat stane (`classify`) a kdy se které cílové zóně dá přednost (`target_for`).
//!
//! Dvě zóny se nikdy nepřekrývají, protože je určuje stav šuplíku, ne poloha: ZAVŘENÝ šuplík (proužek, peek) =
//! přihrádka na soubory (`shelf::zone`, soubory se jen uloží jako odkazy), OTEVŘENÝ naplno (`View::Full`) =
//! nový zástupce (`drawer.rect`). Přihrádka při otevřeném šuplíku nikdy nepřijímá (`hover.rs` jí zónu
//! nedá) a „nový zástupce“ nikdy nepřijímá u zavřeného. Cíl přetažení (`shelf/drop.rs`) je pro obě stejné
//! neviditelné okno, jen s jiným `Kind`; ten se drží do konce tahu.
//!
//! Z puštěného se nic nespouští: výsledek je jen návrh (`Payload`), který UI otevře v editoru akce a člověk
//! ho musí potvrdit („Přidat“); spuštění pak jde dál přes akce (`actions::validate_one`, `needs_confirm`,
//! souhlasy) — příkaz (`command`) z puštěných dat nevznikne nikdy.

use serde::Serialize;

/// Která zóna přetažení je teď aktivní.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// proužek u hrany → přihrádka na soubory
    Shelf,
    /// otevřený šuplík → nový zástupce
    Shortcut,
}

/// Který cíl smí přijmout tah podle stavu šuplíku a voleb. `full` = šuplík otevřený naplno.
pub fn target_for(full: bool, shelf_on: bool, shortcut_on: bool) -> Option<Kind> {
    match (full, shelf_on, shortcut_on) {
        (true, _, true) => Some(Kind::Shortcut),
        (false, true, _) => Some(Kind::Shelf),
        _ => None,
    }
}

/// Co se puštěním navrhne (typ odpovídá `ActionKind` v UI; `command` tu není a nikdy nebude).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Payload {
    /// url | folder | file | app
    pub kind: &'static str,
    /// adresa nebo celá cesta
    pub target: String,
    /// kolik dalších puštěných položek se přeskočilo (navrhuje se jen první)
    pub skipped: usize,
}

const MAX_TARGET: usize = 2048;

/// Je to web, který akce typu Web přijme? (`http(s)://` + host, bez mezer a řídicích znaků)
pub fn is_web_url(s: &str) -> bool {
    let low = s.to_ascii_lowercase();
    let rest = low.strip_prefix("https://").or_else(|| low.strip_prefix("http://"));
    rest.is_some_and(|r| !r.is_empty() && !r.starts_with('/') && !s.chars().any(|c| c.is_whitespace() || c.is_control()))
}

fn is_abs_path(s: &str) -> bool {
    let b = s.as_bytes();
    b.len() >= 3 && b[0].is_ascii_alphabetic() && b[1] == b':' && (b[2] == b'\\' || b[2] == b'/') && !s[2..].contains(':')
}

fn path_kind(path: &str, is_dir: &dyn Fn(&str) -> bool) -> &'static str {
    if path.to_ascii_lowercase().ends_with(".exe") {
        "app"
    } else if is_dir(path) {
        "folder"
    } else {
        "file"
    }
}

/// Přípona (malými písmeny) za poslední tečkou posledního dílu cesty; bez ní prázdný řetězec.
fn ext_of(path: &str) -> String {
    let name = path.rsplit([char::from(92u8), '/']).next().unwrap_or(path);
    name.rsplit_once('.').map(|(_, e)| e.to_ascii_lowercase()).unwrap_or_default()
}

/// Výsledek posouzení puštěných dat.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Verdict {
    /// návrh zástupce (UI otevře editor, nic se neuloží ani nespustí bez potvrzení)
    Propose(Payload),
    /// soubor, který by akce typu Soubor stejně odmítla (allowlist `personal::FILE_ALLOW`) — věta pro člověka
    Refuse(String),
    /// nic použitelného (obyčejný text, relativní cesta, UNC, příliš dlouhé)
    Nothing,
}

/// Soubor (ne složka, ne `.exe`) s příponou mimo allowlist akcí → důvod odmítnutí. `.lnk`, `.url`, `.bat`, `.cmd`
/// a další by editor jen navrhl a hned při ukládání odmítl, takže je nenavrhujeme vůbec.
fn refusal(p: &Payload) -> Option<String> {
    if p.kind != "file" {
        return None;
    }
    let ext = ext_of(&p.target);
    if crate::personal::FILE_ALLOW.contains(&ext.as_str()) {
        return None;
    }
    Some(if ext.is_empty() {
        crate::i18n::tr("Soubor bez přípony Ledge jako zástupce neotevře.", "Ledge can't open a file without an extension as a shortcut.").to_string()
    } else {
        trf!(
            "Soubory typu .{ext} Ledge jako zástupce neotevře (jen dokumenty, obrázky, média, zip a .exe).",
            "Ledge can't open .{ext} files as a shortcut (only documents, images, media, zip and .exe)."
        )
    })
}

/// Puštěná data → návrh nebo odmítnutí. Soubory (`CF_HDROP`) mají přednost před textem; z textu se vezme první
/// řádek, který je web nebo celá cesta. Cokoli jiného (obyčejný text, relativní cesta, UNC, příliš dlouhé) =
/// `Nothing`. `is_dir` dodá volající (disk), ať je funkce čistá.
pub fn judge(files: &[String], text: Option<&str>, is_dir: &dyn Fn(&str) -> bool) -> Verdict {
    match propose(files, text, is_dir) {
        Some(p) => match refusal(&p) {
            Some(why) => Verdict::Refuse(why),
            None => Verdict::Propose(p),
        },
        None => Verdict::Nothing,
    }
}

/// Jen použitelný návrh (odmítnuté soubory a nic = `None`) — zkratka pro testy.
#[cfg(test)]
fn classify(files: &[String], text: Option<&str>, is_dir: &dyn Fn(&str) -> bool) -> Option<Payload> {
    match judge(files, text, is_dir) {
        Verdict::Propose(p) => Some(p),
        _ => None,
    }
}

fn propose(files: &[String], text: Option<&str>, is_dir: &dyn Fn(&str) -> bool) -> Option<Payload> {
    if let Some(first) = files.iter().map(|f| f.trim()).find(|f| !f.is_empty()) {
        if !is_abs_path(first) || first.len() > MAX_TARGET {
            return None;
        }
        let n = files.iter().filter(|f| !f.trim().is_empty()).count();
        return Some(Payload { kind: path_kind(first, is_dir), target: first.replace('/', "\\"), skipped: n - 1 });
    }
    let line = text?.lines().map(str::trim).find(|l| !l.is_empty())?;
    if line.len() > MAX_TARGET {
        return None;
    }
    if is_web_url(line) {
        return Some(Payload { kind: "url", target: line.to_string(), skipped: 0 });
    }
    let unquoted = line.trim_matches('"');
    if is_abs_path(unquoted) {
        return Some(Payload { kind: path_kind(unquoted, is_dir), target: unquoted.replace('/', "\\"), skipped: 0 });
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dirs(p: &str) -> bool {
        !p.contains('.')
    }
    fn s(v: &[&str]) -> Vec<String> {
        v.iter().map(|x| x.to_string()).collect()
    }

    #[test]
    fn zones_never_overlap() {
        // (šuplík naplno, přihrádka zapnutá, nový zástupce zapnutý) → cíl
        let cases = [
            (false, true, true, Some(Kind::Shelf)),
            (false, true, false, Some(Kind::Shelf)),
            (false, false, true, None),
            (false, false, false, None),
            (true, true, true, Some(Kind::Shortcut)),
            (true, false, true, Some(Kind::Shortcut)),
            (true, true, false, None),
            (true, false, false, None),
        ];
        for (full, shelf, shortcut, want) in cases {
            assert_eq!(target_for(full, shelf, shortcut), want, "full={full} shelf={shelf} shortcut={shortcut}");
        }
        // přihrádka nikdy při otevřeném šuplíku, nový zástupce nikdy při zavřeném
        for shelf in [false, true] {
            for shortcut in [false, true] {
                assert_ne!(target_for(true, shelf, shortcut), Some(Kind::Shelf));
                assert_ne!(target_for(false, shelf, shortcut), Some(Kind::Shortcut));
            }
        }
    }

    #[test]
    fn web_urls() {
        let cases = [
            ("https://example.com", true),
            ("HTTP://example.com/a?b=1#c", true),
            ("https://", false),
            ("https:///x", false),
            ("ftp://example.com", false),
            ("javascript:alert(1)", false),
            ("https://example.com/a b", false),
            ("example.com", false),
            ("", false),
        ];
        for (u, want) in cases {
            assert_eq!(is_web_url(u), want, "{u:?}");
        }
    }

    #[test]
    fn classify_table() {
        let none: Vec<String> = vec![];
        let long = "https://example.com/".repeat(200);
        type Case<'a> = (&'a str, Vec<String>, Option<&'a str>, Option<(&'a str, &'a str, usize)>);
        let cases: Vec<Case> = vec![
            ("složka", s(&[r"C:\Users\uzivatel\Projekty"]), None, Some(("folder", r"C:\Users\uzivatel\Projekty", 0))),
            ("soubor", s(&[r"C:\Users\uzivatel\a.pdf"]), None, Some(("file", r"C:\Users\uzivatel\a.pdf", 0))),
            ("exe", s(&[r"C:\Tools\app.EXE"]), None, Some(("app", r"C:\Tools\app.EXE", 0))),
            ("víc souborů → první, ostatní se přeskočí", s(&[r"C:\a\x.txt", r"C:\a\y.txt", r"C:\a\z.txt"]), None, Some(("file", r"C:\a\x.txt", 2))),
            ("soubory mají přednost před textem", s(&[r"C:\a\x.txt"]), Some("https://example.com"), Some(("file", r"C:\a\x.txt", 0))),
            ("relativní cesta ne", s(&["a\\x.txt"]), None, None),
            ("UNC ne", s(&[r"\\server\share\x.txt"]), None, None),
            ("odkaz z prohlížeče", none.clone(), Some("https://example.com/docs"), Some(("url", "https://example.com/docs", 0))),
            ("první řádek textu", none.clone(), Some("\n  https://example.com/a \nsecond"), Some(("url", "https://example.com/a", 0))),
            ("cesta v textu v uvozovkách", none.clone(), Some("\"C:\\Tools\\app.exe\""), Some(("app", r"C:\Tools\app.exe", 0))),
            ("lomítka v cestě", none.clone(), Some("C:/Tools/dir"), Some(("folder", r"C:\Tools\dir", 0))),
            ("obyčejný text ne", none.clone(), Some("jen nějaký text"), None),
            ("příkaz ne", none.clone(), Some("npm run build"), None),
            ("javascript: ne", none.clone(), Some("javascript:alert(1)"), None),
            ("nic", none.clone(), None, None),
            ("prázdný text", none.clone(), Some("  \n "), None),
            ("příliš dlouhé", none.clone(), Some(long.as_str()), None),
        ];
        for (name, files, text, want) in cases {
            let got = classify(&files, text, &dirs);
            assert_eq!(got.as_ref().map(|p| (p.kind, p.target.as_str(), p.skipped)), want, "{name}");
        }
    }

    #[test]
    fn refuses_what_the_allowlist_would_reject() {
        let cases = [(r"C:\a\odkaz.lnk", ".lnk"), (r"C:\a\web.URL", ".url"), (r"C:\a\x.bat", ".bat"), (r"C:\a\x.cmd", ".cmd"), (r"C:\a\x.ps1", ".ps1")];
        for (path, ext) in cases {
            for text in [false, true] {
                let (files, t) = if text { (vec![], Some(path)) } else { (s(&[path]), None) };
                assert!(classify(&files, t, &dirs).is_none(), "{path} text={text}");
                match judge(&files, t, &dirs) {
                    Verdict::Refuse(why) => assert!(why.contains(ext), "{why}"),
                    other => panic!("{path}: {other:?}"),
                }
            }
        }
        // bez přípony
        assert!(matches!(judge(&s(&[r"C:\a\README"]), None, &|_| false), Verdict::Refuse(_)));
        // povolené zůstává návrhem; .exe a složka se neodmítají
        for ok in [r"C:\a\x.pdf", r"C:\a\X.TXT", r"C:\Tools\app.exe"] {
            assert!(matches!(judge(&s(&[ok]), None, &dirs), Verdict::Propose(_)), "{ok}");
        }
        assert!(matches!(judge(&s(&[r"C:\Users\uzivatel\Projekty"]), None, &dirs), Verdict::Propose(_)));
        // text bez cesty = nic, ne odmítnutí
        assert_eq!(judge(&[], Some("jen text"), &dirs), Verdict::Nothing);
    }

    #[test]
    fn never_proposes_a_command() {
        for t in ["cmd /c calc", "powershell -c x", "C:\\x.bat", "C:\\x.ps1"] {
            let p = classify(&[], Some(t), &|_| false);
            assert!(p.is_none_or(|p| p.kind != "command"), "{t}");
        }
    }
}
