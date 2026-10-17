//! Náhled změny v kartě schválení (Edit / Write). Sestaví se **jednou při příchodu žádosti**
//! (`Claude::handle_permission`) a uloží k `Pending.preview` — snapshot soubor nikdy nečte.
//!
//! - Edit: diff `old_string` → `new_string`. Když jde aktuální soubor přečíst a `old_string` v něm je,
//!   diff celého souboru před/po nahrazení (čísla řádků + skutečný kontext; `replace_all` = všechny
//!   výskyty). Jinak jen řetězce bez čísel.
//! - Write: soubor existuje → diff proti aktuálnímu obsahu; neexistuje → „nový soubor, N řádků“ + začátek.
//! - Čtení: nejvýš `READ_MAX` bajtů, termín `READ_WAIT` (síťový disk nesmí podržet kartu), NUL nebo
//!   ne-UTF-8 = „nelze zobrazit“. Chyba čtení = bez porovnání, nic se nevymýšlí.
//!
//! Algoritmus diffu: `diff.rs`. Mapa: `docs/codemap/sessions.md` → Schvalování.

use super::diff::{self, Line, MAX_D, MAX_LINES};
use super::paths::absolutize;
use serde::Serialize;
use serde_json::Value;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::time::Duration;

/// Největší soubor (i vstup Clauda), který se porovná.
pub const READ_MAX: u64 = 512 * 1024;
/// Termín čtení aktuálního souboru.
pub const READ_WAIT: Duration = Duration::from_millis(1500);
/// Kolik řádků nového souboru ukázat.
pub const NEW_HEAD: usize = 40;

/// Co karta ukáže.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Shown {
    /// řádkový diff (`lines`)
    Diff,
    /// soubor neexistuje: začátek nového obsahu (`lines` jen přidané, `total` řádků)
    NewFile,
    /// aktuální soubor nejde přečíst (chyba / termín): začátek nového obsahu, bez porovnání
    Unread,
    /// soubor nebo vstup nad `READ_MAX`
    TooBig,
    /// binární / ne-UTF-8
    Binary,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Preview {
    /// jméno souboru (poslední část cesty)
    pub file: String,
    /// celá cesta (titulek)
    pub path: String,
    pub shown: Shown,
    pub added: u32,
    pub removed: u32,
    pub lines: Vec<Line>,
    /// řádků za stropem
    pub cut: u32,
    /// čísla řádků jsou ze skutečného souboru (jinak je karta nezobrazí)
    pub numbered: bool,
    /// Edit s `replace_all`
    pub replace_all: bool,
    /// kolikrát `old_string` v souboru je (jen když se soubor přečetl)
    pub occurrences: Option<u32>,
    /// rozdíl nad `MAX_D` → ukázaný jako nahrazení bloku
    pub rough: bool,
    /// řádků nového obsahu (NewFile / Unread)
    pub total: u32,
}

/// Výsledek čtení aktuálního souboru.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Disk {
    Text(String),
    Missing,
    TooBig,
    Binary,
    Unreadable,
}

/// Přečte soubor s limitem velikosti (synchronně).
pub fn read_capped(path: &Path, max: u64) -> Disk {
    let f = match std::fs::File::open(path) {
        Ok(f) => f,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Disk::Missing,
        Err(_) => return Disk::Unreadable,
    };
    match f.metadata() {
        Ok(m) if m.is_dir() => return Disk::Unreadable,
        Ok(m) if m.len() > max => return Disk::TooBig,
        _ => {}
    }
    let mut buf = Vec::new();
    if f.take(max + 1).read_to_end(&mut buf).is_err() {
        return Disk::Unreadable;
    }
    if buf.len() as u64 > max {
        return Disk::TooBig;
    }
    text_of(buf)
}

fn text_of(buf: Vec<u8>) -> Disk {
    if buf.contains(&0) {
        return Disk::Binary;
    }
    match String::from_utf8(buf) {
        Ok(s) => Disk::Text(s),
        Err(_) => Disk::Binary,
    }
}

/// `read_capped` s termínem: čte vlákno, po `wait` se na něj nečeká (`Unreadable`).
pub fn read_with_deadline(path: PathBuf, max: u64, wait: Duration) -> Disk {
    let (tx, rx) = mpsc::channel();
    let spawned = std::thread::Builder::new().name("preview-read".into()).spawn(move || {
        let _ = tx.send(read_capped(&path, max));
    });
    if spawned.is_err() {
        return Disk::Unreadable;
    }
    rx.recv_timeout(wait).unwrap_or(Disk::Unreadable)
}

