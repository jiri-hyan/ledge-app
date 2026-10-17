//! Úpravy poznámky přímo v Ledge: čtení (tělo + obrázky jako data URL), uložení s ochranou proti
//! přepsání (hash obsahu — sync mohl soubor mezitím změnit), uložení jako kopie, přepnutí úkolu,
//! rozepsaná úprava (`.draft\edit.json`) a filtr pro kartu / stránku Poznámky.
//! Závisí na `notes` (frontmatter, `check_note_path`, zámky, atomický zápis) a `md` (úkoly, obrázky).
//! Soubor se nikdy nepřejmenuje ani nepřesune; neznámé klíče frontmatteru a `created` zůstanou.

use super::*;
use std::collections::BTreeMap;

/// Otisk obsahu pro detekci změny na disku (SHA-256, prvních 16 bajtů hex).
pub fn hash(bytes: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    Sha256::digest(bytes)[..16].iter().map(|b| format!("{b:02x}")).collect()
}

/// Tělo za frontmatterem bez jednoho úvodního prázdného řádku (ten píše `create`): (úvod, zbytek).
/// Čísla řádků úkolů (`md::tasks`) se počítají ve zbytku.
pub fn body_view(body: &str) -> (&str, &str) {
    let lead = if body.starts_with("\r\n") { 2 } else if body.starts_with('\n') { 1 } else { 0 };
    body.split_at(lead)
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct NoteImage {
    /// odkaz jak stojí v Markdownu (`./a-1.png`)
    pub src: String,
    /// `data:image/…;base64,…`
    pub data: String,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct NoteDetail {
    #[serde(flatten)]
    pub view: NoteView,
    /// tělo bez frontmatteru (viz `body_view`)
    pub body: String,
    /// otisk souboru při čtení — `save` / `toggle_task` ho chtějí zpátky
    pub hash: String,
    pub image_data: Vec<NoteImage>,
}

/// Obrázky dohromady nejvýš tolik (data URL jdou přes IPC).
const MAX_IMAGES_TOTAL: u64 = 40 * 1024 * 1024;

fn mime(p: &Path) -> Option<&'static str> {
    match p.extension()?.to_string_lossy().to_ascii_lowercase().as_str() {
        "png" => Some("image/png"),
        "jpg" | "jpeg" => Some("image/jpeg"),
        "gif" => Some("image/gif"),
        "webp" => Some("image/webp"),
        _ => None,
    }
}

/// Relativní obrázky poznámky jako data URL — jen soubory uvnitř kořene, známé typy, se stropem.
fn load_images(real_root: &Path, dir: &Path, body: &str) -> Vec<NoteImage> {
    let mut out = Vec::new();
    let mut total = 0u64;
    for src in md::image_refs(body).into_iter().take(MAX_IMAGES) {
        let p = dir.join(src.trim_start_matches("./").trim_start_matches(".\\"));
        let Some(m) = mime(&p) else { continue };
        let Ok(real) = p.canonicalize() else { continue };
        let Ok(meta) = std::fs::metadata(&real) else { continue };
        if !real.starts_with(real_root) || !meta.is_file() || meta.len() > MAX_IMAGE as u64 || total + meta.len() > MAX_IMAGES_TOTAL {
            continue;
        }
        let Ok(bytes) = std::fs::read(&real) else { continue };
        total += bytes.len() as u64;
        out.push(NoteImage { src: src.to_string(), data: format!("data:{m};base64,{}", b64_encode(&bytes)) });
    }
    out
}

fn mtime_ms(p: &Path) -> i64 {
    std::fs::metadata(p)
        .and_then(|m| m.modified())
        .ok()
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map_or(0, |d| d.as_millis() as i64)
}

/// Ověřená poznámka: (kanonická cesta, kanonický kořen).
fn resolve(root: &Path, path: &Path) -> Result<(PathBuf, PathBuf), String> {
    let real = check_note_path(root, path)?;
    let real_root = root.canonicalize().map_err(|e| e.to_string())?;
    Ok((real, real_root))
}

fn read_bytes(real: &Path) -> Result<Vec<u8>, String> {
    let meta = std::fs::metadata(real).map_err(|e| e.to_string())?;
    if meta.len() > MAX_NOTE_BYTES {
        return Err(tr("Poznámka je větší než 1 MB — otevři ji v editoru.", "The note is larger than 1 MB — open it in an editor.").into());
    }
    std::fs::read(real).map_err(|e| e.to_string())
}

/// Složka poznámky je obecná (kořen nebo `_obecne`) — bez projektu stačí klíč vynechat.
fn general_folder(real_root: &Path, real: &Path) -> bool {
    let parent = real.parent().unwrap_or(real_root);
    parent == real_root || parent.file_name().is_some_and(|n| n == GENERAL)
}

fn detail(path: &Path, real: &Path, real_root: &Path, bytes: &[u8]) -> NoteDetail {
    let text = String::from_utf8_lossy(bytes);
    let parent = real.parent().unwrap_or(real_root);
    let folder = (parent != real_root).then(|| parent.file_name().map(|n| n.to_string_lossy().into_owned())).flatten();
    let view = parse_note(path, &text, folder.as_deref(), mtime_ms(real));
    let (_, body) = split(&text);
    let body = body_view(body).1;
    NoteDetail { view, body: body.to_string(), hash: hash(bytes), image_data: load_images(real_root, parent, body) }
}

/// Poznámka k úpravě. `path` = cesta z UI (klíč v kartě), ověřená přes `check_note_path`.
pub fn read(root: &Path, path: &Path) -> Result<NoteDetail, String> {
    let (real, real_root) = resolve(root, path)?;
    Ok(detail(path, &real, &real_root, &read_bytes(&real)?))
}

/// Uložení z editoru.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveNote {
    pub path: String,
    /// `hash` z `read` — jiný obsah na disku = `Changed`, nic se nezapíše
    #[serde(default)]
    pub expected_hash: String,
    pub body: String,
    #[serde(default)]
    pub project: Option<String>,
    #[serde(default)]
    pub kind: Option<String>,
    /// lokální čas z výběru připomínky nebo ISO s offsetem; prázdné = bez připomínky
    #[serde(default)]
    pub remind: Option<String>,
    /// nové PNG (base64) — připojí se na konec jako `![](./<jméno>-N.png)`
    #[serde(default)]
    pub images: Vec<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(tag = "outcome", rename_all = "camelCase")]
