//! Galerie vestavěných příkladů widgetů (`templates/*`, zakompilované). Klik v Nastavení = kopie do
//! složky widgetů (nikdy nepřepíše existující soubor). Kopie se sama nespustí: jako každý nový soubor
//! čeká na souhlas v kartě. `.ps1` se zapisuje s BOM (Windows PowerShell 5.1 jinak čte diakritiku špatně).

use super::meta::{self, Kind};
use crate::i18n::tr;
use serde::Serialize;
use std::path::{Path, PathBuf};

pub struct Template {
    pub file: &'static str,
    pub body: &'static str,
    /// kam šablona sahá po síti (`None` = nikam)
    pub net: Option<&'static str>,
}

macro_rules! tpl {
    ($f:literal, $net:expr) => {
        Template { file: $f, body: include_str!(concat!("templates/", $f)), net: $net }
    };
}

pub const ALL: &[Template] = &[
    tpl!("kurz-cnb.ps1", Some("www.cnb.cz")),
    tpl!("kurz-cnb.py", Some("www.cnb.cz")),
    tpl!("pocasi-jinde.ps1", Some("api.open-meteo.com")),
    tpl!("pocasi-jinde.py", Some("api.open-meteo.com")),
    tpl!("docker.ps1", None),
    tpl!("github-ci.ps1", Some("api.github.com (gh)")),
    tpl!("ping.ps1", Some("ping")),
    tpl!("ping.py", Some("ping")),
    tpl!("sit-vpn.ps1", None),
    tpl!("disk.ps1", None),
    tpl!("disk.py", None),
    tpl!("uptime.ps1", None),
    tpl!("uptime.py", None),
    tpl!("odpocet.ps1", None),
    tpl!("odpocet.py", None),
];

#[derive(Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct TemplateView {
    pub file: &'static str,
    pub title: String,
    pub description: Option<String>,
    pub icon: Option<String>,
    pub kind: Kind,
    pub net: Option<&'static str>,
    /// soubor stejného jména už ve složce je
    pub installed: bool,
    pub body: &'static str,
}

fn kind_of(file: &str) -> Option<Kind> {
    Path::new(file).extension().and_then(|e| e.to_str()).and_then(Kind::from_ext)
}

/// Galerie pro UI (`dir` = složka widgetů; jen stat).
pub fn views(dir: &Path) -> Vec<TemplateView> {
    ALL.iter()
        .filter_map(|t| {
            let kind = kind_of(t.file)?;
            let m = meta::parse(t.body, kind.lang()?).ok().flatten()?;
            Some(TemplateView {
                file: t.file,
                title: m.title,
                description: m.description,
                icon: m.icon,
                kind,
                net: t.net,
                installed: dir.join(t.file).exists(),
                body: t.body,
            })
        })
        .collect()
}

/// Zkopíruje šablonu `file` do `dir` (založí ji). Existující soubor nepřepíše.
pub fn install(dir: &Path, file: &str) -> Result<PathBuf, String> {
    let t = ALL.iter().find(|t| t.file == file).ok_or_else(|| tr("Neznámý příklad.", "Unknown example."))?;
    std::fs::create_dir_all(dir).map_err(|e| crate::errors::io(&trf!("Složku {} nejde založit", "Can't create the folder {}", dir.display()), &e))?;
    let path = dir.join(t.file);
    let mut bytes: Vec<u8> = Vec::new();
    if kind_of(t.file) == Some(Kind::Powershell) {
        bytes.extend_from_slice(&[0xEF, 0xBB, 0xBF]);
    }
    bytes.extend_from_slice(t.body.replace('\n', "\r\n").as_bytes());
    use std::io::Write;
    let mut f = std::fs::OpenOptions::new().write(true).create_new(true).open(&path).map_err(|e| match e.kind() {
        std::io::ErrorKind::AlreadyExists => trf!("{} už ve složce je — nepřepisuju ho.", "{} is already in the folder — not overwriting it.", t.file),
        _ => crate::errors::io(&trf!("{} nejde uložit", "Can't save {}", t.file), &e),
    })?;
    f.write_all(&bytes).map_err(|e| crate::errors::io(&trf!("{} nejde uložit", "Can't save {}", t.file), &e))?;
    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_template_is_a_short_valid_widget() {
        assert!(ALL.len() >= 10);
        let mut files = std::collections::HashSet::new();
        for t in ALL {
            assert!(files.insert(t.file), "{} dvakrát", t.file);
            let kind = kind_of(t.file).unwrap_or_else(|| panic!("{}: přípona", t.file));
            let m = meta::parse(t.body, kind.lang().unwrap()).unwrap_or_else(|e| panic!("{}: {e}", t.file)).unwrap_or_else(|| panic!("{}: bez @ledge.widget", t.file));
            assert!(m.description.is_some(), "{}: popis", t.file);
            assert!(m.icon.is_some(), "{}: ikona", t.file);
            assert!(t.body.lines().count() <= 45, "{}: krátký a čitelný ({} řádků)", t.file, t.body.lines().count());
            // co sahá na síť, to v galerii přizná (a naopak)
            let uses_net = t.body.contains("https://api") || t.body.contains("denni_kurz") || t.body.contains("gh api") || t.body.contains("ping") || t.body.contains("Test-Connection");
            assert_eq!(uses_net, t.net.is_some(), "{}: net", t.file);
            assert!(!t.body.contains('\r'), "{}: LF v repu", t.file);
        }
        let v = views(Path::new(r"C:\neexistuje-ledge-test"));
        assert_eq!(v.len(), ALL.len());
        assert!(v.iter().all(|t| !t.installed));
    }

    #[test]
    fn install_copies_once_with_bom_for_powershell() {
        let dir = tempfile::tempdir().unwrap();
        let w = dir.path().join("widgets");
        let p = install(&w, "odpocet.ps1").unwrap();
        let b = std::fs::read(&p).unwrap();
        assert!(b.starts_with(&[0xEF, 0xBB, 0xBF]), "BOM pro PowerShell 5.1");
        assert!(String::from_utf8_lossy(&b).contains("\r\n"));
        assert!(install(&w, "odpocet.ps1").unwrap_err().contains("odpocet.ps1"), "nepřepíše");
        let py = std::fs::read(install(&w, "odpocet.py").unwrap()).unwrap();
        assert!(!py.starts_with(&[0xEF, 0xBB, 0xBF]));
        assert!(install(&w, "../zlo.ps1").is_err());
        assert!(views(&w).iter().any(|t| t.file == "odpocet.ps1" && t.installed));
        // zkopírovaná šablona se najde jako widget
        let scan = super::super::scan_dir(&w, None);
        assert_eq!(scan.widgets.len(), 2);
        assert!(scan.widgets.iter().all(|x| x.meta.is_ok() && !x.sha.is_empty()));
    }
}