/// Náhled pro žádost o `tool` (jen Edit / Write; jinak `None`). `read` dostane absolutní cestu.
pub fn build(tool: &str, input: &Value, cwd: &str, read: impl FnOnce(&str) -> Disk) -> Option<Preview> {
    let s = |k: &str| input.get(k).and_then(Value::as_str);
    let path = absolutize(s("file_path")?, cwd);
    let file = path.rsplit('/').find(|x| !x.is_empty()).unwrap_or(&path).to_string();
    let mut p = Preview {
        file,
        path: path.clone(),
        shown: Shown::Diff,
        added: 0,
        removed: 0,
        lines: vec![],
        cut: 0,
        numbered: false,
        replace_all: false,
        occurrences: None,
        rough: false,
        total: 0,
    };
    match tool {
        "Edit" => {
            let (old, new) = (s("old_string")?, s("new_string")?);
            p.replace_all = input.get("replace_all").and_then(Value::as_bool).unwrap_or(false);
            if let Some(bad) = guard(&[old, new]) {
                p.shown = bad;
                return Some(p);
            }
            if let Disk::Text(cur) = read(&path)
                && let Some((o, n)) = matching(&cur, old, new)
            {
                p.occurrences = Some(cur.matches(o.as_str()).count() as u32);
                let after = if p.replace_all { cur.replace(o.as_str(), &n) } else { cur.replacen(o.as_str(), &n, 1) };
                p.numbered = true;
                fill(&mut p, &cur, &after);
                return Some(p);
            }
            fill(&mut p, old, new);
        }
        "Write" => {
            let content = s("content")?;
            if let Some(bad) = guard(&[content]) {
                p.shown = bad;
                return Some(p);
            }
            match read(&path) {
                Disk::Text(cur) => {
                    p.numbered = true;
                    fill(&mut p, &cur, content);
                }
                Disk::Missing => head(&mut p, Shown::NewFile, content),
                Disk::Unreadable => head(&mut p, Shown::Unread, content),
                Disk::TooBig => p.shown = Shown::TooBig,
                Disk::Binary => p.shown = Shown::Binary,
            }
        }
        _ => return None,
    }
    Some(p)
}

/// Vstup Clauda nad limitem / s NUL → bez diffu.
fn guard(parts: &[&str]) -> Option<Shown> {
    if parts.iter().map(|x| x.len() as u64).sum::<u64>() > READ_MAX {
        Some(Shown::TooBig)
    } else if parts.iter().any(|x| x.contains('\0')) {
        Some(Shown::Binary)
    } else {
        None
    }
}

/// `old_string` v aktuálním souboru; soubor s CRLF a řetězce s LF → zkusí i CRLF podobu.
fn matching(cur: &str, old: &str, new: &str) -> Option<(String, String)> {
    if old.is_empty() {
        return None;
    }
    if cur.contains(old) {
        return Some((old.to_string(), new.to_string()));
    }
    if cur.contains("\r\n") && old.contains('\n') && !old.contains("\r\n") {
        let (o, n) = (old.replace('\n', "\r\n"), new.replace('\n', "\r\n"));
        if cur.contains(o.as_str()) {
            return Some((o, n));
        }
    }
    None
}

fn fill(p: &mut Preview, before: &str, after: &str) {
    let (a, b): (Vec<&str>, Vec<&str>) = (before.lines().collect(), after.lines().collect());
    let (ops, rough) = diff::ops(&a, &b, MAX_D);
    let r = diff::render(&a, &b, &ops, 1, MAX_LINES);
    p.shown = Shown::Diff;
    p.rough = rough;
    (p.added, p.removed, p.cut, p.lines) = (r.added, r.removed, r.cut, r.lines);
}

pub(crate) fn head(p: &mut Preview, shown: Shown, content: &str) {
    let b: Vec<&str> = content.lines().collect();
    let shown_n = b.len().min(NEW_HEAD);
    let ops = vec![diff::Op::Add; shown_n];
    let r = diff::render(&[], &b[..shown_n], &ops, 1, NEW_HEAD);
    p.shown = shown;
    p.numbered = true;
    p.total = b.len() as u32;
    p.added = p.total;
    p.cut = (b.len() - shown_n) as u32;
    p.lines = r.lines;
}

