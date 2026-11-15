//! Chybové řádky dev serveru: rozpoznání chyby (`looks_like_error`), maskování tajných údajů (`mask_secrets`)
//! a kruh posledních řádků pro zadání „dev chyba → poznámka“ (`ErrLines`). Čisté funkce, bez vláken a I/O.

use regex::Regex;
use std::sync::LazyLock;

/// Kolik posledních chybových řádků se drží (a jde do snapshotu jako `ProcView.error_lines`).
pub const ERR_KEEP: usize = 8;
/// Strop jednoho drženého řádku (znaky): snapshot zůstává malý i u minifikovaného výpisu.
pub const ERR_LINE_CAP: usize = 1024;
/// Kolik navazujících řádků stacku (`at …`) se přibalí k chybě bez zvýšení počtu chyb.
const STACK_CTX: u8 = 3;

/// Slovo `error` / `*Error` / `ERR!` / `fatal` jako samostatný token (ne část cesty `error-boundary.tsx`, `src/error.ts`),
/// za ním `:`, `!`, `]`, `)` nebo konec; esbuild `✘`; TS `error TS2304`.
static ERR_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)(?:^|[\s(\[])(?:(?:[a-z]*error|fatal)(?:[:!\])]|$|\s+ts\d)|err!)|✘|\bfailed to\b|\beaddrinuse\b").unwrap()
});
/// „0 errors“, „no errors“, „errors: 0“, „without errors“ = pozitivní zpráva, ne chyba.
static CLEAN_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)\b(?:0|no|zero)\s+(?:errors?|failed)\b|\berrors?:\s*0\b|\bwithout\s+errors?\b").unwrap());
/// Řádek stacku: `    at fn (file:1:2)` / `at file:1:2`.
static STACK_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^\s+at\s+\S").unwrap());

pub fn looks_like_error(line: &str) -> bool {
    !line.starts_with('›') && !CLEAN_RE.is_match(line) && ERR_RE.is_match(line)
}

fn is_stack(line: &str) -> bool {
    STACK_RE.is_match(line)
}

