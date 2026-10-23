//! Vlastní soubory přihrádky: `%LOCALAPPDATA%\Ledge\shelf-drops` (obrázek / text ze schránky, zmenšená kopie).
//! Jen tahle složka, jen soubory přímo v ní; nové jméno se zakládá `create_new` (nikdy nepřepíše cizí ani
//! souběžně vzniklý soubor); celá složka má strop velikosti. Úklid (`sweep`): soubor, na který přihrádka už
//! neodkazuje (odebrání, Vyprázdnit, vypadnutí nad `MAX_ITEMS`), se smaže — při každé změně seznamu a jednou při
//! prvním načtení (zbytky z minula). Čerstvé soubory (< `FRESH_S`) úklid nechá, ať nesmaže právě vkládaný.
//! Uživatelovy soubory (cesty z Průzkumníka) se nikdy nemažou.

use crate::i18n::tr;
use std::io::Write;
use std::path::{Path, PathBuf};

/// Strop celé složky (bajty).
pub const MAX_DIR_BYTES: u64 = 256 * 1024 * 1024;
/// Mladší soubor úklid nesmaže (vkládání ještě nemuselo dojít do seznamu).
pub const FRESH_S: u64 = 60;

pub fn drops_dir() -> PathBuf {
    crate::actionrun::local_dir().join("shelf-drops")
}

fn norm(p: &str) -> String {
    p.replace('/', "\\").trim_end_matches('\\').to_lowercase()
}

/// Kandidáti jména: `stem.ext`, `stem-2.ext`… (stem bez znaků, které Windows nesnese, nejvýš 60 znaků).
pub fn names(stem: &str, ext: &str) -> impl Iterator<Item = String> {
    let clean: String = stem.chars().map(|c| if c.is_control() || "<>:\"/\\|?*".contains(c) { '_' } else { c }).take(60).collect();
    let clean = clean.trim_matches([' ', '.']).to_string();
    let stem = if clean.is_empty() { "clip".to_string() } else { clean };
    let ext = ext.to_string();
    (1..1000).map(move |i| if i == 1 { format!("{stem}.{ext}") } else { format!("{stem}-{i}.{ext}") })
}

/// Vejde se `new` bajtů do složky, kde už je `used`?
pub fn fits(used: u64, new: u64) -> bool {
    used.saturating_add(new) <= MAX_DIR_BYTES
}

fn dir_files(dir: &Path) -> Vec<(PathBuf, u64, u64)> {
    let now = std::time::SystemTime::now();
    let Ok(rd) = std::fs::read_dir(dir) else { return Vec::new() };
    rd.flatten()
        .filter_map(|e| {
            let m = e.metadata().ok().filter(|m| m.is_file())?;
            let age = m.modified().ok().and_then(|t| now.duration_since(t).ok()).map(|d| d.as_secs()).unwrap_or(0);
            Some((e.path(), m.len(), age))
        })
        .collect()
}

/// Co smazat: soubory složky, na které přihrádka neodkazuje a nejsou čerstvé.
pub fn sweep_plan(files: &[(PathBuf, u64, u64)], referenced: &[String]) -> Vec<PathBuf> {
    files
        .iter()
        .filter(|(p, _, age)| *age >= FRESH_S && !referenced.iter().any(|r| norm(r) == norm(&p.to_string_lossy())))
        .map(|(p, _, _)| p.clone())
        .collect()
}

/// Úklid podle seznamu přihrádky (volá `shelf::with` po změně a při prvním načtení).
pub fn sweep(referenced: &[String]) {
    let dir = drops_dir();
    for p in sweep_plan(&dir_files(&dir), referenced) {
        let _ = std::fs::remove_file(p);
    }
}

/// Nový soubor ve `shelf-drops` (`create_new`, volné jméno, strop složky) a odkaz v přihrádce.
pub fn save(stem: &str, ext: &str, bytes: &[u8]) -> Result<PathBuf, String> {
    let dir = drops_dir();
    std::fs::create_dir_all(&dir).map_err(|e| crate::errors::io(tr("Složku přihrádky nejde vytvořit", "Can't create the shelf folder"), &e))?;
    let used: u64 = dir_files(&dir).iter().map(|(_, n, _)| n).sum();
    if !fits(used, bytes.len() as u64) {
        return Err(tr("Přihrádka má plnou složku vložených souborů (256 MB) — odeber z ní něco.", "The shelf's folder of pasted files is full (256 MB) — remove something.").into());
    }
    let ctx = tr("Soubor do přihrádky nejde uložit", "Can't save the file to the shelf");
    for name in names(stem, ext) {
        let path = dir.join(name);
        match std::fs::OpenOptions::new().write(true).create_new(true).open(&path) {
            Ok(mut f) => {
                if let Err(e) = f.write_all(bytes).and_then(|_| f.sync_all()) {
                    drop(f);
                    let _ = std::fs::remove_file(&path);
                    return Err(crate::errors::io(ctx, &e));
                }
                super::add_paths(&[path.to_string_lossy().into_owned()])?;
                return Ok(path);
            }
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(e) => return Err(crate::errors::io(ctx, &e)),
        }
    }
    Err(ctx.into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_and_cap() {
        let n: Vec<String> = names("clip", "png").take(3).collect();
        assert_eq!(n, ["clip.png", "clip-2.png", "clip-3.png"]);
        assert_eq!(names("a:b/c\\d?", "txt").next().unwrap(), "a_b_c_d_.txt", "znaky, které Windows nesnese");
        assert_eq!(names(" .. ", "txt").next().unwrap(), "clip.txt");
        assert_eq!(names(&"x".repeat(200), "png").next().unwrap().len(), 64);
        assert!(fits(0, MAX_DIR_BYTES) && !fits(MAX_DIR_BYTES, 1) && !fits(u64::MAX, 1));
    }

    #[test]
    fn sweep_deletes_unreferenced_old_files() {
        let f = |p: &str, age| (PathBuf::from(p), 10, age);
        let files = [f("D:/drops/a.png", 3600), f("D:/drops/b.txt", 3600), f("D:/drops/c.png", 5)];
        let plan = sweep_plan(&files, &["D:\\drops\\a.png".to_string()]);
        assert_eq!(plan, vec![PathBuf::from("D:/drops/b.txt")], "odkazovaný a čerstvý zůstanou");
        assert_eq!(sweep_plan(&files, &[]).len(), 2);
    }

    #[test]
    fn save_never_overwrites_existing() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("clip.png"), b"cizi").unwrap();
        // jen create_new nad dočasnou složkou (bez přidání do přihrádky)
        let first = names("clip", "png").map(|n| dir.path().join(n)).find(|p| std::fs::OpenOptions::new().write(true).create_new(true).open(p).is_ok()).unwrap();
        assert_eq!(first.file_name().unwrap(), "clip-2.png");
        assert_eq!(std::fs::read(dir.path().join("clip.png")).unwrap(), b"cizi");
    }
}
