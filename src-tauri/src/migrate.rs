//! Migrace dat při přejmenování Notch → Ledge (0.2.0). Běží jednou při startu, před načtením configu.
//!
//! - **Config:** `%APPDATA%\Ledge\config.json` neexistuje a `%APPDATA%\Notch\config.json` ano → config se
//!   **zkopíruje** (starý zůstává, kdyby šéf spustil starý Notch). `notes.root` mířící na starou výchozí
//!   `%USERPROFILE%\Notch\notes` se v kopii vyprázdní (= nová výchozí). Schéma vygeneruje `load_or_seed`.
//! - **Poznámky:** nový výchozí kořen `%USERPROFILE%\Ledge\notes` neexistuje (nebo je prázdný — zakládá ho
//!   vlákno poznámek při každém startu), starý ano, starý nemá značku níže a config ukazuje
//!   na novou výchozí → **přesun** přejmenováním. Jiný disk → kopie do `notes.migrace`, ověření počtu
//!   souborů a bajtů, přejmenování na `notes` a teprve pak smazání starých. Jiná chyba → staré zůstanou.
//!   Do staré složky se pak zapíše `PRESUNUTO-DO-LEDGE.txt` s novou cestou (zároveň značka „hotovo“).
//! - Idempotentní: existující nová data se nikdy nepřepíšou; nic se neděje, když je složka configu
//!   přepsaná z prostředí (`LEDGE_CONFIG_DIR`/`NOTCH_CONFIG_DIR` = testy, automatizace).
//!
//! Rozhodnutí jsou čisté funkce ([`should_copy_config`], [`rewrite_old_root`], [`plan_notes`]); I/O dělá
//! [`run`] nad předanými cestami, takže testy běží jen v temp adresářích.

use serde_json::Value;
use std::io;
use std::path::{Path, PathBuf};

/// Soubor ve staré složce poznámek po přesunu — vysvětlení pro člověka a značka, že migrace proběhla.
pub const MARKER: &str = "PRESUNUTO-DO-LEDGE.txt";

/// Kde leží stará a nová data.
#[derive(Debug, Clone)]
pub struct Paths {
    pub old_cfg_dir: PathBuf,
    pub new_cfg_dir: PathBuf,
    pub old_notes: PathBuf,
    pub new_notes: PathBuf,
}

impl Paths {
    /// Skutečné cesty. `None` = nemigrovat (složka configu přepsaná z prostředí, nebo chybí `USERPROFILE`).
    pub fn real() -> Option<Self> {
        if crate::config::config_dir_override().is_some() {
            return None;
        }
        let home = crate::dirs::home().filter(|h| !h.to_string_lossy().trim().is_empty())?;
        Some(Self {
            old_cfg_dir: crate::config::legacy_config_dir(),
            new_cfg_dir: crate::config::config_dir(),
            old_notes: home.join("Notch").join("notes"),
            new_notes: crate::notes::default_root(),
        })
    }
}

/// Spustí migraci na skutečných cestách; vrací řádky do logu.
pub fn at_startup() -> Vec<String> {
    match Paths::real() {
        Some(p) => run(&p, &|n| std::env::var(n).ok(), &|a, b| std::fs::rename(a, b)),
        None => Vec::new(),
    }
}

// ---------- čisté rozhodování ----------

/// Kopírovat config? Jen když nový chybí a starý existuje — nový se nikdy nepřepisuje.
pub fn should_copy_config(new_exists: bool, old_exists: bool) -> bool {
    !new_exists && old_exists
}

/// Windows cesty: bez ohledu na velikost písmen, `/` vs `\` a koncové lomítko.
fn same_path(a: &Path, b: &Path) -> bool {
    let norm = |p: &Path| p.to_string_lossy().replace('/', "\\").trim_end_matches('\\').to_lowercase();
    norm(a) == norm(b)
}

