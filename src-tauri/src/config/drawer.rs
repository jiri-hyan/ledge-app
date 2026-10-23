//! Šuplík: spouštěč, prodlevy, šířka, sklo a okna s lištou (`drawer.*`).

use super::sections::Sections;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(default, rename_all = "camelCase")]
pub struct DrawerConfig {
    /// Jak se šuplík otevírá myší: "edge" = přitlačit myš k hornímu okraji v zóně proužku,
    /// "dwell" = podržet myš na proužku, "off" = jen zkratkou.
    pub trigger: Trigger,
    /// Prodleva (ms) při najetí na proužek (trigger "hover").
    pub hover_delay_ms: u64,
    /// Prodleva (ms) při přitlačení k okraji.
    pub edge_delay_ms: u64,
    /// Prodleva (ms) při podržení na proužku (delší, aby to neotvíraly záložky prohlížeče).
    pub dwell_delay_ms: u64,
    /// Za jak dlouho (ms) se šuplík zavře, když z něj myš odjede.
    pub close_delay_ms: u64,
    /// Co se stane po najetí myší: "full" = rovnou celý šuplík (výchozí), "compact" = kompaktní náhled
    /// (schválení, session Clauda, právě hraje, čas a limit 5h), "island" = roztažený ostrůvek (jedno
    /// nejdůležitější dění s tlačítky). Z náhledu otevře celý šuplík klik nebo setrvání myši.
    pub hover: crate::hovermode::HoverMode,
    /// Náhled po najetí ("compact" / "island"): po kolika ms setrvání myši se rozbalí celý šuplík.
    /// 0 = jen klikem; jinak 400–10000 (oříznuto). Nad tlačítky a s čekajícím schválením se neodpočítává.
    pub hover_expand_ms: u64,
    /// Šířka šuplíku v logických px.
    pub width: u32,
    /// "acrylic" = rozmazaná plocha za šuplíkem (Win11), "solid" = tmavé matné pozadí.
    pub glass: Glass,
    /// Karty šuplíku, které jsou zapnuté.
    pub sections: Sections,
    /// Úsporný režim: po kolika sekundách nečinnosti se WebView šuplíku úplně zruší.
    /// 0 = držet připravený stále (okamžité otevření, ~150 MB). >0 = v klidu ~desítky MB,
    /// ale první otevření po nečinnosti trvá několik sekund (studený start WebView2).
    pub keep_warm_s: u64,
    /// „Lištová“ okna (prohlížeče s taby nahoře): v nich proužek ustoupí a šuplík se otevírá jen
    /// tlačením myši na horní hranu, aby ho neotvíralo běžné přepínání tabů.
    pub bar_windows: BarWindowsConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(default, rename_all = "camelCase")]
pub struct BarWindowsConfig {
    /// Procesy (jen název exe, na velikosti písmen nezáleží), jejichž okno v popředí má nahoře lištu.
    /// Platí, jen když je okno maximalizované a leží na monitoru proužku. Přidat lze i další
    /// aplikace s lištou nahoře, např. "Code.exe" (VS Code) nebo "WindowsTerminal.exe".
    pub processes: Vec<String>,
    /// "push" = proužek se smrskne na tenkou linku (dokud nepřijde událost) a šuplík otevře jen
    /// skutečný tlak myši nahoru na horní hranu — bez stisknutého tlačítka a bez pohybu do stran.
    /// "hover" = žádné zvláštní chování, platí obecný `trigger`.
    pub trigger: BarTrigger,
    /// Síla tlaku: kolik „counts“ pohybu myši nahoru (raw input, za kurzorem zastaveným na hraně)
    /// je potřeba nasčítat během 600 ms. 60 ≈ 2 mm pohybu u myši s 800 DPI, ≈ 1 mm u 1600 DPI.
    /// Víc = tlačit déle/silněji. Minimum 10.
    pub push_distance: u32,
    /// Záložní pravidlo, kde tlak měřit nejde (tablet, vzdálená plocha — absolutní pohyb):
    /// jak dlouho (ms) souvisle stát na horní hraně. Minimum 100.
    pub push_ms: u64,
}

impl Default for BarWindowsConfig {
    fn default() -> Self {
        Self {
            processes: ["opera.exe", "chrome.exe", "msedge.exe", "firefox.exe", "arc.exe", "brave.exe", "vivaldi.exe"]
                .into_iter()
                .map(String::from)
                .collect(),
            trigger: BarTrigger::Push,
            push_distance: 60,
            push_ms: 300,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum BarTrigger {
    /// jen tlačení na horní hranu
    Push,
    /// beze změny proti ostatním oknům
    Hover,
}

impl Default for DrawerConfig {
    fn default() -> Self {
        Self {
            trigger: Trigger::Hover,
            hover_delay_ms: 140,
            edge_delay_ms: 120,
            dwell_delay_ms: 450,
            close_delay_ms: 350,
            hover: crate::hovermode::HoverMode::Full,
            hover_expand_ms: crate::hovermode::EXPAND_MS,
            width: 1080,
            glass: Glass::Solid,
            sections: Sections::default(),
            keep_warm_s: 0,
            bar_windows: BarWindowsConfig::default(),
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum Trigger {
    /// najetí myší na proužek (krátká prodleva proti náhodnému průjezdu)
    Hover,
    Edge,
    Dwell,
    Off,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum Glass {
    Acrylic,
    Solid,
}
