//! Úložiště poznámek na disku: `NoteView` a jeho parsování (`parse_note`), jména a bezpečné cesty
//! (`slug`, `project_dir_name`, `inside`, `check_note_path`), vytvoření (`create`), sken kořene
//! (`scan`, `list`) a přepnutí `done` (`set_done`). Závisí na `frontmatter.rs`, `md.rs`
//! (titulek, úkoly) a společném I/O v `mod.rs` (`write_atomic`, `path_lock`, obrázky).

use crate::i18n::tr;
use super::{
    DEFAULT_KIND, GENERAL, MAX_IMAGES, MAX_NOTE_BYTES, decode_png, edit, get, iso, local_from_naive, md, parse_time, path_lock,
    render, split, with_done, write_atomic, yaml_value,
};
use chrono::{DateTime, FixedOffset, NaiveDateTime};
use serde::{Deserialize, Serialize};
use std::path::{Component, Path, PathBuf};

/// Čas z názvu souboru `YYYY-MM-DD-HHMMSS-…` (lokální) — záloha, když frontmatter chybí.
fn time_from_name(name: &str) -> Option<DateTime<FixedOffset>> {
    let stamp = name.get(..17)?;
    NaiveDateTime::parse_from_str(stamp, "%Y-%m-%d-%H%M%S").ok().and_then(local_from_naive)
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct NoteView {
    /// absolutní cesta k `.md`
    pub path: String,
    /// id projektu, `None` = obecná
    pub project: Option<String>,
    /// ISO s offsetem
    pub created: String,
    pub created_ms: i64,
    pub remind: Option<String>,
    pub remind_ms: Option<i64>,
    pub done: bool,
    /// první textový řádek bez Markdown značek, zkrácený
    pub title: String,
    pub images: usize,
    /// kategorie (`kind:`), `DEFAULT_KIND` když chybí; neznámé id z druhého PC projde beze změny
    pub kind: String,
    /// poslední úprava v Ledge (ISO s offsetem)
    pub updated: Option<String>,
    /// úkoly `- [x]` / `- [ ]` v těle
    pub tasks_done: usize,
    pub tasks_total: usize,
    /// barva lístku (`color:` — id z `meta::COLORS` nebo `#rrggbb`), `None` = výchozí
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
    /// štítky: `tags:` ze frontmatteru + `#štítek` v těle (`meta::tags_of`)
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
}

fn first_line(body: &str) -> String {
    let line = md::title(body);
    let mut out: String = line.chars().take(120).collect();
    if line.chars().count() > 120 {
        out.push('…');
    }
    out
}

/// Metadata poznámky z textu souboru. Projekt rozhoduje frontmatter (původní id z configu);
/// `folder_project` (jméno složky, `None` pro kořen a `_obecne`) je jen záloha pro ručně psané
/// poznámky. `fallback_ms` (mtime) jen když čas chybí i v názvu.
pub fn parse_note(path: &Path, text: &str, folder_project: Option<&str>, fallback_ms: i64) -> NoteView {
    let (front, body) = split(text);
    let front = front.unwrap_or_default();
    let name = path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
    let created = get(&front, "created").and_then(|s| parse_time(&s)).or_else(|| time_from_name(&name));
    let remind = get(&front, "remind").and_then(|s| parse_time(&s));
    let project = match get(&front, "project").map(|p| p.trim().to_string()).filter(|p| !p.is_empty()) {
        Some(p) if p == GENERAL => None,
        Some(p) => Some(p),
        None => folder_project.filter(|f| *f != GENERAL).map(str::to_string),
    };
    let done = get(&front, "done").is_some_and(|d| matches!(d.to_ascii_lowercase().as_str(), "true" | "yes" | "ano" | "1"));
    let tasks = md::tasks(edit::body_view(body).1);
    NoteView {
        path: path.to_string_lossy().into_owned(),
        project,
        created: created.as_ref().map(iso).unwrap_or_default(),
        created_ms: created.map(|d| d.timestamp_millis()).unwrap_or(fallback_ms),
        remind: remind.as_ref().map(iso),
        remind_ms: remind.map(|d| d.timestamp_millis()),
        done,
        title: first_line(body),
        images: body.matches("![").count(),
        kind: get(&front, "kind").map(|k| k.trim().to_string()).filter(|k| !k.is_empty()).unwrap_or_else(|| DEFAULT_KIND.into()),
        updated: get(&front, "updated").and_then(|s| parse_time(&s)).as_ref().map(iso),
        tasks_done: tasks.iter().filter(|t| t.1).count(),
        tasks_total: tasks.len(),
        color: super::meta::color_of(&front),
        tags: super::meta::tags_of(&front, body),
    }
}

// ---------- jména a cesty ----------

pub(crate) fn fold(c: char) -> char {
    match c {
        'á' | 'ä' | 'à' | 'â' => 'a',
        'č' | 'ç' => 'c',
        'ď' => 'd',
        'é' | 'ě' | 'ë' | 'è' | 'ê' => 'e',
        'í' | 'ï' | 'î' => 'i',
        'ľ' | 'ĺ' => 'l',
        'ň' => 'n',
        'ó' | 'ö' | 'ô' => 'o',
        'ř' | 'ŕ' => 'r',
        'š' => 's',
        'ť' => 't',
        'ú' | 'ů' | 'ü' | 'û' => 'u',
        'ý' | 'ÿ' => 'y',
        'ž' => 'z',
        c => c,
    }
}

/// Krátký ASCII slug z prvního textového řádku (`Zavolat Řehořovi!` → `zavolat-rehorovi`), max 40 znaků.
pub fn slug(text: &str) -> String {
    let line = text.lines().map(str::trim).find(|l| !l.is_empty() && !l.starts_with("![")).unwrap_or("");
    let mut out = String::new();
    for c in line.chars().flat_map(char::to_lowercase).map(fold) {
        if c.is_ascii_alphanumeric() {
            out.push(c);
        } else if !out.ends_with('-') && !out.is_empty() {
            out.push('-');
        }
        if out.len() >= 40 {
            break;
        }
    }
    let out = out.trim_matches('-').to_string();
    if out.is_empty() { "poznamka".into() } else { out }
}

fn reserved(name: &str) -> bool {
    let n = name.to_ascii_lowercase();
    matches!(n.as_str(), "con" | "prn" | "aux" | "nul")
        || ((n.starts_with("com") || n.starts_with("lpt")) && n.len() == 4 && n.as_bytes()[3].is_ascii_digit())
}

/// Bezpečný název složky projektu: jen `[a-z0-9_-]`, nic jiného (žádné `..`, `\`, `:`, `/`).
/// Prázdné / chybějící = `_obecne`. Rezervovaná jména Windows (CON, NUL…) dostanou prefix.
pub fn project_dir_name(project: Option<&str>) -> String {
    let raw = project.unwrap_or("").trim();
    let mut s = String::new();
    for c in raw.chars().flat_map(char::to_lowercase).map(fold) {
        if c.is_ascii_alphanumeric() || c == '_' || c == '-' {
            s.push(c);
        } else if !s.ends_with('-') {
            s.push('-');
        }
    }
    let s: String = s.trim_matches('-').chars().take(64).collect();
    if s.is_empty() {
        GENERAL.into()
    } else if reserved(&s) {
        format!("p-{s}")
    } else {
        s
    }
}

/// Lexikálně uvnitř kořene a bez `..` (ochrana i před cestami z UI).
pub(crate) fn inside(root: &Path, p: &Path) -> bool {
    !p.components().any(|c| matches!(c, Component::ParentDir)) && p.starts_with(root)
}

/// `canonicalize` s krátkým opakováním: atomický zápis jiného vlákna (`write_atomic`) soubor na pár ms
/// přepisuje a Windows ho mezitím hlásí jako neexistující — poznámka by pak „nebyla poznámka Ledge“.
/// Skutečně chybějící soubor ve skutečné složce selže po ~200 ms.
fn canonicalize_settled(path: &Path) -> Option<PathBuf> {
    // ~200 ms: souběžný zápis jiné poznámkové operace (atomické přejmenování, antivir na novém souboru)
    // umí soubor na Windows „schovat“ déle než pár ms — 5 × 10 ms padalo pod zátěží (flaky test kola 24)
    for attempt in 0..20 {
        match path.canonicalize() {
            Ok(p) => return Some(p),
            Err(_) if attempt < 19 && path.parent().is_some_and(Path::is_dir) => std::thread::sleep(std::time::Duration::from_millis(10)),
            Err(_) => return None,
        }
    }
    None
}

/// Cesta k existující poznámce pod kořenem (i po rozbalení symlinků), jinak chyba.
pub fn check_note_path(root: &Path, path: &Path) -> Result<PathBuf, String> {
    let bad = || trf!("Cesta {} není poznámka Ledge.", "Path {} isn't a Ledge note.", path.display());
    if !path.is_absolute() || !inside(root, path) || !path.extension().is_some_and(|e| e.eq_ignore_ascii_case("md")) {
        return Err(bad());
    }
    let real = canonicalize_settled(path).ok_or_else(bad)?;
    let real_root = root.canonicalize().map_err(|_| bad())?;
    if !real.starts_with(&real_root) {
        return Err(bad());
    }
    Ok(real)
}

/// `\\?\C:\x` → `C:\x`, `\\?\UNC\srv\x` → `\\srv\x` (shell / editory verbatim cesty neumí).
pub fn strip_verbatim(p: &Path) -> PathBuf {
    let s = p.to_string_lossy();
    if let Some(rest) = s.strip_prefix(r"\\?\UNC\") {
        PathBuf::from(format!(r"\\{rest}"))
    } else if let Some(rest) = s.strip_prefix(r"\\?\") {
        PathBuf::from(rest)
    } else {
        p.to_path_buf()
    }
}

/// Nová poznámka z UI (nebo z API).
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NewNote {
    pub text: String,
    /// id projektu; `None`/prázdné = obecná
    #[serde(default)]
    pub project: Option<String>,
    /// `2026-10-17T09:00` z `datetime-local` (= lokální čas) nebo ISO s offsetem
    #[serde(default)]
    pub remind: Option<String>,
    /// PNG v base64 (i jako data URL)
    #[serde(default)]
    pub images: Vec<String>,
    /// kategorie (id z `notes.kinds`); `None`/`DEFAULT_KIND` = klíč se nezapíše
    #[serde(default)]
    pub kind: Option<String>,
}

/// Id kategorie: 1–32 znaků `[A-Za-z0-9_-]` (jde rovnou do frontmatteru).
pub fn valid_kind(k: &str) -> bool {
    (1..=32).contains(&k.len()) && k.chars().all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
}

/// Volný základ jména v `dir`: `stem`, jinak `stem-2`, `stem-3`… (ani `.md`, ani první obrázek nesmí existovat).
pub(crate) fn unique_base(dir: &Path, stem: &str) -> String {
    let mut base = stem.to_string();
    let mut n = 2;
    while dir.join(format!("{base}.md")).exists() || dir.join(format!("{base}-1.png")).exists() {
        base = format!("{stem}-{n}");
        n += 1;
    }
    base
}

/// Uloží poznámku (obrázky napřed, `.md` nakonec — každý soubor atomicky). Vrací cestu k `.md`.
pub fn create(root: &Path, note: &NewNote, now: &DateTime<FixedOffset>) -> Result<PathBuf, String> {
    let text = note.text.trim();
    if text.is_empty() && note.images.is_empty() {
        return Err(tr("Prázdná poznámka.", "Empty note.").into());
    }
    if note.images.len() > MAX_IMAGES {
        return Err(trf!("Nejvýš {MAX_IMAGES} obrázků na poznámku.", "At most {MAX_IMAGES} images per note."));
    }
    let images = note.images.iter().map(|b| decode_png(b)).collect::<Result<Vec<_>, _>>()?;
    let remind = match note.remind.as_deref().map(str::trim).filter(|r| !r.is_empty()) {
        Some(r) => Some(parse_time(r).ok_or_else(|| trf!("Neplatné datum připomínky „{r}“.", "Invalid reminder date “{r}”."))?),
        None => None,
    };
    let kind = note.kind.as_deref().map(str::trim).filter(|k| !k.is_empty() && *k != DEFAULT_KIND);
    if let Some(k) = kind.filter(|k| !valid_kind(k)) {
        return Err(trf!("Neplatná kategorie „{k}“.", "Invalid category “{k}”."));
    }
    let dir_name = project_dir_name(note.project.as_deref());
    let dir = root.join(&dir_name);
    if !inside(root, &dir) {
        return Err(tr("Neplatný projekt.", "Invalid project.").into());
    }
    std::fs::create_dir_all(&dir).map_err(|e| crate::errors::io(&trf!("Složku {} nejde založit", "Can't create the folder {}", dir.display()), &e))?;

    let base = unique_base(&dir, &format!("{}-{}", now.format("%Y-%m-%d-%H%M%S"), slug(text)));

    // do frontmatteru PŮVODNÍ id z configu (sanitizace je jen pro jméno složky)
    let mut front = Vec::new();
    if let Some(id) = note.project.as_deref().map(str::trim).filter(|_| dir_name != GENERAL) {
        front.push(format!("project: {}", yaml_value(id)));
    }
    front.push(format!("created: {}", iso(now)));
    if let Some(k) = kind {
        front.push(format!("kind: {k}"));
    }
    if let Some(r) = &remind {
        front.push(format!("remind: {}", iso(r)));
    }
    front.push("done: false".into());

    let mut body = format!("\n{text}\n");
    for (i, png) in images.iter().enumerate() {
        let name = format!("{base}-{}.png", i + 1);
        write_atomic(&dir.join(&name), png)?;
        body.push_str(&format!("\n![](./{name})\n"));
    }
    let path = dir.join(format!("{base}.md"));
    write_atomic(&path, render(&front, &body).as_bytes())?;
    Ok(path)
}

/// Všechny poznámky (kořen + jedna úroveň složek projektů), nejnovější první. Poznámka přímo
/// v kořeni je obecná; složky začínající tečkou (`.draft`) se přeskočí.
pub fn scan(root: &Path) -> Vec<NoteView> {
    let mut out = Vec::new();
    let mut dirs: Vec<(PathBuf, Option<String>)> = vec![(root.to_path_buf(), None)];
    if let Ok(rd) = std::fs::read_dir(root) {
        dirs.extend(rd.flatten().filter(|e| e.file_type().is_ok_and(|t| t.is_dir())).filter_map(|e| {
            let name = e.file_name().to_string_lossy().into_owned();
            (!name.starts_with('.')).then(|| (e.path(), Some(name)))
        }));
    }
    for (dir, folder) in dirs {
        let Ok(rd) = std::fs::read_dir(&dir) else { continue };
        for e in rd.flatten() {
            let p = e.path();
            if !p.extension().is_some_and(|x| x.eq_ignore_ascii_case("md")) {
                continue;
            }
            let Ok(meta) = e.metadata() else { continue };
            if !meta.is_file() || meta.len() > MAX_NOTE_BYTES {
                continue;
            }
            // poznámka s rozbitým UTF-8 (cizí editor) se ukáže, jen s náhradními znaky
            let Ok(bytes) = std::fs::read(&p) else { continue };
            let text = String::from_utf8_lossy(&bytes);
            let mtime = meta
                .modified()
                .ok()
                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|d| d.as_millis() as i64)
                .unwrap_or(0);
            out.push(parse_note(&p, &text, folder.as_deref(), mtime));
        }
    }
    out.sort_by(|a, b| b.created_ms.cmp(&a.created_ms).then_with(|| b.path.cmp(&a.path)));
    out
}

