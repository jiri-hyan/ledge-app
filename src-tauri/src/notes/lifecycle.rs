//! Životní cyklus poznámky mimo úpravy textu: **archiv** (přesun do podsložky `_archiv` vedle poznámky)
//! a **smazání do Koše Windows** přes odklad `<root>\.smazane\<id>\` (Zpět = vrátit, po lhůtě = do Koše).
//!
//! - Archiv: `<složka>\x.md` ↔ `<složka>\_archiv\x.md`; poznámka v kořeni jde do `_obecne\_archiv`. Obrázky,
//!   které poznámka používá jen sama, jdou s ní; sdílený obrázek (odkazuje na něj i jiná poznámka ve složce)
//!   se kopíruje. Relativní odkazy `./x-1.png` tak dál sedí. Starší Ledge `_archiv` nevidí (`scan` čte jednu
//!   úroveň složek) — archivovaná poznámka mu prostě zmizí.
//! - Smazání: soubory se přesunou do `.smazane\<id>\<cesta od kořene>`, `commit` je vrátí na původní místo
//!   a pošle do Koše (v Koši tak mají původní umístění a jdou obnovit); selže-li Koš, poznámka je zpět
//!   na svém místě — nikdy se neztratí. `sweep` dodělá zbytky po pádu při startu.
//! - Čisté jádro (odkazy, výběr vlastních obrázků, plán přesunu) je odděleno od I/O a testované tabulkami;
//!   Koš je vložená funkce (`recycle.rs`), testy na skutečný Koš nesahají.

use super::*;
use std::collections::HashSet;
use std::ffi::OsStr;
use crate::fsx::rename_retry;

/// Podsložka archivu vedle poznámek.
pub const ARCHIVE_DIR: &str = "_archiv";
/// Odklad smazaných (v kořeni, mimo sync).
pub const TRASH_DIR: &str = ".smazane";
/// Kolik z archivované poznámky se čte pro řádek seznamu (hlavička + začátek těla).
const ARCHIVE_HEAD_BYTES: usize = 8 * 1024;

// ---------- čisté ----------

/// Odkaz na obrázek → relativní cesta se `/` bez úvodního `./` (`None` = odkaz nejde přesunout).
pub fn rel_of(src: &str) -> Option<String> {
    let mut s = src.trim().replace('\\', "/");
    while let Some(r) = s.strip_prefix("./") {
        s = r.to_string();
    }
    (!s.is_empty() && !s.starts_with('/') && md::relative_image(&s)).then_some(s)
}

/// Relativní cesty obrázků z těla poznámky (bez duplicit, v pořadí odkazů).
pub fn refs_of(body: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for r in md::image_refs(body).into_iter().filter_map(rel_of) {
        if !out.contains(&r) {
            out.push(r);
        }
    }
    out
}

/// Windows nerozlišuje velikost písmen — porovnání odkazů tedy také ne.
fn key(rel: &str) -> String {
    rel.to_lowercase()
}

/// Rozdělí existující obrázky poznámky na vlastní (jen ona je používá) a sdílené (používá je i jiná).
pub fn split_images(refs: &[String], shared: &HashSet<String>, exists: &dyn Fn(&str) -> bool) -> (Vec<String>, Vec<String>) {
    let (mut own, mut sh) = (Vec::new(), Vec::new());
    for r in refs.iter().filter(|r| exists(r)) {
        if shared.contains(&key(r)) { sh.push(r.clone()) } else { own.push(r.clone()) }
    }
    (own, sh)
}

#[derive(Debug, Clone, PartialEq)]
pub struct Mv {
    pub from: PathBuf,
    pub to: PathBuf,
}

/// Co se má s disky stát: `moves` (soubor poznámky je poslední), `copies` (sdílené obrázky).
#[derive(Debug, Default, PartialEq)]
pub struct Plan {
    pub moves: Vec<Mv>,
    pub copies: Vec<Mv>,
}

