//! Záložka Osobní: režimy (zastaralé `modes`), počasí, rychlé odkazy a kalendáře.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(default, rename_all = "camelCase")]
pub struct ModesConfig {
    /// Zastaralé — záložky řídí `layout`. Starý config s výslovným `false` se převede na rozložení bez
    /// záložky Osobní (`layout::from_legacy`); po načtení je pole odvozené (vždy `true`).
    pub personal_enabled: bool,
}

impl Default for ModesConfig {
    fn default() -> Self {
        Self { personal_enabled: true }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(default, rename_all = "camelCase")]
pub struct PersonalConfig {
    /// Místo pro počasí (Open-Meteo, bez klíče). Nenastavené = karta jen vyzve k nastavení,
    /// žádné volání sítě. Polohu podle IP Ledge nikdy nezjišťuje.
    pub weather: WeatherPlace,
    /// Rychlé odkazy (nejvýš `MAX_LINKS`): web (jen http/https), složka, dokument/obrázek/médium
    /// (allowlist přípon) nebo aplikace (.exe bez argumentů).
    pub links: Vec<PersonalLink>,
    /// Kalendáře z tajného odkazu ICS (Google, Outlook — jen čtení, bez přihlášení), karta Kalendář.
    /// Adresa je tajemství: Ledge ji neloguje a v UI ukazuje jen doménu (`calendar/conf.rs`).
    pub calendars: Vec<crate::calendar::conf::CalendarFeed>,
}

/// Nejvíc rychlých odkazů na záložce Osobní.
pub const MAX_LINKS: usize = 24;
/// Nejdelší cíl odkazu (znaky).
pub const MAX_LINK_TARGET: usize = 1024;

#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(default, rename_all = "camelCase")]
pub struct WeatherPlace {
    /// Zeměpisná šířka (−90 až 90), např. 50.08 pro Prahu. `null` = nenastaveno.
    pub lat: Option<f64>,
    /// Zeměpisná délka (−180 až 180), např. 14.42 pro Prahu. `null` = nenastaveno.
    pub lon: Option<f64>,
    /// Název místa na kartě (jen popisek, např. „Praha“).
    pub label: String,
}

impl WeatherPlace {
    /// Obě souřadnice vyplněné a v rozsahu → (lat, lon). Jinak místo není nastavené (jen jedna
    /// vyplněná souřadnice neprojde `validate`).
    pub fn coords(&self) -> Option<(f64, f64)> {
        let (lat, lon) = (self.lat?, self.lon?);
        ((-90.0..=90.0).contains(&lat) && (-180.0..=180.0).contains(&lon)).then_some((lat, lon))
    }
}

/// Rychlý odkaz na záložce Osobní.
#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(default, rename_all = "camelCase")]
pub struct PersonalLink {
    /// Popisek (povinný).
    pub name: String,
    /// `https://…` (jen http/https), absolutní cesta (`C:\…`) ke složce / souboru (txt, md, pdf,
    /// office, obrázky, mp3/mp4/mkv, zip), nebo aplikace `.exe` — absolutně, nebo jen jméno ze
    /// `%SystemRoot%\System32` / `%SystemRoot%`. Síťové cesty, skripty a zástupce (.lnk/.url)
    /// neprojdou. Spouští se vždy bez argumentů. Povinný, nejvýš 1024 znaků.
    pub target: String,
}
