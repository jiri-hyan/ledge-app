//! Jediné místo, které čte `%USERPROFILE%`, `%APPDATA%`, `%LOCALAPPDATA%` a `CLAUDE_CONFIG_DIR`.
//! V testech (`cfg(test)`) bez přepsání panikuje `appdata`/`localappdata`/`claude_dir` — test nikdy
//! nesmí sáhnout na skutečné `%APPDATA%` ani `~/.claude`. `home()` vrací bez přepsání neexistující
//! falešný profil v tempu (`fake_home`): `Config::default()` ho čtou stovky testů (výchozí kořeny repozitářů)
//! a nikdy do něj nezapisují. Přepsání: `dirs::testing::with(Dirs { .. }, || ..)`, `enter` (stráž) nebo `sandbox()`
//! (vše jen pro vlákno testu; vlákna spuštěná testem přepsání nevidí). Hlídá to test `no_direct_env_reads`.

use std::path::PathBuf;

/// Neprázdná proměnná prostředí jako cesta.
#[cfg(not(test))]
fn var(name: &str) -> Option<PathBuf> {
    std::env::var_os(name).filter(|v| !v.is_empty()).map(PathBuf::from)
}

/// Čtení proměnné; v testech jen přes přepsání (`pick` vybere pole z `Dirs`), jinak panika.
#[cfg(test)]
fn read(name: &str, pick: impl FnOnce(testing::Dirs) -> Option<PathBuf>) -> Option<PathBuf> {
    match testing::current() {
        Some(d) => pick(d),
        None => testing::no_override(name),
    }
}

/// Uživatelův domov (`%USERPROFILE%`).
pub fn home() -> Option<PathBuf> {
    #[cfg(test)]
    return testing::current().map_or_else(|| Some(testing::fake_home()), |d| d.home);
    #[cfg(not(test))]
    var("USERPROFILE")
}

/// `%APPDATA%` (Roaming).
pub fn appdata() -> Option<PathBuf> {
    #[cfg(test)]
    return read("APPDATA", |d| d.appdata);
    #[cfg(not(test))]
    var("APPDATA")
}

/// `%LOCALAPPDATA%`.
pub fn localappdata() -> Option<PathBuf> {
    #[cfg(test)]
    return read("LOCALAPPDATA", |d| d.localappdata);
    #[cfg(not(test))]
    var("LOCALAPPDATA")
}

/// Jednotná záloha, když proměnná chybí: aktuální složka (`.`), jak to dělaly původní volání.
pub fn or_dot(p: Option<PathBuf>) -> PathBuf {
    p.unwrap_or_else(|| PathBuf::from("."))
}

/// `CLAUDE_CONFIG_DIR`, jinak `<home>\.claude`.
pub fn claude_dir() -> Option<PathBuf> {
    #[cfg(test)]
    return read("CLAUDE_CONFIG_DIR", |d| d.claude_dir.or_else(|| d.home.map(|h| h.join(".claude"))));
    #[cfg(not(test))]
    var("CLAUDE_CONFIG_DIR").or_else(|| home().map(|h| h.join(".claude")))
}

#[cfg(test)]
pub mod testing {
    use super::*;
    use std::cell::RefCell;

    /// Přepsané složky pro test; co zůstane `None`, se tváří jako nenastavené.
    #[derive(Clone, Default)]
    pub struct Dirs {
        pub home: Option<PathBuf>,
        pub appdata: Option<PathBuf>,
        pub localappdata: Option<PathBuf>,
        pub claude_dir: Option<PathBuf>,
    }

    thread_local! { static LOCAL: RefCell<Option<Dirs>> = const { RefCell::new(None) }; }

    pub fn current() -> Option<Dirs> {
        LOCAL.with(|l| l.borrow().clone())
    }

    /// Falešný profil pro testy bez přepsání: v tempu, nikdy neexistuje.
    pub fn fake_home() -> PathBuf {
        std::env::temp_dir().join("ledge-test-no-profile")
    }

    pub fn no_override(what: &str) -> ! {
        panic!("dirs: test sáhl na {what} bez přepsání (dirs::testing::with) — testy nesmí číst skutečný profil")
    }