/// Plán přesunu poznámky `name` ze složky `from` do `to`: vlastní obrázky přesun, sdílené kopie,
/// samotná poznámka až nakonec (kdyby cokoli selhalo, poznámka s nerozbitými odkazy zůstane).
pub fn plan_move(name: &OsStr, from: &Path, to: &Path, own: &[String], shared: &[String]) -> Plan {
    let mut moves: Vec<Mv> = own.iter().map(|r| Mv { from: from.join(r), to: to.join(r) }).collect();
    moves.push(Mv { from: from.join(name), to: to.join(name) });
    Plan { moves, copies: shared.iter().map(|r| Mv { from: from.join(r), to: to.join(r) }).collect() }
}

/// Složka archivu pro poznámku ve složce `dir`: vedle ní; poznámka přímo v kořeni jde do `_obecne`.
pub fn archive_dir(root: &Path, dir: &Path) -> PathBuf {
    if dir == root { root.join(GENERAL).join(ARCHIVE_DIR) } else { dir.join(ARCHIVE_DIR) }
}

pub fn is_archived(path: &Path) -> bool {
    path.parent().and_then(Path::file_name).is_some_and(|n| n.eq_ignore_ascii_case(ARCHIVE_DIR))
}

/// Id odkladu smazání: jen `[0-9a-z-]` (jde do cesty).
pub fn valid_id(id: &str) -> Result<(), String> {
    if (1..=40).contains(&id.len()) && id.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-') {
        Ok(())
    } else {
        Err(tr("Neplatné smazání.", "Invalid deletion.").into())
    }
}

// ---------- I/O: přesun s vrácením ----------

fn is_md(p: &Path) -> bool {
    p.extension().is_some_and(|x| x.eq_ignore_ascii_case("md"))
}

/// Provede plán; selže-li cokoli, vrátí zpět, co už stihl (poznámka nezůstane napůl).
/// Cíl, který už existuje, plán předem zamítne (nic se nepřepíše); existující kopie sdíleného obrázku se přeskočí.
pub fn apply(plan: &Plan) -> Result<(), String> {
    if let Some(m) = plan.moves.iter().find(|m| m.to.exists()) {
        return Err(trf!("Na cílovém místě už je soubor „{}“.", "A file “{}” already exists at the destination.", m.to.file_name().unwrap_or_default().to_string_lossy()));
    }
    let io = |e: std::io::Error| crate::errors::io(tr("Poznámku nejde přesunout", "Can't move the note"), &e);
    let mut copied: Vec<&Path> = Vec::new();
    let mut moved: Vec<&Mv> = Vec::new();
    let res = (|| -> Result<(), String> {
        for m in plan.copies.iter().filter(|m| !m.to.exists()) {
            if let Some(d) = m.to.parent() {
                std::fs::create_dir_all(d).map_err(io)?;
            }
            std::fs::copy(&m.from, &m.to).map_err(io)?;
            copied.push(&m.to);
        }
        for m in &plan.moves {
            if let Some(d) = m.to.parent() {
                std::fs::create_dir_all(d).map_err(io)?;
            }
            rename_retry(&m.from, &m.to).map_err(io)?;
            moved.push(m);
        }
        Ok(())
    })();
    if res.is_err() {
        for m in moved.iter().rev() {
            let _ = rename_retry(&m.to, &m.from);
        }
        for c in copied {
            let _ = std::fs::remove_file(c);
        }
    }
    res
}

/// Odkazy z těla poznámky `path` a množina odkazů, které používají jiné poznámky téže složky.
fn refs_and_shared(path: &Path) -> Result<(Vec<String>, HashSet<String>), String> {
    let dir = path.parent().ok_or_else(|| tr("Poznámka bez složky.", "Note without a folder."))?;
    let bytes = std::fs::read(path).map_err(|e| crate::errors::io(tr("Poznámku nejde přečíst", "Can't read the note"), &e))?;
    let refs = refs_of(split(&String::from_utf8_lossy(&bytes)).1);
    let mut shared = HashSet::new();
    if let Ok(rd) = std::fs::read_dir(dir) {
        for e in rd.flatten() {
            let p = e.path();
            if !is_md(&p) || p.file_name() == path.file_name() || !e.metadata().is_ok_and(|m| m.is_file() && m.len() <= MAX_NOTE_BYTES) {
                continue;
            }
            if let Ok(b) = std::fs::read(&p) {
                shared.extend(refs_of(split(&String::from_utf8_lossy(&b)).1).iter().map(|r| key(r)));
            }
        }
    }
    Ok((refs, shared))
}

