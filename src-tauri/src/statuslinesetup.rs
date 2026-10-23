//! Zapojení stavového řádku Ledge do `~/.claude/settings.json` (Nastavení → Claude Code →
//! „Zapojit stavový řádek“). Ledge se stane průchozím wrapperem (`ledge.exe --statusline`,
//! `statusline.rs`): zapíše si limity a spustí PŮVODNÍ příkaz uživatele beze změny výstupu.
//!
//! Čisté `merge` / `unmerge` / `status` nad textem stejným stylem jako `hooksetup` (vlastní `J` se
//! zachovaným pořadím klíčů, nevalidní JSON = chyba, nikdy přepsání; zápis přes `hooksetup::apply` =
//! záloha `settings.json.bak-pred-ledge` jen jednou + atomicky). Původní hodnota klíče `statusLine`
//! (celá, i `padding`/`refreshInterval`, nebo „nebyl“) se uloží do
//! `%LOCALAPPDATA%\Ledge\statusline-original.json` → odpojení ji vrátí přesně na totéž místo.

use crate::hooksetup::{self, J};
use serde::Serialize;
use std::path::{Path, PathBuf};

const KEY: &str = "statusLine";
const FLAG: &str = "--statusline";

/// Uložená původní hodnota `statusLine`; `value: None` = klíč v settings.json nebyl.
#[derive(Debug, Clone, PartialEq)]
pub struct Stored {
    pub value: Option<J>,
}

// ---------- čistá logika ----------

/// Je to příkaz wrapperu Ledge? (`…ledge….exe --statusline`, s uvozovkami i bez, `&` PowerShellu)
pub fn is_ours_cmd(cmd: &str) -> bool {
    let c = cmd.trim().to_lowercase();
    c.ends_with(FLAG) && c.contains("ledge")
}

fn is_ours(v: &J) -> bool {
    v.str("command").is_some_and(is_ours_cmd)
}

/// Příkaz wrapperu pro `statusLine.command`. Claude Code ho na Windows spouští přes Git Bash
/// (když je), jinak přes PowerShell: cesta vždy s `/` (Git Bash bere `\` jako escape); bez mezer
/// a zvláštních znaků bez uvozovek (projde oběma shelly), jinak podle shellu (`"…"` / `& '…'`).
pub fn our_command(exe: &Path, bash: bool) -> String {
    let p = exe.to_string_lossy().replace('\\', "/");
    if p.chars().all(|c| c.is_ascii_alphanumeric() || "/:._-".contains(c)) {
        format!("{p} {FLAG}")
    } else if bash {
        let mut q = String::new();
        for ch in p.chars() {
            if matches!(ch, '"' | '$' | '`' | '\\') {
                q.push('\\');
            }
            q.push(ch);
        }
        format!("\"{q}\" {FLAG}")
    } else {
        format!("& '{}' {FLAG}", p.replace('\'', "''"))
    }
}

fn set(obj: &mut J, k: &str, v: J) {
    if let Some(slot) = obj.get_mut(k) {
        *slot = v;
    } else if let J::Obj(m) = obj {
        m.push((k.to_string(), v));
    }
}

fn root_vec(root: &mut J) -> &mut Vec<(String, J)> {
    match root {
        J::Obj(m) => m,
        _ => unreachable!("hooksetup::parse vrací objekt"),
    }
}

#[derive(Debug, PartialEq)]
pub struct Merged {
    pub text: String,
    pub changed: bool,
    /// `Some` = uložit jako původní hodnotu (dosavadní `statusLine` nebyl náš)
    pub store: Option<Stored>,
}

