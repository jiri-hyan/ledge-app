//! Propojení se složkou — sken souborů a bezpečné cesty (I/O). Sken vrací jen `.md` / `.markdown`
//! (v Ledge jen `.md`) do velikosti jedné poznámky, bez skrytých složek (`.obsidian`, `.trash`, `.git`),
//! bez `logseq/` a bez symlinků a junctionů (nikdy se nesleduje odkaz ven). Chyba čtení SLOŽKY sken
//! přeruší — částečný výpis by vypadal jako hromadné smazání. Soubor, který nejde přečíst nebo je
//! moc velký, jde do `frozen` (plán se ho nedotkne).

use super::folder_sync_plan::{Snap, rel_ok};
use super::{MAX_NOTE_BYTES, md, split};
use crate::i18n::tr;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::fs::Metadata;
use std::path::{Path, PathBuf};

/// Nejvíc souborů ve složce — víc je spíš omyl (celý disk), než vault.
pub const MAX_FILES: usize = 20_000;
/// Nejhlubší vnoření podsložek.
pub const MAX_DEPTH: usize = 16;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// kořen poznámek Ledge: kořen + jedna úroveň složek, jen `.md`
    Ledge,
    /// vybraná složka: rekurzivně, `.md` i `.markdown`
    Ext,
}

#[derive(Debug, Default)]
pub struct Scan {
    pub files: Vec<Snap>,
    /// cesty (`/`), na které se nesahá: velké, nečitelné
    pub frozen: Vec<String>,
    /// kolik velkých souborů se přeskočilo
    pub too_big: usize,
    /// kolik symlinků / junctionů se ignorovalo
    pub links: usize,
    /// kolik položek má jméno mimo UTF-8 (nejde je pojmenovat v plánu → mazání se v tom běhu nepřenáší)
    pub odd_names: usize,
}

/// Otisk obsahu: prvních 16 bajtů SHA-256 jako hex.
pub fn hash_bytes(b: &[u8]) -> String {
    let d = Sha256::digest(b);
    d[..16].iter().map(|x| format!("{x:02x}")).collect()
}

/// Titulek poznámky z celého textu souboru (frontmatter se přeskočí).
pub fn title_of(bytes: &[u8]) -> String {
    let text = String::from_utf8_lossy(bytes);
    md::title(split(&text).1)
}

/// Přípona poznámky: Ledge čte jen `md`, složka i `markdown`; skryté soubory (`.x.md`) ne.
pub fn is_note_name(name: &str, kind: Kind) -> bool {
    if name.starts_with('.') {
        return false;
    }
    let ext = Path::new(name).extension().map(|e| e.to_string_lossy().to_ascii_lowercase());
    matches!((kind, ext.as_deref()), (_, Some("md")) | (Kind::Ext, Some("markdown")))
}

/// Složky, které se nečtou: skryté (`.obsidian`, `.trash`, `.git`), `logseq/` (interní data grafu),
/// `node_modules`.
pub fn skip_dir(name: &str) -> bool {
    name.starts_with('.') || name.eq_ignore_ascii_case("logseq") || name.eq_ignore_ascii_case("node_modules")
}

/// Značky reparse pointů (`winnt.h`), které jsou odkaz: symbolický odkaz a junction (mount point).
const IO_REPARSE_TAG_SYMLINK: u32 = 0xA000_000C;
const IO_REPARSE_TAG_MOUNT_POINT: u32 = 0xA000_0003;
const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x400;

/// Je reparse point s touto značkou odkaz, který se nesleduje? Jen symlink a junction. Cloudoví zástupci
/// (OneDrive `IO_REPARSE_TAG_CLOUD*`, 0x9000_001A a spol.), deduplikace a podobné jsou běžné soubory a složky.
/// Čistá funkce.
pub fn is_link_tag(tag: u32) -> bool {
    matches!(tag, IO_REPARSE_TAG_SYMLINK | IO_REPARSE_TAG_MOUNT_POINT)
}

