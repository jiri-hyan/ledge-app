//! Přenos profilu WebView2 ze složky dat podle starého identifikátoru do složky podle nového (0.16.0).
//!
//! Tauri dává WebView2 složku `%LOCALAPPDATA%\<identifier>`, profil leží v její podsložce `EBWebView`
//! (localStorage, cookies oken „web“ a `Local State` s klíčem k nim). Po změně identifikátoru by se začínalo
//! s prázdným profilem. Tenhle modul ho **zkopíruje** (nikdy nepřesouvá: stará složka zůstává kvůli návratu na
//! předchozí verzi, úklid až později). Stará složka se hledá maskou `cz.*.ledge` (název se nikde nepíše).
//!
//! Postup ([`run`]): `EBWebView` bez cache ([`keep`]) → `<nová>\EBWebView.migrating` → ověření počtu souborů
//! a bajtů → přejmenování na `EBWebView` → značka [`MARKER`] jako poslední. Cokoli selže = start s čistým
//! profilem + řádek v logu; zbytek `.migrating` se příště smaže a začne se znovu. Běží před prvním WebView
//! (okna vznikají až po `setup`), takže jde o obyčejné kopírování souborů. Je-li stará instance ještě živá
//! (Chromium drží `lockfile`), čeká se ≤ 15 s, pak se migrace odloží ([`DEFERRED`]) na další start.
//! **Strop opakování:** každý pokus (odklad i kopie) zvýší počet v [`DEFERRED`] (`{tries, first}`); selhání kopie
//! zapíše [`DEFERRED`] taky, takže „příště znovu“ platí. Po [`MAX_TRIES`] pokusech NEBO po [`MAX_AGE_SECS`] od
//! prvního se migrace vzdá ([`exhausted`]): značka [`MARKER`] s důvodem „vzdáno“ (bez zdroje), nový profil se už
//! neodsouvá a start nečeká (čekání 15 s tedy nejvýš při těch pokusech).
//! Rozhodování jsou čisté funkce ([`find_legacy`], [`plan`], [`keep`], [`is_legacy_name`], [`bump`], [`exhausted`]);
//! I/O jde přes [`Io`].

use serde_json::json;
use std::io;
use std::path::Path;

/// Značka v nové složce: migrace dokončena (JSON `{from, ver, at}`) nebo vzdána
/// (`{from:null, reason:"vzdáno", tries, ver, at}`). Zapisuje se jako poslední.
pub const MARKER: &str = ".migrated-from";
/// Značka v nové složce: migrace čeká na další start (stará instance držela profil / kopie selhala). JSON
/// `{tries, first}`; prázdný soubor ze starších sestavení = 1 pokus, začátek neznámý.
pub const DEFERRED: &str = ".migrate-deferred";
/// Po tolika pokusech (odklad nebo kopie) se migrace vzdá.
pub const MAX_TRIES: u32 = 3;
/// Nebo po tolika sekundách od prvního pokusu (7 dní).
pub const MAX_AGE_SECS: u64 = 7 * 24 * 3600;
const PROFILE: &str = "EBWebView";
const STAGING: &str = "EBWebView.migrating";
const WAIT_MS: u64 = 15_000;
const POLL_MS: u64 = 250;

// ---------- čisté rozhodování ----------

/// Název složky podle masky `cz.*.ledge` (prostřední část neprázdná, jen znaky identifikátoru), ne nový identifikátor.
pub fn is_legacy_name(name: &str, new_id: &str) -> bool {
    let n = name.to_ascii_lowercase();
    if n == new_id.to_ascii_lowercase() {
        return false;
    }
    match n.strip_prefix("cz.").and_then(|r| r.strip_suffix(".ledge")) {
        Some(mid) => !mid.is_empty() && mid.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '.'),
        None => false,
    }
}

/// Kandidát na zdroj: složka s `EBWebView` a čas změny profilu (sekundy, 0 = neznámý).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Candidate {
    pub name: String,
    pub modified: u64,
}

/// Vybraný zdroj a ostatní shody (jen do logu).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pick {
    pub name: String,
    pub others: Vec<String>,
}

/// Nejnovější kandidát podle času změny (při shodě podle názvu, ať je výběr deterministický).
pub fn find_legacy(cands: &[Candidate], new_id: &str) -> Option<Pick> {
    let mut v: Vec<&Candidate> = cands.iter().filter(|c| is_legacy_name(&c.name, new_id)).collect();
    v.sort_by(|a, b| b.modified.cmp(&a.modified).then_with(|| a.name.cmp(&b.name)));
    let (first, rest) = v.split_first()?;
    Some(Pick { name: first.name.clone(), others: rest.iter().map(|c| c.name.clone()).collect() })
}

/// Stav odkladů: kolik pokusů už proběhlo a kdy první (sekundy, 0 = neznámé).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Deferral {
    pub tries: u32,
    pub first: u64,
}

/// Přečte obsah [`DEFERRED`]; prázdný nebo nečitelný = 1 pokus, začátek neznámý.
pub fn parse_deferral(text: &str) -> Deferral {
    let v: serde_json::Value = serde_json::from_str(text).unwrap_or(serde_json::Value::Null);
    let tries = v["tries"].as_u64().map_or(1, |n| n.min(u32::MAX as u64) as u32).max(1);
    Deferral { tries, first: v["first"].as_u64().unwrap_or(0) }
}

