//! Rozbor výstupů gitu pro panel Git v kartě Claude (čisté, bez I/O): `status --porcelain=v2 -z`,
//! `worktree list --porcelain`, `diff --numstat -z` a unified diff → řádky náhledu (`claude::preview::Preview`,
//! tentýž tvar jako náhled změny v kartě schválení, takže ho kreslí `src/sessions/DiffPreview.tsx`).

use crate::claude::diff::{self, Kind, Line};
use crate::claude::preview::{Preview, Shown};
use serde::Serialize;

/// Druh změněného souboru (`status --porcelain=v2`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum FileKind {
    /// řádek `1` — změněný sledovaný soubor
    Changed,
    /// řádek `2` — přejmenovaný / zkopírovaný
    Renamed,
    /// řádek `u` — konflikt slučování
    Conflict,
    /// řádek `?` — nesledovaný
    Untracked,
}

/// Jeden soubor ze `status`. Cesty jsou relativně ke kořeni repa, s `/`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GitFile {
    pub path: String,
    /// původní cesta u přejmenování
    pub orig: Option<String>,
    pub kind: FileKind,
    /// stav v indexu (staged) a v pracovní složce; `.` = beze změny, `?` = nesledovaný
    pub x: String,
    pub y: String,
    /// soubor je v HEAD (jde vrátit do stavu posledního commitu)
    pub in_head: bool,
    /// řádky proti HEAD (`diff HEAD --numstat`); `None` = nevíme (binární, nesledovaný, bez HEAD)
    pub added: Option<u32>,
    pub removed: Option<u32>,
}

impl GitFile {
    pub fn staged(&self) -> bool {
        !matches!(self.x.as_str(), "." | "?")
    }
    pub fn unstaged(&self) -> bool {
        self.y != "."
    }
}

/// Hlavička a soubory ze `status --porcelain=v2 --branch -z`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Porcelain {
    /// `None` = odpojená HEAD
    pub branch: Option<String>,
    /// repo ještě nemá žádný commit
    pub initial: bool,
    pub upstream: Option<String>,
    pub ahead: u32,
    pub behind: u32,
    pub files: Vec<GitFile>,
}

fn file(path: &str, orig: Option<String>, kind: FileKind, xy: &str, m_head: &str) -> GitFile {
    let mut c = xy.chars();
    let x = c.next().map(String::from).unwrap_or_else(|| ".".into());
    let y = c.next().map(String::from).unwrap_or_else(|| ".".into());
    GitFile { path: path.to_string(), orig, kind, x, y, in_head: m_head != "000000", added: None, removed: None }
}

/// `git status --porcelain=v2 --branch -z`. Neznámé řádky se přeskočí.
pub fn status_z(out: &str) -> Porcelain {
    let mut s = Porcelain::default();
    let mut it = out.split('\0');
    while let Some(rec) = it.next() {
        if let Some(h) = rec.strip_prefix("# ") {
            if let Some(b) = h.strip_prefix("branch.head ") {
                s.branch = (b != "(detached)").then(|| b.to_string());
            } else if let Some(o) = h.strip_prefix("branch.oid ") {
                s.initial = o == "(initial)";
            } else if let Some(u) = h.strip_prefix("branch.upstream ") {
                s.upstream = Some(u.to_string());
            } else if let Some(ab) = h.strip_prefix("branch.ab ") {
                for part in ab.split_whitespace() {
                    if let Some(n) = part.strip_prefix('+') {
                        s.ahead = n.parse().unwrap_or(0);
                    } else if let Some(n) = part.strip_prefix('-') {
                        s.behind = n.parse().unwrap_or(0);
                    }
                }
            }
        } else if let Some(r) = rec.strip_prefix("1 ") {
            // XY sub mH mI mW hH hI path
            let f: Vec<&str> = r.splitn(8, ' ').collect();
            if f.len() == 8 {
                s.files.push(file(f[7], None, FileKind::Changed, f[0], f[2]));
            }
        } else if let Some(r) = rec.strip_prefix("2 ") {
            // XY sub mH mI mW hH hI Xscore path \0 origPath
            let f: Vec<&str> = r.splitn(9, ' ').collect();
            let orig = it.next().filter(|o| !o.is_empty()).map(String::from);
            if f.len() == 9 {
                s.files.push(file(f[8], orig, FileKind::Renamed, f[0], f[2]));
            }
        } else if let Some(r) = rec.strip_prefix("u ") {
            // XY sub m1 m2 m3 mW h1 h2 h3 path
            let f: Vec<&str> = r.splitn(10, ' ').collect();
            if f.len() == 10 {
                s.files.push(file(f[9], None, FileKind::Conflict, f[0], f[2]));
            }
        } else if let Some(p) = rec.strip_prefix("? ") {
            s.files.push(GitFile {
                path: p.to_string(),
                orig: None,
                kind: FileKind::Untracked,
                x: "?".into(),
                y: "?".into(),
                in_head: false,
                added: None,
                removed: None,
            });
        }
    }
    s
}