/// Nejnovějších `limit` poznámek napříč projekty přímo z disku. Ledge sám čte z paměti
/// (`note_list` nad výsledkem `scan`), tohle je pro nástroje a testy.
#[allow(dead_code)]
pub fn list(root: &Path, limit: usize) -> Vec<NoteView> {
    let mut v = scan(root);
    v.truncate(limit);
    v
}

/// Označí poznámku jako hotovou / nehotovou (atomický přepis, jen řádek `done`).
/// Read-modify-write je pod zámkem pro danou cestu — dvě rychlá kliknutí (nebo UI + peek) se
/// nepřepíšou navzájem.
pub fn set_done(root: &Path, path: &Path, done: bool) -> Result<(), String> {
    let real = check_note_path(root, path)?;
    let lock = path_lock(&real);
    let _g = lock.lock();
    let bytes = crate::fsx::read_retry(&real).map_err(|e| crate::errors::io(tr("Poznámku nejde přečíst", "Can't read the note"), &e))?;
    write_atomic(&real, with_done(&String::from_utf8_lossy(&bytes), done).as_bytes())
}

#[cfg(test)]
mod tests {
    use super::super::testkit::{at, b64, png, tmp_root};
    use super::*;

    #[test]
    fn slugs() {
        assert_eq!(slug("Zavolat Řehořovi kvůli fakturám!\ndruhý řádek"), "zavolat-rehorovi-kvuli-fakturam");
        assert_eq!(slug("\n\n  # Nadpis  "), "nadpis");
        assert_eq!(slug("![](x.png)\nŽluťoučký kůň"), "zlutoucky-kun");
        assert_eq!(slug(""), "poznamka");
        assert_eq!(slug("🔔🔔"), "poznamka");
        assert!(slug(&"a".repeat(200)).len() <= 40);
    }

