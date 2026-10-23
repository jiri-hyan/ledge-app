//! Projekty v šuplíku a jejich odkazy.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Jeden projekt v šuplíku.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Project {
    /// Unikátní id (kebab-case).
    pub id: String,
    pub name: String,
    /// Zobrazit v šuplíku.
    #[serde(default = "yes")]
    pub enabled: bool,
    pub kind: ProjectKind,
    /// Složka projektu. Když chybí nebo neexistuje, dohledá se podle `remote` v `roots`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    /// Git remote (https/ssh/„owner/repo“) — pro dohledání složky a odkaz na GitHub.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub remote: Option<String>,
    /// Podsložka, ve které se příkaz spouští (např. "web").
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub subdir: Option<String>,
    /// kind=dev: příkaz dev serveru (např. "npm run dev:staging").
    /// kind=app: volitelný příkaz před spuštěním (např. "npm run build").
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub run: Option<String>,
    /// kind=dev: preferovaný port. Když je obsazený cizím procesem, přidělí se jiný volný.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub port: Option<u16>,
    /// kind=dev: přidat `-- --port N` k npm/pnpm příkazu (Vite apod.). Jinak se port předá jen v env PORT.
    #[serde(default = "yes")]
    pub pass_port: bool,
    /// kind=dev: po naběhnutí otevřít prohlížeč.
    #[serde(default = "yes")]
    pub open_browser: bool,
    /// kind=app: cesta k .exe (absolutní nebo relativní k projektu).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub exe: Option<String>,
    /// kind=app: jméno nainstalované aplikace (dohledá se v registru odinstalace), když `exe` chybí.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub app_name: Option<String>,
    /// kind=app: aplikace z Microsoft Store (bez `.exe` a `.lnk`) podle AppUserModelID, např.
    /// `Microsoft.WindowsCalculator_8wekyb3d8bbwe!App`. Spouští se jako `shell:AppsFolder\<aumid>`
    /// (bez sledování procesu); jen znaky `A-Za-z0-9._!-`. `exe` a `appName` mají přednost.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub aumid: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub args: Vec<String>,
    /// Barva identity projektu (#rrggbb).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub accent: Option<String>,
    /// Zkratka v logu (1–2 znaky), když chybí `logo`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub monogram: Option<String>,
    /// Cesta k logu (svg/png/ico/jpg/webp), relativně ke složce projektu. Volitelné: bez něj (nebo když
    /// soubor chybí) se logo najde samo — ikona z index.html / manifestu, obvyklé soubory v repu, ikona exe.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub logo: Option<String>,
    /// Odkazy v rychlých akcích (staging, prod, docs…).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub links: Vec<Link>,
    /// Vzhled dlaždice (ikona, barva, tvar, popisek — `tilelook.rs`). Chybí = automaticky (logo, `accent`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub look: Option<crate::tilelook::TileLook>,
}

fn yes() -> bool {
    true
}

/// AppUserModelID aplikace ze Storu (`Balicek_hash!App`): jen bezpečné znaky, žádná lomítka — ať z něj
/// `shell:AppsFolder\<aumid>` nemůže ukázat na jinou položku shellu.
pub fn valid_aumid(a: &str) -> bool {
    (3..=256).contains(&a.len()) && a.contains('!') && a.chars().all(|c| c.is_ascii_alphanumeric() || "._!-".contains(c))
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum ProjectKind {
    /// Lokální dev server (port, log, otevření prohlížeče).
    Dev,
    /// Desktopová aplikace (.exe), volitelně s přípravným příkazem.
    App,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct Link {
    pub label: String,
    pub url: String,
}