/// Řádky proti HEAD z `git diff HEAD --numstat -z --no-renames`: (cesta, přidané, odebrané);
/// binární soubor (`-`) = `None`.
pub fn numstat_z(out: &str) -> Vec<(String, Option<u32>, Option<u32>)> {
    out.split('\0')
        .filter_map(|rec| {
            let mut f = rec.splitn(3, '\t');
            let (a, r, p) = (f.next()?, f.next()?, f.next()?);
            (!p.is_empty()).then(|| (p.to_string(), a.parse().ok(), r.parse().ok()))
        })
        .collect()
}

/// Doplní souborům počty řádků z `numstat_z`.
pub fn apply_numstat(files: &mut [GitFile], stats: &[(String, Option<u32>, Option<u32>)]) {
    for f in files.iter_mut() {
        if let Some((_, a, r)) = stats.iter().find(|(p, _, _)| *p == f.path) {
            f.added = *a;
            f.removed = *r;
        }
    }
}

/// Jeden záznam `git worktree list --porcelain`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Worktree {
    /// cesta, jak ji vypíše git (na Windows s `/`)
    pub path: String,
    /// krátké jméno větve (`refs/heads/` pryč), `None` = odpojená HEAD / bare
    pub branch: Option<String>,
    pub bare: bool,
    pub detached: bool,
    pub locked: bool,
    pub prunable: bool,
}

/// `git worktree list --porcelain`: první záznam je hlavní checkout.
pub fn worktrees(out: &str) -> Vec<Worktree> {
    let mut all = Vec::new();
    let mut cur: Option<Worktree> = None;
    for line in out.lines() {
        if let Some(p) = line.strip_prefix("worktree ") {
            all.extend(cur.take());
            cur = Some(Worktree { path: p.to_string(), ..Default::default() });
            continue;
        }
        let Some(w) = cur.as_mut() else { continue };
        if let Some(b) = line.strip_prefix("branch ") {
            w.branch = Some(b.strip_prefix("refs/heads/").unwrap_or(b).to_string());
        } else if line == "bare" {
            w.bare = true;
        } else if line == "detached" {
            w.detached = true;
        } else if line == "locked" || line.starts_with("locked ") {
            w.locked = true;
        } else if line == "prunable" || line.starts_with("prunable ") {
            w.prunable = true;
        }
    }
    all.extend(cur);
    all
}

/// Prázdný náhled souboru (`shown` doplní volající).
pub fn empty_preview(path: &str, shown: Shown) -> Preview {
    let file = path.rsplit('/').next().unwrap_or(path).to_string();
    Preview {
        file,
        path: path.to_string(),
        shown,
        added: 0,
        removed: 0,
        lines: Vec::new(),
        cut: 0,
        numbered: true,
        replace_all: false,
        occurrences: None,
        rough: false,
        total: 0,
    }
}

/// `@@ -o[,ol] +n[,nl] @@` → (o, ol, n, nl)
fn hunk_head(line: &str) -> Option<(u32, u32, u32, u32)> {
    let rest = line.strip_prefix("@@ -")?;
    let (ranges, _) = rest.split_once(" @@")?;
    let (old, new) = ranges.split_once(" +")?;
    let range = |r: &str| -> Option<(u32, u32)> {
        match r.split_once(',') {
            Some((a, b)) => Some((a.parse().ok()?, b.parse().ok()?)),
            None => Some((r.parse().ok()?, 1)),
        }
    };
    let (o, ol) = range(old)?;
    let (n, nl) = range(new)?;
    Some((o, ol, n, nl))
}