/// Produkce: `build` se čtením přes `read_with_deadline`.
pub fn for_request(tool: &str, input: &Value, cwd: &str) -> Option<Preview> {
    if !matches!(tool, "Edit" | "Write") {
        return None;
    }
    build(tool, input, cwd, |p| read_with_deadline(PathBuf::from(p), READ_MAX, READ_WAIT))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn read(p: &str) -> Disk {
        read_capped(Path::new(p), READ_MAX)
    }
    fn texts(p: &Preview) -> Vec<String> {
        p.lines
            .iter()
            .map(|l| {
                let sign = match l.kind {
                    diff::Kind::Add => "+",
                    diff::Kind::Del => "-",
                    diff::Kind::Ctx => " ",
                    diff::Kind::Gap => "…",
                };
                format!("{sign}{}", l.text)
            })
            .collect()
    }

    #[test]
    fn read_capped_table() {
        let t = tempfile::tempdir().unwrap();
        let w = |n: &str, b: &[u8]| {
            let p = t.path().join(n);
            std::fs::write(&p, b).unwrap();
            p
        };
        assert_eq!(read_capped(&w("a.txt", "ahoj\nsvěte".as_bytes()), READ_MAX), Disk::Text("ahoj\nsvěte".into()));
        assert_eq!(read_capped(&t.path().join("nic.txt"), READ_MAX), Disk::Missing);
        assert_eq!(read_capped(&w("big.txt", &[b'x'; 101]), 100), Disk::TooBig);
        assert_eq!(read_capped(&w("ok.txt", &[b'x'; 100]), 100), Disk::Text("x".repeat(100)));
        assert_eq!(read_capped(&w("nul.bin", b"a\0b"), READ_MAX), Disk::Binary);
        assert_eq!(read_capped(&w("latin.txt", &[0x61, 0xE9, 0x62]), READ_MAX), Disk::Binary);
        assert_eq!(read_capped(t.path(), READ_MAX), Disk::Unreadable, "složka");
        assert_eq!(read_with_deadline(t.path().join("a.txt"), READ_MAX, READ_WAIT), Disk::Text("ahoj\nsvěte".into()));
    }

    #[test]
    fn edit_table() {
        let t = tempfile::tempdir().unwrap();
        let dir = t.path().to_string_lossy().replace('\\', "/");
        let body: String = (1..=20).map(|i| format!("r{i}\n")).collect();
        std::fs::write(t.path().join("a.rs"), &body).unwrap();
        std::fs::write(t.path().join("crlf.rs"), "x\r\ny\r\nz\r\n").unwrap();
        std::fs::write(t.path().join("dup.rs"), "a\nk\nb\nk\n").unwrap();
        std::fs::write(t.path().join("bin.rs"), b"a\0b").unwrap();

        // existuje: čísla ze souboru, kontext ze souboru
        let p = build("Edit", &json!({"file_path": "a.rs", "old_string": "r10\n", "new_string": "R10\nnový\n"}), &dir, read).unwrap();
        assert_eq!((p.shown, p.numbered, p.added, p.removed, p.occurrences), (Shown::Diff, true, 2, 1, Some(1)));
        assert_eq!(texts(&p), ["…", " r7", " r8", " r9", "-r10", "+R10", "+nový", " r11", " r12", " r13", "…"]);
        assert_eq!(p.lines[4].old, Some(10));
        assert_eq!(p.file, "a.rs");

        // CRLF soubor, LF řetězce
        let p = build("Edit", &json!({"file_path": format!("{dir}/crlf.rs"), "old_string": "x\ny", "new_string": "x\nY"}), "", read).unwrap();
        assert_eq!((p.numbered, texts(&p)), (true, vec![" x".to_string(), "-y".into(), "+Y".into(), " z".into()]));

        // replace_all = všechny výskyty
        let p = build("Edit", &json!({"file_path": "dup.rs", "old_string": "k", "new_string": "K", "replace_all": true}), &dir, read).unwrap();
        assert_eq!((p.replace_all, p.occurrences, p.added, p.removed), (true, Some(2), 2, 2));
        let p = build("Edit", &json!({"file_path": "dup.rs", "old_string": "k", "new_string": "K"}), &dir, read).unwrap();
        assert_eq!((p.occurrences, p.added), (Some(2), 1), "bez replace_all jen první");

        // neexistuje / old_string tam není / binární soubor → jen řetězce, bez čísel
        for (f, what) in [("nic.rs", "neexistuje"), ("a.rs", "old_string v souboru není"), ("bin.rs", "binární")] {
            let p = build("Edit", &json!({"file_path": f, "old_string": "a\nb", "new_string": "a\nc"}), &dir, read).unwrap();
            assert_eq!((p.numbered, p.occurrences, texts(&p)), (false, None, vec![" a".to_string(), "-b".into(), "+c".into()]), "{what}");
        }

        // vstup nad limit / s NUL
        let big = "x".repeat(READ_MAX as usize + 1);
        assert_eq!(build("Edit", &json!({"file_path": "a.rs", "old_string": big, "new_string": ""}), &dir, read).unwrap().shown, Shown::TooBig);
        assert_eq!(build("Edit", &json!({"file_path": "a.rs", "old_string": "a\0", "new_string": ""}), &dir, read).unwrap().shown, Shown::Binary);
        // nečitelný vstup / jiný nástroj
        assert_eq!(build("Edit", &json!({"file_path": "a.rs"}), &dir, read), None);
        assert_eq!(build("Read", &json!({"file_path": "a.rs"}), &dir, read), None);
        assert_eq!(for_request("Bash", &json!({"command": "ls"}), &dir), None);
    }

    #[test]
    fn write_table() {
        let t = tempfile::tempdir().unwrap();
        let dir = t.path().to_string_lossy().replace('\\', "/");
        std::fs::write(t.path().join("a.txt"), "a\nb\nc\n").unwrap();
        std::fs::write(t.path().join("bin.dat"), b"\x00\x01").unwrap();
        std::fs::write(t.path().join("big.txt"), vec![b'x'; READ_MAX as usize + 1]).unwrap();

        // existuje → diff proti aktuálnímu
        let p = build("Write", &json!({"file_path": "a.txt", "content": "a\nB\nc\n"}), &dir, read).unwrap();
        assert_eq!((p.shown, p.numbered, texts(&p)), (Shown::Diff, true, vec![" a".to_string(), "-b".into(), "+B".into(), " c".into()]));

        // neexistuje → nový soubor, N řádků + začátek
        let content: String = (1..=100).map(|i| format!("ř{i}\n")).collect();
        let p = build("Write", &json!({"file_path": "novy.txt", "content": content}), &dir, read).unwrap();
        assert_eq!((p.shown, p.total, p.added, p.cut, p.lines.len()), (Shown::NewFile, 100, 100, 60, NEW_HEAD));
        assert_eq!((p.lines[0].text.as_str(), p.lines[0].new), ("ř1", Some(1)));

        // velký / binární aktuální soubor, nečitelný (termín)
        assert_eq!(build("Write", &json!({"file_path": "big.txt", "content": "x"}), &dir, read).unwrap().shown, Shown::TooBig);
        assert_eq!(build("Write", &json!({"file_path": "bin.dat", "content": "x"}), &dir, read).unwrap().shown, Shown::Binary);
        let p = build("Write", &json!({"file_path": "a.txt", "content": "x\ny"}), &dir, |_| Disk::Unreadable).unwrap();
        assert_eq!((p.shown, p.total, p.lines.len()), (Shown::Unread, 2, 2));
        // obří / binární nový obsah
        let big = "y".repeat(READ_MAX as usize + 1);
        assert_eq!(build("Write", &json!({"file_path": "n.txt", "content": big}), &dir, read).unwrap().shown, Shown::TooBig);
        assert_eq!(build("Write", &json!({"file_path": "n.txt", "content": "a\0"}), &dir, read).unwrap().shown, Shown::Binary);
        // prázdný nový soubor
        let p = build("Write", &json!({"file_path": "prazdny.txt", "content": ""}), &dir, read).unwrap();
        assert_eq!((p.shown, p.total, p.lines.len()), (Shown::NewFile, 0, 0));
        // velký soubor s malou změnou: strop řádků drží (diff jen kolem změny)
        let long: String = (0..5000).map(|i| format!("l{i}\n")).collect();
        std::fs::write(t.path().join("long.txt"), &long).unwrap();
        let p = build("Write", &json!({"file_path": "long.txt", "content": long.replace("l2500\n", "X\n")}), &dir, read).unwrap();
        assert_eq!((p.added, p.removed, p.lines.len(), p.numbered), (1, 1, 10, true));
        assert_eq!(p.lines.iter().find(|l| l.kind == diff::Kind::Del).unwrap().old, Some(2501));
    }
}