pub enum SaveOutcome {
    Saved { note: Box<NoteDetail> },
    /// soubor se od otevření změnil (sync, jiný editor) — UI nabídne načíst znovu / uložit jako kopii
    Changed,
}

/// Projekt, kategorie, připomínka a `updated` do frontmatteru; ostatní řádky beze změny.
/// `general` = soubor leží v obecné složce (jinak „bez projektu“ = `project: _obecne`, aby starší
/// Ledge nepřiřadil projekt podle složky).
pub fn apply_meta(front: &mut Vec<String>, general: bool, req: &SaveNote, now: &DateTime<FixedOffset>) -> Result<(), String> {
    let trimmed = |o: &Option<String>| o.as_deref().map(str::trim).filter(|v| !v.is_empty()).map(str::to_string);
    let remind = match trimmed(&req.remind) {
        Some(r) => Some(parse_time(&r).ok_or_else(|| trf!("Neplatné datum připomínky „{r}“.", "Invalid reminder date “{r}”."))?),
        None => None,
    };
    let kind = trimmed(&req.kind);
    if let Some(k) = kind.as_deref().filter(|k| !valid_kind(k)) {
        return Err(trf!("Neplatná kategorie „{k}“.", "Invalid category “{k}”."));
    }
    match trimmed(&req.project).filter(|p| p != GENERAL) {
        Some(p) => set(front, "project", &yaml_value(&p)),
        None if general => unset(front, "project"),
        None => set(front, "project", GENERAL),
    }
    match kind {
        Some(k) if k == DEFAULT_KIND && get(front, "kind").is_none() => {}
        Some(k) => set(front, "kind", &k),
        None => unset(front, "kind"),
    }
    match remind {
        Some(r) => set(front, "remind", &iso(&r)),
        None => unset(front, "remind"),
    }
    set(front, "updated", &iso(now));
    Ok(())
}