/// Dva soubory se stejným obsahem (oba existují).
fn same_file(a: &Path, b: &Path) -> bool {
    let (Ok(ma), Ok(mb)) = (std::fs::metadata(a), std::fs::metadata(b)) else { return false };
    ma.is_file() && mb.is_file() && ma.len() == mb.len() && std::fs::read(a).ok() == std::fs::read(b).ok()
}

fn relocate(path: &Path, to_dir: &Path) -> Result<PathBuf, String> {
    let from_dir = path.parent().ok_or_else(|| tr("Poznámka bez složky.", "Note without a folder."))?;
    let name = path.file_name().ok_or_else(|| tr("Poznámka bez jména.", "Note without a name."))?;
    let (refs, shared) = refs_and_shared(path)?;
    let (own, sh) = split_images(&refs, &shared, &|r| from_dir.join(r).is_file());
    // obrázek, který v cíli už leží stejný (kopie sdíleného z dřívějšího archivování), se nepřesouvá — zdrojová kopie jen zmizí
    let (dups, own): (Vec<String>, Vec<String>) = own.into_iter().partition(|r| same_file(&from_dir.join(r), &to_dir.join(r)));
    apply(&plan_move(name, from_dir, to_dir, &own, &sh))?;
    for d in dups {
        let _ = std::fs::remove_file(from_dir.join(d));
    }
    Ok(to_dir.join(name))
}

// ---------- archiv ----------

/// Přesune poznámku do `_archiv` (i s vlastními obrázky). Vrací novou cestu.
pub fn archive(root: &Path, path: &Path) -> Result<PathBuf, String> {
    let real = check_note_path(root, path)?;
    let lock = path_lock(&real);
    let _g = lock.lock();
    if is_archived(path) {
        return Err(tr("Poznámka už je v archivu.", "The note is already archived.").into());
    }
    let dir = path.parent().ok_or_else(|| tr("Poznámka bez složky.", "Note without a folder."))?;
    let at_root = real.parent() == root.canonicalize().ok().as_deref();
    relocate(path, &archive_dir(root, if at_root { root } else { dir }))
}

/// Vrátí poznámku z `_archiv` zpátky do její složky. Vrací novou cestu.
pub fn unarchive(root: &Path, path: &Path) -> Result<PathBuf, String> {
    let real = check_note_path(root, path)?;
    let lock = path_lock(&real);
    let _g = lock.lock();
    if !is_archived(path) {
        return Err(tr("Poznámka není v archivu.", "The note isn't archived.").into());
    }
    let back = path.parent().and_then(Path::parent).ok_or_else(|| tr("Poznámka bez složky.", "Note without a folder."))?;
    relocate(path, back)
}

/// Archivované poznámky (nejnovější první): `<složka>\_archiv\*.md` pro každou složku úrovně kořene.
/// Čte se jen začátek souboru (seznam potřebuje hlavičku a titulek) — archiv může být velký.
pub fn scan_archive(root: &Path) -> Vec<NoteView> {
    use std::io::Read;
    let mut out = Vec::new();
    let Ok(rd) = std::fs::read_dir(root) else { return out };
    for d in rd.flatten().filter(|e| e.file_type().is_ok_and(|t| t.is_dir())) {
        let folder = d.file_name().to_string_lossy().into_owned();
        if folder.starts_with('.') {
            continue;
        }
        let Ok(files) = std::fs::read_dir(d.path().join(ARCHIVE_DIR)) else { continue };
        for e in files.flatten() {
            let p = e.path();
            let Ok(meta) = e.metadata() else { continue };
            if !is_md(&p) || !meta.is_file() || meta.len() > MAX_NOTE_BYTES {
                continue;
            }
            let mut head = Vec::new();
            if std::fs::File::open(&p).and_then(|f| f.take(ARCHIVE_HEAD_BYTES as u64).read_to_end(&mut head)).is_err() {
                continue;
            }
            let mtime = meta.modified().ok().and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok()).map_or(0, |d| d.as_millis() as i64);
            out.push(parse_note(&p, &String::from_utf8_lossy(&head), Some(&folder), mtime));
        }
    }
    out.sort_by(|a, b| b.created_ms.cmp(&a.created_ms).then_with(|| b.path.cmp(&a.path)));
    out
}