pub fn deferral_text(d: Deferral) -> String {
    json!({ "tries": d.tries, "first": d.first }).to_string()
}

/// Stav po dalším pokusu (`prev` = dosavadní odklad, `None` = první pokus).
pub fn bump(prev: Option<Deferral>, now: u64) -> Deferral {
    match prev {
        None => Deferral { tries: 1, first: now },
        Some(d) => Deferral { tries: d.tries.saturating_add(1), first: if d.first == 0 { now } else { d.first } },
    }
}

/// Je strop vyčerpán (počet pokusů nebo stáří od prvního)?
pub fn exhausted(d: Deferral, now: u64) -> bool {
    d.tries >= MAX_TRIES || (d.first != 0 && now.saturating_sub(d.first) >= MAX_AGE_SECS)
}

/// Co o stavu ví plánovač.
#[derive(Debug, Clone, Copy, Default)]
pub struct Facts {
    pub has_legacy: bool,
    /// v nové složce je značka dokončení
    pub done: bool,
    /// v nové složce je značka odložení
    pub deferred: bool,
    /// strop odkladů je vyčerpán ([`exhausted`]; má smysl jen s `deferred`)
    pub exhausted: bool,
    /// nový profil už má `Local State` (WebView ho už používal)
    pub new_has_state: bool,
    /// nová složka `EBWebView` existuje (i prázdná / bez `Local State`)
    pub new_profile_dir: bool,
    /// stará instance drží profil
    pub old_in_use: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Why {
    NoLegacy,
    Done,
    NewHasProfile,
    /// strop odkladů vyčerpán: migrace se vzdává
    GaveUp,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Plan {
    Skip(Why),
    /// Kopírovat; `aside` = nový `EBWebView` (čistý, vzniklý po odložení) se nejdřív odsune vedle, nic se nemaže.
    Copy { aside: bool },
    /// Stará instance běží: čekat, po vypršení odložit.
    Busy,
}

pub fn plan(f: Facts) -> Plan {
    if !f.has_legacy {
        return Plan::Skip(Why::NoLegacy);
    }
    if f.done {
        return Plan::Skip(Why::Done);
    }
    if f.deferred && f.exhausted {
        return Plan::Skip(Why::GaveUp);
    }
    // nový profil s uloženým stavem má přednost (nikdy se nepřepisuje) — leda že vznikl až po odložení
    if f.new_has_state && !f.deferred {
        return Plan::Skip(Why::NewHasProfile);
    }
    if f.old_in_use {
        return Plan::Busy;
    }
    Plan::Copy { aside: f.new_profile_dir }
}

/// Patří soubor/složka (relativní cesta uvnitř `EBWebView`) do kopie? Cache a provozní soubory ne.
pub fn keep(rel: &str) -> bool {
    const DENY: [&str; 10] = [
        "cache", "code cache", "gpucache", "grshadercache", "shadercache", "dawncache", "graphitedawncache", "crashpad",
        "lockfile", "lock",
    ];
    !rel.split(['\\', '/']).any(|c| {
        let c = c.to_ascii_lowercase();
        DENY.contains(&c.as_str()) || c.starts_with("browsermetrics")
    })
}

/// Text značky dokončení.
pub fn marker_text(from: &str, ver: &str, at: u64) -> String {
    json!({ "from": from, "ver": ver, "at": at }).to_string()
}

/// Text značky „vzdáno“: bez zdroje, s důvodem a počtem pokusů.
pub fn gave_up_text(tries: u32, ver: &str, at: u64) -> String {
    json!({ "from": null, "reason": "vzdáno", "tries": tries, "ver": ver, "at": at }).to_string()
}

// ---------- I/O ----------

/// Vstupy z OS, v testech podvržené.
pub struct Io<'a> {
    pub rename: &'a dyn Fn(&Path, &Path) -> io::Result<()>,
    pub copy: &'a dyn Fn(&Path, &Path) -> io::Result<u64>,
    /// drží někdo soubor (nelze otevřít ke čtení)?
    pub locked: &'a dyn Fn(&Path) -> bool,
    pub sleep_ms: &'a dyn Fn(u64),
}

/// Skutečné I/O.
pub fn at_startup(new_id: &str, ver: &str) -> Vec<String> {
    let Some(local) = crate::dirs::localappdata() else { return Vec::new() };
    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
    let io = Io {
        rename: &|a, b| std::fs::rename(a, b),
        copy: &|a, b| std::fs::copy(a, b),
        // živý WebView2 drží `lockfile` se sdílením pro čtení → otevření jen ke čtení projde; zámek pozná až žádost o zápis (nic se nezapisuje)
        locked: &|p| p.exists() && std::fs::OpenOptions::new().read(true).write(true).open(p).is_err(),
        sleep_ms: &|ms| std::thread::sleep(std::time::Duration::from_millis(ms)),
    };
    // selhání migrace nesmí zabránit startu — ani panika
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| run(&local, new_id, ver, now, &io))).unwrap_or_else(|_| vec!["webdata: migrace profilu WebView spadla, start s čistým profilem".into()])
}

