//! Koncept rozepsané nové poznámky v `<root>\.draft\` (`draft.json` + `N.png`, přežije zavření
//! šuplíku i pád) a předvyplnění projektu podle cwd poslední Claude session (`preselect`).
//! Závisí na `write_atomic`, `decode_png`, `b64_encode` (`mod.rs`).

use super::{MAX_IMAGE, MAX_IMAGES, PNG_SIG, b64_encode, decode_png, write_atomic};
use crate::i18n::tr;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Rozepsaná poznámka — přežije zavření šuplíku, zrušení WebView i pád. Leží v `<root>\.draft\`
/// (`draft.json` + `N.png`), mimo sken poznámek.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(default, rename_all = "camelCase")]
pub struct DraftMeta {
    pub text: String,
    pub project: String,
    /// šéf projekt sám vybral → nepředvyplňovat
    pub project_touched: bool,
    pub remind: String,
    /// kategorie (id); prázdné = výchozí
    pub kind: String,
}

#[derive(Debug, Clone, Default, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Draft {
    #[serde(flatten)]
    pub meta: DraftMeta,
    /// PNG v base64
    pub images: Vec<String>,
}

pub(crate) fn draft_dir(root: &Path) -> PathBuf {
    root.join(".draft")
}

/// Text, projekt a připomínka (volá se s debounce při psaní — malé, bez obrázků).
pub fn draft_save_meta(root: &Path, meta: &DraftMeta) -> Result<(), String> {
    let dir = draft_dir(root);
    std::fs::create_dir_all(&dir).map_err(|e| crate::errors::io(tr("Koncept nejde uložit", "Can't save the draft"), &e))?;
    write_atomic(&dir.join("draft.json"), serde_json::to_string(meta).map_err(|e| crate::errors::json(tr("Koncept nejde uložit", "Can't save the draft"), &e))?.as_bytes())
}

/// Obrázky konceptu (jen když se změnily): validace jako u poznámky, staré `N.png` pryč.
pub fn draft_save_images(root: &Path, images: &[String]) -> Result<(), String> {
    if images.len() > MAX_IMAGES {
        return Err(trf!("Nejvýš {MAX_IMAGES} obrázků na poznámku.", "At most {MAX_IMAGES} images per note."));
    }
    let pngs = images.iter().map(|b| decode_png(b)).collect::<Result<Vec<_>, _>>()?;
    let dir = draft_dir(root);
    if pngs.is_empty() && !dir.exists() {
        return Ok(()); // nic k uložení ani k úklidu — složku kvůli tomu nezakládat
    }
    std::fs::create_dir_all(&dir).map_err(|e| crate::errors::io(tr("Obrázky konceptu nejdou uložit", "Can't save the draft images"), &e))?;
    for (i, png) in pngs.iter().enumerate() {
        write_atomic(&dir.join(format!("{}.png", i + 1)), png)?;
    }
    for i in pngs.len() + 1.. {
        let p = dir.join(format!("{i}.png"));
        if !p.exists() {
            break;
        }
        std::fs::remove_file(&p).map_err(|e| crate::errors::io(tr("Obrázky konceptu nejdou uložit", "Can't save the draft images"), &e))?;
    }
    Ok(())
}

/// Koncept, nebo `None`, když žádný není (prázdný se nepočítá).
pub fn draft_load(root: &Path) -> Option<Draft> {
    let dir = draft_dir(root);
    let meta: DraftMeta = std::fs::read(dir.join("draft.json")).ok().and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or_default();
    let mut images = Vec::new();
    for i in 1..=MAX_IMAGES {
        let Ok(png) = std::fs::read(dir.join(format!("{i}.png"))) else { break };
        if png.len() <= MAX_IMAGE && png.starts_with(&PNG_SIG) {
            images.push(b64_encode(&png));
        }
    }
    let d = Draft { meta, images };
    (d != Draft::default()).then_some(d)
}

/// Po uložení poznámky (nebo zahození Esc) koncept smazat. Jen naše soubory v `.draft`.
pub fn draft_clear(root: &Path) -> Result<(), String> {
    let dir = draft_dir(root);
    let Ok(rd) = std::fs::read_dir(&dir) else { return Ok(()) };
    for e in rd.flatten() {
        let name = e.file_name().to_string_lossy().into_owned();
        let ours = name == "draft.json" || (name.ends_with(".png") && name[..name.len() - 4].parse::<usize>().is_ok()) || name.ends_with(".tmp");
        if ours {
            std::fs::remove_file(e.path()).map_err(|e| crate::errors::io(tr("Koncept nejde smazat", "Can't delete the draft"), &e))?;
        }
    }
    let _ = std::fs::remove_dir(&dir); // jen když je prázdná
    Ok(())
}

fn norm(p: &str) -> String {
    p.to_lowercase().replace('/', "\\").trim_end_matches('\\').to_string()
}

/// Projekt (id), ve kterém leží `cwd` — nejdelší shoda složky, jen na hranici komponenty cesty
/// (`C:\a` neobsahuje `C:\ab`). `dirs` = (id, složka projektu).
pub fn project_for_cwd<'a>(dirs: &'a [(String, String)], cwd: &str) -> Option<&'a str> {
    let cwd = norm(cwd);
    dirs.iter()
        .map(|(id, d)| (id, norm(d)))
        .filter(|(_, d)| !d.is_empty() && (cwd == *d || cwd.starts_with(&format!("{d}\\"))))
        .max_by_key(|(_, d)| d.len())
        .map(|(id, _)| id.as_str())
}

/// Předvyplněný projekt rychlé poznámky: podle `cwd` naposledy aktivní Claude session
/// (`sessions` = (cwd, updated_at)); když neleží v žádném projektu nebo session není, nic.
pub fn preselect(dirs: &[(String, String)], sessions: &[(String, u64)]) -> Option<String> {
    let (cwd, _) = sessions.iter().max_by_key(|(_, t)| *t)?;
    project_for_cwd(dirs, cwd).map(str::to_string)
}

#[cfg(test)]
mod tests {
    use super::super::scan;
    use super::super::testkit::{b64, png, tmp_root};
    use super::*;

    #[test]
    fn draft_roundtrip_and_clear() {
        let root = tmp_root();
        assert_eq!(draft_load(root.path()), None);
        draft_save_images(root.path(), &[]).unwrap();
        assert!(!root.path().join(".draft").exists(), "prázdné obrázky složku nezakládají");
        let meta = DraftMeta { text: "rozepsáno".into(), project: "atlas".into(), project_touched: true, remind: "2026-10-17T09:00".into(), kind: "napad".into() };
        draft_save_meta(root.path(), &meta).unwrap();
        let (a, b) = (png(1), png(2));
        draft_save_images(root.path(), &[b64(&a), b64(&b)]).unwrap();
        let d = draft_load(root.path()).unwrap();
        assert_eq!(d.meta, meta);
        assert_eq!(d.images, vec![b64(&a), b64(&b)]);
        // ubrání obrázku smaže přebytečný soubor
        draft_save_images(root.path(), &[b64(&b)]).unwrap();
        assert_eq!(draft_load(root.path()).unwrap().images, vec![b64(&b)]);
        assert!(draft_save_images(root.path(), &[b64(b"neni png, jen nejake bajty nahodou dost dlouhe")]).is_err());
        // koncept se do poznámek nepočítá
        assert!(scan(root.path()).is_empty());
        std::fs::write(root.path().join(".draft/cizi.txt"), "x").unwrap();
        draft_clear(root.path()).unwrap();
        assert_eq!(draft_load(root.path()), None);
        assert!(root.path().join(".draft/cizi.txt").exists(), "cizí soubory nemazat");
    }

    #[test]
    fn preselect_by_last_session_cwd() {
        let dirs = vec![
            ("atlas".to_string(), "C:\\Users\\x\\Desktop\\atlas".to_string()),
            ("atlas-web".to_string(), "C:/Users/x/Desktop/atlas/web/".to_string()),
            ("ab".to_string(), "C:\\Users\\x\\Desktop\\ab".to_string()),
        ];
        // sessions (cwd, updated_at), očekávaný projekt
        type Case<'a> = (&'a [(&'a str, u64)], Option<&'a str>);
        let cases: &[Case] = &[
            (&[], None),
            (&[("c:\\users\\x\\desktop\\atlas", 1)], Some("atlas")),
            (&[("C:/Users/x/Desktop/atlas/src", 1)], Some("atlas")),
            // nejdelší shoda
            (&[("C:\\Users\\x\\Desktop\\atlas\\web\\src", 1)], Some("atlas-web")),
            // hranice komponenty: „abc“ není „ab“
            (&[("C:\\Users\\x\\Desktop\\abc", 1)], None),
            // rozhoduje poslední aktivní session, i když neleží v projektu
            (&[("C:\\Users\\x\\Desktop\\ab", 5), ("C:\\jinde", 9)], None),
            (&[("C:\\jinde", 5), ("C:\\Users\\x\\Desktop\\ab\\x", 9)], Some("ab")),
        ];
        for (sessions, want) in cases {
            let s: Vec<(String, u64)> = sessions.iter().map(|(c, t)| (c.to_string(), *t)).collect();
            assert_eq!(preselect(&dirs, &s).as_deref(), *want, "{sessions:?}");
        }
    }
}