    #[test]
    fn project_dir_names_are_safe() {
        assert_eq!(project_dir_name(Some("atlas")), "atlas");
        assert_eq!(project_dir_name(Some("Nova-OS")), "nova-os");
        assert_eq!(project_dir_name(None), GENERAL);
        assert_eq!(project_dir_name(Some("  ")), GENERAL);
        for evil in ["..", "../..", "..\\..\\Windows", "C:\\", "C:\\Windows", "/", "/etc/passwd", "\\\\server\\share", "a/../../b", ":"] {
            let d = project_dir_name(Some(evil));
            assert!(!d.contains("..") && !d.contains(['/', '\\', ':']), "{evil} → {d}");
            let root = Path::new("C:\\koren");
            assert!(inside(root, &root.join(&d)), "{evil} → {d}");
        }
        assert_eq!(project_dir_name(Some("CON")), "p-con");
        assert_eq!(project_dir_name(Some("com1")), "p-com1");
        assert_eq!(project_dir_name(Some("computer")), "computer");
    }

    #[test]
    fn create_writes_note_images_and_frontmatter() {
        let root = tmp_root();
        let now = at("2026-10-16T14:03:22+02:00");
        let img = png(10);
        let note = NewNote {
            text: "Zavolat Řehořovi\n\nkvůli fakturám".into(),
            project: Some("atlas".into()),
            remind: Some("2026-10-17T09:00:00+02:00".into()),
            images: vec![b64(&img), format!("data:image/png;base64,{}", b64(&img))],
            kind: None,
        };
        let path = create(root.path(), &note, &now).unwrap();
        assert_eq!(path, root.path().join("atlas").join("2026-10-16-140322-zavolat-rehorovi.md"));
        let text = std::fs::read_to_string(&path).unwrap();
        assert_eq!(
            text,
            "---\nproject: atlas\ncreated: 2026-10-16T14:03:22+02:00\nremind: 2026-10-17T09:00:00+02:00\ndone: false\n---\n\n\
             Zavolat Řehořovi\n\nkvůli fakturám\n\n![](./2026-10-16-140322-zavolat-rehorovi-1.png)\n\n![](./2026-10-16-140322-zavolat-rehorovi-2.png)\n"
        );
        assert_eq!(std::fs::read(root.path().join("atlas/2026-10-16-140322-zavolat-rehorovi-2.png")).unwrap(), img);
        // žádné .tmp nezůstaly
        let left: Vec<_> = std::fs::read_dir(root.path().join("atlas")).unwrap().flatten().filter(|e| e.path().to_string_lossy().ends_with(".tmp")).collect();
        assert!(left.is_empty());

        // stejná vteřina + stejný text → nepřepíše, přidá -2
        let again = create(root.path(), &NewNote { text: "Zavolat Řehořovi".into(), project: Some("atlas".into()), ..Default::default() }, &now).unwrap();
        assert!(again.ends_with("2026-10-16-140322-zavolat-rehorovi-2.md"));
        assert_eq!(std::fs::read_to_string(&path).unwrap(), text);

        // obecná: bez klíče project, ve složce _obecne
        let g = create(root.path(), &NewNote { text: "nápad".into(), ..Default::default() }, &now).unwrap();
        assert_eq!(g.parent().unwrap().file_name().unwrap(), GENERAL);
        assert!(!std::fs::read_to_string(&g).unwrap().contains("project:"));
    }