/// Značka reparse pointu (`dwReserved0` z `FindFirstFileW`); `None` = nejde zjistit.
#[cfg(windows)]
fn reparse_tag(path: &Path) -> Option<u32> {
    use std::os::windows::ffi::OsStrExt;
    use windows::Win32::Storage::FileSystem::{FindClose, FindFirstFileW, WIN32_FIND_DATAW};
    use windows::core::PCWSTR;
    let wide: Vec<u16> = path.as_os_str().encode_wide().chain(Some(0)).collect();
    let mut data = WIN32_FIND_DATAW::default();
    // SAFETY: `wide` je nulou zakončený řetězec platný po celé volání, `data` je platný výstupní buffer
    let h = unsafe { FindFirstFileW(PCWSTR(wide.as_ptr()), &mut data) }.ok()?;
    // SAFETY: `h` je platný handle právě vrácený `FindFirstFileW`
    let _ = unsafe { FindClose(h) };
    Some(data.dwReserved0)
}

/// Odkaz (symlink / junction) — nikdy se nesleduje. Jiný reparse point (OneDrive zástupce…) je obyčejný
/// soubor / složka; neznámou značku bereme opatrně jako odkaz.
pub fn is_link(path: &Path, m: &Metadata) -> bool {
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        if m.file_type().is_symlink() {
            return true;
        }
        m.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0 && reparse_tag(path).is_none_or(is_link_tag)
    }
    #[cfg(not(windows))]
    {
        let _ = path;
        m.file_type().is_symlink()
    }
}

/// Leží `rel` v zmrazené položce nebo je s ní shodná (odkaz na složku zmrazí i soubory pod ní)? Bez ohledu
/// na velikost písmen. Čistá funkce.
pub fn under_frozen(frozen: &[String], rel: &str) -> bool {
    let r = rel.to_lowercase();
    frozen.iter().any(|f| {
        let f = f.to_lowercase();
        r == f || r.strip_prefix(&f).is_some_and(|rest| rest.starts_with('/'))
    })
}

pub fn mtime_us(m: &Metadata) -> i64 {
    m.modified().ok().and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok()).map(|d| d.as_micros() as i64).unwrap_or(0)
}

/// Dřívější otisky (`rel` malými písmeny → mtime µs, velikost, hash): nezměněný soubor se nehashuje znovu.
pub type Prev = HashMap<String, (i64, u64, String)>;

pub fn scan(base: &Path, kind: Kind, prev: &Prev) -> Result<Scan, String> {
    walk(base, kind, prev, true)
}

/// Kolik poznámek je ve složce (bez čtení obsahu).
pub fn count(base: &Path, kind: Kind) -> Result<usize, String> {
    Ok(walk(base, kind, &Prev::new(), false)?.files.len())
}

fn walk(base: &Path, kind: Kind, prev: &Prev, read: bool) -> Result<Scan, String> {
    let mut out = Scan::default();
    let max_depth = if kind == Kind::Ledge { 1 } else { MAX_DEPTH };
    let mut stack: Vec<(PathBuf, String, usize)> = vec![(base.to_path_buf(), String::new(), 0)];
    while let Some((dir, prefix, depth)) = stack.pop() {
        let rd = std::fs::read_dir(&dir).map_err(|e| crate::errors::io(&trf!("Složku {} nejde přečíst", "Can't read the folder {}", dir.display()), &e))?;
        for entry in rd {
            let entry = entry.map_err(|e| crate::errors::io(&trf!("Složku {} nejde přečíst", "Can't read the folder {}", dir.display()), &e))?;
            let Some(name) = entry.file_name().to_str().map(str::to_string) else {
                // jméno mimo UTF-8 nejde zapsat do plánu — hlásí se, ať se v tomhle běhu nemaže
                out.odd_names += 1;
                continue;
            };
            let rel = if prefix.is_empty() { name.clone() } else { format!("{prefix}/{name}") };
            let Ok(meta) = entry.metadata() else {
                out.frozen.push(rel);
                continue;
            };
            if is_link(&entry.path(), &meta) {
                // dřív spárovaný soubor (nebo složka) teď odkaz: zmrazit, ne „smazáno“
                out.links += 1;
                out.frozen.push(rel);
            } else if meta.is_dir() {
                if depth < max_depth && if kind == Kind::Ledge { !name.starts_with('.') } else { !skip_dir(&name) } {
                    stack.push((entry.path(), rel, depth + 1));
                }
            } else if meta.is_file() && is_note_name(&name, kind) && rel_ok(&rel) {
                file(&entry.path(), rel, &meta, (kind, prev, read), &mut out);
                if out.files.len() > MAX_FILES {
                    return Err(trf!(
                        "Ve složce je víc než {MAX_FILES} poznámek — vyber menší složku (třeba jen vault nebo pages).",
                        "There are more than {MAX_FILES} notes in the folder — pick a smaller folder (just the vault or pages)."
                    ));
                }
            }
        }
    }
    out.files.sort_by(|a, b| a.rel.cmp(&b.rel));
    Ok(out)
}