/// Míří `notes.root` (po rozbalení `%VAR%`) na starou výchozí složku?
fn root_is(root: &str, target: &Path, var: &dyn Fn(&str) -> Option<String>) -> bool {
    let r = root.trim();
    !r.is_empty() && crate::notes::expand_env(r, var).is_ok_and(|p| same_path(Path::new(&p), target))
}

/// Text configu s vyprázdněným `notes.root`, pokud mířil na starou výchozí složku. `Ok(None)` = beze
/// změny (kopíruje se doslova), `Err` = config není platný JSON objekt (taky se kopíruje doslova).
pub fn rewrite_old_root(text: &str, old_default: &Path, var: &dyn Fn(&str) -> Option<String>) -> Result<Option<String>, String> {
    let mut v: Value = serde_json::from_str(text.trim_start_matches('\u{feff}')).map_err(|e| e.to_string())?;
    if !v.is_object() {
        return Err("config není JSON objekt".into());
    }
    let Some(root) = v.pointer("/notes/root").and_then(Value::as_str) else { return Ok(None) };
    if !root_is(root, old_default, var) {
        return Ok(None);
    }
    v["notes"]["root"] = Value::String(String::new());
    serde_json::to_string_pretty(&v).map(|t| Some(t + "\n")).map_err(|e| e.to_string())
}

