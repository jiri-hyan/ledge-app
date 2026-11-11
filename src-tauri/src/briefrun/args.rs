//! Čisté sestavení běhu zadání: argumenty `claude -p` (nové zadání / pokračování), prostředí dítěte
//! (klíč JEN v env, nikdy v argumentech), MCP config s Ledge, text role a zprávy na stdin.

use crate::i18n::tr;

/// Nástroje MCP Ledge, které session ze zadání smí volat bez karty schválení (jen poznámky — čtení
/// zadání, otázky, log, návrh hotového bodu, nová poznámka). Stav zadání mění jen Ledge, ne session.
pub const NOTE_TOOLS: [&str; 5] = ["notes_read", "notes_ask", "notes_log", "notes_propose_done", "notes_create"];

/// Proměnné, které by v `claude -p` přebily API klíč (`ANTHROPIC_AUTH_TOKEN` vyhrává nad
/// `ANTHROPIC_API_KEY`) nebo by běh pustily přes předplatné → z prostředí dítěte pryč.
pub const ENV_REMOVE: [&str; 2] = ["ANTHROPIC_AUTH_TOKEN", "CLAUDE_CODE_OAUTH_TOKEN"];
pub const ENV_KEY: &str = "ANTHROPIC_API_KEY";

/// Jeden běh zadání (vše už ověřené vrstvou příkazu).
#[derive(Debug, Clone, PartialEq)]
pub struct Brief {
    /// uuid session (nové pro `send`, uložené pro `resume`)
    pub session: String,
    pub resume: bool,
    pub model: String,
    /// zbývající rozpočet v USD (> 0)
    pub budget_left: f64,
    /// `ledge.exe` pro stdio MCP (`--mcp`, klíč `Ledge/mcp` si proxy čte sama)
    pub ledge_exe: String,
}

/// Model z poznámky / configu: alias nebo id modelu (`[a-z0-9.-]`, nezačíná `-`), jinak chyba.
pub fn clean_model(m: &str) -> Result<String, String> {
    let m = m.trim().to_ascii_lowercase();
    let ok = !m.is_empty() && m.len() <= 64 && !m.starts_with('-') && m.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'.' || b == b'_');
    if ok { Ok(m) } else { Err(tr("Neplatný model zadání (čekám sonnet, opus nebo haiku).", "Invalid brief model (expected sonnet, opus or haiku).").into()) }
}

/// Session id: uuid tvaru 8-4-4-4-12 hex (jen takové jde do `--session-id` / `--resume`).
pub fn valid_session(s: &str) -> bool {
    let parts: Vec<&str> = s.split('-').collect();
    parts.len() == 5 && parts.iter().zip([8, 4, 4, 4, 12]).all(|(p, n)| p.len() == n && p.bytes().all(|b| b.is_ascii_hexdigit()))
}

/// Nové uuid v4 z 16 náhodných bajtů.
pub fn uuid_v4(mut b: [u8; 16]) -> String {
    b[6] = (b[6] & 0x0f) | 0x40;
    b[8] = (b[8] & 0x3f) | 0x80;
    let h: String = b.iter().map(|x| format!("{x:02x}")).collect();
    format!("{}-{}-{}-{}-{}", &h[0..8], &h[8..12], &h[12..16], &h[16..20], &h[20..32])
}

/// Inline `--mcp-config`: server `ledge` přes stdio (`ledge.exe --mcp`) — stejný jako `commands::mcp::desktop_config`.
pub fn mcp_config(ledge_exe: &str) -> String {
    serde_json::json!({ "mcpServers": { "ledge": { "command": ledge_exe, "args": ["--mcp"] } } }).to_string()
}

/// Argumenty `claude` (bez klíče — ten jde jen do env, viz `build_env`).
pub fn build_args(b: &Brief) -> Vec<String> {
    let tools: Vec<String> = NOTE_TOOLS.iter().map(|t| format!("mcp__ledge__{t}")).collect();
    let mut v: Vec<String> = [
        "-p",
        "--input-format",
        "stream-json",
        "--output-format",
        "stream-json",
        "--verbose",
        "--model",
        &b.model,
        "--max-budget-usd",
        &budget_arg(b.budget_left),
        "--permission-mode",
        "acceptEdits",
        "--mcp-config",
        &mcp_config(&b.ledge_exe),
        "--allowedTools",
        &tools.join(","),
        "--append-system-prompt",
        ROLE,
    ]
    .iter()
    .map(|s| s.to_string())
    .collect();
    v.push(if b.resume { "--resume" } else { "--session-id" }.into());
    v.push(b.session.clone());
    v
}

/// Strop pro `--max-budget-usd` na 2 desetinná místa, dolů (nikdy víc, než zbývá).
pub fn budget_arg(left: f64) -> String {
    format!("{:.2}", (left.max(0.0) * 100.0).floor() / 100.0)
}

/// Prostředí dítěte: (nastavit, odebrat). Klíč jen tady.
pub fn build_env(key: &str) -> (Vec<(String, String)>, Vec<String>) {
    (vec![(ENV_KEY.into(), key.into())], ENV_REMOVE.iter().map(|s| s.to_string()).collect())
}

/// Text role (`--append-system-prompt`).
pub const ROLE: &str = "Jsi orchestrátor tohoto projektu — drž se jeho CLAUDE.md. Zadání je poznámka Ledge: čti ji \
přes mcp__ledge__notes_read(path). Je-li něco nejasné, zapiš otázky přes mcp__ledge__notes_ask a ukonči tah — \
nehádej. Průběh zapisuj krátce přes mcp__ledge__notes_log. Hotový bod jen navrhni přes \
mcp__ledge__notes_propose_done s důkazem (commit nebo test); nikdy ho neoznačuj [x]. Navazující práci mimo \
zadání zapiš jako novou poznámku do projektu přes mcp__ledge__notes_create a nedělej ji.";