    #[test]
    fn create_writes_kind_unless_default() {
        let root = tmp_root();
        let now = at("2026-10-16T14:03:22+02:00");
        let k = |kind: Option<&str>| NewNote { text: "x".into(), kind: kind.map(Into::into), ..Default::default() };
        let p = create(root.path(), &k(Some("napad")), &now).unwrap();
        assert_eq!(std::fs::read_to_string(&p).unwrap(), "---\ncreated: 2026-10-16T14:03:22+02:00\nkind: napad\ndone: false\n---\n\nx\n");
        assert_eq!(scan(root.path())[0].kind, "napad");
        for none in [None, Some(DEFAULT_KIND), Some(" ")] {
            let p = create(root.path(), &k(none), &now).unwrap();
            assert!(!std::fs::read_to_string(&p).unwrap().contains("kind"), "{none:?}");
        }
        assert!(create(root.path(), &k(Some("a: b")), &now).is_err());
    }

    #[test]
    fn create_rejects_bad_input_without_writing() {
        let root = tmp_root();
        let now = at("2026-10-16T14:03:22+02:00");
        assert!(create(root.path(), &NewNote { text: "  ".into(), ..Default::default() }, &now).is_err());
        let bad_img = NewNote { text: "x".into(), images: vec![b64(b"not a png at all, just some bytes......")], ..Default::default() };
        assert!(create(root.path(), &bad_img, &now).is_err());
        let bad_remind = NewNote { text: "x".into(), remind: Some("zítra".into()), ..Default::default() };
        assert!(create(root.path(), &bad_remind, &now).is_err());
        assert_eq!(std::fs::read_dir(root.path()).unwrap().count(), 0, "nic se nesmělo zapsat");
    }