fn candidates(local: &Path, new_id: &str) -> Vec<Candidate> {
    let Ok(rd) = std::fs::read_dir(local) else { return Vec::new() };
    rd.flatten()
        .filter_map(|e| {
            let name = e.file_name().to_string_lossy().into_owned();
            if !is_legacy_name(&name, new_id) {
                return None;
            }
            let prof = e.path().join(PROFILE);
            if !prof.is_dir() {
                return None;
            }
            let modified = std::fs::metadata(prof.join("Local State"))
                .or_else(|_| std::fs::metadata(&prof))
                .and_then(|m| m.modified())
                .ok()
                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                .map_or(0, |d| d.as_secs());
            Some(Candidate { name, modified })
        })
        .collect()
}

/// Provede migraci v `local` (= `%LOCALAPPDATA%`); vrací řádky do logu (prázdné = nebylo co dělat).
pub fn run(local: &Path, new_id: &str, ver: &str, now: u64, io: &Io) -> Vec<String> {
    let Some(pick) = find_legacy(&candidates(local, new_id), new_id) else { return Vec::new() };
    let mut log = Vec::new();
    if !pick.others.is_empty() {
        log.push(format!("webdata: víc starých složek profilu, bere se nejnovější ({}), ostatní: {}", pick.name, pick.others.join(", ")));
    }
    let old = local.join(&pick.name).join(PROFILE);
    let new_dir = local.join(new_id);
    let new_prof = new_dir.join(PROFILE);
    let deferral = std::fs::read_to_string(new_dir.join(DEFERRED)).ok().map(|t| parse_deferral(&t));
    let facts = |busy: bool| Facts {
        has_legacy: true,
        done: new_dir.join(MARKER).exists(),
        deferred: deferral.is_some(),
        exhausted: deferral.is_some_and(|d| exhausted(d, now)),
        new_has_state: new_prof.join("Local State").exists(),
        new_profile_dir: new_prof.exists(),
        old_in_use: busy,
    };
    let mut waited = 0;
    let p = loop {
        let busy = (io.locked)(&old.join("lockfile"));
        let p = plan(facts(busy));
        if p != Plan::Busy || waited >= WAIT_MS {
            break p;
        }
        (io.sleep_ms)(POLL_MS);
        waited += POLL_MS;
    };
    match p {
        Plan::Skip(Why::NoLegacy) | Plan::Skip(Why::Done) => {}
        Plan::Skip(Why::NewHasProfile) => log.push("webdata: nový profil už existuje, starý se nekopíruje".into()),
        Plan::Skip(Why::GaveUp) => log.push(give_up(&new_dir, &pick.name, ver, now, deferral.map_or(MAX_TRIES, |d| d.tries))),
        Plan::Busy => {
            let d = bump(deferral, now);
            let r = std::fs::create_dir_all(&new_dir).and_then(|_| std::fs::write(new_dir.join(DEFERRED), deferral_text(d)));
            log.push(match &r {
                Ok(()) => format!("webdata: stará instance drží profil ({waited} ms), přenos se odkládá na další start (pokus {}/{MAX_TRIES})", d.tries),
                Err(e) => format!("webdata: stará instance drží profil, odklad se nezapsal: {e}"),
            });
            if r.is_ok() && exhausted(d, now) {
                log.push(give_up(&new_dir, &pick.name, ver, now, d.tries));
            }
        }
        Plan::Copy { aside } => {
            // pokus se započítá předem: selhání i pád uprostřed ho nevrátí, po stropu se migrace vzdá
            let d = bump(deferral, now);
            let _ = std::fs::create_dir_all(&new_dir).and_then(|_| std::fs::write(new_dir.join(DEFERRED), deferral_text(d)));
            match copy_profile(&old, &new_dir, &pick.name, ver, now, aside, io) {
                Ok(s) => {
                    log.push(format!("webdata: profil WebView zkopírován ({s}), starý ponechán"));
                    if aside {
                        log.push(format!(
                            "webdata: POZOR dosavadní profil WebView (vzniklý po odkladu) odsunut do {} a nasazen starý ({})",
                            new_dir.join(format!("{PROFILE}.clean-{now}")).display(),
                            pick.name
                        ));
                    }
                }
                Err(e) => {
                    log.push(format!("webdata: přenos profilu selhal ({e}), start s čistým profilem; příště znovu (pokus {}/{MAX_TRIES})", d.tries));
                    if exhausted(d, now) {
                        log.push(give_up(&new_dir, &pick.name, ver, now, d.tries));
                    }
                }
            }
        }
    }
    log
}

/// Konečná značka „vzdáno“: nový profil zůstává, starý se nedotkne, DEFERRED pryč.
fn give_up(new_dir: &Path, from: &str, ver: &str, now: u64, tries: u32) -> String {
    match std::fs::create_dir_all(new_dir).and_then(|_| std::fs::write(new_dir.join(MARKER), gave_up_text(tries, ver, now))) {
        Ok(()) => {
            let _ = std::fs::remove_file(new_dir.join(DEFERRED));
            format!("webdata: přenos profilu WebView se vzdal (pokusů {tries}), nový profil zůstává, starý ({from}) ponechán nedotčen")
        }
        Err(e) => format!("webdata: přenos profilu WebView se mělo vzdát, značka se nezapsala: {e}"),
    }
}