/// Tělo k zápisu: úvodní prázdný řádek, text, nové obrázky (zapíše je do `dir` jako `<base>-N.png`).
fn compose_body(dir: &Path, base: &str, text: &str, pngs: &[Vec<u8>]) -> Result<String, String> {
    let mut body = format!("\n{}\n", text.trim_end());
    let mut n = 1;
    for png in pngs {
        while dir.join(format!("{base}-{n}.png")).exists() {
            n += 1;
        }
        let name = format!("{base}-{n}.png");
        write_atomic(&dir.join(&name), png)?;
        body.push_str(&format!("\n![](./{name})\n"));
    }
    Ok(body)
}

fn check_content(req: &SaveNote) -> Result<Vec<Vec<u8>>, String> {
    if req.body.trim().is_empty() && req.images.is_empty() {
        return Err(tr("Prázdná poznámka — smazat ji jde jen ručně.", "Empty note — it can only be deleted by hand.").into());
    }
    if req.images.len() > MAX_IMAGES {
        return Err(trf!("Nejvýš {MAX_IMAGES} obrázků na poznámku.", "At most {MAX_IMAGES} images per note."));
    }
    req.images.iter().map(|b| decode_png(b)).collect()
}

/// Přepíše poznámku (atomicky, pod zámkem cesty). Jiný obsah na disku než při čtení = `Changed`.
pub fn save(root: &Path, req: &SaveNote, now: &DateTime<FixedOffset>) -> Result<SaveOutcome, String> {
    let path = Path::new(&req.path);
    let (real, real_root) = resolve(root, path)?;
    let lock = path_lock(&real);
    let _g = lock.lock();
    let bytes = read_bytes(&real)?;
    if hash(&bytes) != req.expected_hash {
        return Ok(SaveOutcome::Changed);
    }
    let pngs = check_content(req)?;
    let text = String::from_utf8_lossy(&bytes);
    let mut front = split(&text).0.unwrap_or_default();
    apply_meta(&mut front, general_folder(&real_root, &real), req, now)?;
    let dir = real.parent().ok_or_else(|| tr("Poznámka bez složky.", "Note without a folder."))?;
    let base = real.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
    let body = compose_body(dir, &base, &req.body, &pngs)?;
    let out = render(&front, &body);
    write_atomic(&real, out.as_bytes())?;
    Ok(SaveOutcome::Saved { note: Box::new(detail(path, &real, &real_root, out.as_bytes())) })
}

/// Uloží úpravu jako NOVOU poznámku vedle původní (konflikt se syncem) — obrázky `./…` dál sedí.
/// Původní soubor se nemění. Vrací cestu nové poznámky.
pub fn save_copy(root: &Path, req: &SaveNote, now: &DateTime<FixedOffset>) -> Result<PathBuf, String> {
    let (real, real_root) = resolve(root, Path::new(&req.path))?;
    let pngs = check_content(req)?;
    let mut front = vec![format!("created: {}", iso(now))];
    apply_meta(&mut front, general_folder(&real_root, &real), req, now)?;
    unset(&mut front, "updated");
    front.push("done: false".into());
    let dir = real.parent().ok_or_else(|| tr("Poznámka bez složky.", "Note without a folder."))?;
    let base = unique_base(dir, &format!("{}-{}", now.format("%Y-%m-%d-%H%M%S"), slug(&req.body)));
    let body = compose_body(dir, &base, &req.body, &pngs)?;
    let name = format!("{base}.md");
    write_atomic(&dir.join(&name), render(&front, &body).as_bytes())?;
    // cesta ve tvaru, jak ji zná UI (vedle původní), ne kanonická `\\?\…`
    Ok(Path::new(&req.path).with_file_name(name))
}

/// Přepne úkol na řádku `line` (řádek těla z `read`, od 0) — jen ten znak a `updated`.
pub fn toggle_task(root: &Path, path: &Path, line: usize, checked: bool, expected_hash: &str, now: &DateTime<FixedOffset>) -> Result<SaveOutcome, String> {
    let (real, real_root) = resolve(root, path)?;
    let lock = path_lock(&real);
    let _g = lock.lock();
    let bytes = read_bytes(&real)?;
    if hash(&bytes) != expected_hash {
        return Ok(SaveOutcome::Changed);
    }
    let text = String::from_utf8_lossy(&bytes);
    let (front, body) = split(&text);
    let mut front = front.unwrap_or_default();
    let (lead, rest) = body_view(body);
    let new_rest = md::toggle_line(rest, line, checked)?;
    set(&mut front, "updated", &iso(now));
    let out = render(&front, &format!("{lead}{new_rest}"));
    write_atomic(&real, out.as_bytes())?;
    Ok(SaveOutcome::Saved { note: Box::new(detail(path, &real, &real_root, out.as_bytes())) })
}