    /// Přepíše složky pro toto vlákno do konce platnosti stráže.
    pub fn enter(d: Dirs) -> impl Drop {
        struct Restore(Option<Dirs>);
        impl Drop for Restore {
            fn drop(&mut self) {
                LOCAL.with(|l| *l.borrow_mut() = self.0.take());
            }
        }
        Restore(LOCAL.with(|l| l.replace(Some(d))))
    }

    /// Spustí `f` s přepsanými složkami jen pro toto vlákno.
    pub fn with<R>(d: Dirs, f: impl FnOnce() -> R) -> R {
        let _r = enter(d);
        f()
    }

    /// Stráž s `appdata`/`localappdata`/`home` v dočasné složce (test se dotkne jen jí).
    pub fn sandbox() -> (tempfile::TempDir, impl Drop) {
        let t = tempfile::tempdir().unwrap();
        let p = t.path().to_path_buf();
        let g = enter(Dirs { home: Some(p.join("home")), appdata: Some(p.join("appdata")), localappdata: Some(p.join("local")), claude_dir: None });
        (t, g)
    }
}

#[cfg(test)]
mod tests {
    use super::testing::{Dirs, with};
    use super::*;
    use std::path::Path;

    #[test]
    fn override_is_used_and_restored() {
        let d = Dirs { home: Some("/h".into()), appdata: Some("/a".into()), localappdata: None, claude_dir: None };
        with(d, || {
            assert_eq!(home().as_deref(), Some(Path::new("/h")));
            assert_eq!(appdata().as_deref(), Some(Path::new("/a")));
            assert_eq!(localappdata(), None);
            assert_eq!(claude_dir(), Some(PathBuf::from("/h").join(".claude")));
            with(Dirs { claude_dir: Some("/c".into()), ..Default::default() }, || {
                assert_eq!(claude_dir().as_deref(), Some(Path::new("/c")));
                assert_eq!(home(), None);
            });
            assert!(home().is_some());
        });
    }

    #[test]
    fn without_override_panics_instead_of_touching_real_profile() {
        assert_eq!(home(), Some(testing::fake_home()));
        for f in [appdata, localappdata, claude_dir] {
            assert!(std::panic::catch_unwind(f).is_err());
        }
    }

    #[test]
    fn or_dot_table() {
        assert_eq!(or_dot(None), PathBuf::from("."));
        assert_eq!(or_dot(Some("/x".into())), PathBuf::from("/x"));
    }

    /// Čtení profilu z prostředí patří jen sem. Výjimky mají důvod: `statusline.rs` běží jako samostatný
    /// wrapper bez konfigurace Ledge, `update.rs` hledá `ProgramFiles` (ne profil).
    #[test]
    fn no_direct_env_reads() {
        const ALLOWED: [&str; 1] = ["dirs.rs"];
        fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
            for e in std::fs::read_dir(dir).unwrap().flatten() {
                let p = e.path();
                if p.is_dir() {
                    walk(&p, out);
                } else if p.extension().is_some_and(|x| x == "rs") {
                    out.push(p);
                }
            }
        }
        let mut files = Vec::new();
        walk(&Path::new(env!("CARGO_MANIFEST_DIR")).join("src"), &mut files);
        let needles = ["USERPROFILE", "APPDATA", "LOCALAPPDATA", "CLAUDE_CONFIG_DIR"];
        let mut bad = Vec::new();
        for f in files {
            if ALLOWED.contains(&f.file_name().unwrap().to_str().unwrap()) {
                continue;
            }
            let text = std::fs::read_to_string(&f).unwrap();
            for (i, line) in text.lines().enumerate() {
                let l = line.trim_start();
                if l.starts_with("//") || !(l.contains("env::var(") || l.contains("env::var_os(")) {
                    continue;
                }
                if needles.iter().any(|n| l.contains(&format!("\"{n}\""))) {
                    bad.push(format!("{}:{}", f.display(), i + 1));
                }
            }
        }
        assert!(bad.is_empty(), "přímé čtení env mimo dirs.rs: {bad:?}");
    }
}