    #[test]
    fn create_never_writes_outside_root() {
        let outer = tmp_root();
        let root = outer.path().join("notes");
        std::fs::create_dir_all(&root).unwrap();
        let now = at("2026-10-16T14:03:22+02:00");
        for evil in ["..", "../..", "..\\x", "C:\\", "/", "/tmp", "\\\\srv\\x"] {
            let p = create(&root, &NewNote { text: "x".into(), project: Some(evil.into()), ..Default::default() }, &now).unwrap();
            assert!(p.starts_with(&root), "{evil} → {}", p.display());
            assert!(!p.components().any(|c| matches!(c, Component::ParentDir)));
        }
        // nad kořenem nic nevzniklo
        let siblings: Vec<_> = std::fs::read_dir(outer.path()).unwrap().flatten().map(|e| e.file_name()).collect();
        assert_eq!(siblings, vec![std::ffi::OsString::from("notes")]);
    }

    #[test]
    fn scan_list_and_set_done_roundtrip() {
        let root = tmp_root();
        let a = create(root.path(), &NewNote { text: "starší".into(), project: Some("a".into()), ..Default::default() }, &at("2026-10-15T10:00:00+02:00")).unwrap();
        let b = create(root.path(), &NewNote { text: "novější".into(), ..Default::default() }, &at("2026-10-16T10:00:00+02:00")).unwrap();
        // ručně psaná poznámka (Claude) bez frontmatteru → čas z názvu, projekt ze složky
        std::fs::create_dir_all(root.path().join("orbit")).unwrap();
        std::fs::write(root.path().join("orbit/2026-10-14-080000-rucni.md"), "# Ruční poznámka\n").unwrap();
        std::fs::write(root.path().join("orbit/ignoruj.txt"), "x").unwrap();
        let all = scan(root.path());
        assert_eq!(all.iter().map(|n| n.title.as_str()).collect::<Vec<_>>(), vec!["novější", "starší", "Ruční poznámka"]);
        assert_eq!(all[0].project, None);
        assert_eq!(all[1].project.as_deref(), Some("a"));
        assert_eq!(all[2].project.as_deref(), Some("orbit"));
        assert!(all[2].created.starts_with("2026-10-14T08:00:00"));
        assert_eq!(list(root.path(), 2).len(), 2);

        set_done(root.path(), &a, true).unwrap();
        assert!(scan(root.path()).iter().find(|n| n.path == a.to_string_lossy()).unwrap().done);
        set_done(root.path(), &a, false).unwrap();
        assert!(!scan(root.path()).iter().find(|n| n.path == a.to_string_lossy()).unwrap().done);
        let _ = b;
    }