// ---------- filtr karty / stránky ----------

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct NoteQuery {
    /// id kategorie; `None` = všechny
    pub kind: Option<String>,
    /// id projektu, `_obecne` = bez projektu, `None` = všechny
    pub project: Option<String>,
    /// i hotové (na konci, nejnovější první)
    pub include_done: bool,
    pub limit: Option<usize>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct QueryResult {
    pub notes: Vec<NoteView>,
    /// kolik poznámek filtru odpovídá (bez stropu `limit`)
    pub total: usize,
}

/// Výběr pro záložku: nehotové jako karta (připomínky do konce dneška první), pak hotové.
/// `all` je seřazené nejnovější první (jak vrací `scan`); klonuje se jen to, co se vrátí.
pub fn query(all: &[NoteView], q: &NoteQuery, today_end_ms: i64) -> QueryResult {
    let hit = |n: &&NoteView| {
        q.kind.as_deref().is_none_or(|k| n.kind == k)
            && match q.project.as_deref() {
                None => true,
                Some(GENERAL) => n.project.is_none(),
                Some(p) => n.project.as_deref() == Some(p),
            }
    };
    let today = |n: &NoteView| !n.done && n.remind_ms.is_some_and(|r| r < today_end_ms);
    let mut first: Vec<&NoteView> = all.iter().filter(hit).filter(|n| today(n)).collect();
    first.sort_by_key(|n| n.remind_ms);
    let open = all.iter().filter(hit).filter(|n| !n.done && !today(n));
    let done = all.iter().filter(hit).filter(|n| n.done && q.include_done);
    let picked: Vec<&NoteView> = first.into_iter().chain(open).chain(done).collect();
    let total = picked.len();
    QueryResult { notes: picked.into_iter().take(q.limit.unwrap_or(usize::MAX)).cloned().collect(), total }
}

/// Počty nehotových poznámek podle kategorie (záložky karty) — počítá se jednou po přenačtení.
pub fn open_kind_counts(all: &[NoteView]) -> BTreeMap<String, usize> {
    let mut m = BTreeMap::new();
    for n in all.iter().filter(|n| !n.done) {
        *m.entry(n.kind.clone()).or_insert(0) += 1;
    }
    m
}

// ---------- rozepsaná úprava ----------

/// Rozepsaná úprava existující poznámky (`<root>\.draft\edit.json`) — přežije zavření šuplíku
/// (Win+Shift+S ukradne fokus) i zrušení WebView. Nové obrázky se do ní neukládají.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(default, rename_all = "camelCase")]
pub struct EditDraft {
    pub path: String,
    /// otisk souboru, nad kterým se upravuje (při uložení se porovná s diskem)
    pub hash: String,
    pub text: String,
    pub project: String,
    pub kind: String,
    pub remind: String,
}

const EDIT_DRAFT: &str = "edit.json";

pub fn edit_draft_save(root: &Path, d: Option<&EditDraft>) -> Result<(), String> {
    let dir = draft_dir(root);
    match d {
        Some(d) => {
            std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
            write_atomic(&dir.join(EDIT_DRAFT), serde_json::to_string(d).map_err(|e| e.to_string())?.as_bytes())
        }
        None => {
            let _ = std::fs::remove_file(dir.join(EDIT_DRAFT));
            let _ = std::fs::remove_dir(&dir); // jen když je prázdná
            Ok(())
        }
    }
}

pub fn edit_draft_load(root: &Path) -> Option<EditDraft> {
    let b = std::fs::read(draft_dir(root).join(EDIT_DRAFT)).ok()?;
    serde_json::from_slice::<EditDraft>(&b).ok().filter(|d| !d.path.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp_root() -> tempfile::TempDir {
        tempfile::Builder::new().prefix("ledge-edit-").tempdir_in(std::env::temp_dir()).unwrap()
    }

    fn at(s: &str) -> DateTime<FixedOffset> {
        DateTime::parse_from_rfc3339(s).unwrap()
    }

    fn req(path: &Path, hash: &str, body: &str) -> SaveNote {
        SaveNote { path: path.to_string_lossy().into_owned(), expected_hash: hash.into(), body: body.into(), ..Default::default() }
    }

    const NOTE: &str = "---\nproject: a\nautor: claude\ntags:\n  - x\ncreated: 2026-10-16T14:03:22+02:00\nremind: 2026-10-17T09:00:00+02:00\ndone: false\nzz: 1\n---\n\nText\n\n- [ ] úkol\n\n![](./n-1.png)\n";

    #[test]
    fn apply_meta_table() {
        let now = at("2026-10-18T10:00:00+02:00");
        let base: Vec<String> = ["project: a", "x: 1", "  - y", "kind: napad", "done: false"].map(String::from).to_vec();
        // projekt, kategorie, připomínka, obecná složka, očekávaný frontmatter
        type Case<'a> = (Option<&'a str>, Option<&'a str>, Option<&'a str>, bool, &'a [&'a str]);
        let cases: &[Case] = &[
            (Some("b"), Some("napad"), None, false, &["project: b", "x: 1", "  - y", "kind: napad", "done: false"]),
            (None, Some("ukol"), None, true, &["x: 1", "  - y", "kind: ukol", "done: false"]),
            (None, Some("ukol"), None, false, &["project: _obecne", "x: 1", "  - y", "kind: ukol", "done: false"]),
            (Some("Můj projekt"), None, None, false, &["project: \"Můj projekt\"", "x: 1", "  - y", "done: false"]),
            (Some("a"), Some("cizi-z-druheho-pc"), Some("2026-10-20T09:00:00+02:00"), false, &["project: a", "x: 1", "  - y", "kind: cizi-z-druheho-pc", "done: false", "remind: 2026-10-20T09:00:00+02:00"]),
        ];
        for (project, kind, remind, general, want) in cases {
            let mut f = base.clone();
            let r = SaveNote { project: project.map(Into::into), kind: kind.map(Into::into), remind: remind.map(Into::into), ..Default::default() };
            apply_meta(&mut f, *general, &r, &now).unwrap();
            let mut want: Vec<String> = want.iter().map(|s| s.to_string()).collect();
            want.push("updated: 2026-10-18T10:00:00+02:00".into());
            assert_eq!(f, want, "{project:?} {kind:?}");
        }
        // výchozí kategorie se nepřipisuje, když klíč chybí
        let mut f = vec!["done: false".to_string()];
        apply_meta(&mut f, true, &SaveNote { kind: Some(DEFAULT_KIND.into()), ..Default::default() }, &now).unwrap();
        assert_eq!(f, vec!["done: false", "updated: 2026-10-18T10:00:00+02:00"]);
        for bad in [SaveNote { kind: Some("a b".into()), ..Default::default() }, SaveNote { remind: Some("zítra".into()), ..Default::default() }] {
            assert!(apply_meta(&mut f.clone(), true, &bad, &now).is_err());
        }
    }

    #[test]
    fn kind_default_and_project_marker_parse() {
        let v = parse_note(Path::new("C:\\n\\a\\x.md"), "---\ndone: false\n---\nx", Some("a"), 0);
        assert_eq!((v.kind.as_str(), v.project.as_deref()), (DEFAULT_KIND, Some("a")));
        let v = parse_note(Path::new("C:\\n\\a\\x.md"), "---\nproject: _obecne\nkind: feedback\nupdated: 2026-10-18T10:00:00+02:00\n---\n- [x] a\n- [ ] b", Some("a"), 0);
        assert_eq!((v.kind.as_str(), v.project, v.updated.as_deref()), ("feedback", None, Some("2026-10-18T10:00:00+02:00")));
        assert_eq!((v.tasks_done, v.tasks_total), (1, 2));
    }

    #[test]
    fn save_roundtrip_keeps_unknown_keys_created_and_name() {
        let root = tmp_root();
        let dir = root.path().join("a");
        std::fs::create_dir_all(&dir).unwrap();
        let p = dir.join("n.md");
        std::fs::write(&p, NOTE).unwrap();
        std::fs::write(dir.join("n-1.png"), [0x89, b'P', b'N', b'G']).unwrap();
        let d = read(root.path(), &p).unwrap();
        assert_eq!(d.body, "Text\n\n- [ ] úkol\n\n![](./n-1.png)\n");
        assert_eq!(d.image_data.len(), 1);
        assert!(d.image_data[0].data.starts_with("data:image/png;base64,"));
        assert_eq!(d.view.path, p.to_string_lossy());

        // beze změny textu: tělo bajtově stejné, přibude jen updated
        let now = at("2026-10-18T10:00:00+02:00");
        let mut r = req(&p, &d.hash, &d.body);
        r.project = Some("a".into());
        r.remind = Some("2026-10-17T09:00:00+02:00".into());
        let SaveOutcome::Saved { note } = save(root.path(), &r, &now).unwrap() else { panic!("changed") };
        let text = std::fs::read_to_string(&p).unwrap();
        assert_eq!(text, NOTE.replace("zz: 1\n", "zz: 1\nupdated: 2026-10-18T10:00:00+02:00\n"));
        assert_eq!(note.hash, hash(text.as_bytes()));

        // nová kategorie + text, bez připomínky; stejný soubor, jiný nevznikl
        let mut r = req(&p, &note.hash, "Nový text");
        r.project = Some("a".into());
        r.kind = Some("napad".into());
        save(root.path(), &r, &now).unwrap();
        let text = std::fs::read_to_string(&p).unwrap();
        assert!(text.contains("autor: claude\ntags:\n  - x\ncreated: 2026-10-16T14:03:22+02:00\n") && text.contains("kind: napad") && !text.contains("remind"), "{text}");
        assert!(text.ends_with("---\n\nNový text\n"), "{text}");
        assert_eq!(std::fs::read_dir(&dir).unwrap().count(), 2, "žádný tmp ani kopie");
        // starý Ledge (`with_done`) nové klíče zachová
        let old = with_done(&text, true);
        assert!(old.contains("\nkind: napad\n") && old.contains("\nupdated: 2026-10-18T10:00:00+02:00\n") && old.contains("\ndone: true\n"), "{old}");
    }

    #[test]
    fn save_detects_change_on_disk() {
        let root = tmp_root();
        let p = root.path().join("n.md");
        std::fs::write(&p, NOTE).unwrap();
        let d = read(root.path(), &p).unwrap();
        // mezitím přišel sync
        std::fs::write(&p, NOTE.replace("Text", "Text z druhého PC")).unwrap();
        let now = at("2026-10-18T10:00:00+02:00");
        assert_eq!(save(root.path(), &req(&p, &d.hash, "moje"), &now).unwrap(), SaveOutcome::Changed);
        assert_eq!(toggle_task(root.path(), &p, 2, true, &d.hash, &now).unwrap(), SaveOutcome::Changed);
        assert!(std::fs::read_to_string(&p).unwrap().contains("z druhého PC"), "nic se nepřepsalo");
        // kopie vedle, originál netknutý
        let mut r = req(&p, &d.hash, "moje verze");
        r.kind = Some("feedback".into());
        let c = save_copy(root.path(), &r, &now).unwrap();
        assert_ne!(c, p);
        let copy = std::fs::read_to_string(&c).unwrap();
        assert_eq!(copy, "---\ncreated: 2026-10-18T10:00:00+02:00\nkind: feedback\ndone: false\n---\n\nmoje verze\n");
        assert!(c.file_name().unwrap().to_string_lossy().starts_with("2026-10-18-100000-moje-verze"));
    }

    #[test]
    fn toggle_task_flips_right_line_only() {
        let root = tmp_root();
        let p = root.path().join("n.md");
        std::fs::write(&p, NOTE).unwrap();
        let d = read(root.path(), &p).unwrap();
        let now = at("2026-10-18T10:00:00+02:00");
        assert!(toggle_task(root.path(), &p, 0, true, &d.hash, &now).is_err(), "řádek 0 není úkol");
        let SaveOutcome::Saved { note } = toggle_task(root.path(), &p, 2, true, &d.hash, &now).unwrap() else { panic!() };
        assert_eq!((note.view.tasks_done, note.view.tasks_total), (1, 1));
        let text = std::fs::read_to_string(&p).unwrap();
        assert_eq!(text, NOTE.replace("- [ ] úkol", "- [x] úkol").replace("zz: 1\n", "zz: 1\nupdated: 2026-10-18T10:00:00+02:00\n"));
    }

    #[test]
    fn save_appends_images_and_refuses_outside_paths() {
        let outer = tmp_root();
        let root = outer.path().join("notes");
        std::fs::create_dir_all(&root).unwrap();
        let p = root.join("n.md");
        std::fs::write(&p, "---\ndone: false\n---\n\nx\n").unwrap();
        std::fs::write(root.join("n-1.png"), b"stary").unwrap();
        let d = read(&root, &p).unwrap();
        let mut png = vec![0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];
        png.extend([7u8; 40]);
        let mut r = req(&p, &d.hash, "x");
        r.images = vec![b64_encode(&png)];
        save(&root, &r, &at("2026-10-18T10:00:00+02:00")).unwrap();
        assert!(std::fs::read_to_string(&p).unwrap().ends_with("\nx\n\n![](./n-2.png)\n"));
        assert_eq!(std::fs::read(root.join("n-2.png")).unwrap(), png);
        assert_eq!(std::fs::read(root.join("n-1.png")).unwrap(), b"stary");

        let victim = outer.path().join("cizi.md");
        std::fs::write(&victim, "x").unwrap();
        for bad in [victim.clone(), root.join("..").join("cizi.md")] {
            assert!(read(&root, &bad).is_err());
            assert!(save(&root, &req(&bad, &hash(b"x"), "y"), &at("2026-10-18T10:00:00+02:00")).is_err());
        }
        // obrázek mimo kořen se nenačte
        std::fs::write(&p, "---\n---\n![](./../ven.png)\n![](C:/x.png)").unwrap();
        std::fs::write(outer.path().join("ven.png"), b"x").unwrap();
        assert!(read(&root, &p).unwrap().image_data.is_empty());
    }

    #[test]
    fn query_filters_and_orders() {
        let mk = |t: &str, kind: &str, project: Option<&str>, remind: Option<i64>, done: bool| NoteView {
            path: t.into(),
            project: project.map(Into::into),
            created: String::new(),
            created_ms: 0,
            remind: None,
            remind_ms: remind,
            done,
            title: t.into(),
            images: 0,
            kind: kind.into(),
            updated: None,
            tasks_done: 0,
            tasks_total: 0,
        };
        let all = [
            mk("nova", "napad", Some("a"), None, false),
            mk("hotova", "napad", Some("a"), None, true),
            mk("dnes", "napad", None, Some(500), false),
            mk("info", DEFAULT_KIND, None, None, false),
            mk("zitra", "napad", Some("b"), Some(5_000), false),
        ];
        let titles = |q: NoteQuery| query(&all, &q, 1_000).notes.into_iter().map(|n| n.title).collect::<Vec<_>>();
        assert_eq!(titles(NoteQuery { kind: Some("napad".into()), ..Default::default() }), ["dnes", "nova", "zitra"]);
        assert_eq!(titles(NoteQuery { kind: Some("napad".into()), include_done: true, ..Default::default() }), ["dnes", "nova", "zitra", "hotova"]);
        assert_eq!(titles(NoteQuery { project: Some(GENERAL.into()), ..Default::default() }), ["dnes", "info"]);
        assert_eq!(titles(NoteQuery { project: Some("a".into()), include_done: true, ..Default::default() }), ["nova", "hotova"]);
        let r = query(&all, &NoteQuery { limit: Some(2), ..Default::default() }, 1_000);
        assert_eq!((r.notes.len(), r.total), (2, 4));
        let c = open_kind_counts(&all);
        assert_eq!((c.get("napad"), c.get(DEFAULT_KIND)), (Some(&3), Some(&1)));
    }

    #[test]
    fn edit_draft_roundtrip() {
        let root = tmp_root();
        assert!(edit_draft_load(root.path()).is_none());
        let d = EditDraft { path: "C:\\n\\a.md".into(), hash: "ab".into(), text: "x".into(), ..Default::default() };
        edit_draft_save(root.path(), Some(&d)).unwrap();
        assert_eq!(edit_draft_load(root.path()), Some(d));
        // koncept nové poznámky edit.json nesmaže a naopak
        draft_save_meta(root.path(), &DraftMeta { text: "nová".into(), ..Default::default() }).unwrap();
        draft_clear(root.path()).unwrap();
        assert!(edit_draft_load(root.path()).is_some());
        edit_draft_save(root.path(), None).unwrap();
        assert!(edit_draft_load(root.path()).is_none());
        assert!(!root.path().join(".draft").exists());
    }
}
