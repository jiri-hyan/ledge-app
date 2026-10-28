//! „Co je nového“ a předchozí verze: `<local>/updates/seen.json` = naposledy spuštěná verze, verze před
//! ní (cíl „Vrátit předchozí verzi“) a verze, jejíž poznámky už uživatel viděl.
//!
//! Zapisuje se jen při startu (když se verze změnila) a po kliku „Rozumím“ / zavření upozornění — žádné
//! časovače. Poznámky samotné leží v manifestu staženého instalátoru (`updcache::Manifest.notes`), takže
//! zobrazení nepotřebuje síť; verze nainstalovaná jinak (ručně) poznámky nemá a nic se neukáže.
//!
//! **Otisky** (`known`): SHA256 instalátoru každé verze, kterou Ledge stáhl a ověřil — lokálně, mimo
//! dosah obsahu vydání a mimo cache (úklid ji maže). Návrat ([`check_known`]) odmítne vydání, které
//! se na GitHubu od té doby přepsalo jiným instalátorem.

use crate::update::parse_version;
use serde::{Deserialize, Serialize};
use std::path::Path;

pub const FILE: &str = "seen.json";

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Seen {
    /// naposledy spuštěná verze
    pub seen: Option<String>,
    /// verze, která běžela před `seen`
    pub previous: Option<String>,
    /// verze, jejíž „Co je nového“ už uživatel viděl (nebo nemá smysl ukazovat)
    pub notes_read: Option<String>,
    /// SHA256 ověřených instalátorů podle verze (nejvýš [`MAX_KNOWN`], nejnovější zápisy)
    pub known: Vec<Known>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Known {
    pub version: String,
    pub sha256: String,
}

/// Kolik otisků držet (CI vydává často; návrat míří na nedávné verze).
pub const MAX_KNOWN: usize = 32;

/// Zapamatovat SHA256 ověřeného instalátoru verze `version` (přepíše starší záznam téže verze). Čistá funkce.
pub fn remember(st: &Seen, version: &str, sha256: &str) -> Seen {
    let mut known: Vec<Known> = st.known.iter().filter(|k| k.version != version).cloned().collect();
    known.push(Known { version: version.into(), sha256: sha256.to_ascii_lowercase() });
    let skip = known.len().saturating_sub(MAX_KNOWN);
    Seen { known: known.into_iter().skip(skip).collect(), ..st.clone() }
}

/// Souhlasí SHA256 vydání s tím, co Ledge u téže verze dřív sám ověřil? Bez záznamu (ruční instalace)
/// projde — ověření proti poznámkám a `digest` platí dál. Čistá funkce.
pub fn check_known(st: &Seen, version: &str, sha256: &str) -> Result<(), String> {
    match st.known.iter().find(|k| k.version == version) {
        Some(k) if !k.sha256.eq_ignore_ascii_case(sha256) => Err(trf!(
            "Vydání {version} má jiný instalátor (SHA256), než jaký Ledge u této verze dřív ověřil — vydání se nejspíš přepsalo. Nic nespouštím.",
            "Release {version} has a different installer (SHA256) than the one Ledge verified for this version before — the release was probably overwritten. Not running anything."
        )),
        _ => Ok(()),
    }
}

/// Stav po startu verze `current`. Čistá funkce.
/// - první spuštění vůbec (čistá instalace): nic neukazovat, předchozí neznáme;
/// - stejná verze: beze změny;
/// - jiná než minule: předchozí = minulá (při návratu tedy novější). Přečtené poznámky se nemění —
///   [`unread`] je porovnává verzí, takže po návratu ani po opětovné aktualizaci se nic neukáže znovu.
pub fn on_start(old: &Seen, current: &str) -> Seen {
    match old.seen.as_deref() {
        None => Seen { seen: Some(current.into()), previous: None, notes_read: Some(current.into()), known: old.known.clone() },
        Some(s) if s == current => old.clone(),
        Some(s) => Seen { seen: Some(current.into()), previous: Some(s.into()), notes_read: old.notes_read.clone(), known: old.known.clone() },
    }
}

/// Ukázat v šuplíku upozornění „Co je nového ve verzi X“? Jen s poznámkami a jen když běžící verze je
/// novější než ta, jejíž poznámky uživatel viděl naposledy (nečitelné = jiná → ukázat). Čistá funkce.
pub fn unread(st: &Seen, current: &str, has_notes: bool) -> bool {
    has_notes
        && match st.notes_read.as_deref() {
            None => true,
            Some(r) => match (parse_version(r), parse_version(current)) {
                (Some(a), Some(b)) => a < b,
                _ => r != current,
            },
        }
}

/// Po přečtení: poznámky verze `current` už neukazovat (a starších taky).
pub fn mark_read(st: &Seen, current: &str) -> Seen {
    let newer_read = matches!(st.notes_read.as_deref().and_then(parse_version).zip(parse_version(current)), Some((a, b)) if a > b);
    Seen { notes_read: if newer_read { st.notes_read.clone() } else { Some(current.into()) }, ..st.clone() }
}

/// Načíst (chybí / rozbité = prázdné — jako první spuštění).
pub fn load(dir: &Path) -> Seen {
    crate::fsx::load_json_or_default(&dir.join(FILE))
}

/// Uložit atomicky (tmp + rename).
pub fn save(dir: &Path, st: &Seen) -> Result<(), String> {
    std::fs::create_dir_all(dir).map_err(|e| crate::errors::io(&dir.display().to_string(), &e))?;
    let body = serde_json::to_string_pretty(st).map_err(|e| e.to_string())?;
    crate::fsx::write_atomic(&dir.join(FILE), body.as_bytes()).map_err(|e| crate::errors::io(&dir.display().to_string(), &e))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn s(seen: Option<&str>, previous: Option<&str>, read: Option<&str>) -> Seen {
        Seen { seen: seen.map(Into::into), previous: previous.map(Into::into), notes_read: read.map(Into::into), known: Vec::new() }
    }

    #[test]
    fn on_start_table() {
        #[rustfmt::skip]
        let cases: Vec<(Seen, &str, Seen)> = vec![
            // čistá instalace / první běh s touhle funkcí: nic neukazovat
            (Seen::default(),                                  "0.2.15", s(Some("0.2.15"), None, Some("0.2.15"))),
            // stejná verze: beze změny (nepřečtené zůstává nepřečtené)
            (s(Some("0.2.15"), Some("0.2.14"), Some("0.2.14")), "0.2.15", s(Some("0.2.15"), Some("0.2.14"), Some("0.2.14"))),
            // aktualizace: předchozí = minulá, poznámky nepřečtené
            (s(Some("0.2.14"), Some("0.2.13"), Some("0.2.14")), "0.2.15", s(Some("0.2.15"), Some("0.2.14"), Some("0.2.14"))),
            // návrat na starší: předchozí = novější, přečtené beze změny
            (s(Some("0.2.15"), Some("0.2.14"), Some("0.2.15")), "0.2.14", s(Some("0.2.14"), Some("0.2.15"), Some("0.2.15"))),
            // a zase zpět na novější
            (s(Some("0.2.14"), Some("0.2.15"), Some("0.2.15")), "0.2.15", s(Some("0.2.15"), Some("0.2.14"), Some("0.2.15"))),
            // nečitelná minulá verze: bere se jako jiná, ne jako návrat
            (s(Some("nevim"), None, None),                      "0.2.15", s(Some("0.2.15"), Some("nevim"), None)),
        ];
        for (old, cur, want) in cases {
            assert_eq!(on_start(&old, cur), want, "{old:?} → {cur}");
        }
    }

    #[test]
    fn unread_table() {
        #[rustfmt::skip]
        let cases: Vec<(Option<&str>, &str, bool, bool)> = vec![
            // přečteno, běží, poznámky → ukázat
            (Some("0.2.14"), "0.2.15", true,  true),
            (Some("0.2.14"), "0.2.15", false, false), // bez poznámek nic
            (Some("0.2.15"), "0.2.15", true,  false), // už viděl
            (Some("0.2.15"), "0.2.14", true,  false), // po návratu na starší nic
            (None,           "0.2.15", true,  true),
            (Some("nevim"),  "0.2.15", true,  true),
            (Some("nevim"),  "nevim",  true,  false),
        ];
        for (read, cur, notes, want) in cases {
            assert_eq!(unread(&s(Some(cur), None, read), cur, notes), want, "{read:?} {cur} {notes}");
        }
        // přečtení: nikdy nesníží už přečtenou novější verzi
        assert_eq!(mark_read(&s(Some("0.2.15"), None, Some("0.2.14")), "0.2.15").notes_read.as_deref(), Some("0.2.15"));
        assert_eq!(mark_read(&s(Some("0.2.14"), None, Some("0.2.15")), "0.2.14").notes_read.as_deref(), Some("0.2.15"));
        assert_eq!(mark_read(&Seen::default(), "0.2.15").notes_read.as_deref(), Some("0.2.15"));
    }

    #[test]
    fn known_sha_table() {
        let a = "a".repeat(64);
        let b = "b".repeat(64);
        let st = remember(&Seen::default(), "0.2.14", &a.to_uppercase());
        #[rustfmt::skip]
        let cases: Vec<(&str, &str, bool)> = vec![
            ("0.2.14", a.as_str(), true),   // stejný otisk (i jiná velikost písmen)
            ("0.2.14", b.as_str(), false),  // přepsané vydání
            ("0.2.13", b.as_str(), true),   // bez záznamu (ruční instalace) → projde
        ];
        for (v, sha, ok) in cases {
            assert_eq!(check_known(&st, v, sha).is_ok(), ok, "{v} {sha}");
        }
        assert!(check_known(&st, "0.2.14", &b).unwrap_err().contains("přepsalo"));
        // přepis záznamu téže verze, strop počtu (nejstarší zápisy pryč)
        let st = remember(&st, "0.2.14", &b);
        assert_eq!(st.known, vec![Known { version: "0.2.14".into(), sha256: b.clone() }]);
        let many = (0..40).fold(Seen::default(), |s, i| remember(&s, &format!("0.3.{i}"), &a));
        assert_eq!(many.known.len(), MAX_KNOWN);
        assert_eq!(many.known.first().map(|k| k.version.as_str()), Some("0.3.8"));
        // on_start otisky nezahodí
        assert_eq!(on_start(&many, "0.4.0").known.len(), MAX_KNOWN);
    }

    #[test]
    fn load_save_roundtrip_and_broken_file() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().join("updates");
        assert_eq!(load(&dir), Seen::default());
        let st = s(Some("0.2.15"), Some("0.2.14"), None);
        save(&dir, &st).unwrap();
        assert_eq!(load(&dir), st);
        std::fs::write(dir.join(FILE), "{rozbité").unwrap();
        assert_eq!(load(&dir), Seen::default());
        // starší / cizí tvar: chybějící pole = výchozí
        std::fs::write(dir.join(FILE), r#"{"seen":"0.2.1","jine":1}"#).unwrap();
        assert_eq!(load(&dir), s(Some("0.2.1"), None, None));
    }
}