    #[test]
    fn set_done_refuses_paths_outside_root() {
        let outer = tmp_root();
        let root = outer.path().join("notes");
        std::fs::create_dir_all(root.join("a")).unwrap();
        let victim = outer.path().join("cizi.md");
        std::fs::write(&victim, "---\ndone: false\n---\n").unwrap();
        for p in [victim.clone(), root.join("a").join("..").join("..").join("cizi.md"), PathBuf::from("cizi.md"), root.join("a/x.txt")] {
            assert!(set_done(&root, &p, true).is_err(), "{}", p.display());
        }
        assert_eq!(std::fs::read_to_string(&victim).unwrap(), "---\ndone: false\n---\n");
    }

    #[test]
    fn project_id_is_kept_folder_is_sanitized() {
        let root = tmp_root();
        let now = at("2026-10-16T14:03:22+02:00");
        let p = create(root.path(), &NewNote { text: "x".into(), project: Some("Nova-OS".into()), ..Default::default() }, &now).unwrap();
        assert_eq!(p.parent().unwrap().file_name().unwrap(), "nova-os");
        assert!(std::fs::read_to_string(&p).unwrap().contains("\nproject: Nova-OS\n"));
        let q = create(root.path(), &NewNote { text: "y".into(), project: Some("můj: projekt".into()), ..Default::default() }, &now).unwrap();
        assert!(std::fs::read_to_string(&q).unwrap().contains("\nproject: \"můj: projekt\"\n"));
        // ručně: poznámka v kořeni = obecná, frontmatter má přednost před složkou
        std::fs::write(root.path().join("2026-10-10-100000-koren.md"), "kořen\n").unwrap();
        std::fs::create_dir_all(root.path().join("jina-slozka")).unwrap();
        std::fs::write(root.path().join("jina-slozka/2026-10-10-100000-a.md"), "---\nproject: Orbit\n---\na\n").unwrap();
        let all = scan(root.path());
        let by = |t: &str| all.iter().find(|n| n.title == t).unwrap().project.clone();
        assert_eq!(by("x").as_deref(), Some("Nova-OS"));
        assert_eq!(by("y").as_deref(), Some("můj: projekt"));
        assert_eq!(by("kořen"), None);
        assert_eq!(by("a").as_deref(), Some("Orbit"));
    }