fn copy_profile(old: &Path, new_dir: &Path, from: &str, ver: &str, now: u64, aside: bool, io: &Io) -> Result<String, String> {
    let staging = new_dir.join(STAGING);
    let new_prof = new_dir.join(PROFILE);
    if staging.exists() {
        std::fs::remove_dir_all(&staging).map_err(|e| format!("úklid {STAGING}: {e}"))?;
    }
    let fail = |msg: String| {
        let _ = std::fs::remove_dir_all(&staging);
        Err(msg)
    };
    std::fs::create_dir_all(new_dir).map_err(|e| format!("{}: {e}", new_dir.display()))?;
    if let Err(e) = copy_filtered(old, &staging, "", io) {
        return fail(format!("kopie: {e}"));
    }
    match (tally(old, ""), tally(&staging, "")) {
        (Ok(a), Ok(b)) if a == b => {}
        (Ok(a), Ok(b)) => return fail(format!("kopie nesedí: {} souborů/{} B proti {} souborů/{} B", a.0, a.1, b.0, b.1)),
        (Err(e), _) | (_, Err(e)) => return fail(format!("ověření: {e}")),
    }
    let parked = new_dir.join(format!("{PROFILE}.clean-{now}"));
    if aside && let Err(e) = (io.rename)(&new_prof, &parked) {
        return fail(format!("odsunutí čistého profilu: {e}"));
    }
    if let Err(e) = (io.rename)(&staging, &new_prof) {
        if aside {
            let _ = (io.rename)(&parked, &new_prof); // vrátit, co jsme odsunuli
        }
        return fail(format!("přejmenování: {e}"));
    }
    let (files, bytes) = tally(&new_prof, "").unwrap_or_default();
    // značka jako poslední; bez ní by se příště jen poznalo „nový profil existuje“ (stejný výsledek, žádná škoda).
    // Pád mezi přejmenováním a značkou: DEFERRED už nese pokus započtený předem (viz `run`), takže opakování
    // je omezené stropem a nasadí tutéž kopii starého profilu (mezitím nic nezapisovalo — WebView ještě nejel).
    let m = std::fs::write(new_dir.join(MARKER), marker_text(from, ver, now));
    let _ = std::fs::remove_file(new_dir.join(DEFERRED));
    match m {
        Ok(()) => Ok(format!("{files} souborů, {bytes} B")),
        Err(e) => Ok(format!("{files} souborů, {bytes} B; značku se nepodařilo zapsat: {e}")),
    }
}

fn copy_filtered(from: &Path, to: &Path, rel: &str, io: &Io) -> io::Result<()> {
    std::fs::create_dir_all(to)?;
    for entry in std::fs::read_dir(from)? {
        let entry = entry?;
        let name = entry.file_name().to_string_lossy().into_owned();
        let r = if rel.is_empty() { name.clone() } else { format!("{rel}\\{name}") };
        if !keep(&r) {
            continue;
        }
        if entry.file_type()?.is_dir() {
            copy_filtered(&entry.path(), &to.join(&name), &r, io)?;
        } else {
            (io.copy)(&entry.path(), &to.join(&name))?;
        }
    }
    Ok(())
}