// ---------- smazání do Koše přes odklad ----------

fn parked_dir(root: &Path, id: &str) -> PathBuf {
    root.join(TRASH_DIR).join(id)
}

/// Všechny soubory pod `dir` (rekurzivně, odklad je malý).
fn files_under(dir: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let Ok(rd) = std::fs::read_dir(dir) else { return out };
    for e in rd.flatten() {
        let p = e.path();
        if e.file_type().is_ok_and(|t| t.is_dir()) { out.extend(files_under(&p)) } else { out.push(p) }
    }
    out
}

/// Smaže prázdné složky odspodu (`dir` včetně).
fn prune(dir: &Path) {
    if let Ok(rd) = std::fs::read_dir(dir) {
        for e in rd.flatten().filter(|e| e.file_type().is_ok_and(|t| t.is_dir())) {
            prune(&e.path());
        }
    }
    let _ = std::fs::remove_dir(dir);
}

/// Přesune poznámku a její vlastní obrázky do odkladu `.smazane\<id>\`. Vrací, kolik souborů to bylo.
pub fn park(root: &Path, path: &Path, id: &str) -> Result<usize, String> {
    valid_id(id)?;
    let real = check_note_path(root, path)?;
    let lock = path_lock(&real);
    let _g = lock.lock();
    let dir = path.parent().ok_or_else(|| tr("Poznámka bez složky.", "Note without a folder."))?;
    let (refs, shared) = refs_and_shared(path)?;
    let (own, _) = split_images(&refs, &shared, &|r| dir.join(r).is_file());
    let mut files: Vec<PathBuf> = own.iter().map(|r| dir.join(r)).collect();
    files.push(path.to_path_buf());
    let to = parked_dir(root, id);
    let moves = files
        .iter()
        .map(|f| f.strip_prefix(root).map(|rel| Mv { from: f.clone(), to: to.join(rel) }).map_err(|_| trf!("Cesta {} není poznámka Ledge.", "Path {} isn't a Ledge note.", f.display())))
        .collect::<Result<Vec<_>, _>>()?;
    apply(&Plan { moves, copies: vec![] })?;
    Ok(files.len())
}

/// Odklad zpět na původní místa (Zpět). Je-li některé místo mezitím obsazené, nic se nepřesune.
/// Vrací cesty obnovených poznámek (`.md`).
pub fn unpark(root: &Path, id: &str) -> Result<Vec<PathBuf>, String> {
    valid_id(id)?;
    let from = parked_dir(root, id);
    let files = files_under(&from);
    if files.is_empty() {
        return Err(tr("Smazání už je dokončené — poznámka je v Koši.", "The deletion is already final — the note is in the Recycle Bin.").into());
    }
    let moves: Vec<Mv> = files.iter().filter_map(|f| f.strip_prefix(&from).ok().map(|rel| Mv { from: f.clone(), to: root.join(rel) })).collect();
    apply(&Plan { moves: moves.clone(), copies: vec![] })?;
    prune(&from);
    Ok(moves.into_iter().map(|m| m.to).filter(|p| is_md(p)).collect())
}

