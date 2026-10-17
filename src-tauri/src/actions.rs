//! Rychlé akce (config `actions`): tvar, validace a čistá rozhodnutí — potřebuje akce potvrzení,
//! otisk pro uložený souhlas, příkazová řádka, barva monogramu. Spouštění je v `actionrun.rs`,
//! ikony v `actionicons.rs`, Tauri příkazy v `commands/actions.rs`. Config smí měnit i Claude a
//! průvodce, takže cíl je nedůvěryhodný: web jen http(s), cesty přes `personal::normalize`,
//! příkaz bez uloženého souhlasu se vždy potvrzuje.

use crate::i18n::tr;
use crate::personal;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Nejvíc akcí v configu.
pub const MAX_ACTIONS: usize = 64;
const MAX_TARGET: usize = 1024;
pub const MAX_LABEL: usize = 80;
const MAX_GROUP: usize = 40;
const MAX_ARGS: usize = 32;
/// Výchozí termín skrytého příkazu (s); `timeoutS` 1–3600.
pub const DEFAULT_TIMEOUT_S: u32 = 60;
/// Přípony vlastní ikony (`icon`: cesta k obrázku).
pub const ICON_EXT: &[&str] = &["png", "ico", "jpg", "jpeg", "gif", "webp", "bmp", "svg"];

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ActionKind {
    /// web (jen http/https)
    #[default]
    Url,
    /// dokument / obrázek / médium (allowlist přípon jako u odkazů Osobní)
    File,
    Folder,
    /// `.exe` (absolutně, nebo jen jméno ze složky Windows); `args` = s argumenty
    App,
    /// příkaz pro `cmd` / PowerShell — jen s potvrzením nebo uloženým souhlasem
    Command,
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ShowOn {
    Work,
    Personal,
    #[default]
    Both,
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Shell {
    #[default]
    Cmd,
    Powershell,
}

fn is_default<T: Default + PartialEq>(v: &T) -> bool {
    *v == T::default()
}
fn default_timeout() -> u32 {
    DEFAULT_TIMEOUT_S
}
fn is_default_timeout(v: &u32) -> bool {
    *v == DEFAULT_TIMEOUT_S
}

/// Rychlá akce: dlaždice na jedno kliknutí (karta „Rychlé akce“) a položka palety příkazů.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(default, rename_all = "camelCase")]
pub struct Action {
    /// Jednoznačné id (a–z, 0–9, `-`, `_`), povinné.
    pub id: String,
    /// Popisek dlaždice, povinný (nejvýš 80 znaků).
    pub label: String,
    pub kind: ActionKind,
    /// url: `https://…`; file/folder/app: absolutní cesta (`app` i holé jméno `.exe` ze složky
    /// Windows); command: příkazová řádka (`npm run build`, `C:\skripty\zaloha.bat`).
    pub target: String,
    /// Argumenty pro `app` a `command` (každý zvlášť, bez uvozovek).
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub args: Vec<String>,
    /// Pracovní složka pro `app` / `command` (absolutní); výchozí = domovská složka.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cwd: Option<String>,
    /// Ikona: chybí / `"auto"` = favicon webu (stáhne se jednou) nebo ikona souboru / složky / exe;
    /// `"monogram"` = písmena v barvě; jinak absolutní cesta k obrázku (png, ico, jpg, svg…).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,
    /// Skupina na kartě (nepovinné, nejvýš 40 znaků).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub group: Option<String>,
    /// Ptát se před každým spuštěním. Příkaz bez souhlasu uloženého v Nastavení se ptá vždy.
    #[serde(skip_serializing_if = "is_default")]
    pub confirm: bool,
    /// Na které záložce: `work` (Práce), `personal` (Osobní), `both` (výchozí).
    #[serde(skip_serializing_if = "is_default")]
    pub show_on: ShowOn,
    /// Jen `command`: `cmd` (výchozí) nebo `powershell` (`-NoProfile`).
    #[serde(skip_serializing_if = "is_default")]
    pub shell: Shell,
    /// Jen `command`: `true` = ve viditelném terminálu; `false` (výchozí) = bez okna, výstup do hlášky.
    #[serde(skip_serializing_if = "is_default")]
    pub terminal: bool,
    /// Jen skrytý `command`: termín v sekundách (1–3600, výchozí 60) — pak se strom procesu zabije.
    #[serde(default = "default_timeout", skip_serializing_if = "is_default_timeout")]
    pub timeout_s: u32,
}

impl Default for Action {
    fn default() -> Self {
        Self {
            id: String::new(),
            label: String::new(),
            kind: ActionKind::Url,
            target: String::new(),
            args: Vec::new(),
            cwd: None,
            icon: None,
            group: None,
            confirm: false,
            show_on: ShowOn::Both,
            shell: Shell::Cmd,
            terminal: false,
            timeout_s: DEFAULT_TIMEOUT_S,
        }
    }
}

impl Action {
    /// Spouští kód s argumenty (příkaz, aplikace s `args`) — bez uloženého souhlasu se potvrzuje.
    pub fn risky(&self) -> bool {
        self.kind == ActionKind::Command || (self.kind == ActionKind::App && !self.args.is_empty())
    }
}

fn valid_id(id: &str) -> bool {
    !id.is_empty() && id.len() <= 64 && id.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-' || c == '_')
}

/// Absolutní cesta s písmenem disku (bez UNC, ADS, koncových teček) — jen tvar, bez statu.
fn abs_path(t: &str) -> Result<(), String> {
    let n = personal::normalize(t.trim())?;
    let b = n.as_bytes();
    if b.len() >= 3 && b[0].is_ascii_alphabetic() && b[1] == b':' && (b[2] == b'\\' || b[2] == b'/') {
        Ok(())
    } else {
        Err(tr("cesta musí být absolutní (C:\\…)", "the path must be absolute (C:\\…)").into())
    }
}

/// Holé jméno `.exe` (`notepad.exe`) — hledá se jen ve složce Windows (`personal::classify`).
fn bare_exe(t: &str) -> bool {
    let t = t.trim();
    !t.contains(['\\', '/', ':', '"']) && t.to_ascii_lowercase().ends_with(".exe")
}

/// Tvarová kontrola jedné akce (bez disku). Chyba = config se neaplikuje.
pub fn validate_one(a: &Action) -> Result<(), String> {
    let name = if a.label.trim().is_empty() { a.id.as_str() } else { a.label.as_str() };
    let err = |m: String| Err(trf!("Akce „{name}“: {m}", "Action “{name}”: {m}"));
    if !valid_id(&a.id) {
        return err(tr("id smí mít jen a–z, 0–9, - a _ (1–64 znaků).", "the id may only have a–z, 0–9, - and _ (1–64 characters).").into());
    }
    if a.label.trim().is_empty() || a.label.chars().count() > MAX_LABEL {
        return err(trf!("popisek je povinný (nejvýš {MAX_LABEL} znaků).", "the label is required (at most {MAX_LABEL} characters)."));
    }
    let t = a.target.trim();
    if t.is_empty() || a.target.chars().count() > MAX_TARGET || a.target.contains('\0') {
        return err(trf!("cíl je povinný (nejvýš {MAX_TARGET} znaků).", "the target is required (at most {MAX_TARGET} characters)."));
    }
    match a.kind {
        ActionKind::Url => {
            if !matches!(personal::classify(t, &[], &|_| false, &|_| false), personal::LinkTarget::Web(_)) {
                return err(tr("web musí začínat http:// nebo https://.", "a website must start with http:// or https://.").into());
            }
        }
        ActionKind::File | ActionKind::Folder => abs_path(t).or_else(&err)?,
        ActionKind::App => {
            if !bare_exe(t) {
                abs_path(t).or_else(&err)?;
                if !t.trim_end_matches(['.', ' ']).to_ascii_lowercase().ends_with(".exe") {
                    return err(tr("aplikace musí být .exe (skript spusť jako příkaz).", "an app must be an .exe (run a script as a command).").into());
                }
            }
        }
        ActionKind::Command => {}
    }
    if !a.args.is_empty() && !matches!(a.kind, ActionKind::App | ActionKind::Command) {
        return err(tr("argumenty jdou jen u aplikace a příkazu.", "arguments only work for an app or a command.").into());
    }
    if a.args.len() > MAX_ARGS || a.args.iter().any(|x| x.len() > MAX_TARGET || x.contains(['\0', '"', '\n', '\r'])) {
        return err(trf!("nejvýš {MAX_ARGS} argumentů, bez uvozovek a nových řádků.", "at most {MAX_ARGS} arguments, without quotes and new lines."));
    }
    if let Some(cwd) = &a.cwd
        && !cwd.trim().is_empty()
    {
        abs_path(cwd).map_err(|e| trf!("Akce „{name}“: pracovní složka — {e}", "Action “{name}”: working folder — {e}"))?;
    }
    if let Some(icon) = a.icon.as_deref().map(str::trim)
        && !icon.is_empty()
        && icon != "auto"
        && icon != "monogram"
    {
        abs_path(icon).map_err(|e| trf!("Akce „{name}“: ikona — {e}", "Action “{name}”: icon — {e}"))?;
        let ext = Path::new(icon).extension().map(|e| e.to_string_lossy().to_ascii_lowercase()).unwrap_or_default();
        if !ICON_EXT.contains(&ext.as_str()) {
            return err(trf!("ikona musí být obrázek ({}).", "the icon must be an image ({}).", ICON_EXT.join(", ")));
        }
    }
    if a.group.as_ref().is_some_and(|g| g.chars().count() > MAX_GROUP) {
        return err(trf!("skupina má nejvýš {MAX_GROUP} znaků.", "the group has at most {MAX_GROUP} characters."));
    }
    if !(1..=3600).contains(&a.timeout_s) {
        return err(tr("timeoutS musí být 1–3600 s.", "timeoutS must be 1–3600 s.").into());
    }
    Ok(())
}

/// Celý seznam: počet, jednoznačná id, každá akce zvlášť.
pub fn validate(list: &[Action]) -> Result<(), String> {
    if list.len() > MAX_ACTIONS {
        return Err(trf!("actions: nejvýš {MAX_ACTIONS} akcí (je jich {}).", "actions: at most {MAX_ACTIONS} actions (there are {}).", list.len()));
    }
    let mut seen = std::collections::HashSet::new();
    for a in list {
        validate_one(a)?;
        if !seen.insert(a.id.as_str()) {
            return Err(trf!("Id akce „{}“ je dvakrát.", "Action id “{}” is there twice.", a.id));
        }
    }
    Ok(())
}

/// Otisk toho, co akce doopravdy spustí (SHA-256, hex). Souhlas platí jen pro přesně tenhle
/// příkaz — změna cíle, argumentů, složky nebo shellu (třeba synchronizací) ho zneplatní.
pub fn fingerprint(a: &Action) -> String {
    use sha2::{Digest, Sha256};
    let mut h = Sha256::new();
    let kind = serde_json::to_string(&a.kind).unwrap_or_default();
    let shell = serde_json::to_string(&a.shell).unwrap_or_default();
    for part in [kind.as_str(), a.target.trim(), a.cwd.as_deref().unwrap_or("").trim(), shell.as_str(), if a.terminal { "t" } else { "h" }] {
        h.update(part.as_bytes());
        h.update([0u8]);
    }
    for x in &a.args {
        h.update(x.as_bytes());
        h.update([1u8]);
    }
    h.finalize().iter().map(|b| format!("{b:02x}")).collect()
}

/// Musí se člověk před spuštěním potvrdit? `confirm` vždy; rizikové akce bez souhlasu taky.
pub fn needs_confirm(a: &Action, consented: bool) -> bool {
    a.confirm || (a.risky() && !consented)
}

/// Argument pro `cmd` (a ukázku `.exe` s argumenty): v uvozovkách, když obsahuje mezeru nebo zvláštní
/// znak (uvozovky uvnitř validace nepustí). Koncová `\` se před uvozovkou zdvojí — jinak by z ní
/// program (pravidla MSVC) udělal `\"` a slepil argument s dalšími.
fn quote(x: &str) -> String {
    if x.is_empty() || x.contains(|c: char| c.is_whitespace() || "&|<>^()%!,;=".contains(c)) {
        let tail = x.len() - x.trim_end_matches('\\').len();
        format!("\"{x}{}\"", "\\".repeat(tail))
    } else {
        x.to_string()
    }
}

/// Argument pro PowerShell doslova: holé slovo jen z bezpečných znaků, jinak v apostrofech
/// (apostrofy včetně typografických zdvojené). Ve dvojitých uvozovkách by PowerShell rozbalil
/// `$proměnné` a zpětný apostrof; holé `#`, `@`, `,`, `;` nebo `'` by změnily význam řádky.
fn quote_ps(x: &str) -> String {
    if !x.is_empty() && x.chars().all(|c| c.is_alphanumeric() || r"-_./\:=+%~".contains(c)) {
        return x.to_string();
    }
    let mut out = String::from("'");
    for c in x.chars() {
        if matches!(c, '\'' | '\u{2018}' | '\u{2019}' | '\u{201a}' | '\u{201b}') {
            out.push(c);
        }
        out.push(c);
    }
    out.push('\'');
    out
}

/// Příkazová řádka, jak ji uvidí člověk v potvrzení a jak ji dostane shell (argumenty podle shellu).
pub fn command_line(a: &Action) -> String {
    let ps = a.kind == ActionKind::Command && a.shell == Shell::Powershell;
    let mut line = a.target.trim().to_string();
    for x in &a.args {
        line.push(' ');
        line.push_str(&if ps { quote_ps(x) } else { quote(x) });
    }
    line
}

/// Co ukázat v potvrzení: u příkazu celá řádka se shellem, jinak cíl.
pub fn confirm_text(a: &Action) -> String {
    match a.kind {
        ActionKind::Command => {
            let sh = if a.shell == Shell::Powershell { "PowerShell" } else { "cmd" };
            format!("{sh}: {}", command_line(a))
        }
        ActionKind::App if !a.args.is_empty() => command_line(a),
        _ => a.target.trim().to_string(),
    }
}

/// Barva monogramu podle popisku (stabilní, z palety akcentů UI).
pub fn monogram_color(label: &str) -> &'static str {
    const PALETTE: &[&str] = &["#60a5fa", "#a78bfa", "#2dd4bf", "#4ade80", "#fbbf24", "#f87171", "#f472b6", "#fb923c"];
    let h = label.bytes().fold(2166136261u32, |h, b| (h ^ b as u32).wrapping_mul(16777619));
    PALETTE[h as usize % PALETTE.len()]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn act(kind: ActionKind, target: &str) -> Action {
        Action { id: "a".into(), label: "A".into(), kind, target: target.into(), ..Default::default() }
    }

    /// (akce, projde?) — tvar cíle podle typu.
    #[test]
    fn validate_table() {
        let cases: Vec<(Action, bool)> = vec![
            (act(ActionKind::Url, "https://mapy.cz"), true),
            (act(ActionKind::Url, "http://localhost:5173"), true),
            (act(ActionKind::Url, "javascript:alert(1)"), false),
            (act(ActionKind::Url, "file:///C:/x"), false),
            (act(ActionKind::Url, "mapy.cz"), false),
            (act(ActionKind::Folder, r"C:\Users\x\Fotky"), true),
            (act(ActionKind::Folder, r"\\server\share"), false),
            (act(ActionKind::Folder, r"relativni"), false),
            (act(ActionKind::File, r"C:\a.txt:evil.exe"), false),
            (act(ActionKind::App, "notepad.exe"), true),
            (act(ActionKind::App, r"C:\Hry\hra.exe"), true),
            (act(ActionKind::App, r"C:\x\run.bat"), false),
            (act(ActionKind::Command, "npm run build"), true),
            (act(ActionKind::Command, ""), false),
            (Action { id: "A b".into(), ..act(ActionKind::Url, "https://x.cz") }, false),
            (Action { label: " ".into(), ..act(ActionKind::Url, "https://x.cz") }, false),
            (Action { args: vec!["-x".into()], ..act(ActionKind::Url, "https://x.cz") }, false),
            (Action { args: vec!["a\"b".into()], ..act(ActionKind::Command, "echo") }, false),
            (Action { args: vec!["--port".into(), "5173".into()], ..act(ActionKind::App, r"C:\a\b.exe") }, true),
            (Action { cwd: Some("rel".into()), ..act(ActionKind::Command, "dir") }, false),
            (Action { cwd: Some(r"C:\repo".into()), ..act(ActionKind::Command, "dir") }, true),
            (Action { icon: Some("auto".into()), ..act(ActionKind::Url, "https://x.cz") }, true),
            (Action { icon: Some(r"C:\i\logo.png".into()), ..act(ActionKind::Url, "https://x.cz") }, true),
            (Action { icon: Some(r"C:\i\logo.exe".into()), ..act(ActionKind::Url, "https://x.cz") }, false),
            (Action { timeout_s: 0, ..act(ActionKind::Command, "dir") }, false),
        ];
        for (a, ok) in cases {
            assert_eq!(validate_one(&a).is_ok(), ok, "{a:?} → {:?}", validate_one(&a));
        }
    }

    #[test]
    fn list_limits_and_unique_ids() {
        let a = act(ActionKind::Url, "https://x.cz");
        assert!(validate(&[a.clone(), Action { id: "b".into(), ..a.clone() }]).is_ok());
        assert!(validate(&[a.clone(), a.clone()]).unwrap_err().contains("dvakrát"));
        let many: Vec<Action> = (0..=MAX_ACTIONS).map(|i| Action { id: format!("a{i}"), ..a.clone() }).collect();
        assert!(validate(&many).is_err());
    }

    /// (akce, souhlas, potvrzovat?) — příkaz bez souhlasu vždy, `confirm` vždy, web nikdy sám.
    #[test]
    fn confirm_table() {
        let cmd = act(ActionKind::Command, "dir");
        let app_args = Action { args: vec!["-x".into()], ..act(ActionKind::App, r"C:\a.exe") };
        let cases = [
            (act(ActionKind::Url, "https://x.cz"), false, false),
            (Action { confirm: true, ..act(ActionKind::Url, "https://x.cz") }, true, true),
            (cmd.clone(), false, true),
            (cmd.clone(), true, false),
            (Action { confirm: true, ..cmd }, true, true),
            (act(ActionKind::App, r"C:\a.exe"), false, false),
            (app_args.clone(), false, true),
            (app_args, true, false),
        ];
        for (a, consent, want) in cases {
            assert_eq!(needs_confirm(&a, consent), want, "{a:?} consent={consent}");
        }
    }

    #[test]
    fn fingerprint_tracks_what_runs_not_how_it_looks() {
        let a = Action { args: vec!["x".into()], ..act(ActionKind::Command, "echo") };
        let f = fingerprint(&a);
        assert_eq!(f.len(), 64);
        // popisek, ikona, skupina, potvrzení → stejný otisk
        let cosmetic = Action { label: "Jiný".into(), icon: Some("monogram".into()), group: Some("g".into()), confirm: true, ..a.clone() };
        assert_eq!(fingerprint(&cosmetic), f);
        // cokoli, co mění spuštěný kód → jiný otisk
        for changed in [
            Action { target: "del".into(), ..a.clone() },
            Action { args: vec!["y".into()], ..a.clone() },
            Action { args: vec!["x".into(), "y".into()], ..a.clone() },
            Action { cwd: Some(r"C:\x".into()), ..a.clone() },
            Action { shell: Shell::Powershell, ..a.clone() },
            Action { terminal: true, ..a.clone() },
        ] {
            assert_ne!(fingerprint(&changed), f, "{changed:?}");
        }
        // hranice mezi argumenty: ["a b"] ≠ ["a", "b"]
        let one = Action { args: vec!["a b".into()], ..a.clone() };
        let two = Action { args: vec!["a".into(), "b".into()], ..a };
        assert_ne!(fingerprint(&one), fingerprint(&two));
    }

    #[test]
    fn command_line_quotes_only_when_needed() {
        let a = Action { args: vec!["--out".into(), r"C:\Moje složka".into(), "a&b".into(), "".into()], ..act(ActionKind::Command, " npm run x ") };
        assert_eq!(command_line(&a), r#"npm run x --out "C:\Moje složka" "a&b" """#);
        assert_eq!(confirm_text(&a), format!("cmd: {}", command_line(&a)));
        let ps = Action { shell: Shell::Powershell, ..act(ActionKind::Command, "Get-Date") };
        assert_eq!(confirm_text(&ps), "PowerShell: Get-Date");
        assert_eq!(confirm_text(&act(ActionKind::Url, "https://x.cz")), "https://x.cz");
    }

    /// (argument, cmd, PowerShell) — argument musí do programu dojít doslova.
    #[test]
    fn args_quoting_per_shell() {
        let cases: &[(&str, &str, &str)] = &[
            ("--port", "--port", "--port"),
            ("C:/a.txt", "C:/a.txt", "C:/a.txt"),
            (r"C:\Moje složka\", r#""C:\Moje složka\\""#, r"'C:\Moje složka\'"),
            (r"C:\a b\\", r#""C:\a b\\\\""#, r"'C:\a b\\'"),
            (r"C:\bez\", r"C:\bez\", r"C:\bez\"),
            ("a b$HOME", r#""a b$HOME""#, "'a b$HOME'"),
            ("$env:USERNAME", "$env:USERNAME", "'$env:USERNAME'"),
            ("it's", "it's", "'it''s'"),
            ("it\u{2019}s ok", "\"it\u{2019}s ok\"", "'it\u{2019}\u{2019}s ok'"),
            ("#123", "#123", "'#123'"),
            ("a,b", r#""a,b""#, "'a,b'"),
            ("@x", "@x", "'@x'"),
            ("`n", "`n", "'`n'"),
            ("", r#""""#, "''"),
            ("žluťoučký", "žluťoučký", "žluťoučký"),
        ];
        for (arg, cmd, ps) in cases {
            let a = Action { args: vec![(*arg).into()], ..act(ActionKind::Command, "prog") };
            assert_eq!(command_line(&a), format!("prog {cmd}"), "cmd {arg:?}");
            let p = Action { shell: Shell::Powershell, ..a };
            assert_eq!(command_line(&p), format!("prog {ps}"), "ps {arg:?}");
        }
    }

    #[test]
    fn serde_defaults_and_compact_output() {
        let a: Action = serde_json::from_str(r#"{"id":"w","label":"Web","target":"https://x.cz"}"#).unwrap();
        assert_eq!(a.kind, ActionKind::Url);
        assert_eq!(a.show_on, ShowOn::Both);
        assert_eq!(a.timeout_s, DEFAULT_TIMEOUT_S);
        let v = serde_json::to_value(&a).unwrap();
        // výchozí hodnoty se do configu nepíšou (čitelný soubor)
        assert_eq!(v, serde_json::json!({ "id": "w", "label": "Web", "kind": "url", "target": "https://x.cz" }));
        let c: Action = serde_json::from_str(r#"{"id":"c","label":"B","kind":"command","target":"x","showOn":"work","shell":"powershell","terminal":true,"timeoutS":5}"#).unwrap();
        assert_eq!((c.kind, c.show_on, c.shell, c.terminal, c.timeout_s), (ActionKind::Command, ShowOn::Work, Shell::Powershell, true, 5));
    }

    /// Starý config bez `actions` platí beze změny; špatná akce shodí validaci celého configu.
    #[test]
    fn config_compat_and_schema() {
        use crate::config::{Config, validate};
        let old: Config = serde_json::from_str("{}").unwrap();
        assert!(old.actions.is_empty() && validate(&old).is_ok());
        let cfg: Config = serde_json::from_str(r#"{"actions":[{"id":"mapy","label":"Mapy","target":"https://mapy.cz","showOn":"personal"}]}"#).unwrap();
        assert!(validate(&cfg).is_ok());
        let bad: Config = serde_json::from_str(r#"{"actions":[{"id":"x","label":"X","target":"javascript:alert(1)"}]}"#).unwrap();
        assert!(validate(&bad).unwrap_err().contains("http"));
        let schema = crate::config::schema_json();
        assert!(schema.contains("\"actions\"") && schema.contains("showOn") && schema.contains("powershell"));
    }

    #[test]
    fn monogram_color_is_stable() {
        assert_eq!(monogram_color("Mapy"), monogram_color("Mapy"));
        assert!(monogram_color("").starts_with('#'));
    }
}