/// Unified diff jednoho souboru (`git diff -U3 -- <soubor>`) → řádky náhledu s čísly ze souboru.
/// Mezi bloky (a před prvním, když nezačíná na 1. řádku) je `Gap`. Nad `max_lines` řádků se zbytek jen
/// počítá do `cut`; `added`/`removed` jsou vždy celé. Binární rozdíl → `Shown::Binary`.
pub fn unified(path: &str, text: &str, max_lines: usize) -> Preview {
    let mut p = empty_preview(path, Shown::Diff);
    let mut left = (0u32, 0u32); // zbývá řádků bloku (staré, nové)
    let mut no = (0u32, 0u32); // další číslo řádku (staré, nové)
    let mut hunks = 0;
    let push = |p: &mut Preview, l: Line| {
        if p.lines.len() < max_lines {
            p.lines.push(l);
        } else {
            p.cut += 1;
        }
    };
    for raw in text.split('\n') {
        let line = raw.strip_suffix('\r').unwrap_or(raw);
        if left == (0, 0) {
            if let Some((o, ol, n, nl)) = hunk_head(line) {
                if hunks > 0 || o > 1 || n > 1 {
                    push(&mut p, Line { kind: Kind::Gap, old: None, new: None, text: String::new() });
                }
                hunks += 1;
                left = (ol, nl);
                // prázdný rozsah (`-0,0`) ukazuje na řádek PŘED změnou
                no = (if ol == 0 { o + 1 } else { o }, if nl == 0 { n + 1 } else { n });
            } else if hunks == 0 && (line.starts_with("Binary files ") || line == "GIT binary patch") {
                p.shown = Shown::Binary;
                return p;
            }
            continue;
        }
        let (kind, body) = match line.as_bytes().first() {
            Some(b' ') => (Kind::Ctx, &line[1..]),
            Some(b'-') => (Kind::Del, &line[1..]),
            Some(b'+') => (Kind::Add, &line[1..]),
            Some(b'\\') => continue, // „\ No newline at end of file“
            // prázdný kontextový řádek (některé nástroje ořezávají mezeru)
            None => (Kind::Ctx, ""),
            _ => {
                left = (0, 0);
                continue;
            }
        };
        let text = diff::clip(body);
        match kind {
            Kind::Ctx => {
                push(&mut p, Line { kind, old: Some(no.0), new: Some(no.1), text });
                no = (no.0 + 1, no.1 + 1);
                left = (left.0.saturating_sub(1), left.1.saturating_sub(1));
            }
            Kind::Del => {
                p.removed += 1;
                push(&mut p, Line { kind, old: Some(no.0), new: None, text });
                no.0 += 1;
                left.0 = left.0.saturating_sub(1);
            }
            Kind::Add => {
                p.added += 1;
                push(&mut p, Line { kind, old: None, new: Some(no.1), text });
                no.1 += 1;
                left.1 = left.1.saturating_sub(1);
            }
            Kind::Gap => {}
        }
    }
    p
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn status_table() {
        let out = [
            "# branch.oid 1234",
            "# branch.head w3/wtree",
            "# branch.upstream origin/w3/wtree",
            "# branch.ab +2 -1",
            "1 .M N... 100644 100644 100644 aaa aaa src/a b.rs",
            "1 A. N... 000000 100644 100644 000 bbb nový.txt",
            "2 R. N... 100644 100644 100644 ccc ccc R100 nové jméno.md",
            "staré jméno.md",
            "u UU N... 100644 100644 100644 100644 d1 d2 d3 konflikt.rs",
            "? dir/neznámý soubor.txt",
            "",
        ]
        .join("\0");
        let s = status_z(&out);
        assert_eq!((s.branch.as_deref(), s.upstream.as_deref(), s.ahead, s.behind, s.initial), (Some("w3/wtree"), Some("origin/w3/wtree"), 2, 1, false));
        type Row<'a> = (&'a str, Option<&'a str>, FileKind, &'a str, &'a str, bool);
        let rows: Vec<Row> =
            s.files.iter().map(|f| (f.path.as_str(), f.orig.as_deref(), f.kind, f.x.as_str(), f.y.as_str(), f.in_head)).collect();
        assert_eq!(
            rows,
            vec![
                ("src/a b.rs", None, FileKind::Changed, ".", "M", true),
                ("nový.txt", None, FileKind::Changed, "A", ".", false),
                ("nové jméno.md", Some("staré jméno.md"), FileKind::Renamed, "R", ".", true),
                ("konflikt.rs", None, FileKind::Conflict, "U", "U", true),
                ("dir/neznámý soubor.txt", None, FileKind::Untracked, "?", "?", false),
            ]
        );
        assert!(!s.files[0].staged() && s.files[0].unstaged());
        assert!(s.files[1].staged() && !s.files[1].unstaged());
        assert!(!s.files[4].staged() && s.files[4].unstaged());
    }

    #[test]
    fn status_detached_initial_and_garbage() {
        let s = status_z("# branch.oid (initial)\0# branch.head (detached)\0nesmysl\0\0");
        assert_eq!(s, Porcelain { branch: None, initial: true, ..Default::default() });
        // bez upstreamu žádné ab
        let s = status_z("# branch.oid abc\0# branch.head main\0");
        assert_eq!((s.branch.as_deref(), s.upstream, s.ahead), (Some("main"), None, 0));
    }

    #[test]
    fn numstat_and_apply() {
        let st = numstat_z("3\t1\tsrc/a b.rs\0-\t-\tlogo.png\0\0");
        assert_eq!(st, vec![("src/a b.rs".into(), Some(3), Some(1)), ("logo.png".into(), None, None)]);
        let mut s = status_z("1 .M N... 100644 100644 100644 a a src/a b.rs\0? x\0");
        apply_numstat(&mut s.files, &st);
        assert_eq!((s.files[0].added, s.files[0].removed), (Some(3), Some(1)));
        assert_eq!((s.files[1].added, s.files[1].removed), (None, None));
    }

    #[test]
    fn worktree_list_table() {
        let out = "worktree C:/Users/x/Desktop/Ledge\nHEAD abc\nbranch refs/heads/main\n\n\
                   worktree C:/Users/x/Desktop/ledge wt\nHEAD def\nbranch refs/heads/w3/wtree\nlocked kvůli disku\n\n\
                   worktree C:/tmp/odpojeny\nHEAD 123\ndetached\nprunable gitdir file points to non-existent location\n\n";
        let w = worktrees(out);
        assert_eq!(w.len(), 3);
        assert_eq!(w[0], Worktree { path: "C:/Users/x/Desktop/Ledge".into(), branch: Some("main".into()), ..Default::default() });
        assert_eq!(w[1], Worktree { path: "C:/Users/x/Desktop/ledge wt".into(), branch: Some("w3/wtree".into()), locked: true, ..Default::default() });
        assert_eq!(w[2], Worktree { path: "C:/tmp/odpojeny".into(), detached: true, prunable: true, ..Default::default() });
        assert!(worktrees("").is_empty());
        let bare = worktrees("worktree /srv/r.git\nbare\n");
        assert!(bare[0].bare && bare[0].branch.is_none());
    }

    #[test]
    fn unified_two_hunks_with_numbers() {
        let text = "diff --git a/x.rs b/x.rs\nindex 1..2 100644\n--- a/x.rs\n+++ b/x.rs\n\
                    @@ -1,3 +1,3 @@\n a\r\n-b\n+B\n c\n@@ -10,2 +10,3 @@ fn x()\n k\n+nový\n l\n\\ No newline at end of file\n";
        let p = unified("src/x.rs", text, 400);
        assert_eq!((p.file.as_str(), p.shown, p.added, p.removed, p.cut, p.numbered), ("x.rs", Shown::Diff, 2, 1, 0, true));
        let got: Vec<(Kind, Option<u32>, Option<u32>, &str)> = p.lines.iter().map(|l| (l.kind, l.old, l.new, l.text.as_str())).collect();
        assert_eq!(
            got,
            vec![
                (Kind::Ctx, Some(1), Some(1), "a"),
                (Kind::Del, Some(2), None, "b"),
                (Kind::Add, None, Some(2), "B"),
                (Kind::Ctx, Some(3), Some(3), "c"),
                (Kind::Gap, None, None, ""),
                (Kind::Ctx, Some(10), Some(10), "k"),
                (Kind::Add, None, Some(11), "nový"),
                (Kind::Ctx, Some(11), Some(12), "l"),
            ]
        );
    }

    #[test]
    fn unified_edge_cases() {
        // řádek obsahu začínající „--“ / „++“ uvnitř bloku není hlavička
        let p = unified("a", "@@ -1,2 +1,2 @@\n--- x\n+++ y\n-a\n+b\n", 400);
        assert_eq!((p.added, p.removed, p.lines.len()), (2, 2, 4));
        // nový soubor: -0,0 → čísla od 1, žádná mezera na začátku
        let p = unified("n", "new file mode 100644\n--- /dev/null\n+++ b/n\n@@ -0,0 +1,2 @@\n+a\n+b\n", 400);
        assert_eq!(p.lines.iter().map(|l| (l.kind, l.new)).collect::<Vec<_>>(), vec![(Kind::Add, Some(1)), (Kind::Add, Some(2))]);
        // binární
        assert_eq!(unified("p.png", "diff --git a/p.png b/p.png\nBinary files a/p.png and b/p.png differ\n", 400).shown, Shown::Binary);
        // jen změna práv → žádné řádky
        let p = unified("s", "diff --git a/s b/s\nold mode 100644\nnew mode 100755\n", 400);
        assert_eq!((p.shown, p.lines.len(), p.added), (Shown::Diff, 0, 0));
        // strop: zbytek do cut, počty celé
        let body: String = (0..10).map(|i| format!("+r{i}\n")).collect();
        let p = unified("big", &format!("@@ -0,0 +1,10 @@\n{body}"), 4);
        assert_eq!((p.lines.len(), p.cut, p.added), (4, 6, 10));
        // dlouhý řádek se zkrátí
        let p = unified("l", &format!("@@ -1 +1 @@\n-{}\n+x\n", "y".repeat(1000)), 400);
        assert!(p.lines[0].text.chars().count() < 1000);
    }
}