/// Odklad do Koše: soubory se vrátí na původní místa (Koš si tak pamatuje, kam patří, a jde obnovit)
/// a hned se pošlou do Koše. Obsazené místo → soubor jde do Koše z odkladu. Selže-li Koš, soubory zůstanou
/// na původním místě (poznámka se neztratí, jen se zase ukáže) a vrátí se chyba.
pub fn commit(root: &Path, id: &str, recycle: &dyn Fn(&[PathBuf]) -> Result<(), String>) -> Result<usize, String> {
    valid_id(id)?;
    let from = parked_dir(root, id);
    let mut out = Vec::new();
    for f in files_under(&from) {
        let Ok(rel) = f.strip_prefix(&from) else { continue };
        let orig = root.join(rel);
        let back = !orig.exists() && orig.parent().is_none_or(|d| std::fs::create_dir_all(d).is_ok()) && rename_retry(&f, &orig).is_ok();
        out.push(if back { orig } else { f });
    }
    let res = if out.is_empty() { Ok(()) } else { recycle(&out) };
    prune(&from);
    let _ = std::fs::remove_dir(root.join(TRASH_DIR)); // jen když je prázdná
    res.map(|()| out.len())
}

/// Zbytky odkladu po pádu / ukončení Ledge: každý odklad (kromě `skip`) do Koše. Vrací, kolik se jich vyřídilo.
pub fn sweep(root: &Path, skip: &dyn Fn(&str) -> bool, recycle: &dyn Fn(&[PathBuf]) -> Result<(), String>) -> usize {
    let Ok(rd) = std::fs::read_dir(root.join(TRASH_DIR)) else { return 0 };
    let ids: Vec<String> = rd.flatten().filter(|e| e.file_type().is_ok_and(|t| t.is_dir())).map(|e| e.file_name().to_string_lossy().into_owned()).collect();
    ids.iter().filter(|id| !skip(id) && commit(root, id, recycle).is_ok()).count()
}

#[cfg(test)]
mod tests {
    use super::super::testkit::{png, tmp_root};
    use super::*;
    use std::cell::RefCell;

    fn s(v: &[&str]) -> Vec<String> {
        v.iter().map(|x| x.to_string()).collect()
    }

    #[test]
    fn refs_are_relative_normalized_and_unique() {
        assert_eq!(rel_of("./a-1.png").as_deref(), Some("a-1.png"));
        assert_eq!(rel_of(".\\img\\a.png").as_deref(), Some("img/a.png"));
        for bad in ["", "../a.png", "C:/a.png", "/a.png", "https://x.example.com/a.png", "a/../b.png"] {
            assert_eq!(rel_of(bad), None, "{bad}");
        }
        let body = "x ![](./a-1.png) y ![z](a-1.png) ![](./b.png) ![](../ven.png) ![](https://example.com/c.png)";
        assert_eq!(refs_of(body), s(&["a-1.png", "b.png"]));
    }

    #[test]
    fn own_and_shared_images_table() {
        let shared: HashSet<String> = ["b.png", "c.png"].map(String::from).into();
        let exists = |r: &str| r != "chybi.png";
        // vlastní, sdílený (bez ohledu na velikost písmen), chybějící soubor se neřeší
        let (own, sh) = split_images(&s(&["a.png", "B.PNG", "chybi.png", "d.png"]), &shared, &exists);
        assert_eq!((own, sh), (s(&["a.png", "d.png"]), s(&["B.PNG"])));
        let plan = plan_move(OsStr::new("x.md"), Path::new("D"), Path::new("D/_archiv"), &s(&["a.png"]), &s(&["b.png"]));
        let mv = |a: &str, b: &str| Mv { from: PathBuf::from(a), to: PathBuf::from(b) };
        assert_eq!(plan.moves, vec![mv("D/a.png", "D/_archiv/a.png"), mv("D/x.md", "D/_archiv/x.md")], "poznámka je poslední");
        assert_eq!(plan.copies, vec![mv("D/b.png", "D/_archiv/b.png")]);
    }