/// Zapojí wrapper: cizí `statusLine` nahradí na místě (ostatní klíče objektu — `padding`,
/// `refreshInterval` — zůstanou), chybějící přidá na konec. Už zapojený jen přepíše cestu k exe.
pub fn merge(text: &str, ours: &str) -> Result<Merged, String> {
    let old = hooksetup::parse(text)?;
    let mut root = old.clone();
    let m = root_vec(&mut root);
    let idx = m.iter().rposition(|(k, _)| k == KEY);
    let cur = idx.map(|i| m[i].1.clone());
    let cur_ours = cur.as_ref().is_some_and(is_ours);
    let mut new_val = match &cur {
        Some(v @ J::Obj(_)) => v.clone(),
        _ => J::Obj(vec![]),
    };
    set(&mut new_val, "type", J::Str("command".into()));
    set(&mut new_val, "command", J::Str(ours.into()));
    match idx {
        Some(i) => m[i].1 = new_val,
        None => m.push((KEY.into(), new_val)),
    }
    let changed = root != old;
    Ok(Merged {
        text: if changed { hooksetup::render(&root, text) } else { text.to_string() },
        changed,
        store: (!cur_ours).then_some(Stored { value: cur }),
    })
}

/// Odpojí wrapper: vrátí uloženou původní hodnotu (`None` / nic uloženého = klíč smaže).
/// `statusLine`, který není náš, nechá být.
pub fn unmerge(text: &str, stored: Option<&Stored>) -> Result<(String, bool), String> {
    let old = hooksetup::parse(text)?;
    let mut root = old.clone();
    let m = root_vec(&mut root);
    let Some(i) = m.iter().rposition(|(k, _)| k == KEY).filter(|&i| is_ours(&m[i].1)) else {
        return Ok((text.to_string(), false));
    };
    match stored.and_then(|s| s.value.clone()) {
        Some(v) => m[i].1 = v,
        None => {
            m.remove(i);
        }
    }
    let changed = root != old;
    Ok((if changed { hooksetup::render(&root, text) } else { text.to_string() }, changed))
}

/// Stav pro UI: `full` = zapojený wrapper Ledge, `foreign` = jiný stavový řádek (zapojení ho
/// obalí), `none` = žádný, `error` = settings.json nejde přečíst.
pub fn status(text: &str) -> (&'static str, Option<String>) {
    let Ok(root) = hooksetup::parse(text) else { return ("error", None) };
    match root.get(KEY) {
        None | Some(J::Null) => ("none", None),
        Some(v) if is_ours(v) => ("full", None),
        Some(v) => ("foreign", v.str("command").map(str::to_string)),
    }
}

pub fn render_stored(s: &Stored) -> String {
    let v = J::obj(vec![("had", J::Bool(s.value.is_some())), ("value", s.value.clone().unwrap_or(J::Null))]);
    serde_json::to_string_pretty(&v).unwrap_or_default()
}

pub fn parse_stored(text: &str) -> Result<Stored, String> {
    let v: J = serde_json::from_str(text.trim_start_matches('\u{feff}')).map_err(|e| crate::errors::json(crate::i18n::tr("Uložená původní hodnota stavového řádku", "The saved original status line"), &e))?;
    match (v.get("had"), v.get("value")) {
        (Some(J::Bool(true)), Some(x)) => Ok(Stored { value: Some(x.clone()) }),
        (Some(J::Bool(false)), _) => Ok(Stored { value: None }),
        _ => Err("statusline-original.json: chybí had/value".into()),
    }
}

/// Původní příkaz, který má wrapper spustit; nikdy sám sebe (zacyklení).
pub fn original_command(s: &Stored) -> Option<String> {
    let v = s.value.as_ref()?;
    if v.str("type").is_some_and(|t| t != "command") {
        return None;
    }
    v.str("command").map(str::trim).filter(|c| !c.is_empty() && !is_ours_cmd(c)).map(str::to_string)
}

// ---------- soubory ----------

/// `%LOCALAPPDATA%\Ledge\statusline-original.json` (testy: `LEDGE_CONFIG_DIR\local`).
pub fn store_path() -> PathBuf {
    crate::actionrun::local_dir().join("statusline-original.json")
}