/// Jeden řádek stream-json na stdin (formát uživatelské zprávy — z čistého CLI neověřeno).
pub fn user_line(text: &str) -> String {
    let v = serde_json::json!({ "type": "user", "message": { "role": "user", "content": text } });
    format!("{v}\n")
}

/// První zpráva nové session: kde je zadání (text zadání jde přes MCP, ne přes argumenty).
pub fn first_message(note_path: &str) -> String {
    format!("Zadání je v poznámce Ledge: {note_path}\nPřečti ho přes mcp__ledge__notes_read a pracuj podle něj.")
}

/// Zpráva při „Pokračovat“: šéf odpověděl / zkontroloval.
pub fn resume_message(note_path: &str) -> String {
    format!(
        "Šéf odpověděl v poznámce {note_path} (sekce Otázky) nebo zkontroloval body. Přečti ji znovu přes \
mcp__ledge__notes_read a pokračuj."
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    const KEY: &str = "sk-ant-api03-TESTKLIC-uzivatel";

    fn brief(resume: bool) -> Brief {
        Brief { session: "0f8fad5b-d9cb-469f-a165-70867728950e".into(), resume, model: "sonnet".into(), budget_left: 1.999, ledge_exe: r"C:\Program Files\Ledge\ledge.exe".into() }
    }

    #[test]
    fn args_send_vs_resume() {
        for (resume, flag) in [(false, "--session-id"), (true, "--resume")] {
            let a = build_args(&brief(resume));
            assert_eq!(a[0], "-p");
            let at = |f: &str| a.iter().position(|x| x == f).map(|i| a[i + 1].as_str());
            assert_eq!(at(flag), Some("0f8fad5b-d9cb-469f-a165-70867728950e"), "{flag}");
            assert_eq!(at(if resume { "--session-id" } else { "--resume" }), None);
            assert_eq!(at("--input-format"), Some("stream-json"));
            assert_eq!(at("--output-format"), Some("stream-json"));
            assert!(a.contains(&"--verbose".to_string()));
            assert_eq!(at("--model"), Some("sonnet"));
            assert_eq!(at("--max-budget-usd"), Some("1.99"), "dolů, nikdy víc");
            assert_eq!(at("--permission-mode"), Some("acceptEdits"));
            assert_eq!(at("--append-system-prompt"), Some(ROLE));
            let mcp: serde_json::Value = serde_json::from_str(at("--mcp-config").unwrap()).unwrap();
            assert_eq!(mcp["mcpServers"]["ledge"]["command"], r"C:\Program Files\Ledge\ledge.exe");
            assert_eq!(mcp["mcpServers"]["ledge"]["args"], serde_json::json!(["--mcp"]));
            let tools = at("--allowedTools").unwrap();
            assert!(tools.split(',').all(|t| t.starts_with("mcp__ledge__notes_")), "{tools}");
            assert!(!a.iter().any(|x| x.contains("sk-ant")), "klíč nikdy v argumentech");
        }
    }

    #[test]
    fn env_carries_key_and_drops_tokens() {
        let (set, remove) = build_env(KEY);
        assert_eq!(set, vec![("ANTHROPIC_API_KEY".to_string(), KEY.to_string())]);
        assert!(remove.contains(&"ANTHROPIC_AUTH_TOKEN".to_string()));
        assert!(remove.contains(&"CLAUDE_CODE_OAUTH_TOKEN".to_string()));
        assert!(!build_args(&brief(false)).join(" ").contains(KEY));
    }

    #[test]
    fn budget_model_session_table() {
        for (left, want) in [(2.0, "2.00"), (0.019, "0.01"), (-1.0, "0.00"), (1.5, "1.50")] {
            assert_eq!(budget_arg(left), want, "{left}");
        }
        for (m, ok) in [("sonnet", true), (" Opus ", true), ("claude-sonnet-4-5", true), ("", false), ("--model", false), ("a b", false), ("x&y", false)] {
            assert_eq!(clean_model(m).is_ok(), ok, "{m}");
        }
        assert!(valid_session(&uuid_v4([7; 16])));
        assert_eq!(&uuid_v4([0; 16])[14..15], "4");
        for (s, ok) in [("0f8fad5b-d9cb-469f-a165-70867728950e", true), ("0f8fad5b-d9cb-469f-a165-70867728950", false), ("--resume", false), ("0f8fad5b-d9cb-469f-a165-70867728950g", false)] {
            assert_eq!(valid_session(s), ok, "{s}");
        }
    }

    #[test]
    fn messages_are_json_lines() {
        let l = user_line(&first_message(r"C:\n\app\zadani.md"));
        assert!(l.ends_with('\n') && !l[..l.len() - 1].contains('\n'), "jeden řádek");
        let v: serde_json::Value = serde_json::from_str(l.trim()).unwrap();
        assert_eq!(v["type"], "user");
        assert_eq!(v["message"]["role"], "user");
        assert!(v["message"]["content"].as_str().unwrap().contains(r"C:\n\app\zadani.md"));
        assert!(resume_message("x.md").contains("x.md"));
        for t in ["notes_read", "notes_ask", "notes_log", "notes_propose_done", "notes_create", "[x]"] {
            assert!(ROLE.contains(t), "{t}");
        }
    }
}