    #[test]
    fn archive_dir_and_ids() {
        let root = Path::new("R");
        assert_eq!(archive_dir(root, Path::new("R")), Path::new("R/_obecne/_archiv"));
        assert_eq!(archive_dir(root, Path::new("R/app")), Path::new("R/app/_archiv"));
        assert!(is_archived(Path::new("R/app/_archiv/x.md")) && !is_archived(Path::new("R/app/x.md")));
        for ok in ["1700000000000-1", "a-b-2"] {
            assert!(valid_id(ok).is_ok(), "{ok}");
        }
        for bad in ["", "..", "a/b", "a\\b", "A", &"x".repeat(41)] {
            assert!(valid_id(bad).is_err(), "{bad}");
        }
    }

    /// Složka projektu `app` s poznámkou `a.md` (obrázky `a-1.png` vlastní, `sdileny.png` i v `b.md`).
    fn fixture(root: &Path) -> (PathBuf, PathBuf) {
        let dir = root.join("app");
        std::fs::create_dir_all(&dir).unwrap();
        let a = dir.join("a.md");
        std::fs::write(&a, "---\nproject: app\ncreated: 2026-10-18T10:00:00+02:00\ndone: false\n---\n\nText\n\n![](./a-1.png)\n\n![](./sdileny.png)\n").unwrap();
        std::fs::write(dir.join("b.md"), "---\ncreated: 2026-10-18T11:00:00+02:00\n---\n\nDruhá ![](./sdileny.png)\n").unwrap();
        std::fs::write(dir.join("a-1.png"), png(0)).unwrap();
        std::fs::write(dir.join("sdileny.png"), png(1)).unwrap();
        (dir, a)
    }

    #[test]
    fn archive_moves_own_images_copies_shared_and_restores() {
        let outer = tmp_root();
        let root = outer.path().join("notes");
        let (dir, a) = fixture(&root);
        let before = std::fs::read(&a).unwrap();

        let moved = archive(&root, &a).unwrap();
        assert_eq!(moved, dir.join("_archiv").join("a.md"));
        assert!(!a.exists() && !dir.join("a-1.png").exists(), "vlastní obrázek odešel s poznámkou");
        assert!(dir.join("_archiv/a-1.png").is_file() && dir.join("_archiv/sdileny.png").is_file());
        assert!(dir.join("sdileny.png").is_file(), "sdílený zůstal i pro druhou poznámku");
        assert_eq!(std::fs::read(&moved).unwrap(), before, "poznámka se nezměnila ani o bajt");
        // sken: aktivní poznámky ji nevidí, archiv ano
        assert_eq!(scan(&root).iter().map(|n| n.path.clone()).collect::<Vec<_>>(), vec![dir.join("b.md").to_string_lossy().into_owned()]);
        let arch = scan_archive(&root);
        assert_eq!((arch.len(), arch[0].project.as_deref(), arch[0].images), (1, Some("app"), 2));
        // editor ji přečte i s obrázky (odkazy dál sedí)
        assert_eq!(edit::read(&root, &moved).unwrap().image_data.len(), 2);
        assert!(archive(&root, &moved).is_err(), "podruhé ne");

        let back = unarchive(&root, &moved).unwrap();
        assert_eq!(back, a);
        assert_eq!(std::fs::read(&a).unwrap(), before);
        assert!(dir.join("a-1.png").is_file() && dir.join("sdileny.png").is_file());
        assert!(scan_archive(&root).is_empty());
        assert!(unarchive(&root, &a).is_err(), "není v archivu");
    }

    #[test]
    fn archive_of_root_note_goes_to_general_and_collision_changes_nothing() {
        let outer = tmp_root();
        let root = outer.path().join("notes");
        std::fs::create_dir_all(&root).unwrap();
        let n = root.join("r.md");
        std::fs::write(&n, "---\ndone: false\n---\nx\n").unwrap();
        let moved = archive(&root, &n).unwrap();
        assert_eq!(moved, root.join("_obecne/_archiv/r.md"));

        // kolize: v cíli už je soubor se stejným jménem → chyba a nic se nehne
        let (dir, a) = fixture(&root);
        std::fs::create_dir_all(dir.join("_archiv")).unwrap();
        std::fs::write(dir.join("_archiv/a.md"), "jiná").unwrap();
        let e = archive(&root, &a).unwrap_err();
        assert!(e.contains("a.md"), "{e}");
        assert!(a.is_file() && dir.join("a-1.png").is_file() && !dir.join("_archiv/a-1.png").exists());
    }