/// (počet souborů, bajty) vybraných souborů ([`keep`]) rekurzivně.
fn tally(dir: &Path, rel: &str) -> io::Result<(u64, u64)> {
    let mut out = (0, 0);
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let name = entry.file_name().to_string_lossy().into_owned();
        let r = if rel.is_empty() { name } else { format!("{rel}\\{name}") };
        if !keep(&r) {
            continue;
        }
        if entry.file_type()?.is_dir() {
            let (f, b) = tally(&entry.path(), &r)?;
            out.0 += f;
            out.1 += b;
        } else {
            out.0 += 1;
            out.1 += entry.metadata()?.len();
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;
    use std::fs;
    use std::path::PathBuf;

    const NEW: &str = "app.ledge.desktop";
    const OLD: &str = "cz.acme.ledge";

    fn cand(n: &str, m: u64) -> Candidate {
        Candidate { name: n.into(), modified: m }
    }

    #[test]
    fn legacy_name_table() {
        for (n, want) in [
            ("cz.acme.ledge", true),
            ("CZ.Acme.Ledge", true),
            ("cz.a-b.ledge", true),
            ("cz..ledge", false),
            ("cz.ledge", false),
            ("app.ledge.desktop", false),
            ("Ledge", false),
            ("ledge-public", false),
            ("cz.acme.ledge.bak", false),
            ("cz.ac me.ledge", false),
            ("cz.acme.notch", false),
        ] {
            assert_eq!(is_legacy_name(n, NEW), want, "{n}");
        }
        assert!(!is_legacy_name("cz.acme.ledge", "cz.acme.ledge"), "nový identifikátor není starý");
    }

    #[test]
    fn find_legacy_table() {
        assert_eq!(find_legacy(&[], NEW), None);
        assert_eq!(find_legacy(&[cand("Ledge", 9), cand(NEW, 9)], NEW), None);
        assert_eq!(find_legacy(&[cand("cz.a.ledge", 5)], NEW), Some(Pick { name: "cz.a.ledge".into(), others: vec![] }));
        let p = find_legacy(&[cand("cz.a.ledge", 5), cand("cz.b.ledge", 7), cand("cz.c.ledge", 7)], NEW).unwrap();
        assert_eq!(p.name, "cz.b.ledge", "nejnovější, při shodě podle názvu");
        assert_eq!(p.others, vec!["cz.c.ledge".to_string(), "cz.a.ledge".to_string()]);
    }

    #[test]
    fn plan_table() {
        let f = |g: &dyn Fn(&mut Facts)| {
            let mut x = Facts { has_legacy: true, ..Default::default() };
            g(&mut x);
            plan(x)
        };
        assert_eq!(f(&|x| x.has_legacy = false), Plan::Skip(Why::NoLegacy));
        assert_eq!(f(&|_| {}), Plan::Copy { aside: false });
        assert_eq!(f(&|x| x.new_profile_dir = true), Plan::Copy { aside: true }, "prázdný EBWebView se odsune");
        assert_eq!(f(&|x| x.done = true), Plan::Skip(Why::Done));
        assert_eq!(f(&|x| x.new_has_state = true), Plan::Skip(Why::NewHasProfile));
        assert_eq!(f(&|x| x.old_in_use = true), Plan::Busy);
        assert_eq!(f(&|x| { x.old_in_use = true; x.done = true }), Plan::Skip(Why::Done));
        assert_eq!(f(&|x| { x.old_in_use = true; x.new_has_state = true }), Plan::Skip(Why::NewHasProfile));
        // po odložení vznikl čistý profil → kopie s odsunutím
        assert_eq!(f(&|x| { x.deferred = true; x.new_has_state = true; x.new_profile_dir = true }), Plan::Copy { aside: true });
        // strop vyčerpán → vzdát; bez odkladu `exhausted` nic neznamená
        assert_eq!(f(&|x| { x.deferred = true; x.exhausted = true; x.new_has_state = true }), Plan::Skip(Why::GaveUp));
        assert_eq!(f(&|x| { x.deferred = true; x.exhausted = true; x.old_in_use = true }), Plan::Skip(Why::GaveUp), "nečeká se");
        assert_eq!(f(&|x| { x.deferred = true; x.exhausted = true; x.done = true }), Plan::Skip(Why::Done));
        assert_eq!(f(&|x| x.exhausted = true), Plan::Copy { aside: false });
    }

    #[test]
    fn deferral_table() {
        let d = |tries, first| Deferral { tries, first };
        // čtení: prázdný/nečitelný soubor = 1 pokus, začátek neznámý
        for (t, want) in [("", d(1, 0)), ("x", d(1, 0)), ("{}", d(1, 0)), (r#"{"tries":2,"first":50}"#, d(2, 50)), (r#"{"tries":0}"#, d(1, 0))] {
            assert_eq!(parse_deferral(t), want, "{t:?}");
        }
        assert_eq!(parse_deferral(&deferral_text(d(3, 7))), d(3, 7));
        // další pokus
        assert_eq!(bump(None, 100), d(1, 100));
        assert_eq!(bump(Some(d(1, 100)), 200), d(2, 100), "začátek se zachová");
        assert_eq!(bump(Some(d(1, 0)), 200), d(2, 200), "neznámý začátek se doplní");
        // strop: počet NEBO stáří
        let week = MAX_AGE_SECS;
        for (x, now, want) in [
            (d(1, 100), 100, false),
            (d(MAX_TRIES - 1, 100), 100 + week - 1, false),
            (d(MAX_TRIES, 100), 100, true),
            (d(1, 100), 100 + week, true),
            (d(1, 0), u64::MAX, false),
            (d(1, 500), 100, false),
        ] {
            assert_eq!(exhausted(x, now), want, "{x:?} @{now}");
        }
    }

    #[test]
    fn gave_up_text_has_no_source() {
        let v: serde_json::Value = serde_json::from_str(&gave_up_text(3, "0.16.0", 42)).unwrap();
        assert_eq!((v["from"].is_null(), v["reason"].as_str(), v["tries"].as_u64(), v["at"].as_u64()), (true, Some("vzdáno"), Some(3), Some(42)));
    }

    #[test]
    fn keep_table() {
        for (p, want) in [
            ("Local State", true),
            ("Default\\Local Storage\\leveldb\\000003.log", true),
            ("Default\\Local Storage\\leveldb\\LOCK", false),
            ("Default\\Network\\Cookies", true),
            ("Default\\Cache\\Cache_Data\\f_000001", false),
            ("Default\\Code Cache\\js\\x", false),
            ("Default\\GPUCache", false),
            ("GrShaderCache\\data_0", false),
            ("ShaderCache", false),
            ("Crashpad\\settings.dat", false),
            ("BrowserMetrics-spare.pma", false),
            ("lockfile", false),
            ("Default\\IndexedDB\\x", true),
            ("Default/Cache/x", false),
            ("Default\\Service Worker\\CacheStorage\\a", true),
        ] {
            assert_eq!(keep(p), want, "{p}");
        }
    }

    #[test]
    fn marker_text_is_json() {
        let v: serde_json::Value = serde_json::from_str(&marker_text(OLD, "0.16.0", 42)).unwrap();
        assert_eq!((v["from"].as_str(), v["ver"].as_str(), v["at"].as_u64()), (Some(OLD), Some("0.16.0"), Some(42)));
    }

    // ----- I/O nad dočasnou složkou -----

    struct T {
        _d: tempfile::TempDir,
        local: PathBuf,
    }
    fn t() -> T {
        let d = tempfile::tempdir().unwrap();
        let local = d.path().join("local");
        fs::create_dir_all(&local).unwrap();
        T { _d: d, local }
    }
    fn put(p: &Path, s: &str) {
        fs::create_dir_all(p.parent().unwrap()).unwrap();
        fs::write(p, s).unwrap();
    }
    fn seed_old(t: &T, name: &str) {
        let e = t.local.join(name).join(PROFILE);
        put(&e.join("Local State"), "ls");
        put(&e.join("Default/Local Storage/leveldb/000003.log"), "abc");
        put(&e.join("Default/Local Storage/leveldb/LOCK"), "");
        put(&e.join("Default/Network/Cookies"), "cookie");
        put(&e.join("Default/Cache/Cache_Data/f1"), "cachecache");
        put(&e.join("GPUCache/x"), "g");
    }
    type RenameFn<'a> = &'a dyn Fn(&Path, &Path) -> io::Result<()>;
    type CopyFn<'a> = &'a dyn Fn(&Path, &Path) -> io::Result<u64>;
    fn go(t: &T, locked: &dyn Fn(&Path) -> bool, rename: Option<RenameFn>, copy: Option<CopyFn>) -> (Vec<String>, u64) {
        go_at(t, 100, locked, rename, copy)
    }
    fn go_at(t: &T, now: u64, locked: &dyn Fn(&Path) -> bool, rename: Option<RenameFn>, copy: Option<CopyFn>) -> (Vec<String>, u64) {
        let slept = Cell::new(0);
        let std_rename = |a: &Path, b: &Path| fs::rename(a, b);
        let std_copy = |a: &Path, b: &Path| fs::copy(a, b);
        let sleep = |ms: u64| slept.set(slept.get() + ms);
        let io = Io { rename: rename.unwrap_or(&std_rename), copy: copy.unwrap_or(&std_copy), locked, sleep_ms: &sleep };
        let log = run(&t.local, NEW, "0.16.0", now, &io);
        (log, slept.get())
    }
    fn free(_: &Path) -> bool {
        false
    }
    fn newp(t: &T) -> PathBuf {
        t.local.join(NEW)
    }

    #[test]
    fn no_legacy_is_silent_and_creates_nothing() {
        let t = t();
        fs::create_dir_all(t.local.join("Ledge")).unwrap();
        let (log, slept) = go(&t, &free, None, None);
        assert!(log.is_empty() && slept == 0);
        assert!(!newp(&t).exists());
    }

    #[test]
    fn copies_state_without_cache_and_marks_last() {
        let t = t();
        seed_old(&t, OLD);
        let (log, _) = go(&t, &free, None, None);
        assert!(log.len() == 1 && log[0].contains("zkopírován"), "{log:?}");
        let n = newp(&t).join(PROFILE);
        assert_eq!(fs::read_to_string(n.join("Default/Network/Cookies")).unwrap(), "cookie");
        assert!(n.join("Local State").exists() && n.join("Default/Local Storage/leveldb/000003.log").exists());
        assert!(!n.join("Default/Cache").exists() && !n.join("GPUCache").exists());
        assert!(!n.join("Default/Local Storage/leveldb/LOCK").exists());
        assert!(!newp(&t).join(STAGING).exists());
        assert!(newp(&t).join(MARKER).exists());
        assert!(t.local.join(OLD).join(PROFILE).join("Default/Cache/Cache_Data/f1").exists(), "starý nedotčen");
    }

    #[test]
    fn idempotent_after_success() {
        let t = t();
        seed_old(&t, OLD);
        go(&t, &free, None, None);
        let (log, _) = go(&t, &free, None, None);
        assert!(log.is_empty(), "{log:?}");
    }

    #[test]
    fn new_profile_with_state_is_never_overwritten() {
        let t = t();
        seed_old(&t, OLD);
        put(&newp(&t).join(PROFILE).join("Local State"), "mine");
        let (log, _) = go(&t, &free, None, None);
        assert!(log[0].contains("už existuje"));
        assert_eq!(fs::read_to_string(newp(&t).join(PROFILE).join("Local State")).unwrap(), "mine");
        assert!(!newp(&t).join(MARKER).exists());
    }

    #[test]
    fn empty_new_dir_and_empty_profile_dir_are_fine() {
        let t = t();
        seed_old(&t, OLD);
        fs::create_dir_all(newp(&t).join(PROFILE)).unwrap(); // Tauri/WebView založil prázdné
        let (log, _) = go(&t, &free, None, None);
        assert!(log[0].contains("zkopírován"), "{log:?}");
        assert!(newp(&t).join(PROFILE).join("Local State").exists());
        assert!(newp(&t).join("EBWebView.clean-100").exists(), "prázdný se odsunul, nesmazal");
    }

    #[test]
    fn leftover_staging_is_discarded_and_redone() {
        let t = t();
        seed_old(&t, OLD);
        put(&newp(&t).join(STAGING).join("junk"), "x");
        let (log, _) = go(&t, &free, None, None);
        assert!(log[0].contains("zkopírován"));
        assert!(!newp(&t).join(PROFILE).join("junk").exists());
    }

    #[test]
    fn busy_waits_then_defers_and_next_start_copies_over_clean_profile() {
        let t = t();
        seed_old(&t, OLD);
        let (log, slept) = go(&t, &|_| true, None, None);
        assert!(log[0].contains("odkládá"), "{log:?}");
        assert_eq!(slept, WAIT_MS);
        assert!(newp(&t).join(DEFERRED).exists() && !newp(&t).join(MARKER).exists());
        // aplikace jela dál s čistým profilem
        put(&newp(&t).join(PROFILE).join("Local State"), "clean");
        let (log, slept) = go(&t, &free, None, None);
        assert_eq!(slept, 0);
        assert!(log[0].contains("zkopírován"), "{log:?}");
        assert_eq!(fs::read_to_string(newp(&t).join(PROFILE).join("Local State")).unwrap(), "ls");
        assert_eq!(fs::read_to_string(newp(&t).join("EBWebView.clean-100").join("Local State")).unwrap(), "clean");
        assert!(newp(&t).join(MARKER).exists() && !newp(&t).join(DEFERRED).exists());
    }

    #[test]
    fn busy_that_clears_in_time_copies() {
        let t = t();
        seed_old(&t, OLD);
        let n = Cell::new(0);
        let (log, slept) = go(&t, &|_| { n.set(n.get() + 1); n.get() <= 3 }, None, None);
        assert!(log[0].contains("zkopírován"), "{log:?}");
        assert_eq!(slept, 3 * POLL_MS);
    }

    #[test]
    fn copy_failure_keeps_old_cleans_staging_and_retries() {
        let t = t();
        seed_old(&t, OLD);
        let bad = |_: &Path, _: &Path| -> io::Result<u64> { Err(io::Error::other("disk plný")) };
        let (log, _) = go(&t, &free, None, Some(&bad));
        assert!(log[0].contains("selhal") && log[0].contains("disk plný"), "{log:?}");
        assert!(!newp(&t).join(STAGING).exists() && !newp(&t).join(PROFILE).exists() && !newp(&t).join(MARKER).exists());
        // „příště znovu“ platí: značka odkladu je zapsaná (jinak by WebView založil čistý profil a migrace by se už nezopakovala)
        assert_eq!(parse_deferral(&fs::read_to_string(newp(&t).join(DEFERRED)).unwrap()), Deferral { tries: 1, first: 100 });
        // WebView mezitím založil čistý profil → další start ho odsune a nasadí starý
        put(&newp(&t).join(PROFILE).join("Local State"), "clean");
        let (log, _) = go(&t, &free, None, None);
        assert!(log[0].contains("zkopírován") && log[1].contains("POZOR") && log[1].contains("EBWebView.clean-100"), "{log:?}");
        assert!(newp(&t).join(MARKER).exists() && !newp(&t).join(DEFERRED).exists());
    }

    #[test]
    fn repeated_copy_failure_gives_up_after_max_tries_and_never_touches_new_profile() {
        let t = t();
        seed_old(&t, OLD);
        let bad = |_: &Path, _: &Path| -> io::Result<u64> { Err(io::Error::other("zamčeno")) };
        for i in 1..=MAX_TRIES {
            let (log, _) = go(&t, &free, None, Some(&bad));
            assert!(log[0].contains(&format!("pokus {i}/{MAX_TRIES}")), "{log:?}");
            // uživatel mezitím pracuje na novém profilu
            put(&newp(&t).join(PROFILE).join("Local State"), "mine");
        }
        // poslední pokus vzdal: konečná značka bez zdroje, žádný odklad, nový profil nedotčen
        let m: serde_json::Value = serde_json::from_str(&fs::read_to_string(newp(&t).join(MARKER)).unwrap()).unwrap();
        assert!(m["from"].is_null() && m["reason"] == "vzdáno" && m["tries"] == MAX_TRIES, "{m}");
        assert!(!newp(&t).join(DEFERRED).exists());
        assert_eq!(fs::read_to_string(newp(&t).join(PROFILE).join("Local State")).unwrap(), "mine");
        // další starty: ticho, bez čekání, bez kopírování
        let (log, slept) = go(&t, &|_| true, None, None);
        assert!(log.is_empty() && slept == 0, "{log:?}");
        assert!(!newp(&t).join("EBWebView.clean-100").exists());
    }

    #[test]
    fn permanently_busy_waits_at_most_max_tries_times() {
        let t = t();
        seed_old(&t, OLD);
        let mut total = 0;
        for i in 1..=MAX_TRIES {
            let (log, slept) = go(&t, &|_| true, None, None);
            total += slept;
            assert_eq!(slept, WAIT_MS, "pokus {i}");
            assert!(log[0].contains(&format!("pokus {i}/{MAX_TRIES}")), "{log:?}");
            put(&newp(&t).join(PROFILE).join("Local State"), "mine");
        }
        assert_eq!(total, MAX_TRIES as u64 * WAIT_MS);
        assert!(newp(&t).join(MARKER).exists() && !newp(&t).join(DEFERRED).exists(), "vzdáno");
        let (log, slept) = go(&t, &|_| true, None, None);
        assert!(log.is_empty() && slept == 0, "po vzdání se nečeká: {log:?}");
        assert_eq!(fs::read_to_string(newp(&t).join(PROFILE).join("Local State")).unwrap(), "mine");
    }

    #[test]
    fn deferral_older_than_max_age_gives_up_without_waiting_or_copying() {
        let t = t();
        seed_old(&t, OLD);
        put(&newp(&t).join(PROFILE).join("Local State"), "mine");
        fs::write(newp(&t).join(DEFERRED), deferral_text(Deferral { tries: 1, first: 1000 })).unwrap();
        let (log, slept) = go_at(&t, 1000 + MAX_AGE_SECS, &|_| true, None, None);
        assert!(log[0].contains("vzdal") && slept == 0, "{log:?}");
        assert!(newp(&t).join(MARKER).exists() && !newp(&t).join(DEFERRED).exists());
        assert_eq!(fs::read_to_string(newp(&t).join(PROFILE).join("Local State")).unwrap(), "mine");
        // mladší odklad se ještě zkouší
        let t2 = t_with_deferral(1000);
        let (log, _) = go_at(&t2, 1000 + MAX_AGE_SECS - 1, &free, None, None);
        assert!(log[0].contains("zkopírován"), "{log:?}");
    }

    fn t_with_deferral(first: u64) -> T {
        let t = t();
        seed_old(&t, OLD);
        put(&newp(&t).join(PROFILE).join("Local State"), "mine");
        fs::write(newp(&t).join(DEFERRED), deferral_text(Deferral { tries: 1, first })).unwrap();
        t
    }

    #[test]
    fn legacy_empty_deferred_file_counts_as_one_try() {
        let t = t();
        seed_old(&t, OLD);
        fs::create_dir_all(newp(&t)).unwrap();
        fs::write(newp(&t).join(DEFERRED), "").unwrap();
        let bad = |_: &Path, _: &Path| -> io::Result<u64> { Err(io::Error::other("x")) };
        let (log, _) = go(&t, &free, None, Some(&bad));
        assert!(log[0].contains("pokus 2/"), "{log:?}");
        assert_eq!(parse_deferral(&fs::read_to_string(newp(&t).join(DEFERRED)).unwrap()), Deferral { tries: 2, first: 100 });
    }

    #[test]
    fn rename_failure_leaves_no_marker_and_restores_aside() {
        let t = t();
        seed_old(&t, OLD);
        put(&newp(&t).join(PROFILE).join("Local State"), "clean");
        fs::write(newp(&t).join(DEFERRED), "").unwrap();
        // první přejmenování (odsunutí) projde, druhé (staging → EBWebView) selže
        let calls = Cell::new(0);
        let ren = |a: &Path, b: &Path| -> io::Result<()> {
            calls.set(calls.get() + 1);
            if calls.get() == 2 { Err(io::Error::from(io::ErrorKind::PermissionDenied)) } else { fs::rename(a, b) }
        };
        let (log, _) = go(&t, &free, Some(&ren), None);
        assert!(log[0].contains("selhal"), "{log:?}");
        assert_eq!(fs::read_to_string(newp(&t).join(PROFILE).join("Local State")).unwrap(), "clean", "odsunuté vráceno");
        assert!(!newp(&t).join(MARKER).exists() && !newp(&t).join(STAGING).exists());
    }

    #[test]
    fn two_legacy_folders_pick_newest_and_log() {
        let t = t();
        seed_old(&t, "cz.aaa.ledge");
        seed_old(&t, "cz.bbb.ledge");
        put(&t.local.join("cz.bbb.ledge").join(PROFILE).join("Local State"), "newer");
        let past = std::time::SystemTime::now() - std::time::Duration::from_secs(3600);
        fs::File::options().write(true).open(t.local.join("cz.aaa.ledge").join(PROFILE).join("Local State")).unwrap().set_modified(past).unwrap();
        let (log, _) = go(&t, &free, None, None);
        assert!(log[0].contains("nejnovější (cz.bbb.ledge)"), "{log:?}");
        assert_eq!(fs::read_to_string(newp(&t).join(PROFILE).join("Local State")).unwrap(), "newer");
    }

    #[test]
    fn legacy_without_profile_dir_is_ignored() {
        let t = t();
        fs::create_dir_all(t.local.join(OLD).join("Other")).unwrap();
        let (log, _) = go(&t, &free, None, None);
        assert!(log.is_empty());
    }
}
