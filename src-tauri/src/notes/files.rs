//! Výběr souborů projektu pro odkazy `@cesta` v poznámce: čistý fuzzy filtr (`fuzzy`) a seznam
//! souborů z `git ls-files` (`git_files`, jediný I/O, bez cache a bez časovačů — volá se na dotaz).

use std::path::Path;

/// Nejvýš tolik návrhů.
pub const MAX_HITS: usize = 20;
/// Nejvýš tolik cest se z gitu vezme (obří repo nesmí zahltit paměť).
const MAX_FILES: usize = 200_000;

/// Skóre shody `query` v `path` (vyšší = lepší), `None` = nesedí. Znaky dotazu musí jít za sebou
/// (nezáleží na velikosti, `\` = `/`). Body: shoda v názvu souboru, souvislé úseky, začátek slova;
/// kratší cesta vyhrává při shodě.
fn score(path: &str, query: &str) -> Option<i64> {
    let p: Vec<char> = path.chars().flat_map(char::to_lowercase).collect();
    let q: Vec<char> = query.chars().flat_map(char::to_lowercase).map(|c| if c == '\\' { '/' } else { c }).collect();
    let name_at = p.iter().rposition(|&c| c == '/' || c == '\\').map_or(0, |i| i + 1);
    let (mut pi, mut s, mut prev): (usize, i64, Option<usize>) = (0, 0, None);
    for &qc in &q {
        let at = (pi..p.len()).find(|&i| p[i] == qc || (qc == '/' && p[i] == '\\'))?;
        s += 1;
        if prev.is_some_and(|b| b + 1 == at) {
            s += 4;
        }
        if at == 0 || matches!(p[at - 1], '/' | '\\' | '_' | '-' | '.' | ' ') {
            s += 3;
        }
        if at >= name_at {
            s += 2;
        }
        prev = Some(at);
        pi = at + 1;
    }
    // název souboru bez přípony přesně = dotaz: nejlepší shoda
    let stem: String = p[name_at..].iter().take_while(|&&c| c != '.').collect();
    if stem == q.iter().collect::<String>() {
        s += 8;
    }
    Some(s * 10 - p.len() as i64 / 4)
}

/// Nejlepší shody (nejvýš `max`), nejlepší první; shodné skóre drží pořadí vstupu. Prázdný dotaz =
/// prvních `max` souborů.
pub fn fuzzy(files: &[String], query: &str, max: usize) -> Vec<String> {
    let q = query.trim().trim_start_matches('@');
    if q.is_empty() {
        return files.iter().take(max).cloned().collect();
    }
    let mut hits: Vec<(i64, usize)> = files.iter().enumerate().filter_map(|(i, f)| score(f, q).map(|s| (s, i))).collect();
    hits.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)));
    hits.into_iter().take(max).map(|(_, i)| files[i].clone()).collect()
}

/// Rozdělí výstup `git ls-files -z` na cesty (relativní, `/`).
pub fn parse_listing(out: &[u8]) -> Vec<String> {
    out.split(|&b| b == 0).filter(|s| !s.is_empty()).take(MAX_FILES).map(|s| String::from_utf8_lossy(s).into_owned()).collect()
}

/// Sledované i nové (neignorované) soubory v `dir` přes `git ls-files`; bez gitu / mimo repo = prázdné.
/// Žádné okno (`CREATE_NO_WINDOW`), žádný stav po návratu.
pub fn git_files(dir: &Path) -> Vec<String> {
    use std::os::windows::process::CommandExt;
    let out = std::process::Command::new("git")
        .args(["ls-files", "-z", "--cached", "--others", "--exclude-standard"])
        .current_dir(dir)
        .creation_flags(0x0800_0000)
        .stdin(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .output();
    match out {
        Ok(o) if o.status.success() => parse_listing(&o.stdout),
        _ => Vec::new(),
    }
}

/// Návrhy souborů pro `@`: `git_files` + `fuzzy` (max `MAX_HITS`).
pub fn project_files(dir: &Path, query: &str) -> Vec<String> {
    fuzzy(&git_files(dir), query, MAX_HITS)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn files() -> Vec<String> {
        ["README.md", "src/components/Header.tsx", "src/components/Footer.tsx", "src/lib/header_utils.ts", "docs/guide/headers.md", "package.json", "src/main.ts"]
            .iter()
            .map(|s| s.to_string())
            .collect()
    }

    #[test]
    fn fuzzy_table() {
        let f = files();
        let top = |q: &str| fuzzy(&f, q, 20);
        assert_eq!(top("header")[0], "src/components/Header.tsx");
        assert_eq!(top("HEADER").len(), 3);
        assert_eq!(top("srcmain"), vec!["src/main.ts"]);
        assert_eq!(top("src\\main"), vec!["src/main.ts"]);
        assert_eq!(top("@pkg"), vec!["package.json"]);
        assert_eq!(top("pack"), vec!["package.json"]);
        assert!(top("zzz").is_empty());
        // název souboru slovo vyhrává nad shodou v adresáři
        assert_eq!(top("foot")[0], "src/components/Footer.tsx");
        // prázdný dotaz = prvních N, strop N
        assert_eq!(fuzzy(&f, "  ", 3), f[..3].to_vec());
        assert_eq!(fuzzy(&f, "s", 2).len(), 2);
        assert_eq!(fuzzy(&[], "x", 5), Vec::<String>::new());
    }

    #[test]
    fn fuzzy_caps_at_twenty_in_project_files_contract() {
        let many: Vec<String> = (0..100).map(|i| format!("src/f{i}.rs")).collect();
        assert_eq!(fuzzy(&many, "f", MAX_HITS).len(), MAX_HITS);
    }

    #[test]
    fn listing_is_nul_separated_and_unicode_safe() {
        assert_eq!(parse_listing("a/b.rs\0č š/ž.md\0\0".as_bytes()), vec!["a/b.rs", "č š/ž.md"]);
        assert!(parse_listing(b"").is_empty());
    }

    #[test]
    fn git_files_outside_repo_or_missing_dir_is_empty() {
        let dir = tempfile::tempdir().unwrap();
        // prázdná složka není repo (nebo git chybí) → nic, žádná chyba
        assert!(git_files(&dir.path().join("neexistuje")).is_empty());
    }
}
