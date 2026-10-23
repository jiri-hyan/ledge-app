//! Cache stažených instalátorů Ledge: tiché stažení nové verze předem, úklid a návrat k předchozí verzi.
//!
//! **Kde.** `<local>/updates/<tag>/<asset>` + `<local>/updates/<tag>/release.json` ([`Manifest`]: SHA256
//! z poznámek, `digest` GitHubu, poznámky k vydání). `<local>` = `actionrun::local_dir()`
//! (`%LOCALAPPDATA%\Ledge`, v testech dočasná složka) — ne `%TEMP%`: ten čistí Windows (Storage Sense)
//! a instalátor předchozí verze musí přežít do návratu. Instalace jde do `%LOCALAPPDATA%\Ledge` taky,
//! ale šablona NSIS při aktualizaci maže jen své soubory, ne podsložky (odinstalace s „Ponechat
//! nastavení“ odškrtnutým smaže celé `%LOCALAPPDATA%\Ledge` — i s cache, správně).
//!
//! **Stav souboru.** Stahuje se do `<asset>.part`, po ověření SHA256 (`update::verify`) se přejmenuje
//! a teprve pak se zapíše manifest. Položka je „hotová“ jen s manifestem i souborem; cokoli jiného
//! (přerušené stažení, cizí složka) úklid smaže. Před spuštěním se SHA256 počítá znovu.
//!
//! **Co se drží** ([`plan_cleanup`]): instalátor běžící verze (po příští aktualizaci z něj bude návrat),
//! instalátor pro návrat ([`plan_rollback`]) a nejnovější stažená verze novější než běžící. Nic víc —
//! na disku jsou tak nejvýš tři instalátory (~3 MB každý).
//!
//! **Průběh** ([`progress_due`]): UI se obnoví jen při změně celého procenta (bez známé velikosti po
//! 512 KiB) — žádné časovače, jen skutečné bajty.

use crate::update::{Release, Version, parse_version};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

pub const MANIFEST: &str = "release.json";
/// Podsložka `<local>` s cache instalátorů.
pub const DIR: &str = "updates";
/// Bez známé velikosti obnovit průběh po tolika bajtech.
const STEP_UNKNOWN: u64 = 512 * 1024;

/// Ověřovací údaje a poznámky staženého vydání (`release.json` vedle instalátoru).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Manifest {
    pub tag: String,
    pub version: String,
    pub asset: String,
    pub sha256: String,
    #[serde(default)]
    pub digest: Option<String>,
    #[serde(default)]
    pub notes: String,
    #[serde(default)]
    pub published: String,
}

impl Manifest {
    pub fn of(rel: &Release) -> Self {
        Self {
            tag: rel.tag.clone(),
            version: rel.version.clone(),
            asset: rel.asset.clone(),
            sha256: rel.sha256.clone(),
            digest: rel.digest.clone(),
            notes: rel.notes.clone(),
            published: rel.published.clone(),
        }
    }

    /// Vydání pro `update::verify` (adresa ke stažení už není potřeba).
    pub fn release(&self) -> Release {
        Release {
            tag: self.tag.clone(),
            version: self.version.clone(),
            notes: self.notes.clone(),
            published: self.published.clone(),
            asset: self.asset.clone(),
            sha256: self.sha256.clone(),
            digest: self.digest.clone(),
            download_url: None,
        }
    }

    /// Patří manifest k vydání `rel` (stejný tag, soubor i otisky)?
    pub fn matches(&self, rel: &Release) -> bool {
        self.tag == rel.tag && self.asset == rel.asset && self.sha256.eq_ignore_ascii_case(&rel.sha256) && self.digest == rel.digest
    }
}

/// Jedna podsložka cache.
#[derive(Debug, Clone, PartialEq)]
pub struct Entry {
    /// jméno složky (tag)
    pub name: String,
    pub manifest: Option<Manifest>,
    /// instalátor z manifestu na disku je
    pub has_file: bool,
}