    #[test]
    fn scan_tolerates_bad_utf8_and_skips_dot_dirs() {
        let root = tmp_root();
        std::fs::create_dir_all(root.path().join("p")).unwrap();
        std::fs::write(root.path().join("p/2026-10-10-100000-a.md"), b"cesky \xe8esky\n").unwrap();
        std::fs::create_dir_all(root.path().join(".draft")).unwrap();
        std::fs::write(root.path().join(".draft/x.md"), "skrytá").unwrap();
        let all = scan(root.path());
        assert_eq!(all.len(), 1);
        assert!(all[0].title.starts_with("cesky "));
    }

    #[test]
    fn verbatim_prefix_is_stripped() {
        assert_eq!(strip_verbatim(Path::new(r"\\?\C:\Users\x\a.md")), PathBuf::from(r"C:\Users\x\a.md"));
        assert_eq!(strip_verbatim(Path::new(r"\\?\UNC\srv\share\a.md")), PathBuf::from(r"\\srv\share\a.md"));
        assert_eq!(strip_verbatim(Path::new(r"C:\a.md")), PathBuf::from(r"C:\a.md"));
    }

    #[test]
    fn concurrent_set_done_keeps_file_valid() {
        let root = tmp_root();
        let p = create(root.path(), &NewNote { text: "x".into(), ..Default::default() }, &at("2026-10-16T14:03:22+02:00")).unwrap();
        let hs: Vec<_> = (0..8)
            .map(|i| {
                let (r, p) = (root.path().to_path_buf(), p.clone());
                std::thread::spawn(move || set_done(&r, &p, i % 2 == 0).unwrap())
            })
            .collect();
        hs.into_iter().for_each(|h| h.join().unwrap());
        let text = crate::fsx::read_string_retry(&p).unwrap();
        assert_eq!(text.matches("done:").count(), 1);
        assert!(std::fs::read_dir(p.parent().unwrap()).unwrap().flatten().all(|e| !e.file_name().to_string_lossy().ends_with(".tmp")));
    }
}
