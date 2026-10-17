//! Cesty pro hlídač nedokončené práce: lexikální normalizace, klíč pro porovnání (Windows bez
//! ohledu na velikost), cesta vůči kořeni repa a rozbor `git status --porcelain`. Čisté funkce bez
//! I/O; používá `hygiene.rs`.

/// Lexikální normalizace cesty s `/`: pryč `.`, `..` (nad kořen/disk nejde) a zdvojená/koncová lomítka.
pub fn lexical(p: &str) -> String {
    if p.is_empty() {
        return String::new();
    }
    let mut out: Vec<&str> = Vec::new();
    for (i, seg) in p.split('/').enumerate() {
        match seg {
            "." => {}
            "" if i > 0 => {}
            ".." if out.last().is_some_and(|l| *l != ".." && !l.is_empty() && !l.ends_with(':')) => {
                out.pop();
            }
            _ => out.push(seg),
        }
    }
    if out == [""] { "/".into() } else { out.join("/") }
}

/// Klíč pro porovnání cest: `/` místo `\`, lexikálně normalizovaná, na Windows bez ohledu na velikost.
pub fn path_key(p: &str) -> String {
    let p = lexical(&p.replace('\\', "/"));
    let p = p.trim_end_matches('/');
    if cfg!(windows) { p.to_lowercase() } else { p.to_string() }
}

fn is_abs(p: &str) -> bool {
    let b = p.as_bytes();
    p.starts_with('/') || (b.len() >= 3 && b[0].is_ascii_alphabetic() && b[1] == b':' && b[2] == b'/')
}

/// Absolutní, lexikálně normalizovaná cesta s `/` (relativní se vztáhne k `cwd` session).
pub fn absolutize(path: &str, cwd: &str) -> String {
    let p = path.replace('\\', "/");
    if is_abs(&p) || cwd.is_empty() {
        lexical(&p)
    } else {
        lexical(&format!("{}/{}", cwd.replace('\\', "/"), p))
    }
}

/// Cesta relativně ke kořeni repa (původní velikost písmen). Mimo repo → `None`.
pub fn repo_relative(root: &str, abs: &str) -> Option<String> {
    let root = lexical(&root.replace('\\', "/"));
    let root = root.trim_end_matches('/');
    let abs = lexical(&abs.replace('\\', "/"));
    let head = abs.get(..root.len())?;
    let tail = abs.get(root.len()..)?.strip_prefix('/')?;
    (path_key(head) == path_key(root) && !tail.is_empty()).then(|| tail.to_string())
}

/// Rozbalí C-uvozovanou cestu gitu (`"a\"b\303\251"`). Vstup začíná za otevírací `"`.
/// Vrací cestu a zbytek za zavírací `"`.
fn unquote(s: &str) -> Option<(String, &str)> {
    let b = s.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    while i < b.len() {
        match b[i] {
            b'"' => return Some((String::from_utf8_lossy(&out).into_owned(), &s[i + 1..])),
            b'\\' if i + 1 < b.len() => {
                let c = b[i + 1];
                if (b'0'..=b'3').contains(&c) && i + 3 < b.len() {
                    let oct = std::str::from_utf8(&b[i + 1..i + 4]).ok()?;
                    out.push(u8::from_str_radix(oct, 8).ok()?);
                    i += 4;
                    continue;
                }
                out.push(match c {
                    b'n' => b'\n',
                    b't' => b'\t',
                    b'r' => b'\r',
                    b'a' => 7,
                    b'b' => 8,
                    b'f' => 12,
                    b'v' => 11,
                    other => other,
                });
                i += 2;
            }
            c => {
                out.push(c);
                i += 1;
            }
        }
    }
    None
}

/// Jedna cesta z porcelain řádku: uvozovaná, nebo (u přejmenování) do ` -> `, jinak do konce.
fn take_path(s: &str, rename: bool) -> Option<(String, &str)> {
    if let Some(q) = s.strip_prefix('"') {
        return unquote(q);
    }
    match s.find(" -> ").filter(|_| rename) {
        Some(i) => Some((s[..i].to_string(), &s[i..])),
        None => Some((s.to_string(), "")),
    }
}

/// Cesty z `git status --porcelain` (v1, bez `-z`): u přejmenování/kopie (`R a -> b`) obě,
/// uvozované cesty rozbalené, untracked složky (`?? dir/`) s koncovým `/`.
pub fn porcelain_paths(out: &str) -> Vec<String> {
    let mut v = Vec::new();
    for line in out.lines() {
        let (Some(xy), Some(rest)) = (line.get(..2), line.get(3..)) else { continue };
        if line.as_bytes()[2] != b' ' || rest.is_empty() {
            continue;
        }
        let rename = xy.contains(['R', 'C']);
        let Some((first, after)) = take_path(rest, rename) else { continue };
        v.push(first);
        if let Some((second, _)) = after.strip_prefix(" -> ").and_then(|r| take_path(r, false)) {
            v.push(second);
        }
    }
    v
}