impl Entry {
    /// Verze hotové položky (manifest + soubor + tag souhlasí se jménem složky a je čisté x.y.z).
    pub fn version(&self) -> Option<Version> {
        let m = self.manifest.as_ref().filter(|m| self.has_file && m.tag == self.name)?;
        parse_version(&m.tag)
    }
}

/// Podsložky cache (soubory v kořeni — `seen.json` — se ignorují). Chybí-li složka, je prázdná.
pub fn scan(root: &Path) -> Vec<Entry> {
    let Ok(rd) = std::fs::read_dir(root) else { return Vec::new() };
    let mut out: Vec<Entry> = rd
        .flatten()
        .filter(|e| e.file_type().is_ok_and(|t| t.is_dir()))
        .map(|e| {
            let dir = e.path();
            let manifest = std::fs::read_to_string(dir.join(MANIFEST)).ok().and_then(|s| serde_json::from_str::<Manifest>(&s).ok());
            // jen `*-setup.exe` s bezpečným jménem — podvržený manifest s `x.cmd` se nikdy nespustí (`cmd /c`)
            let has_file = manifest.as_ref().is_some_and(|m| crate::update::safe_asset(&m.asset) && dir.join(&m.asset).is_file());
            Entry { name: e.file_name().to_string_lossy().into_owned(), manifest, has_file }
        })
        .collect();
    out.sort_by(|a, b| a.name.cmp(&b.name));
    out
}

/// Jméno složky, které jde bezpečně smazat: `[A-Za-z0-9._-]`, aspoň jeden alfanumerický znak a nekončí
/// tečkou ani mezerou — Windows (`GetFullPathNameW`) koncové tečky ořízne, takže `...` by se stalo
/// samotnou `updates` a `remove_dir_all` by smazal celou cache.
fn safe_name(s: &str) -> bool {
    s.bytes().all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'_' | b'-'))
        && s.bytes().any(|b| b.is_ascii_alphanumeric())
        && !s.ends_with(['.', ' '])
}

/// Instalátor hotové položky.
pub fn installer(root: &Path, e: &Entry) -> Option<PathBuf> {
    let m = e.manifest.as_ref()?;
    e.version().map(|_| root.join(&e.name).join(&m.asset))
}

/// Hotová položka pro vydání `rel` (shodný manifest), bez přepočtu SHA256.
pub fn cached<'a>(entries: &'a [Entry], rel: &Release) -> Option<&'a Entry> {
    entries.iter().find(|e| e.version().is_some() && e.manifest.as_ref().is_some_and(|m| m.matches(rel)))
}

/// Hotová položka s verzí `v`.
pub fn of_version(entries: &[Entry], v: Version) -> Option<&Entry> {
    entries.iter().find(|e| e.version() == Some(v))
}

/// Jak se vrátit k předchozí verzi.
#[derive(Debug, Clone, PartialEq)]
pub enum Rollback {
    /// ověřený instalátor je v cache (složka = tag)
    Local { tag: String, version: Version },
    /// předchozí verzi známe, instalátor ne → na klik stáhnout vydání `v<verze>` se stejným ověřením
    Remote { version: Version },
    /// není kam (čistá instalace, nebo už běží starší verze po návratu)
    None,
}

/// Kam vede „Vrátit předchozí verzi“. `previous` = verze, která běžela před touhle (`updseen`). Čistá funkce.
/// - předchozí je starší: z cache, jinak ze stažení podle tagu;
/// - předchozí je novější (právě jsme se vrátili): nic — zpět na novou verzi vede běžná aktualizace;
/// - předchozí neznáme: nejnovější starší instalátor v cache, jinak nic.
pub fn plan_rollback(entries: &[Entry], current: Version, previous: Option<Version>) -> Rollback {
    let local = |v: Version| of_version(entries, v).map(|e| Rollback::Local { tag: e.name.clone(), version: v });
    match previous {
        Some(p) if p < current => local(p).unwrap_or(Rollback::Remote { version: p }),
        Some(_) => Rollback::None,
        None => entries.iter().filter_map(Entry::version).filter(|v| *v < current).max().and_then(local).unwrap_or(Rollback::None),
    }
}