static MASKS: LazyLock<Vec<(Regex, &'static str)>> = LazyLock::new(|| {
    let r = |p: &str, to: &'static str| (Regex::new(p).unwrap(), to);
    vec![
        // heslo v URL: scheme://user:heslo@host
        r(r"(://[^/\s:@]+:)[^@\s/]+@", "${1}***@"),
        // parametry dotazu `?token=…`, `&api_key=…`
        r(r#"(?i)([?&;][a-z0-9_.-]*(?:token|key|secret|passw(?:or)?d|pwd|sig|signature|credential)[a-z0-9_.-]*=)[^&;\s"']+"#, "${1}***"),
        // `Authorization: Bearer …`, `Bearer …`, `Basic …`
        r(r"(?i)\b(bearer|basic)\s+[A-Za-z0-9._~+/=-]{8,}", "${1} ***"),
        // `NAME=hodnota` s názvem nesoucím TOKEN / SECRET / PASSWORD / KEY / CREDENTIAL
        r(r#"(?i)\b([a-z0-9_.-]*(?:token|secret|passw(?:or)?d|pwd|api_?key|access_?key|private_?key|credential|authorization)[a-z0-9_.-]*)\s*=\s*("[^"]*"|'[^']*'|[^\s&;]+)"#, "${1}=***"),
        // JSON `"password": "…"`
        r(r#"(?i)("[a-z0-9_.-]*(?:token|secret|passw(?:or)?d|pwd|api_?key|credential|authorization)[a-z0-9_.-]*"\s*:\s*)"[^"]*""#, r#"${1}"***""#),
        // známé tvary tokenů bez názvu: GitHub, OpenAI/Anthropic, AWS, Slack, JWT
        r(r"\b(?:ghp|gho|ghu|ghs|ghr)_[A-Za-z0-9]{20,}|\bgithub_pat_[A-Za-z0-9_]{20,}|\bsk-[A-Za-z0-9_-]{20,}|\bAKIA[0-9A-Z]{16}\b|\bxox[baprs]-[A-Za-z0-9-]{10,}|\beyJ[A-Za-z0-9_-]{8,}\.[A-Za-z0-9_-]{8,}\.[A-Za-z0-9_-]{8,}", "***"),
    ]
});

/// Nahradí tajné údaje v textu `***` dřív, než půjde do snapshotu / poznámky (tokeny, klíče, hesla v URL a `KEY=…`).
pub fn mask_secrets(s: &str) -> String {
    let mut out = s.to_string();
    for (re, to) in MASKS.iter() {
        out = re.replace_all(&out, *to).into_owned();
    }
    out
}

fn cap_chars(s: &str, cap: usize) -> String {
    if s.chars().count() <= cap { s.to_string() } else { s.chars().take(cap).collect::<String>() + "…" }
}

/// Posledních `ERR_KEEP` chybových řádků (maskovaných, ořezaných) + krátký stack za chybou.
#[derive(Debug, Default)]
pub struct ErrLines {
    pub lines: Vec<String>,
    ctx: u8,
}

impl ErrLines {
    /// Zpracuje další řádek výstupu; `true` = je to chyba (zvyšuje počet `errors`). Řádky stacku hned za chybou
    /// se přibalí k řádkům, ale do počtu nejdou (jedna chyba = jedna chyba, ne 12 rámců).
    pub fn feed(&mut self, line: &str) -> bool {
        let err = looks_like_error(line);
        if err {
            self.ctx = STACK_CTX;
        } else if self.ctx > 0 && is_stack(line) {
            self.ctx -= 1;
        } else {
            self.ctx = 0;
            return false;
        }
        self.lines.push(cap_chars(&mask_secrets(line), ERR_LINE_CAP));
        if self.lines.len() > ERR_KEEP {
            self.lines.remove(0);
        }
        err
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn looks_like_error_table() {
        for (line, want) in [
            ("Error: boom", true),
            ("TypeError: x is not a function", true),
            ("npm ERR! code ELIFECYCLE", true),
            ("✘ [ERROR] Could not resolve \"./x\"", true),
            ("[vite] Internal server error: Failed to parse source", true),
            ("src/App.tsx(3,5): error TS2304: Cannot find name 'x'.", true),
            ("Module not found: Error: Can't resolve './a'", true),
            ("Error: listen EADDRINUSE: address already in use :::3000", true),
            ("Failed to compile.", true),
            ("failed to load config from vite.config.ts", true),
            ("Found 0 errors. Watching for file changes.", false),
            ("compiled successfully, no errors", false),
            ("errors: 0", false),
            ("  ✓ src/error-boundary.tsx", false),
            ("GET /src/error-boundary.tsx 200", false),
            ("hmr update /src/lib/error.ts", false),
            ("no error handling needed here", false),
            ("› npm run dev", false),
            ("    at Object.<anonymous> (/app/x.js:1:2)", false),
            ("VITE v5 ready in 300 ms", false),
        ] {
            assert_eq!(looks_like_error(line), want, "{line}");
        }
    }

    #[test]
    fn mask_secrets_table() {
        for (inp, want) in [
            ("GET /api?token=abc123&x=1", "GET /api?token=***&x=1"),
            ("fetch https://u:hunter2@host/x failed", "fetch https://u:***@host/x failed"),
            ("Authorization: Bearer abcdef123456.xyz", "Authorization: Bearer ***"),
            ("API_KEY=sk_live_123 npm run dev", "API_KEY=*** npm run dev"),
            ("DB_PASSWORD='p a s s' node x", "DB_PASSWORD=*** node x"),
            (r#"{"password": "abc", "user": "bob"}"#, r#"{"password": "***", "user": "bob"}"#),
            ("push ghp_abcdefghijklmnopqrstuvwxyz0123 ok", "push *** ok"),
            ("key sk-ant-abcdefghijklmnopqrstuvwx", "key ***"),
            ("jwt eyJhbGciOiJI.eyJzdWIiOiIx.SflKxwRJSMeK done", "jwt *** done"),
            ("&secret=zz;next", "&secret=***;next"),
            ("http://localhost:5173/src/main.tsx?t=17", "http://localhost:5173/src/main.tsx?t=17"),
            ("Error: boom at author=bob", "Error: boom at author=bob"),
        ] {
            assert_eq!(mask_secrets(inp), want, "{inp}");
        }
    }

    #[test]
    fn feed_counts_errors_keeps_stack_and_caps() {
        let mut e = ErrLines::default();
        assert!(!e.feed("ready in 200 ms"));
        assert!(e.lines.is_empty());
        assert!(e.feed("Error: bad token=abc"));
        for _ in 0..5 {
            assert!(!e.feed("    at f (/a.js:1:1)"));
        }
        assert!(!e.feed("plain"));
        assert!(!e.feed("    at orphan (/a.js:1:1)"));
        // chyba + 3 rámce stacku (další se zahodí), tajemství maskované
        assert_eq!(e.lines, ["Error: bad token=***", "    at f (/a.js:1:1)", "    at f (/a.js:1:1)", "    at f (/a.js:1:1)"]);
        for i in 0..20 {
            e.feed(&format!("Error: n{i}"));
        }
        assert_eq!(e.lines.len(), ERR_KEEP);
        assert_eq!(e.lines.last().unwrap(), "Error: n19");
        let long = format!("Error: {}", "x".repeat(5000));
        e.feed(&long);
        assert!(e.lines.last().unwrap().chars().count() <= ERR_LINE_CAP + 1);
    }
}
