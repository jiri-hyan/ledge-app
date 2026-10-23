//! „Nepřátelské“ testy parserů cizích vstupů (jen testy): výstup gitu, soubory skriptů / widgetů, text
//! z poznámek a ze schránky, cesty. Generátor skládá kousky s vícebajtovými znaky, uvozovkami, `\0`, CRLF
//! a řídicími znaky; požadavek je jediný — žádná panika a konečný čas. Správnost výsledků hlídají tabulkové
//! testy v samotných modulech. Generátor: `testrng.rs`.

use crate::testrng::Rng;
use std::panic::catch_unwind;

const ROUNDS: usize = 2500;

/// Spustí `f` na `ROUNDS` náhodných vstupech z `bits`; při panice vypíše vstup.
fn fuzz(seed: u64, bits: &[&str], max: usize, f: impl Fn(&str) + std::panic::RefUnwindSafe) {
    let mut rng = Rng(seed);
    for i in 0..ROUNDS {
        let s = rng.soup(bits, max);
        assert!(catch_unwind(|| f(&s)).is_ok(), "panika v kole {i}, vstup {s:?}");
    }
}

const COMMON: &[&str] = &["é", "ů", "ß", "İ", "🙂", "\u{0}", "\u{200d}", "\r\n", "\n", "\t", " ", "\"", "'", "\\", "\u{feff}"];

fn with(extra: &[&'static str]) -> Vec<&'static str> {
    let mut v: Vec<&'static str> = COMMON.to_vec();
    v.extend_from_slice(extra);
    v
}

#[test]
fn git_output_parsers() {
    use crate::sessiongit::parse;
    let bits = with(&["1 .M N... 100644 100644 100644 a b ", "? ", "R  ", " M ", "?? ", "!! ", "UU ", "## ", " -> ", "\u{0}", "12\t", "-\t-\t", "a/b.txt", "worktree ", "HEAD ", "branch refs/heads/x", "detached", "prunable", "bare", "@@ -1,2 +3,4 @@", "@@ -0,0 +1 @@", "+", "-", "Binary files", "\\ No newline"]);
    fuzz(1, &bits, 16, |s| {
        let _ = parse::status_z(s);
        let _ = parse::numstat_z(s);
        let _ = parse::worktrees(s);
        let _ = parse::unified("a.txt", s, 50);
        let _ = parse::unified("a.txt", s, 0);
        let _ = crate::claude::paths::porcelain_paths(s);
    });
}

#[test]
fn path_helpers() {
    use crate::claude::paths;
    let bits = with(&["C:", "c:\\", "\\\\", "//", "..", ".", "\\..\\", "/", "~", "%", "?", "*", "con", "a b", "\u{202e}", "x.md"]);
    fuzz(2, &bits, 12, |s| {
        let _ = paths::lexical(s);
        let _ = paths::path_key(s);
        let _ = paths::absolutize(s, "C:\\work");
        let _ = paths::absolutize("a", s);
        let _ = paths::repo_relative("C:\\r", s);
        let _ = paths::repo_relative(s, "C:\\r\\a");
        let _ = crate::notes::openwith::expand_env(s, |k| (k.len() % 2 == 0).then(|| "é".to_string()));
        let _ = crate::personal::normalize(s);
    });
}

#[test]
fn script_and_widget_headers() {
    use crate::scripts::meta::{Lang, header_text, parse, split_args};
    let bits = with(&["# ", "// ", "REM ", "rem ", "@rem ", ":: ", "<#", "#>", "---", "name: ", "title: ", "icon: ", "args: ", "interval: ", "10s", "5m", "0", "-1", "99999999999999", "true", "off", "\"a b\"", "'x", "ledge:", "@ledge", "param(", "#!/usr/bin/env node", "/*", "*/", "\"\"\""]);
    fuzz(3, &bits, 14, |s| {
        for lang in [Lang::Powershell, Lang::Cmd, Lang::Python, Lang::Node] {
            let _ = parse(s, lang);
            let _ = crate::widgets::meta::parse(s, lang);
        }
        let _ = split_args(s);
        let _ = header_text(s.as_bytes(), false);
        let _ = header_text(s.as_bytes(), true);
        let _ = crate::widgets::meta::parse_interval(s);
    });
    // ne-UTF-8 bajty a useknutá vícebajtová sekvence v hlavičce
    for raw in [&b"\xff\xfe# name: a"[..], b"# name: \xc3", b"\xef\xbb\xbf# name: x\n", b"", b"\0\0\0"] {
        let _ = header_text(raw, false);
        let _ = header_text(raw, true);
    }
}

#[test]
fn note_text_and_clipboard() {
    use chrono::{FixedOffset, TimeZone};
    let now = FixedOffset::east_opt(2 * 3600).unwrap().with_ymd_and_hms(2026, 10, 23, 14, 5, 0).unwrap();
    let bits = with(&["zítra", "dnes", "pozítří", "v ", "ve ", "pátek", "v pondělí", "za hodinu", "za 2 dny", "15:30", "9.", "10. 11.", "1.1.2027", "24:00", "99:99", "ráno", "odpoledne", "večer", "rano", "ve 3", "0", "29. 2.", "31. 4.", "T", "Z", "příští týden", "za 99999999999 dní", "za -3 dny"]);
    fuzz(4, &bits, 10, |s| {
        let _ = crate::notes::when::suggest(s, &now);
        let _ = crate::notes::when::parse_when(s, &now);
    });
    let bits = with(&["password", "token", "sk-", "ghp_", "eyJ", "AKIA", "-----BEGIN", "PRIVATE KEY", "Bearer ", "=", ":", "A1b2C3d4E5f6G7h8", "http://x", "x".repeat(300).leak()]);
    fuzz(5, &bits, 8, |s| {
        let _ = crate::cliphist::filter::text_verdict(s, true);
        let _ = crate::cliphist::filter::text_verdict(s, false);
        let _ = crate::cliphist::filter::looks_secret(s);
        let units: Vec<u16> = s.encode_utf16().chain([0xD800, 0xDC00, 0xDFFF, 0]).collect();
        let _ = crate::cliphist::filter::utf16_text(&units);
        let _ = crate::cliphist::filter::utf16_text(&units[..units.len() / 2]);
        let _ = crate::cliphist::filter::exe_key(s);
    });
}

#[test]
fn meeting_links_and_calendar_urls() {
    use crate::calendar::{conf, meeting};
    let bits = with(&["https://", "http://", "teams.microsoft.com/l/meetup-join/", "zoom.us/j/", "meet.google.com/", "%", "%C3", "%zz", "%00", "@", ":", "/", "?", "#", "&", "=", "msteams:", "webcal://", "webcals://", ".", "example.com", "<", ">", "9999999999999"]);
    fuzz(6, &bits, 12, |s| {
        let _ = meeting::find(s);
        let _ = meeting::links(s);
        let _ = meeting::percent_decode(s);
        let _ = meeting::classify(s);
        let _ = conf::normalize_url(s);
        let _ = conf::domain(s);
        let _ = conf::key(s);
        let mut cur = None;
        meeting::offer(&mut cur, 1, s);
        if let Some((_, j)) = cur {
            let _ = meeting::app_url(&j);
            let _ = meeting::launch_uri(&j);
        }
    });
}