/// Které složky smazat. Drží se: běžící verze, cíl návratu, a z novějších jen `pending` (nejnovější
/// dostupná) — neznáme-li ji (start před první kontrolou), nejnovější novější. Všechno ostatní (starší,
/// přerušené, cizí) pryč — kromě `busy`: složka právě běžícího stažení (ještě bez manifestu). Čistá funkce.
pub fn plan_cleanup(entries: &[Entry], current: Version, rollback: &Rollback, pending: Option<Version>, busy: Option<&str>) -> Vec<String> {
    let newest_newer = entries.iter().filter_map(Entry::version).filter(|v| *v > current).max();
    let pending = pending.or(newest_newer);
    let keep = |v: Version| v == current || Some(v) == pending || matches!(rollback, Rollback::Local { version, .. } if *version == v);
    entries.iter().filter(|e| !e.version().is_some_and(keep) && Some(e.name.as_str()) != busy).map(|e| e.name.clone()).collect()
}

/// Smaže složky z [`plan_cleanup`] (jen přímé podsložky `root`, jména bez oddělovačů). Vrátí počet.
pub fn remove(root: &Path, names: &[String]) -> usize {
    names
        .iter()
        .filter(|n| safe_name(n))
        .filter(|n| std::fs::remove_dir_all(root.join(n)).is_ok())
        .count()
}

/// Uložit manifest atomicky (tmp + rename).
pub fn write_manifest(dir: &Path, m: &Manifest) -> Result<(), String> {
    let tmp = dir.join(format!("{MANIFEST}.tmp"));
    let body = serde_json::to_string_pretty(m).map_err(|e| e.to_string())?;
    std::fs::write(&tmp, body).map_err(|e| crate::errors::io(&tmp.display().to_string(), &e))?;
    std::fs::rename(&tmp, dir.join(MANIFEST)).map_err(|e| crate::errors::io(&dir.display().to_string(), &e))
}

/// Obnovit průběh v UI? Celá procenta (se známou velikostí), jinak po 512 KiB. Čistá funkce.
pub fn progress_due(last: u64, got: u64, total: Option<u64>) -> bool {
    match total {
        Some(t) if t > 0 => pct(last, t) != pct(got, t),
        _ => last / STEP_UNKNOWN != got / STEP_UNKNOWN,
    }
}

fn pct(n: u64, total: u64) -> u64 {
    n.min(total).saturating_mul(100) / total
}

/// Stav tichého stažení dostupné verze (`Snapshot.update.fetch`).
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum Fetch {
    /// nic se nestahuje (vypnuto, nic nového, nebo čeká na kontrolu)
    Idle,
    /// `total` = velikost z hlavičky; bez ní UI ukáže neurčitý průběh
    Downloading { got: u64, total: Option<u64> },
    /// instalátor je stažený a ověřený — „Restartovat a aktualizovat“
    Ready,
    /// lidská věta; znovu jen na klik „Zkusit znovu“ (žádné tiché smyčky)
    Failed { duvod: String },
}

/// Co udělat s dostupným vydáním po kontrole.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Step {
    Nothing,
    /// je v cache → hned připraveno, bez sítě
    MarkReady,
    Download,
}