/// Je soubor (repo-relativní) v porcelain výstupu? `dirty` = cesty z porcelainu prohnané `path_key`.
/// Položka může být složka (`?? nova/`) — pak pokrývá vše pod ní.
pub(super) fn is_dirty(file: &str, dirty: &[String]) -> bool {
    let k = path_key(file);
    dirty.iter().any(|d| *d == k || k.strip_prefix(d.as_str()).is_some_and(|r| r.starts_with('/')))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn path_helpers() {
        assert_eq!(absolutize("src\\a.rs", "C:\\r\\sub"), "C:/r/sub/src/a.rs");
        assert_eq!(absolutize("./a.rs", "C:/r/"), "C:/r/a.rs");
        assert_eq!(absolutize("D:\\x\\a.rs", "C:/r"), "D:/x/a.rs");
        assert_eq!(repo_relative("C:/r", "C:/r/src/a.rs").as_deref(), Some("src/a.rs"));
        assert_eq!(repo_relative("C:/r/", "C:/r/a b/č.rs").as_deref(), Some("a b/č.rs"));
        // jiný repo se stejným prefixem / mimo repo / samotný kořen
        assert_eq!(repo_relative("C:/r", "C:/rx/a.rs"), None);
        assert_eq!(repo_relative("C:/r", "D:/r/a.rs"), None);
        assert_eq!(repo_relative("C:/r", "C:/r"), None);
        if cfg!(windows) {
            assert_eq!(repo_relative("c:/Users/R", "C:/users/r/Src/A.rs").as_deref(), Some("Src/A.rs"));
            assert_eq!(path_key("Src\\A.rs"), "src/a.rs");
        }
        assert_eq!(path_key("./dir/"), path_key("dir"));
        // `..` a `.` se normalizují před porovnáním
        assert_eq!(absolutize("..\\b\\.\\c.rs", "C:/r/sub"), "C:/r/b/c.rs");
        assert_eq!(absolutize("C:/r/x/../y//z.rs", ""), "C:/r/y/z.rs");
        assert_eq!(repo_relative("C:/r/sub/..", "C:/r/a/../b.rs").as_deref(), Some("b.rs"));
        assert_eq!(repo_relative("C:/r", "C:/r/../rx/a.rs"), None);
    }

    #[test]
    fn lexical_table() {
        let cases = [
            ("C:/a/b/../c", "C:/a/c"),
            ("C:/a/./b/", "C:/a/b"),
            ("C:/..", "C:/.."),
            ("/a/../b", "/b"),
            ("/", "/"),
            ("a/../../b", "../b"),
            ("./a", "a"),
            ("", ""),
        ];
        for (i, o) in cases {
            assert_eq!(lexical(i), o, "{i}");
        }
    }

    #[test]
    fn porcelain_table() {
        let cases: &[(&str, &[&str])] = &[
            (" M src/a.rs\n", &["src/a.rs"]),
            ("M  src/a.rs\nA  b.rs\n D smazany.rs\n", &["src/a.rs", "b.rs", "smazany.rs"]),
            ("?? nova/\n?? x.txt\n", &["nova/", "x.txt"]),
            ("R  stary.rs -> novy.rs\n", &["stary.rs", "novy.rs"]),
            ("RM a b.rs -> c d.rs\n", &["a b.rs", "c d.rs"]),
            ("C  a.rs -> kopie.rs\n", &["a.rs", "kopie.rs"]),
            // mezery se neuvozují, uvozovky a zpětná lomítka ano, ne-ASCII oktalově (quotePath=true)
            (" M dir s mezerou/a.rs\n", &["dir s mezerou/a.rs"]),
            (" M \"uvoz\\\"ovka.rs\"\n", &["uvoz\"ovka.rs"]),
            ("?? \"p\\305\\231\\303\\255klad.rs\"\n", &["příklad.rs"]),
            ("R  \"a \\\"q\\\".rs\" -> \"b\\\\c.rs\"\n", &["a \"q\".rs", "b\\c.rs"]),
            // šipka v názvu mimo přejmenování je součást cesty
            (" M a -> b.rs\n", &["a -> b.rs"]),
            ("\n#\nX\n M \n", &[]),
        ];
        for (out, want) in cases {
            assert_eq!(porcelain_paths(out), want.iter().map(|w| w.to_string()).collect::<Vec<_>>(), "{out:?}");
        }
    }

    fn dirty_of(files: &[&str], porcelain: &[&str]) -> Vec<String> {
        let dirty: Vec<String> = porcelain.iter().map(|d| path_key(d)).collect();
        files.iter().filter(|f| is_dirty(f, &dirty)).map(|f| f.to_string()).collect()
    }

    #[test]
    fn dirty_intersection() {
        let files = ["src/a.rs", "nova/deep/x.ts", "README.md", "src/clean.rs"];
        assert_eq!(dirty_of(&files, &["src/a.rs", "nova/", "jiny.rs", "src/clean.rsx"]), vec!["src/a.rs", "nova/deep/x.ts"]);
        assert!(dirty_of(&files, &[]).is_empty());
        assert!(dirty_of(&[], &["src/a.rs"]).is_empty());
        if cfg!(windows) {
            assert_eq!(dirty_of(&["Src/A.rs"], &["src/a.rs"]), vec!["Src/A.rs"]);
        }
    }
}