/// Co s poznámkami.
#[derive(Debug, PartialEq, Eq)]
pub enum NotesPlan {
    Move,
    Skip(&'static str),
}

/// `new_has_data` = nová složka existuje a není prázdná (prázdnou zakládá vlákno poznámek při každém
/// startu — kdyby se brala jako „existuje“, jeden nepovedený přesun by migraci zablokoval navždy).
/// `target_is_new_default` = config po migraci ukládá poznámky do nové výchozí složky.
pub fn plan_notes(old_exists: bool, old_marked: bool, new_has_data: bool, target_is_new_default: bool) -> NotesPlan {
    if !old_exists {
        NotesPlan::Skip("stará složka poznámek neexistuje")
    } else if old_marked {
        NotesPlan::Skip("stará složka už je označená jako přesunutá")
    } else if new_has_data {
        NotesPlan::Skip("nová složka poznámek už existuje — nepřepisuji ji")
    } else if !target_is_new_default {
        NotesPlan::Skip("config ukládá poznámky jinam než do nové výchozí složky")
    } else {
        NotesPlan::Move
    }
}

/// Kam config po migraci ukládá poznámky — je to nová výchozí složka? Prázdné i neplatné `notes.root`
/// znamená výchozí (stejně jako `lib.rs` → `notes_root`). `None` = config nejde přečíst → nerozhoduj.
fn targets_new_default(cfg_path: &Path, new_default: &Path, var: &dyn Fn(&str) -> Option<String>) -> Option<bool> {
    let text = match std::fs::read_to_string(cfg_path) {
        Ok(t) => t,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Some(true),
        Err(_) => return None,
    };
    let v: Value = serde_json::from_str(text.trim_start_matches('\u{feff}')).ok()?;
    let root = v.pointer("/notes/root").and_then(Value::as_str).unwrap_or("");
    if root.trim().is_empty() {
        return Some(true);
    }
    Some(match crate::notes::resolve_root_with(root, var) {
        Ok(p) => same_path(&p, new_default),
        Err(_) => true,
    })
}

// ---------- I/O ----------

/// Provede migraci nad `p`. `var` = proměnné prostředí, `rename` = přesun složky (v testech podvržený,
/// aby šel nasimulovat jiný disk). Vrací řádky do logu; nikdy nepanikaří.
pub fn run(p: &Paths, var: &dyn Fn(&str) -> Option<String>, rename: &dyn Fn(&Path, &Path) -> io::Result<()>) -> Vec<String> {
    let mut log = Vec::new();
    let new_cfg = p.new_cfg_dir.join("config.json");
    let old_cfg = p.old_cfg_dir.join("config.json");
    if should_copy_config(new_cfg.exists(), old_cfg.is_file()) {
        match copy_config(&old_cfg, &new_cfg, &p.old_notes, var) {
            Ok(msg) => log.push(format!("migrace: config {} → {} ({msg})", old_cfg.display(), new_cfg.display())),
            Err(e) => log.push(format!("migrace: config se nepodařilo zkopírovat: {e}")),
        }
    }
    let Some(target_default) = targets_new_default(&new_cfg, &p.new_notes, var) else {
        log.push(format!("migrace: {} nejde přečíst — poznámky nechávám na místě", new_cfg.display()));
        return log;
    };
    let plan = plan_notes(p.old_notes.is_dir(), p.old_notes.join(MARKER).exists(), has_data(&p.new_notes), target_default);
    match plan {
        NotesPlan::Skip(why) => {
            // tichý běžný případ (nic starého) do logu nepatří
            if p.old_notes.is_dir() && !p.old_notes.join(MARKER).exists() {
                log.push(format!("migrace: poznámky {} nepřesouvám — {why}", p.old_notes.display()));
            }
        }
        NotesPlan::Move => match move_notes(&p.old_notes, &p.new_notes, rename) {
            Ok(how) => {
                log.push(format!("migrace: poznámky {} → {} ({how})", p.old_notes.display(), p.new_notes.display()));
                if let Err(e) = write_marker(&p.old_notes, &p.new_notes) {
                    log.push(format!("migrace: {MARKER} se nepodařilo zapsat: {e}"));
                }
            }
            Err(e) => log.push(format!("migrace: poznámky se nepřesunuly, staré zůstaly v {}: {e}", p.old_notes.display())),
        },
    }
    log
}

fn copy_config(old: &Path, new: &Path, old_notes: &Path, var: &dyn Fn(&str) -> Option<String>) -> Result<&'static str, String> {
    let bytes = std::fs::read(old).map_err(|e| format!("čtení {}: {e}", old.display()))?;
    let (out, msg) = match std::str::from_utf8(&bytes).map_err(|e| e.to_string()).and_then(|t| rewrite_old_root(t, old_notes, var)) {
        Ok(Some(text)) => (text.into_bytes(), "notes.root ze staré výchozí složky → prázdné"),
        Ok(None) => (bytes, "doslova"),
        Err(_) => (bytes, "doslova, není platný JSON"),
    };
    let dir = new.parent().ok_or("config bez složky")?;
    std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    let tmp = new.with_extension("json.migrace");
    std::fs::write(&tmp, &out).map_err(|e| format!("{}: {e}", tmp.display()))?;
    if new.exists() {
        let _ = std::fs::remove_file(&tmp);
        return Err("nový config mezitím vznikl — nechávám ho".into());
    }
    std::fs::rename(&tmp, new).map_err(|e| {
        let _ = std::fs::remove_file(&tmp);
        format!("{}: {e}", new.display())
    })?;
    Ok(msg)
}

/// `ERROR_NOT_SAME_DEVICE` (17) — přejmenování přes hranici disku.
fn crosses_devices(e: &io::Error) -> bool {
    e.raw_os_error() == Some(17) || e.kind() == io::ErrorKind::CrossesDevices
}

/// Existuje a není to prázdná složka (soubor, nečitelná složka i složka s obsahem = data, nesahat).
fn has_data(p: &Path) -> bool {
    match std::fs::symlink_metadata(p) {
        Err(_) => false,
        Ok(m) if m.is_dir() => std::fs::read_dir(p).map(|mut rd| rd.next().is_some()).unwrap_or(true),
        Ok(_) => true,
    }
}

