//! Chyby frontendu do `ledge.log` (příkaz `ui_log`): `window.onerror` / `unhandledrejection` / pád vykreslení
//! v `ErrorBoundary`. Bez toho zůstala chyba jen v konzoli WebView2, kterou uživatel nemá.
//! Ochrany: délka řádku je omezená, řídicí znaky pryč, cesty se zahladí (`<cesta>`), frekvence má strop
//! (`MAX_PER_WINDOW` za `WINDOW_MS`) — zacyklená chyba log nezaplaví. Nic z poznámek se sem neposílá.

use std::sync::Mutex;

/// nejvíc znaků zprávy v logu
pub const MAX_LEN: usize = 240;
/// strop zápisů za okno
pub const MAX_PER_WINDOW: u32 = 20;
/// délka okna ve ms
pub const WINDOW_MS: u64 = 60_000;

/// Zpráva z UI → jeden řádek bez řídicích znaků, cesty zahlazené, max `MAX_LEN` znaků.
pub fn clean(msg: &str) -> String {
    let mut out = String::new();
    for tok in msg.split_whitespace() {
        if !out.is_empty() {
            out.push(' ');
        }
        out.push_str(if looks_like_path(tok) { "<cesta>" } else { tok });
    }
    if out.chars().count() > MAX_LEN {
        out = out.chars().take(MAX_LEN).collect::<String>() + "…";
    }
    out
}

/// `C:\…`, `C:/…`, `\\server\…` (i v uvozovkách / závorkách okolo)
fn looks_like_path(tok: &str) -> bool {
    let t = tok.trim_start_matches(|c: char| !c.is_alphanumeric() && c != '\\');
    let b: Vec<char> = t.chars().take(3).collect();
    (b.len() == 3 && b[0].is_ascii_alphabetic() && b[1] == ':' && (b[2] == '\\' || b[2] == '/'))
        || t.starts_with("\\\\")
}

/// Druh události z UI → štítek; neznámý = `error`.
pub fn kind_label(kind: &str) -> &'static str {
    match kind {
        "promise" => "promise",
        "render" => "render",
        _ => "error",
    }
}

/// Okno z labelu (`note-<id>` / `desk-<id>` → jen předpona, id se do logu nedostane).
pub fn window_label(label: &str) -> &str {
    label.split('-').next().unwrap_or(label)
}

/// Řádek logu, nebo `None` při prázdné zprávě.
pub fn line(label: &str, kind: &str, msg: &str) -> Option<String> {
    let m = clean(msg);
    (!m.is_empty()).then(|| format!("ui[{}] {}: {m}", window_label(label), kind_label(kind)))
}

/// Okno frekvence: počet zápisů od `start`.
#[derive(Default)]
pub struct Limiter {
    start: u64,
    n: u32,
}

impl Limiter {
    /// Smí se teď zapsat? (`now` = ms z libovolných hodin, rostoucí.)
    pub fn admit(&mut self, now: u64) -> bool {
        if self.n == 0 || now.saturating_sub(self.start) >= WINDOW_MS {
            self.start = now;
            self.n = 0;
        }
        if self.n >= MAX_PER_WINDOW {
            return false;
        }
        self.n += 1;
        true
    }
}

static LIMIT: Mutex<Limiter> = Mutex::new(Limiter { start: 0, n: 0 });

/// Zapíše chybu z UI do logu (s limitem); vrací, zda se zapsalo.
pub fn record(label: &str, kind: &str, msg: &str, now_ms: u64) -> bool {
    let Some(l) = line(label, kind, msg) else {
        return false;
    };
    if !LIMIT.lock().is_ok_and(|mut g| g.admit(now_ms)) {
        return false;
    }
    crate::state::log(&l);
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clean_table() {
        let long = "x".repeat(500);
        let cases: Vec<(String, String)> = vec![
            (
                "TypeError: x is undefined".into(),
                "TypeError: x is undefined".into(),
            ),
            ("a\nb\t c\r\n".into(), "a b c".into()),
            (
                "nelze číst C:\\Users\\uzivatel\\app\\a.md hotovo".into(),
                "nelze číst <cesta> hotovo".into(),
            ),
            (
                "file \"D:/app/x.ts\" fail".into(),
                "file <cesta> fail".into(),
            ),
            ("\\\\server\\share\\a".into(), "<cesta>".into()),
            (
                "https://example.com/a.js:1:2".into(),
                "https://example.com/a.js:1:2".into(),
            ),
            ("   ".into(), "".into()),
            (long, format!("{}…", "x".repeat(MAX_LEN))),
        ];
        for (inp, want) in cases {
            assert_eq!(clean(&inp), want, "{inp:?}");
        }
    }

    #[test]
    fn line_table() {
        assert_eq!(
            line("drawer", "error", "boom").as_deref(),
            Some("ui[drawer] error: boom")
        );
        assert_eq!(
            line("note-123abc", "promise", "x").as_deref(),
            Some("ui[note] promise: x")
        );
        assert_eq!(
            line("desk-w1", "render", "x").as_deref(),
            Some("ui[desk] render: x")
        );
        assert_eq!(
            line("studio", "cokoli", "x").as_deref(),
            Some("ui[studio] error: x")
        );
        assert_eq!(line("studio", "error", " \n "), None);
    }

    #[test]
    fn limiter_caps_and_reopens() {
        let mut l = Limiter::default();
        for i in 0..MAX_PER_WINDOW {
            assert!(l.admit(1000 + u64::from(i)), "{i}");
        }
        assert!(!l.admit(2000), "nad strop");
        assert!(!l.admit(1000 + WINDOW_MS - 1));
        assert!(l.admit(1000 + WINDOW_MS), "nové okno");
    }
}