/// Uložený původní `statusLine`: soubor chybí = `Ok(None)`, rozbitý = chyba (nehádat).
pub fn load_stored(path: &Path) -> Result<Option<Stored>, String> {
    match std::fs::read_to_string(path) {
        Ok(t) => parse_stored(&t).map(Some).map_err(|e| crate::errors::prefixed(&path.display().to_string(), &e)),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(crate::errors::io(&path.display().to_string(), &e)),
    }
}

/// Atomický zápis (tmp + rename) — sdílí ho i wrapper (`statusline.rs`) pro vzorek limitů.
pub fn write_atomic(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    let dir = path.parent().ok_or_else(|| std::io::Error::other("bez složky"))?;
    std::fs::create_dir_all(dir)?;
    let name = path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
    let tmp = dir.join(format!("{name}.{}.tmp", std::process::id()));
    std::fs::write(&tmp, bytes)?;
    std::fs::rename(&tmp, path).inspect_err(|_| {
        let _ = std::fs::remove_file(&tmp);
    })
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StatusLineStatus {
    /// none | foreign | full | error
    pub state: &'static str,
    /// příkaz, který wrapper spouští (zapojeno) / který obalí (cizí)
    pub command: Option<String>,
    pub error: Option<String>,
    pub path: String,
}

pub fn status_at(settings: &Path, store: &Path) -> StatusLineStatus {
    let path = settings.display().to_string();
    let err = |e: String| StatusLineStatus { state: "error", command: None, error: Some(e), path: path.clone() };
    let text = match hooksetup::read(settings) {
        Ok(t) => t,
        Err(e) => return err(e),
    };
    if let Err(e) = hooksetup::parse(&text) {
        return err(e);
    }
    let (state, foreign) = status(&text);
    let command = if state == "full" { load_stored(store).ok().flatten().as_ref().and_then(original_command) } else { foreign };
    StatusLineStatus { state, command, error: None, path }
}

/// Zapojí wrapper: nejdřív uloží původní hodnotu, pak zapíše settings.json. Vrací „změněno?“.
pub fn install_at(settings: &Path, store: &Path, ours: &str) -> Result<bool, String> {
    hooksetup::apply(settings, |t| {
        let m = merge(t, ours)?;
        if let Some(s) = &m.store {
            write_atomic(store, render_stored(s).as_bytes()).map_err(|e| crate::errors::io(&store.display().to_string(), &e))?;
        }
        Ok((m.text, m.changed))
    })
}

/// Odpojí wrapper a vrátí původní hodnotu; uložená hodnota se po úspěchu smaže.
pub fn remove_at(settings: &Path, store: &Path) -> Result<bool, String> {
    let stored = load_stored(store)?;
    let changed = hooksetup::apply(settings, |t| unmerge(t, stored.as_ref()))?;
    let text = hooksetup::read(settings)?;
    if status(&text).0 != "full" {
        let _ = std::fs::remove_file(store);
    }
    Ok(changed)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{Value, json};

    const OURS: &str = "C:/Users/x/AppData/Local/Ledge/ledge.exe --statusline";

    fn v(t: &str) -> Value {
        serde_json::from_str(t).unwrap()
    }

    #[test]
    fn our_command_table() {
        let cases = [
            (r"C:\Users\x\AppData\Local\Ledge\ledge.exe", true, "C:/Users/x/AppData/Local/Ledge/ledge.exe --statusline"),
            (r"C:\Users\x\AppData\Local\Ledge\ledge.exe", false, "C:/Users/x/AppData/Local/Ledge/ledge.exe --statusline"),
            (r"C:\Users\Jan Novák\Ledge\ledge.exe", true, "\"C:/Users/Jan Novák/Ledge/ledge.exe\" --statusline"),
            (r"C:\Users\Jan Novák\Ledge\ledge.exe", false, "& 'C:/Users/Jan Novák/Ledge/ledge.exe' --statusline"),
            (r"C:\a$b\ledge.exe", true, "\"C:/a\\$b/ledge.exe\" --statusline"),
            (r"C:\O'Neil\ledge.exe", false, "& 'C:/O''Neil/ledge.exe' --statusline"),
        ];
        for (exe, bash, want) in cases {
            let got = our_command(Path::new(exe), bash);
            assert_eq!(got, want, "{exe} bash={bash}");
            assert!(is_ours_cmd(&got));
        }
        for (cmd, want) in [("bash ~/.claude/statusline.sh", false), ("ledge.exe", false), ("node ledge-stats.mjs --statusline", true), ("  C:/LEDGE/LEDGE.EXE --STATUSLINE  ", true)] {
            assert_eq!(is_ours_cmd(cmd), want, "{cmd}");
        }
    }

    #[test]
    fn merge_wraps_foreign_and_unmerge_restores_exactly() {
        let text = "{\n  \"model\": \"opus\",\n  \"statusLine\": {\n    \"padding\": 2,\n    \"type\": \"command\",\n    \"command\": \"bash ~/.claude/statusline.sh\",\n    \"refreshInterval\": 5\n  },\n  \"zzz\": 1\n}\n";
        let m = merge(text, OURS).unwrap();
        assert!(m.changed);
        let j = v(&m.text);
        assert_eq!(j["statusLine"], json!({"padding": 2, "type": "command", "command": OURS, "refreshInterval": 5}));
        // pořadí: klíč na místě, uvnitř objektu taky
        assert!(m.text.find("\"model\"").unwrap() < m.text.find("\"statusLine\"").unwrap() && m.text.find("\"statusLine\"").unwrap() < m.text.find("\"zzz\"").unwrap());
        assert!(m.text.find("\"padding\"").unwrap() < m.text.find("\"type\"").unwrap());
        let stored = m.store.clone().unwrap();
        assert_eq!(original_command(&stored).as_deref(), Some("bash ~/.claude/statusline.sh"));
        assert_eq!(status(&m.text), ("full", None));
        // uložená hodnota přežije zápis do souboru
        let back = parse_stored(&render_stored(&stored)).unwrap();
        assert_eq!(back, stored);
        // odpojení = původní text (stejné odsazení 2 mezery jako Claude Code)
        let (out, changed) = unmerge(&m.text, Some(&back)).unwrap();
        assert!(changed);
        assert_eq!(out, text);
        // idempotence: znovu zapojit nic neuloží (už je náš), jen jiná cesta k exe
        let again = merge(&m.text, OURS).unwrap();
        assert_eq!((again.changed, again.store), (false, None));
        let moved = merge(&m.text, "D:/ledge.exe --statusline").unwrap();
        assert!(moved.changed && moved.store.is_none());
        assert_eq!(v(&moved.text)["statusLine"]["refreshInterval"], 5);
    }

    #[test]
    fn missing_statusline_is_added_and_removed() {
        let text = "{\n  \"model\": \"opus\"\n}\n";
        let m = merge(text, OURS).unwrap();
        assert_eq!(v(&m.text), json!({"model": "opus", "statusLine": {"type": "command", "command": OURS}}));
        let stored = m.store.unwrap();
        assert_eq!(stored, Stored { value: None });
        assert_eq!(original_command(&stored), None, "bez původního příkazu wrapper nic nespouští");
        assert_eq!(parse_stored(&render_stored(&stored)).unwrap(), stored);
        assert_eq!(unmerge(&m.text, Some(&stored)).unwrap().0, text);
        // nic uloženého (smazaný soubor) → klíč zmizí
        assert_eq!(unmerge(&m.text, None).unwrap().0, text);
        // prázdný soubor
        let e = merge("", OURS).unwrap();
        assert_eq!(v(&e.text)["statusLine"]["command"], OURS);
    }

    #[test]
    fn foreign_shapes_are_kept() {
        // cizí statusLine, který není náš, odpojení nechá
        let foreign = r#"{"statusLine":{"type":"command","command":"x.sh"}}"#;
        assert_eq!(unmerge(foreign, None).unwrap(), (foreign.to_string(), false));
        assert_eq!(status(foreign), ("foreign", Some("x.sh".into())));
        assert_eq!(status("{}"), ("none", None));
        assert_eq!(status(""), ("none", None));
        assert_eq!(status(r#"{"statusLine":null}"#), ("none", None));
        assert_eq!(status("{rozbité").0, "error");
        // neobjektová hodnota se uloží a vrátí přesně
        let odd = r#"{"statusLine":"divné"}"#;
        let m = merge(odd, OURS).unwrap();
        assert_eq!(m.store, Some(Stored { value: Some(J::Str("divné".into())) }));
        assert_eq!(v(&unmerge(&m.text, m.store.as_ref()).unwrap().0), json!({"statusLine": "divné"}));
        // rozbitý JSON = chyba, žádné přepsání
        assert!(merge("{nejson", OURS).is_err());
        assert!(merge("[1]", OURS).is_err());
        assert!(unmerge("{nejson", None).is_err());
        assert!(parse_stored("{}").is_err());
        assert!(parse_stored("nejson").is_err());
    }

    #[test]
    fn original_command_never_ours() {
        let s = |v: Value| Stored { value: Some(serde_json::from_value::<J>(v).unwrap()) };
        let cases = [
            (s(json!({"type": "command", "command": " node a.mjs "})), Some("node a.mjs")),
            (s(json!({"command": "node a.mjs"})), Some("node a.mjs")),
            (s(json!({"type": "command", "command": OURS})), None),
            (s(json!({"type": "command", "command": ""})), None),
            (s(json!({"type": "jiný", "command": "x"})), None),
            (Stored { value: None }, None),
        ];
        for (st, want) in cases {
            assert_eq!(original_command(&st).as_deref(), want, "{st:?}");
        }
    }

    #[test]
    fn files_install_and_remove() {
        let dir = tempfile::tempdir().unwrap();
        let settings = dir.path().join("settings.json");
        let store = dir.path().join("local").join("statusline-original.json");
        let text = "{\n  \"statusLine\": {\n    \"type\": \"command\",\n    \"command\": \"bash ~/s.sh\"\n  }\n}\n";
        std::fs::write(&settings, text).unwrap();
        assert_eq!(status_at(&settings, &store).state, "foreign");
        assert!(install_at(&settings, &store, OURS).unwrap());
        let st = status_at(&settings, &store);
        assert_eq!((st.state, st.command.as_deref()), ("full", Some("bash ~/s.sh")));
        assert_eq!(std::fs::read_to_string(dir.path().join("settings.json.bak-pred-ledge")).unwrap(), text);
        // druhé zapojení nepřepíše uložený originál naším příkazem
        assert!(!install_at(&settings, &store, OURS).unwrap());
        assert_eq!(original_command(&load_stored(&store).unwrap().unwrap()).as_deref(), Some("bash ~/s.sh"));
        assert!(remove_at(&settings, &store).unwrap());
        assert_eq!(std::fs::read_to_string(&settings).unwrap(), text);
        assert!(!store.exists(), "po odpojení uložená hodnota pryč");
        // rozbitá uložená hodnota = chyba, settings.json se nemění
        assert!(install_at(&settings, &store, OURS).unwrap());
        std::fs::write(&store, "{rozbité").unwrap();
        let before = std::fs::read_to_string(&settings).unwrap();
        assert!(remove_at(&settings, &store).is_err());
        assert_eq!(std::fs::read_to_string(&settings).unwrap(), before);
        // nevalidní settings.json zůstane netknutý
        std::fs::write(&settings, "{rozbité").unwrap();
        assert!(install_at(&settings, &store, OURS).is_err());
        assert_eq!(std::fs::read_to_string(&settings).unwrap(), "{rozbité");
        assert_eq!(status_at(&settings, &store).state, "error");
    }

    #[test]
    fn write_atomic_replaces() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("a").join("x.json");
        write_atomic(&p, b"1").unwrap();
        write_atomic(&p, b"22").unwrap();
        assert_eq!(std::fs::read(&p).unwrap(), b"22");
        assert_eq!(std::fs::read_dir(p.parent().unwrap()).unwrap().count(), 1, "žádný tmp");
    }
}