fn file(path: &Path, rel: String, meta: &Metadata, (kind, prev, read): (Kind, &Prev, bool), out: &mut Scan) {
    if meta.len() > MAX_NOTE_BYTES {
        out.too_big += 1;
        out.frozen.push(rel);
        return;
    }
    let (mtime, size) = (mtime_us(meta), meta.len());
    if !read {
        out.files.push(Snap { rel, size, mtime, hash: String::new(), title: None });
        return;
    }
    if let Some((_, _, h)) = prev.get(&rel.to_lowercase()).filter(|(m, s, _)| *m == mtime && *s == size) {
        out.files.push(Snap { rel, size, mtime, hash: h.clone(), title: None });
        return;
    }
    match std::fs::read(path) {
        Ok(bytes) if bytes.len() as u64 <= MAX_NOTE_BYTES => {
            let title = (kind == Kind::Ledge).then(|| title_of(&bytes));
            out.files.push(Snap { rel, size, mtime, hash: hash_bytes(&bytes), title });
        }
        Ok(_) => {
            out.too_big += 1;
            out.frozen.push(rel);
        }
        Err(_) => out.frozen.push(rel),
    }
}

/// Cesta `base/rel` bez toho, aby vedla mimo `base`: každá existující složka po cestě musí být obyčejná
/// složka (ne odkaz), chybějící se (s `create`) založí. Soubor, který už existuje, nesmí být odkaz.
pub fn safe_path(base: &Path, rel: &str, create: bool) -> Result<PathBuf, String> {
    let bad = || trf!("Cesta „{rel}“ není bezpečná.", "The path “{rel}” isn't safe.");
    if !rel_ok(rel) {
        return Err(bad());
    }
    let parts: Vec<&str> = rel.split('/').collect();
    let mut cur = base.to_path_buf();
    for (i, part) in parts.iter().enumerate() {
        cur.push(part);
        let last = i + 1 == parts.len();
        match std::fs::symlink_metadata(&cur) {
            Ok(m) if is_link(&cur, &m) => return Err(bad()),
            Ok(m) if !last && !m.is_dir() => return Err(bad()),
            Ok(m) if last && m.is_dir() => return Err(bad()),
            Ok(_) => {}
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                if !last && create {
                    std::fs::create_dir(&cur).map_err(|e| crate::errors::io(&trf!("Složku {} nejde založit", "Can't create the folder {}", cur.display()), &e))?;
                }
            }
            Err(e) => return Err(crate::errors::io(&trf!("{} nejde přečíst", "Can't read {}", cur.display()), &e)),
        }
    }
    Ok(cur)
}

/// Přečte soubor pro kopii: obyčejný soubor do limitu a s očekávaným obsahem. `Ok(None)` = mezitím se
/// změnil (nebo zmizel) → krok se přeskočí a vyřeší se příště.
pub fn read_checked(path: &Path, expect: &str) -> Result<Option<Vec<u8>>, String> {
    match std::fs::symlink_metadata(path) {
        Ok(m) if is_link(path, &m) || !m.is_file() || m.len() > MAX_NOTE_BYTES => return Ok(None),
        Ok(_) => {}
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(crate::errors::io(&trf!("{} nejde přečíst", "Can't read {}", path.display()), &e)),
    }
    let bytes = std::fs::read(path).map_err(|e| crate::errors::io(&trf!("{} nejde přečíst", "Can't read {}", path.display()), &e))?;
    Ok((bytes.len() as u64 <= MAX_NOTE_BYTES && hash_bytes(&bytes) == expect).then_some(bytes))
}

/// Text pro prázdnou složku / chybu výběru.
pub fn not_a_folder(p: &Path) -> String {
    trf!("Složka {} neexistuje nebo není dostupná.", "The folder {} doesn't exist or isn't available.", p.display())
}

/// Nejde o složku, kterou by Ledge nedokázal projít.
pub fn too_shallow() -> String {
    tr("Vyber konkrétní složku, ne celý disk.", "Pick a specific folder, not a whole drive.").into()
}
