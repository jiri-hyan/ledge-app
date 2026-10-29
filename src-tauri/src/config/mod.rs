//! Konfigurace Ledge — jeden JSON soubor, který edituje šéf (přes Nastavení) i Claude (přímo).
//!
//! Umístění: `%APPDATA%\Ledge\config.json`, vedle něj `config.schema.json` (generované ze struktur
//! níže přes schemars), aby editor i agent viděli přesný tvar. Soubor se sleduje a po uložení se
//! hned aplikuje. Nevalidní soubor se NEaplikuje — drží se poslední dobrá verze a chyba se ukáže.

mod claude;
mod drawer;
mod hotkeys;
mod io;
mod notes;
mod personal;
mod project;
mod sections;
#[cfg(test)]
mod tests;
mod validate;

pub use claude::*;
pub use drawer::*;
pub use hotkeys::*;
pub use io::*;
pub use notes::*;
pub use personal::*;
pub use project::*;
pub use sections::*;
pub use validate::*;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(default, rename_all = "camelCase")]
pub struct Config {
    /// Odkaz na schéma (jen pro editory, Ledge ho ignoruje).
    #[serde(rename = "$schema", skip_serializing_if = "Option::is_none")]
    pub schema: Option<String>,
    pub strip: StripConfig,
    pub drawer: DrawerConfig,
    pub modes: ModesConfig,
    pub claude: ClaudeConfig,
    pub hotkeys: HotkeyConfig,
    /// Rychlé poznámky a připomínky (zkratka `hotkeys.note`).
    pub notes: NotesConfig,
    /// Obsah záložky „Osobní“: místo pro počasí a rychlé odkazy.
    pub personal: PersonalConfig,
    /// Kořenové složky, ve kterých se hledají repa podle git remote (přímé podsložky). Do první z nich
    /// klonuje tlačítko „Naklonovat“ u projektu, který na tomhle PC chybí.
    pub roots: Vec<String>,
    /// Spouštět Ledge po přihlášení do Windows (HKCU\\…\\Run).
    pub autostart: bool,
    /// Upozornění na novou verzi Ledge (GitHub Releases přes veřejné API, záložně `gh`; instalace jen na klik).
    pub update: UpdateConfig,
    /// Vzhled: motiv, akcent, hustota, písmo, animace, proužek (`appearance.rs`). Pořadí karet je v `layout`.
    pub appearance: crate::appearance::AppearanceConfig,
    pub projects: Vec<Project>,
    /// Upozornění na neuloženou / nepushnutou práci v repech (chip v hlavičce, proužek): vypínač a „Přečteno“.
    /// Chybí = upozorňovat jako dřív. Ztišení jednoho repa je `projects[].muteDirty`.
    #[serde(skip_serializing_if = "GitAlerts::is_default")]
    pub git_alerts: GitAlerts,
    /// Rychlé akce (web, soubor, složka, aplikace, příkaz) — karta „Rychlé akce“ a paleta (`actions.rs`).
    pub actions: Vec<crate::actions::Action>,
    /// Jazyk rozhraní (`ui.lang`).
    pub ui: crate::uiconf::UiConfig,
    /// Průvodce prvním spuštěním (`onboarding/`): `done: false` jen u čerstvě založeného configu.
    pub onboarding: crate::onboarding::OnboardingConfig,
    /// Paleta příkazů (`palette.compact`).
    pub palette: crate::paletteconf::PaletteConfig,
    /// Script Commands: složka skriptů s hlavičkou `@ledge.*` → paleta (`scripts/`).
    pub scripts: crate::scripts::ScriptsConfig,
    /// Widgety ze skriptů: složka s hlavičkou `@ledge.widget` → karty a položky proužku (`widgets/`).
    pub widgets: crate::widgets::WidgetsConfig,
    /// Přihrádka na soubory: tah z Průzkumníka na proužek uloží cesty (`shelf/`).
    pub shelf: crate::shelf::ShelfConfig,
    /// Časovač: předvolby odpočtu, toast po doběhnutí, segment v proužku (`timer.rs`).
    pub timer: crate::timer::TimerConfig,
    /// Historie schránky v paletě (`cliphist/`): výchozí vypnuto, jen text, jen paměť.
    pub clipboard: crate::cliphist::ClipboardConfig,
    /// Právě hraje: vzhled karty, čas a ozdoba v proužku (`mediaconf.rs`).
    pub media: crate::mediaconf::MediaConfig,
    /// Kontextové návrhy v šuplíku a puls proužku u GitHubu: které zdroje jsou zapnuté (`suggest/`).
    pub suggest: crate::suggest::SuggestConfig,
    /// GitHub: PR k revizi, vlastní PR s CI a notifikace z přihlášeného `gh` (`github/`). Výchozí vypnuto.
    pub github: crate::github::GithubConfig,
    /// Systém: které disky a jednotky sítě v kartě Systém (`sysmon/`).
    pub system: crate::sysmon::SystemConfig,
    /// Rozložení šuplíku: záložky, karty na nich (pořadí, velikost), skryté části karet (`layout.rs`).
    /// Jediný zdroj pravdy pro to, co šuplík ukazuje — `drawer.sections` se z něj odvozuje.
    pub layout: crate::layout::LayoutConfig,
    /// Účty Microsoft a Google (OAuth, `accounts/`): vlastní registrace aplikace (Client ID), co Ledge
    /// z účtu čte. Přihlášení samo je ve Správci pověření Windows, nikdy tady. Výchozí = nic nepřihlášeno.
    pub accounts: crate::accounts::conf::AccountsConfig,
    /// Lokální MCP server pro Clauda (`mcp/`): Claude smí číst a měnit nastavení Ledge (zástupci, akce,
    /// rozložení, proužek, vzhled), nic nespouští, nic neposílá ven. Výchozí vypnuto; klíč je ve Správci
    /// pověření Windows (`Ledge/mcp`), nikdy tady.
    pub mcp: crate::mcp::McpConfig,
    /// Widgety na ploše (`desk/`): výchozí chování nových widgetů (vrstva, schovat ve hře, průhlednost,
    /// přichycení). Které widgety a kde jsou, je v `%LOCALAPPDATA%\Ledge\desk.json` (jen tento PC).
    pub desk: crate::desk::DeskConfig,
    /// Chytřejší šuplík (kolo 20): klávesy Alt+1…9, přetažení na šuplík, kolečko nad záložkami, zvuk u
    /// schvalování, stav incidentů Anthropicu (`smartconf.rs`). Zvuk a síť jsou výchozí vypnuté.
    pub smart: crate::smartconf::SmartConfig,
    /// Karta Rychlé dlaždice Windows (kolo 23, `winquick.rs`): výběr dlaždic, připnuté složky, přepínání výstupu (výchozí vypnuto).
    pub winquick: crate::winquick::WinQuickConfig,
    /// Oznámení (kolo 26, `notify/`): KDE (Notch v proužku / toast Windows, záložní toast) a JAK (doba,
    /// styl animace pro každou událost). ZDA rozhodují dál vypínače událostí (`claude.toast`, …).
    pub notify: crate::notify::NotifyConfig,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            schema: Some("./config.schema.json".into()),
            strip: StripConfig::default(),
            drawer: DrawerConfig::default(),
            modes: ModesConfig::default(),
            claude: ClaudeConfig::default(),
            hotkeys: HotkeyConfig::default(),
            notes: NotesConfig::default(),
            personal: PersonalConfig::default(),
            roots: default_roots(),
            autostart: true,
            update: UpdateConfig::default(),
            appearance: Default::default(),
            projects: Vec::new(),
            git_alerts: GitAlerts::default(),
            actions: Vec::new(),
            ui: Default::default(),
            onboarding: Default::default(),
            palette: Default::default(),
            scripts: Default::default(),
            widgets: Default::default(),
            shelf: Default::default(),
            timer: Default::default(),
            clipboard: Default::default(),
            media: Default::default(),
            suggest: Default::default(),
            github: Default::default(),
            system: Default::default(),
            layout: Default::default(),
            accounts: Default::default(),
            mcp: Default::default(),
            desk: Default::default(),
            smart: Default::default(),
            winquick: Default::default(),
            notify: Default::default(),
        }
    }
}

fn default_roots() -> Vec<String> {
    crate::dirs::home().map(|p| vec![format!("{}\\Desktop", p.display())]).unwrap_or_default()
}

/// Config proužku žije v `stripconf.rs` (položky, vlastní text, barvy, „v klidu skryté“); re-export,
/// ať se importy `config::StripConfig` / `config::StripItem` jinde nemění.
pub use crate::stripconf::{StripConfig, StripItem};