    #[test]
    fn paths_outside_the_root_are_refused() {
        let outer = tmp_root();
        let root = outer.path().join("notes");
        std::fs::create_dir_all(&root).unwrap();
        let victim = outer.path().join("cizi.md");
        std::fs::write(&victim, "x").unwrap();
        for bad in [victim.clone(), root.join("..").join("cizi.md")] {
            assert!(archive(&root, &bad).is_err() && park(&root, &bad, "1-1").is_err());
        }
        assert!(victim.is_file());
        assert!(park(&root, &root.join("x.md"), "../x").is_err(), "id jde do cesty");
    }

    #[test]
    fn delete_goes_through_parking_undo_and_recycle() {
        let outer = tmp_root();
        let root = outer.path().join("notes");
        let (dir, a) = fixture(&root);
        let before = std::fs::read(&a).unwrap();

        assert_eq!(park(&root, &a, "100-1").unwrap(), 2, "poznámka + vlastní obrázek, sdílený zůstává");
        assert!(!a.exists() && !dir.join("a-1.png").exists() && dir.join("sdileny.png").is_file());
        assert!(root.join(".smazane/100-1/app/a.md").is_file());
        assert!(scan(&root).iter().all(|n| !n.path.ends_with("a.md")), "odklad se neskenuje");

        // Zpět
        assert_eq!(unpark(&root, "100-1").unwrap(), vec![a.clone()]);
        assert_eq!(std::fs::read(&a).unwrap(), before);
        assert!(dir.join("a-1.png").is_file() && !root.join(".smazane/100-1").exists());
        assert!(unpark(&root, "100-1").is_err(), "podruhé už není co vracet");

        // lhůta vypršela → původní místa a Koš (falešný Koš jen zapíše cesty a soubory smaže)
        park(&root, &a, "100-2").unwrap();
        let seen = RefCell::new(Vec::new());
        let fake = |paths: &[PathBuf]| -> Result<(), String> {
            for p in paths {
                assert!(p.is_file(), "do Koše jde soubor na svém původním místě: {}", p.display());
                std::fs::remove_file(p).unwrap();
            }
            seen.borrow_mut().extend(paths.iter().cloned());
            Ok(())
        };
        assert_eq!(commit(&root, "100-2", &fake).unwrap(), 2);
        let mut got = seen.borrow().clone();
        got.sort();
        let mut want = vec![a.clone(), dir.join("a-1.png")];
        want.sort();
        assert_eq!(got, want);
        assert!(!a.exists() && !root.join(".smazane").exists(), "po sobě uklidí");
        assert!(dir.join("b.md").is_file() && dir.join("sdileny.png").is_file(), "cizí poznámka je netknutá");
    }

    #[test]
    fn failed_recycle_leaves_the_note_in_place_and_sweep_finishes_leftovers() {
        let outer = tmp_root();
        let root = outer.path().join("notes");
        let (dir, a) = fixture(&root);
        park(&root, &a, "7-1").unwrap();
        let broken = |_: &[PathBuf]| -> Result<(), String> { Err("Koš není k dispozici".into()) };
        assert!(commit(&root, "7-1", &broken).is_err());
        assert!(a.is_file() && dir.join("a-1.png").is_file(), "poznámka se nikdy neztratí");

        // zbytky po pádu: dva odklady, jeden zrovna běží (skip)
        park(&root, &a, "8-1").unwrap();
        let b = dir.join("b.md");
        park(&root, &b, "9-1").unwrap();
        let ok = |paths: &[PathBuf]| -> Result<(), String> {
            paths.iter().for_each(|p| std::fs::remove_file(p).unwrap());
            Ok(())
        };
        assert_eq!(sweep(&root, &|id| id == "9-1", &ok), 1);
        assert!(!a.exists() && root.join(".smazane/9-1/app/b.md").is_file(), "běžící smazání sweep nechá být");
    }
}