/// Rozhodnutí po kontrole. `auto` = tiché stažení zapnuté (a kontrola taky), `user` = klik „Zkusit
/// znovu“ (stáhne i s vypnutým tichým stažením). Selhání se samo neopakuje; stahování běží jen jedno. Čistá funkce.
pub fn next_step(auto: bool, user: bool, has_release: bool, in_cache: bool, fetch: &Fetch) -> Step {
    if !has_release || matches!(fetch, Fetch::Downloading { .. }) {
        return Step::Nothing;
    }
    if in_cache {
        return if matches!(fetch, Fetch::Ready) { Step::Nothing } else { Step::MarkReady };
    }
    match fetch {
        Fetch::Failed { .. } if !user => Step::Nothing,
        _ if user || auto => Step::Download,
        _ => Step::Nothing,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn v(s: &str) -> Version {
        parse_version(s).unwrap()
    }

    fn man(tag: &str) -> Manifest {
        Manifest {
            tag: tag.into(),
            version: tag.trim_start_matches('v').into(),
            asset: format!("App_{}_x64-setup.exe", tag.trim_start_matches('v')),
            sha256: "a".repeat(64),
            digest: None,
            notes: "- feat: x".into(),
            published: String::new(),
        }
    }

    fn done(tag: &str) -> Entry {
        Entry { name: tag.into(), manifest: Some(man(tag)), has_file: true }
    }

    fn partial(tag: &str) -> Entry {
        Entry { name: tag.into(), manifest: None, has_file: false }
    }

    #[test]
    fn entry_is_complete_only_with_manifest_file_and_matching_tag() {
        assert_eq!(done("v0.2.14").version(), Some(v("0.2.14")));
        assert_eq!(partial("v0.2.14").version(), None);
        assert_eq!(Entry { has_file: false, ..done("v0.2.14") }.version(), None);
        // složka jinak pojmenovaná než tag v manifestu = podvržená / rozbitá
        assert_eq!(Entry { name: "v0.2.15".into(), ..done("v0.2.14") }.version(), None);
        assert_eq!(done("latest").version(), None);
    }

    #[test]
    fn rollback_plan_table() {
        let cur = v("0.2.15");
        #[rustfmt::skip]
        let cases: Vec<(Vec<Entry>, Option<&str>, Rollback)> = vec![
            // předchozí v cache → místně
            (vec![done("v0.2.14"), done("v0.2.15")], Some("0.2.14"), Rollback::Local { tag: "v0.2.14".into(), version: v("0.2.14") }),
            // předchozí není v cache (ruční instalace, starý mechanismus) → stáhnout podle tagu, ne jiný starší
            (vec![done("v0.2.10"), done("v0.2.15")], Some("0.2.14"), Rollback::Remote { version: v("0.2.14") }),
            (vec![partial("v0.2.14")], Some("0.2.14"), Rollback::Remote { version: v("0.2.14") }),
            // právě po návratu (předchozí je novější) → nic dalšího staršího
            (vec![done("v0.2.13"), done("v0.2.16")], Some("0.2.16"), Rollback::None),
            (vec![done("v0.2.15")], Some("0.2.15"), Rollback::None),
            // předchozí neznáme → nejnovější starší z cache
            (vec![done("v0.2.12"), done("v0.2.13"), done("v0.2.16")], None, Rollback::Local { tag: "v0.2.13".into(), version: v("0.2.13") }),
            (vec![done("v0.2.16"), partial("v0.2.14")], None, Rollback::None),
            (vec![], None, Rollback::None),
        ];
        for (entries, prev, want) in cases {
            assert_eq!(plan_rollback(&entries, cur, prev.map(v)), want, "{prev:?} {entries:?}");
        }
    }

    #[test]
    fn cleanup_plan_table() {
        let cur = v("0.2.15");
        let all = || vec![done("v0.2.12"), done("v0.2.14"), done("v0.2.15"), done("v0.2.16"), done("v0.2.17"), partial("v0.2.18"), partial("cizi")];
        let local14 = Rollback::Local { tag: "v0.2.14".into(), version: v("0.2.14") };
        #[rustfmt::skip]
        let cases: Vec<(Rollback, Option<&str>, Vec<&str>)> = vec![
            // drží běžící, návrat a dostupnou novější; přerušené a cizí pryč
            (local14.clone(), Some("0.2.17"), vec!["cizi", "v0.2.12", "v0.2.16", "v0.2.18"]),
            // dostupná je 0.2.16 (0.2.17 stažená dřív a pak stažená z vydání) → drží 0.2.16
            (local14.clone(), Some("0.2.16"), vec!["cizi", "v0.2.12", "v0.2.17", "v0.2.18"]),
            // před první kontrolou: nejnovější novější zůstane
            (local14.clone(), None, vec!["cizi", "v0.2.12", "v0.2.16", "v0.2.18"]),
            // návrat se stáhne z vydání → žádná starší v cache není potřeba
            (Rollback::Remote { version: v("0.2.14") }, None, vec!["cizi", "v0.2.12", "v0.2.14", "v0.2.16", "v0.2.18"]),
            (Rollback::None, Some("0.2.15"), vec!["cizi", "v0.2.12", "v0.2.14", "v0.2.16", "v0.2.17", "v0.2.18"]),
        ];
        for (rb, pending, want) in cases {
            let mut got = plan_cleanup(&all(), cur, &rb, pending.map(v), None);
            got.sort();
            assert_eq!(got, want, "{rb:?} {pending:?}");
        }
        assert!(plan_cleanup(&[], cur, &Rollback::None, None, None).is_empty());
        // složka právě běžícího stažení (bez manifestu) se nemaže, i když jinak „přerušená“
        let mut got = plan_cleanup(&all(), cur, &local14, Some(v("0.2.17")), Some("v0.2.18"));
        got.sort();
        assert_eq!(got, ["cizi", "v0.2.12", "v0.2.16"]);
    }

    #[test]
    fn progress_steps_on_whole_percent_or_512k() {
        #[rustfmt::skip]
        let cases: Vec<(u64, u64, Option<u64>, bool)> = vec![
            (0,       0,         Some(1000), false),
            (0,       9,         Some(1000), false),  // 0,9 % → pořád 0 %
            (9,       10,        Some(1000), true),   // 1 %
            (10,      19,        Some(1000), false),
            (990,     1000,      Some(1000), true),   // 100 %
            (1000,    1200,      Some(1000), false),  // přes deklarovanou velikost — nepřekročí 100 %
            (0,       100,       None,       false),
            (0,       512*1024,  None,       true),
            (512*1024, 600*1024, None,       false),
            (0,       5,         Some(0),    false),  // nulová velikost = neznámá
        ];
        for (last, got, total, want) in cases {
            assert_eq!(progress_due(last, got, total), want, "{last} {got} {total:?}");
        }
    }

    #[test]
    fn next_step_table() {
        let failed = Fetch::Failed { duvod: "offline".into() };
        let dl = Fetch::Downloading { got: 1, total: None };
        #[rustfmt::skip]
        let cases: Vec<(bool, bool, bool, bool, Fetch, Step)> = vec![
            // auto, user, release, cache, stav → krok
            (true,  false, false, false, Fetch::Idle,   Step::Nothing),   // není co
            (true,  false, true,  false, Fetch::Idle,   Step::Download),
            (false, false, true,  false, Fetch::Idle,   Step::Nothing),   // tiché stažení vypnuté → klik Instalovat
            (false, true,  true,  false, Fetch::Idle,   Step::Download),
            (true,  false, true,  true,  Fetch::Idle,   Step::MarkReady), // jen jednou na verzi
            (false, false, true,  true,  Fetch::Idle,   Step::MarkReady),
            (true,  false, true,  true,  Fetch::Ready,  Step::Nothing),
            (true,  false, true,  false, failed.clone(), Step::Nothing),  // žádné tiché opakování
            (true,  true,  true,  false, failed.clone(), Step::Download),
            (true,  true,  true,  false, dl.clone(),    Step::Nothing),   // jedno stahování naráz
            (true,  false, true,  true,  dl,            Step::Nothing),
        ];
        for (auto, user, rel, cache, fetch, want) in cases {
            assert_eq!(next_step(auto, user, rel, cache, &fetch), want, "{auto} {user} {rel} {cache} {fetch:?}");
        }
    }

    #[test]
    fn scan_cleanup_and_manifest_roundtrip_in_temp_dir() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        assert!(scan(&root.join("chybi")).is_empty());
        for tag in ["v0.2.14", "v0.2.15"] {
            let dir = root.join(tag);
            std::fs::create_dir_all(&dir).unwrap();
            let m = man(tag);
            std::fs::write(dir.join(&m.asset), "x").unwrap();
            write_manifest(&dir, &m).unwrap();
        }
        std::fs::create_dir_all(root.join("v0.2.13")).unwrap();
        std::fs::write(root.join("v0.2.13").join("App_0.2.13_x64-setup.exe.part"), "x").unwrap();
        std::fs::write(root.join("seen.json"), "{}").unwrap();
        let entries = scan(root);
        assert_eq!(entries.iter().map(|e| e.name.as_str()).collect::<Vec<_>>(), ["v0.2.13", "v0.2.14", "v0.2.15"]);
        assert_eq!(entries[1].manifest.as_ref(), Some(&man("v0.2.14")));
        assert_eq!(installer(root, &entries[2]), Some(root.join("v0.2.15").join("App_0.2.15_x64-setup.exe")));
        let rb = plan_rollback(&entries, v("0.2.15"), Some(v("0.2.14")));
        let gone = plan_cleanup(&entries, v("0.2.15"), &rb, None, None);
        assert_eq!(gone, ["v0.2.13"]);
        assert_eq!(remove(root, &gone), 1);
        // nebezpečná jména se nemažou
        assert_eq!(remove(root, &["..".into(), "a/b".into(), String::new()]), 0);
        assert!(root.join("seen.json").is_file() && !root.join("v0.2.13").exists());
        let rel = man("v0.2.15").release();
        assert_eq!(cached(&scan(root), &rel).map(|e| e.name.as_str()), Some("v0.2.15"));
        let other = Release { sha256: "b".repeat(64), ..rel };
        assert!(cached(&scan(root), &other).is_none(), "jiný otisk = jiné vydání");
    }

    /// Podvržený `release.json` s jiným souborem než `*-setup.exe` (např. `x.cmd`, spouštěl by se přes
    /// `cmd /c`) nikdy nedá hotovou položku — návrat ani instalace ho nespustí.
    #[test]
    fn forged_manifest_with_non_installer_is_never_complete() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        for (tag, asset, ok) in [
            ("v0.2.14", "x.cmd", false),
            ("v0.2.13", "App-setup.exe.cmd", false),
            ("v0.2.12", r"..\App_x64-setup.exe", false),
            ("v0.2.11", "App_0.2.11_x64-setup.exe", true),
        ] {
            let dir = root.join(tag);
            std::fs::create_dir_all(&dir).unwrap();
            if !asset.contains('\\') {
                std::fs::write(dir.join(asset), "x").unwrap();
            }
            write_manifest(&dir, &Manifest { asset: asset.into(), ..man(tag) }).unwrap();
            let e = scan(root).into_iter().find(|e| e.name == tag).unwrap();
            assert_eq!(e.version().is_some(), ok, "{asset}");
            assert_eq!(installer(root, &e).is_some(), ok, "{asset}");
        }
        let entries = scan(root);
        assert_eq!(plan_rollback(&entries, v("0.2.15"), Some(v("0.2.14"))), Rollback::Remote { version: v("0.2.14") });
    }

    /// Jména končící tečkou (Windows je ořízne: `...` → samotná `updates`) ani bez písmen se nemažou.
    #[test]
    fn remove_refuses_names_windows_would_trim() {
        #[rustfmt::skip]
        let cases = [
            ("v0.2.14", true), ("cizi", true), ("a-b_c.d", true),
            ("...", false), (".", false), ("..", false), ("v0.2.14.", false), ("v0.2.14 ", false),
            ("x ", false), ("-", false), ("", false), ("a/b", false), (r"a\b", false),
        ];
        for (n, want) in cases {
            assert_eq!(safe_name(n), want, "{n:?}");
        }
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path().join("updates");
        std::fs::create_dir_all(root.join("v0.2.14")).unwrap();
        assert_eq!(remove(&root, &["...".into(), "v0.2.14.".into(), ".".into()]), 0);
        assert!(root.join("v0.2.14").is_dir(), "cache zůstala celá");
    }

    #[test]
    fn fetch_serializes_for_ui() {
        let j = |f: Fetch| serde_json::to_value(f).unwrap();
        assert_eq!(j(Fetch::Idle)["state"], "idle");
        let d = j(Fetch::Downloading { got: 5, total: None });
        assert_eq!((d["state"].as_str(), d["got"].as_u64(), d["total"].is_null()), (Some("downloading"), Some(5), true));
        assert_eq!(j(Fetch::Ready)["state"], "ready");
        assert_eq!(j(Fetch::Failed { duvod: "x".into() })["duvod"], "x");
    }
}