fn move_notes(old: &Path, new: &Path, rename: &dyn Fn(&Path, &Path) -> io::Result<()>) -> Result<String, String> {
    if let Some(parent) = new.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("{}: {e}", parent.display()))?;
    }
    // prázdná nová složka (z dřívějšího startu) uhne; `remove_dir` neprázdnou nikdy nesmaže
    if new.is_dir() {
        std::fs::remove_dir(new).map_err(|e| format!("prázdná {} nejde uvolnit: {e}", new.display()))?;
    }
    match rename(old, new) {
        Ok(()) => Ok("přejmenováním".into()),
        Err(e) if crosses_devices(&e) => copy_verify_delete(old, new),
        Err(e) => Err(e.to_string()),
    }
}

/// Jiný disk: kopie do sourozence `<new>.migrace`, ověření, přejmenování na `new`, smazání starých.
/// Při jakékoli chybě před smazáním zůstanou staré nedotčené a rozpracovaná kopie se uklidí.
fn copy_verify_delete(old: &Path, new: &Path) -> Result<String, String> {
    let name = new.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_else(|| "notes".into());
    let staging = new.with_file_name(format!("{name}.migrace"));
    if staging.exists() {
        // zbytek našeho dřívějšího nedokončeného pokusu — staré poznámky jsou pořád na místě
        std::fs::remove_dir_all(&staging).map_err(|e| format!("úklid {}: {e}", staging.display()))?;
    }
    let fail = |msg: String| {
        let _ = std::fs::remove_dir_all(&staging);
        Err(msg)
    };
    if let Err(e) = copy_tree(old, &staging) {
        return fail(format!("kopie na jiný disk: {e}"));
    }
    match (tally(old), tally(&staging)) {
        (Ok(a), Ok(b)) if a == b => {}
        (Ok(a), Ok(b)) => return fail(format!("kopie nesedí: {} souborů/{} B proti {} souborů/{} B", a.0, a.1, b.0, b.1)),
        (Err(e), _) | (_, Err(e)) => return fail(format!("ověření kopie: {e}")),
    }
    if let Err(e) = std::fs::rename(&staging, new) {
        return fail(format!("{} → {}: {e}", staging.display(), new.display()));
    }
    let (files, _) = tally(new).unwrap_or_default();
    match std::fs::remove_dir_all(old) {
        Ok(()) => Ok(format!("kopií přes disk, {files} souborů ověřeno, staré smazány")),
        Err(e) => Ok(format!("kopií přes disk, {files} souborů ověřeno; starou složku se nepodařilo celou smazat: {e}")),
    }
}

fn copy_tree(from: &Path, to: &Path) -> io::Result<()> {
    std::fs::create_dir(to)?;
    for entry in std::fs::read_dir(from)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        let dst = to.join(entry.file_name());
        if ty.is_dir() {
            copy_tree(&entry.path(), &dst)?;
        } else {
            std::fs::copy(entry.path(), &dst)?;
        }
    }
    Ok(())
}

/// (počet souborů, součet bajtů) rekurzivně.
fn tally(dir: &Path) -> io::Result<(u64, u64)> {
    let mut out = (0, 0);
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        if entry.file_type()?.is_dir() {
            let (f, b) = tally(&entry.path())?;
            out.0 += f;
            out.1 += b;
        } else {
            out.0 += 1;
            out.1 += entry.metadata()?.len();
        }
    }
    Ok(out)
}

fn write_marker(old: &Path, new: &Path) -> io::Result<()> {
    std::fs::create_dir_all(old)?;
    let text = format!(
        "Notch se od 2026-10-17 jmenuje Ledge.\r\nPoznámky z této složky se přesunuly do:\r\n{}\r\n\r\nTenhle soubor zároveň říká Ledge, že migrace proběhla — nemaž ho, dokud tu nechceš nic dalšího.\r\n",
        new.display()
    );
    std::fs::write(old.join(MARKER), text)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    struct Env {
        _tmp: tempfile::TempDir,
        home: PathBuf,
        p: Paths,
    }

    fn env() -> Env {
        let tmp = tempfile::Builder::new().prefix("ledge-migrate-").tempdir_in(std::env::temp_dir()).unwrap();
        let home = tmp.path().join("home");
        let appdata = tmp.path().join("appdata");
        fs::create_dir_all(&home).unwrap();
        fs::create_dir_all(&appdata).unwrap();
        let p = Paths {
            old_cfg_dir: appdata.join("Notch"),
            new_cfg_dir: appdata.join("Ledge"),
            old_notes: home.join("Notch").join("notes"),
            new_notes: home.join("Ledge").join("notes"),
        };
        Env { _tmp: tmp, home, p }
    }

    impl Env {
        fn var(&self) -> impl Fn(&str) -> Option<String> + '_ {
            move |n| (n == "USERPROFILE").then(|| self.home.to_string_lossy().into_owned())
        }
        fn run(&self, rename: &dyn Fn(&Path, &Path) -> io::Result<()>) -> Vec<String> {
            let var = self.var();
            run(&self.p, &var, rename)
        }
        fn put(&self, path: &Path, text: &str) {
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(path, text).unwrap();
        }
        fn old_cfg(&self) -> PathBuf {
            self.p.old_cfg_dir.join("config.json")
        }
        fn new_cfg(&self) -> PathBuf {
            self.p.new_cfg_dir.join("config.json")
        }
        fn seed_notes(&self) {
            self.put(&self.p.old_notes.join("_obecne").join("2026-10-16-140322-a.md"), "---\nproject: _obecne\n---\n\nA\n");
            self.put(&self.p.old_notes.join("atlas").join("2026-10-16-140400-b.md"), "B");
            self.put(&self.p.old_notes.join("atlas").join("2026-10-16-140400-b-1.png"), "png-bytes");
            self.put(&self.p.old_notes.join(".state.json"), "{}");
        }
    }

    fn fs_rename(a: &Path, b: &Path) -> io::Result<()> {
        fs::rename(a, b)
    }
    fn cross_device(_: &Path, _: &Path) -> io::Result<()> {
        Err(io::Error::from_raw_os_error(17))
    }
    fn denied(_: &Path, _: &Path) -> io::Result<()> {
        Err(io::Error::from_raw_os_error(5))
    }

    fn root_of(path: &Path) -> String {
        let v: Value = serde_json::from_str(&fs::read_to_string(path).unwrap()).unwrap();
        v.pointer("/notes/root").and_then(Value::as_str).unwrap_or("<chybí>").to_string()
    }

    #[test]
    fn plan_notes_table() {
        use NotesPlan::*;
        // (stará existuje, stará označená, nová existuje, config míří na novou výchozí) → plán
        #[rustfmt::skip]
        let cases = [
            ("vše připraveno",          true,  false, false, true,  Move),
            ("nic starého",             false, false, false, true,  Skip("stará složka poznámek neexistuje")),
            ("už přesunuto (značka)",   true,  true,  false, true,  Skip("stará složka už je označená jako přesunutá")),
            ("nová má data",            true,  false, true,  true,  Skip("nová složka poznámek už existuje — nepřepisuji ji")),
            ("vlastní kořen v configu", true,  false, false, false, Skip("config ukládá poznámky jinam než do nové výchozí složky")),
        ];
        for (name, old, marked, new, target, want) in cases {
            assert_eq!(plan_notes(old, marked, new, target), want, "{name}");
        }
        assert!(should_copy_config(false, true));
        assert!(!should_copy_config(true, true));
        assert!(!should_copy_config(false, false));
        assert!(!should_copy_config(true, false));
    }

    #[test]
    fn rewrite_old_root_table() {
        let old = Path::new(r"C:\Users\x\Notch\notes");
        let var = |n: &str| (n == "USERPROFILE").then(|| r"C:\Users\x".to_string());
        #[rustfmt::skip]
        let cases: [(&str, &str, Option<&str>); 8] = [
            ("proměnná",                r#"{"notes":{"root":"%USERPROFILE%\\Notch\\notes"}}"#, Some("")),
            ("absolutně, jiná písmena", r#"{"notes":{"root":"c:/users/X/notch/NOTES/"}}"#,    Some("")),
            ("s mezerami okolo",        r#"{"notes":{"root":"  C:\\Users\\x\\Notch\\notes "}}"#, Some("")),
            ("prázdné",                 r#"{"notes":{"root":""}}"#,                            None),
            ("vlastní kořen",           r#"{"notes":{"root":"D:\\Data\\notes"}}"#,             None),
            ("podsložka staré",         r#"{"notes":{"root":"C:\\Users\\x\\Notch\\notes\\x"}}"#, None),
            ("bez sekce notes",         r#"{"autostart":true}"#,                               None),
            ("neznámá proměnná",        r#"{"notes":{"root":"%NIC%\\notes"}}"#,                None),
        ];
        for (name, text, want) in cases {
            let got = rewrite_old_root(text, old, &var).unwrap();
            let root = got.as_deref().map(|t| {
                let v: Value = serde_json::from_str(t).unwrap();
                v.pointer("/notes/root").unwrap().as_str().unwrap().to_string()
            });
            assert_eq!(root.as_deref(), want, "{name}");
        }
        // ostatní klíče přežijí přepis
        let t = rewrite_old_root("\u{feff}{\"autostart\":true,\"notes\":{\"enabled\":false,\"root\":\"%USERPROFILE%\\\\Notch\\\\notes\"}}", old, &var).unwrap().unwrap();
        let v: Value = serde_json::from_str(&t).unwrap();
        assert_eq!(v["autostart"], Value::Bool(true));
        assert_eq!(v["notes"]["enabled"], Value::Bool(false));
        assert!(rewrite_old_root("{nejson", old, &var).is_err());
        assert!(rewrite_old_root("[1]", old, &var).is_err());
    }

    #[test]
    fn nothing_old_does_nothing() {
        let e = env();
        assert!(e.run(&fs_rename).is_empty());
        assert!(!e.p.new_cfg_dir.exists(), "migrace nesmí zakládat nový config");
        assert!(!e.p.new_notes.exists());
    }

    #[test]
    fn copies_config_and_moves_notes() {
        let e = env();
        let old_text = "{\"autostart\":true,\"notes\":{\"root\":\"%USERPROFILE%\\\\Notch\\\\notes\"}}";
        e.put(&e.old_cfg(), old_text);
        e.seed_notes();
        let log = e.run(&fs_rename);
        assert_eq!(log.len(), 2, "{log:?}");
        // config: kopie (starý beze změny), root vyprázdněn
        assert_eq!(fs::read_to_string(e.old_cfg()).unwrap(), old_text);
        assert_eq!(root_of(&e.new_cfg()), "");
        // poznámky: v novém, ve starém jen značka s novou cestou
        assert_eq!(tally(&e.p.new_notes).unwrap().0, 4);
        assert_eq!(fs::read_to_string(e.p.new_notes.join("atlas").join("2026-10-16-140400-b.md")).unwrap(), "B");
        let left: Vec<_> = fs::read_dir(&e.p.old_notes).unwrap().flatten().map(|d| d.file_name()).collect();
        assert_eq!(left, vec![std::ffi::OsString::from(MARKER)]);
        assert!(fs::read_to_string(e.p.old_notes.join(MARKER)).unwrap().contains(&e.p.new_notes.display().to_string()));
        // idempotence: druhý běh nic nedělá ani nehlásí
        assert!(e.run(&fs_rename).is_empty());
        assert_eq!(tally(&e.p.new_notes).unwrap().0, 4);
    }

    #[test]
    fn config_copied_verbatim_when_root_is_custom_or_invalid() {
        for (name, text) in [
            ("vlastní kořen", "{\"notes\":{\"root\":\"D:\\\\Data\\\\notes\"}}"),
            ("rozbitý JSON", "{ tohle není json"),
        ] {
            let e = env();
            e.put(&e.old_cfg(), text);
            e.seed_notes();
            let log = e.run(&fs_rename);
            assert_eq!(fs::read_to_string(e.new_cfg()).unwrap(), text, "{name}");
            // poznámky zůstávají — config je ukládá jinam / nejde přečíst
            assert!(!e.p.new_notes.exists(), "{name}");
            assert_eq!(tally(&e.p.old_notes).unwrap().0, 4, "{name}");
            assert!(!e.p.old_notes.join(MARKER).exists(), "{name}");
            assert!(log.iter().any(|l| l.contains("poznámk")), "{name}: {log:?}");
        }
    }

    #[test]
    fn never_overwrites_new_data() {
        let e = env();
        e.put(&e.old_cfg(), "{\"autostart\":false}");
        e.put(&e.new_cfg(), "{\"autostart\":true}");
        e.seed_notes();
        e.put(&e.p.new_notes.join("_obecne").join("nova.md"), "nová");
        let log = e.run(&fs_rename);
        assert_eq!(fs::read_to_string(e.new_cfg()).unwrap(), "{\"autostart\":true}");
        assert_eq!(tally(&e.p.new_notes).unwrap().0, 1);
        assert_eq!(tally(&e.p.old_notes).unwrap().0, 4, "staré zůstávají");
        assert!(!e.p.old_notes.join(MARKER).exists());
        assert!(log.iter().any(|l| l.contains("nová složka poznámek už existuje")), "{log:?}");
    }

    #[test]
    fn marker_stops_second_move() {
        let e = env();
        e.put(&e.p.old_notes.join(MARKER), "už");
        e.put(&e.p.old_notes.join("_obecne").join("pozdni.md"), "zapsal starý Notch po migraci");
        assert!(e.run(&fs_rename).is_empty());
        assert!(!e.p.new_notes.exists());
    }

    #[test]
    fn cross_device_copies_verifies_and_deletes() {
        let e = env();
        e.seed_notes();
        // zbytek dřívějšího nedokončeného pokusu se uklidí
        e.put(&e.p.new_notes.with_file_name("notes.migrace").join("polovicni.md"), "x");
        let log = e.run(&cross_device);
        assert!(log[0].contains("kopií přes disk, 4 souborů ověřeno, staré smazány"), "{log:?}");
        assert_eq!(tally(&e.p.new_notes).unwrap().0, 4);
        assert!(!e.p.new_notes.join("polovicni.md").exists());
        assert!(!e.p.new_notes.with_file_name("notes.migrace").exists());
        assert_eq!(tally(&e.p.old_notes).unwrap().0, 1, "ve staré jen značka");
        assert!(e.p.old_notes.join(MARKER).exists());
    }

    #[test]
    fn other_rename_error_leaves_old_untouched() {
        let e = env();
        e.seed_notes();
        let log = e.run(&denied);
        assert!(log[0].contains("nepřesunuly"), "{log:?}");
        assert!(!e.p.new_notes.exists());
        assert_eq!(tally(&e.p.old_notes).unwrap().0, 4);
        assert!(!e.p.old_notes.join(MARKER).exists());
        // mezitím vlákno poznámek založí prázdný nový kořen — příští start to přesto zkusí znovu a projde
        fs::create_dir_all(&e.p.new_notes).unwrap();
        let log = e.run(&fs_rename);
        assert!(log[0].contains("přejmenováním"), "{log:?}");
        assert_eq!(tally(&e.p.new_notes).unwrap().0, 4);
    }

    #[test]
    fn notes_move_without_any_config() {
        // čerstvý stroj s poznámkami, ale bez configu → výchozí kořen → přesun
        let e = env();
        e.seed_notes();
        let log = e.run(&fs_rename);
        assert_eq!(log.len(), 1, "{log:?}");
        assert!(!e.p.new_cfg_dir.exists());
        assert_eq!(tally(&e.p.new_notes).unwrap().0, 4);
    }
}
