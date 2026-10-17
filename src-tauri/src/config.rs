//! Konfigurace Ledge — jeden JSON soubor, který edituje šéf (přes Nastavení) i Claude (přímo).
//!
//! Umístění: `%APPDATA%\Ledge\config.json`, vedle něj `config.schema.json` (generované ze struktur
//! níže přes schemars), aby editor i agent viděli přesný tvar. Soubor se sleduje a po uložení se
//! hned aplikuje. Nevalidní soubor se NEaplikuje — drží se poslední dobrá verze a chyba se ukáže.

use crate::configerr::{FieldError, fe};
use crate::i18n::tr;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

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
    /// Vzhled: motiv, akcent, hustota, písmo, animace, proužek, pořadí karet (`appearance.rs`).
    pub appearance: crate::appearance::AppearanceConfig,
    pub projects: Vec<Project>,
    /// Rychlé akce (web, soubor, složka, aplikace, příkaz) — karta „Rychlé akce“ a paleta (`actions.rs`).
    pub actions: Vec<crate::actions::Action>,
    /// Jazyk rozhraní (`ui.lang`).
    pub ui: crate::uiconf::UiConfig,
    /// Průvodce prvním spuštěním (`onboarding/`): `done: false` jen u čerstvě založeného configu.
    pub onboarding: crate::onboarding::OnboardingConfig,
    /// Paleta příkazů (`palette.compact`).
    pub palette: crate::paletteconf::PaletteConfig,
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
            actions: Vec::new(),
            ui: Default::default(),
            onboarding: Default::default(),
            palette: Default::default(),
        }
    }
}

fn default_roots() -> Vec<String> {
    std::env::var("USERPROFILE")
        .map(|p| vec![format!("{}\\Desktop", p)])
        .unwrap_or_default()
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(default, rename_all = "camelCase")]
pub struct StripConfig {
    /// Proužek nahoře uprostřed. false = zůstane jen svítící linka při událostech.
    pub enabled: bool,
    /// Které informace proužek ukazuje (pořadí = pořadí v proužku).
    pub items: Vec<StripItem>,
    /// Neprůhlednost pozadí proužku 0–1.
    pub opacity: f32,
    /// Monitor: "primary" nebo index od 0 (pořadí z Windows).
    pub monitor: String,
    /// Revize výchozích položek, které už byly do `items` jednou doplněny (`attention::upgrade_items`).
    /// Ručně neměnit; 0 = starý config → při načtení se doplní `waiting`. Chybějící v souboru = 0
    /// (ne výchozí hodnota struktury — ta má aktuální revizi pro nový config).
    #[serde(default)]
    pub items_rev: u32,
}

impl Default for StripConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            items: vec![StripItem::Mode, StripItem::Clock, StripItem::Running, StripItem::Claude, StripItem::Waiting, StripItem::Repos],
            opacity: 0.82,
            monitor: "primary".into(),
            items_rev: crate::attention::ITEMS_REV,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum StripItem {
    Mode,
    /// čas + den a datum
    Clock,
    /// běžící servery jménem a portem
    Running,
    Claude,
    /// kolik rep má neuložené / nepushnuté změny
    Repos,
    /// limity Claude 5h / týden (`limitslive::strip_segments`); výchozí vypnuto
    Limits,
    /// kolik Claude session čeká na tebe (schválení nebo otázka); 0 = nic
    Waiting,
}

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

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(default, rename_all = "camelCase")]
pub struct Sections {
    pub projects: bool,
    pub running: bool,
    pub claude: bool,
    /// Karta „Poznámky“ (poslední poznámky + připomínky na dnes).
    pub notes: bool,
}

impl Default for Sections {
    fn default() -> Self {
        Self { projects: true, running: true, claude: true, notes: true }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(default, rename_all = "camelCase")]
pub struct ModesConfig {
    /// Záložka „Osobní“ (běžné věci mimo práci): čas, počasí, rychlé odkazy, obecné poznámky
    /// a připomínky. Výchozí zapnutá; starý config s výslovným `false` zůstane vypnutý.
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

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(default, rename_all = "camelCase")]
pub struct ClaudeConfig {
    /// Lokální most pro hooky Claude Code (jen 127.0.0.1).
    pub enabled: bool,
    pub port: u16,
    /// Jak dlouho (s) Ledge drží žádost o schválení. Pak ji vrátí terminálu (nikdy neschválí sám).
    pub approval_timeout_s: u64,
    /// Hlídač nedokončených session: soubory, které session sama upravila (Edit/Write/…) a nezacommitovala,
    /// a její commity, které nejsou na žádném remote. `off` = nehlídat, `notify` = jen ukázat v Ledge,
    /// `block` = při Stop vrátit session do práce s důvodem (nikdy při `stop_hook_active`).
    #[serde(default)]
    pub enforce_commit: EnforceCommit,
    /// Otázky Clauda (AskUserQuestion): jak dlouho (s) Ledge drží otázku a čeká na odpověď kliknutím;
    /// pak ji pustí do okna Claude. 0 = Ledge otázky jen ukazuje. Nejvýš 550 (hook má 600 s).
    pub question_hold_s: u64,
    /// Připomínka resetu kvóty: když 5h nebo týdenní limit dosáhl 100 %, v čase resetu krátký peek
    /// a bliknutí proužku (ve fullscreen nic). Data ze stavového řádku Ledge (`limitslive.rs`).
    pub limits_reminder: bool,
    /// Oznámení Windows (toast), když Claude čeká na schválení / otázku nebo dokončí turn. Bez fokusu,
    /// nikdy ve fullscreen hře, v herním režimu ani při „nerušit“; stejná session + událost max 1× za 30 s.
    pub toast: bool,
}

impl Default for ClaudeConfig {
    fn default() -> Self {
        Self { enabled: true, port: 47800, approval_timeout_s: 45, enforce_commit: EnforceCommit::Notify, question_hold_s: 540, limits_reminder: true, toast: false }
    }
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum EnforceCommit {
    /// nehlídat (žádné volání gitu)
    Off,
    /// jen ukázat štítky v šuplíku a proužku
    #[default]
    Notify,
    /// Stop hook vrátí `{"decision":"block"}` → Claude musí commitnout / pushnout
    Block,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(default, rename_all = "camelCase")]
pub struct UpdateConfig {
    /// Kontrolovat nové verze (60 s po startu, pak každých `intervalH` h). Jen upozorní chipem
    /// v hlavičce šuplíku — nainstaluje se až na klik. Dotazy jdou přes veřejné GitHub API bez tokenu,
    /// záložně přes přihlášené `gh` (soukromé repo, limit API).
    pub check: bool,
    /// GitHub repo s vydáními, tvar `owner/name`.
    pub repo: String,
    /// Jak často kontrolovat (hodiny, 1–48).
    pub interval_h: u32,
}

impl Default for UpdateConfig {
    fn default() -> Self {
        Self { check: true, repo: "jiri-hyan/ledge-app".into(), interval_h: 6 }
    }
}

/// `owner/name` z GitHubu: dvě neprázdné části z `[A-Za-z0-9._-]`, ne `.`/`..`.
pub fn valid_repo(s: &str) -> bool {
    let part = |p: &str| !p.is_empty() && p != "." && p != ".." && p.bytes().all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'_' | b'-'));
    let mut it = s.split('/');
    matches!((it.next(), it.next(), it.next()), (Some(o), Some(n), None) if part(o) && part(n))
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(default, rename_all = "camelCase")]
pub struct HotkeyConfig {
    /// Globální zkratky vůbec registrovat. Výchozí `false`: šéf je nepoužívá, šuplík se otevírá myší
    /// a poznámka tlačítkem. Starý config se zkratkami vypsanými, ale bez `enabled`, tak dostane
    /// `false` záměrně. Zbylá pole platí jen při `enabled: true` (i validace kolizí).
    pub enabled: bool,
    /// Otevřít/zavřít šuplík (s fokusem).
    pub toggle: String,
    /// Povolit čekající žádost Claude (registruje se jen, když nějaká čeká).
    pub approve: String,
    /// Zamítnout čekající žádost Claude.
    pub deny: String,
    /// Rychlá poznámka: otevře šuplík (s fokusem) rovnou na zápisu poznámky.
    pub note: String,
    /// Herní režim zapnout / vypnout (`gamemode.rs`). Prázdné = bez zkratky. Funguje i v herním režimu.
    pub game_mode: String,
}

impl Default for HotkeyConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            toggle: "Ctrl+Alt+Space".into(),
            approve: "Ctrl+Alt+Y".into(),
            deny: "Ctrl+Alt+D".into(),
            note: "Ctrl+Alt+N".into(),
            game_mode: String::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(default, rename_all = "camelCase")]
pub struct NotesConfig {
    /// Poznámky zapnuté (zkratka, karta, připomínky).
    pub enabled: bool,
    /// Kořen úložiště (mimo repa projektů; se `sync` je to pracovní kopie repa poznámek). Prázdné = `%USERPROFILE%\Ledge\notes`. Uvnitř složka podle id
    /// projektu (`_obecne` = bez projektu), v ní `<YYYY-MM-DD-HHMMSS>-<slug>.md` + obrázky `…-1.png`.
    pub root: String,
    /// Synchronizace kořene poznámek mezi počítači přes soukromé git repo (`notesync.rs`).
    pub sync: NotesSyncConfig,
    /// Kategorie poznámek (klíč `kind:` ve frontmatteru) — záložky karty a výběr v editoru, v tomhle
    /// pořadí. Poznámka bez `kind` patří do `info`. Neznámé `kind` (z druhého PC) se ukáže jako text.
    pub kinds: Vec<NoteKind>,
}

impl Default for NotesConfig {
    fn default() -> Self {
        Self { enabled: true, root: String::new(), sync: NotesSyncConfig::default(), kinds: default_note_kinds() }
    }
}

/// Jedna kategorie poznámek.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct NoteKind {
    /// Id do frontmatteru (`kind: napad`): 1–32 znaků `[A-Za-z0-9_-]`.
    pub id: String,
    /// Název záložky (česky).
    pub label: String,
    /// Barva `#rrggbb` (prázdné = šedá).
    #[serde(default)]
    pub color: String,
    /// Vestavěná ikona: `bulb`, `chat`, `checklist`, `note`, `flag`, `star`, `bug`, `heart` (prázdné = tečka).
    #[serde(default)]
    pub icon: String,
}

pub const NOTE_KIND_ICONS: [&str; 8] = ["bulb", "chat", "checklist", "note", "flag", "star", "bug", "heart"];

/// Výchozí kategorie poznámek (česky) — hodnota `NotesConfig::default`.
pub fn default_note_kinds() -> Vec<NoteKind> {
    default_note_kinds_in(crate::uiconf::Lang::Cs)
}

/// Výchozí kategorie s popisky v jazyce `lang` (id a barvy stejné). Popisky jsou DATA configu: anglické
/// dostane jen čerstvý config z průvodce v angličtině, nebo config, kterému sekce `notes.kinds` chybí.
pub fn default_note_kinds_in(lang: crate::uiconf::Lang) -> Vec<NoteKind> {
    let en = lang == crate::uiconf::Lang::En;
    let k = |id: &str, cs: &str, english: &str, color: &str, icon: &str| NoteKind {
        id: id.into(),
        label: if en { english } else { cs }.into(),
        color: color.into(),
        icon: icon.into(),
    };
    vec![
        k("napad", "Nápad", "Idea", "#fbbf24", "bulb"),
        k("feedback", "Feedback", "Feedback", "#a78bfa", "chat"),
        k("ukol", "K zapracování", "To work on", "#60a5fa", "checklist"),
        k("info", "Poznámka", "Note", "#94a3b8", "note"),
    ]
}

fn validate_note_kinds(kinds: &[NoteKind]) -> Result<(), FieldError> {
    if kinds.len() > 12 {
        return Err(fe("notes.kinds", tr("notes.kinds: nejvýš 12 kategorií.", "notes.kinds: at most 12 categories.")));
    }
    let mut seen = std::collections::HashSet::new();
    for (i, k) in kinds.iter().enumerate() {
        let at = |f: &str| format!("notes.kinds.{i}.{f}");
        if !crate::notes::valid_kind(&k.id) {
            return Err(fe(at("id"), trf!("notes.kinds: id „{}“ chce 1–32 znaků a-z, 0-9, - nebo _.", "notes.kinds: id “{}” needs 1–32 characters a-z, 0-9, - or _.", k.id)));
        }
        if !seen.insert(k.id.as_str()) {
            return Err(fe(at("id"), trf!("notes.kinds: id „{}“ je dvakrát.", "notes.kinds: id “{}” is there twice.", k.id)));
        }
        if k.label.trim().is_empty() {
            return Err(fe(at("label"), trf!("notes.kinds: „{}“ nemá label.", "notes.kinds: “{}” has no label.", k.id)));
        }
        let hex = k.color.strip_prefix('#').is_some_and(|h| h.len() == 6 && h.chars().all(|c| c.is_ascii_hexdigit()));
        if !k.color.is_empty() && !hex {
            return Err(fe(at("color"), trf!("notes.kinds: barva „{}“ u „{}“ chce tvar #rrggbb.", "notes.kinds: color “{}” of “{}” needs the form #rrggbb.", k.color, k.id)));
        }
        if !k.icon.is_empty() && !NOTE_KIND_ICONS.contains(&k.icon.as_str()) {
            return Err(fe(at("icon"), trf!("notes.kinds: ikona „{}“ neexistuje (jde: {}).", "notes.kinds: icon “{}” doesn't exist (available: {}).", k.icon, NOTE_KIND_ICONS.join(", "))));
        }
    }
    Ok(())
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(default, rename_all = "camelCase")]
pub struct NotesSyncConfig {
    /// Kořen poznámek je pracovní kopie gitu: Ledge sám commitne, stáhne (rebase) a pushne. Přihlášení
    /// jde přes git credential manager (`gh auth setup-git`), nikdy se neptá oknem.
    pub enabled: bool,
    /// Soukromé repo pro poznámky: `https://…`, `git@…` / `ssh://…` (nebo absolutní cesta / `file://`
    /// pro testy). Výchozí prázdné — nový uživatel si sync zapne sám v Nastavení. Starý config bez
    /// `remote` dostane `origin` pracovní kopie poznámek (`onboarding::upgrade_loaded`).
    pub remote: String,
    /// Jak často synchronizovat (minuty, 1–120). Navíc 10 s po lokální změně a při otevření šuplíku
    /// (nejvýš jednou za minutu).
    pub interval_min: u32,
}

impl Default for NotesSyncConfig {
    fn default() -> Self {
        Self { enabled: false, remote: String::new(), interval_min: 5 }
    }
}

/// Povolené tvary remote poznámek: https, ssh (`git@`, `ssh://`), lokální (`file://`, absolutní cesta).
/// Žádné `http://` (heslo by šlo nešifrovaně), žádné mezery ani řídicí znaky, nic začínající `-`.
pub fn valid_notes_remote(r: &str) -> bool {
    let r = r.trim();
    if r.is_empty() || r.starts_with('-') || r.chars().any(|c| c.is_whitespace() || c.is_control()) {
        return false;
    }
    let after = |p: &str| r.strip_prefix(p).is_some_and(|rest| !rest.is_empty());
    after("https://") || after("ssh://") || after("git@") || after("file://") || Path::new(r).is_absolute()
}

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
}

fn yes() -> bool {
    true
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

/// Přepsaná složka configu z prostředí (`LEDGE_CONFIG_DIR`, starší `NOTCH_CONFIG_DIR`) — pro testy a
/// automatizaci. Když je nastavená, migrace ze starého Notche se nespouští.
pub fn config_dir_override() -> Option<PathBuf> {
    ["LEDGE_CONFIG_DIR", "NOTCH_CONFIG_DIR"]
        .iter()
        .find_map(|k| std::env::var(k).ok().filter(|v| !v.trim().is_empty()))
        .map(PathBuf::from)
}

/// `%APPDATA%\Ledge` (config, schéma, log).
pub fn config_dir() -> PathBuf {
    if let Some(dir) = config_dir_override() {
        return dir;
    }
    let base = std::env::var("APPDATA").unwrap_or_else(|_| ".".into());
    Path::new(&base).join("Ledge")
}

/// `%APPDATA%\Notch` — složka starého produktu; čte ji jen migrace (`migrate.rs`).
pub fn legacy_config_dir() -> PathBuf {
    let base = std::env::var("APPDATA").unwrap_or_else(|_| ".".into());
    Path::new(&base).join("Notch")
}

pub fn config_path() -> PathBuf {
    config_dir().join("config.json")
}

pub fn schema_json() -> String {
    let schema = schemars::schema_for!(Config);
    serde_json::to_string_pretty(&schema).unwrap_or_default()
}

/// Načte config. `Ok(None)` = soubor neexistuje.
pub fn load(path: &Path) -> Result<Option<Config>, String> {
    let text = match std::fs::read_to_string(path) {
        Ok(t) => t,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(trf!("Nejde přečíst {}: {e}", "Can't read {}: {e}", path.display())),
    };
    let text = text.trim_start_matches('\u{feff}');
    let raw: serde_json::Value = serde_json::from_str(text).map_err(|e| format!("config.json: {e}"))?;
    let mut cfg = Config::deserialize(&raw).map_err(|e| format!("config.json: {e}"))?;
    crate::onboarding::upgrade_loaded(&raw, &mut cfg);
    // dřívější `drawer.animationMs` → `appearance.motion` (jedno místo pro pohyb)
    crate::appearance::motion::upgrade_loaded(&raw, &mut cfg);
    // nová položka proužku `waiting` i u stávajícího configu (jednou; odebraná zůstane odebraná)
    crate::attention::upgrade_items(&mut cfg.strip);
    validate(&cfg)?;
    Ok(Some(cfg))
}

/// Validace s textem chyby (zpětně kompatibilní tvar; pole viz `validate_field`).
pub fn validate(cfg: &Config) -> Result<(), String> {
    validate_field(cfg).map_err(|e| e.message)
}

/// Validace configu: první chyba i s cestou k poli (Nastavení ji ukáže u pole).
pub fn validate_field(cfg: &Config) -> Result<(), FieldError> {
    let mut seen = std::collections::HashSet::new();
    for (i, p) in cfg.projects.iter().enumerate() {
        let at = |f: &str| format!("projects.{i}.{f}");
        if p.id.trim().is_empty() {
            return Err(fe(at("id"), trf!("Projekt „{}“ nemá id.", "Project “{}” has no id.", p.name)));
        }
        if !seen.insert(p.id.as_str()) {
            return Err(fe(at("id"), trf!("Id projektu „{}“ je dvakrát.", "Project id “{}” is there twice.", p.id)));
        }
        if p.kind == ProjectKind::Dev && p.run.as_deref().unwrap_or("").trim().is_empty() {
            return Err(fe(at("run"), trf!("Projekt „{}“ (dev) nemá příkaz `run`.", "Project “{}” (dev) has no `run` command.", p.id)));
        }
        if p.kind == ProjectKind::App && p.exe.is_none() && p.app_name.is_none() {
            return Err(fe(at("exe"), trf!("Projekt „{}“ (app) potřebuje `exe` nebo `appName`.", "Project “{}” (app) needs `exe` or `appName`.", p.id)));
        }
        if let Some(a) = &p.accent {
            let ok = a.len() == 7 && a.starts_with('#') && a[1..].chars().all(|c| c.is_ascii_hexdigit());
            if !ok {
                return Err(fe(at("accent"), trf!("Projekt „{}“: accent musí být #rrggbb.", "Project “{}”: accent must be #rrggbb.", p.id)));
            }
        }
    }
    let w = &cfg.personal.weather;
    if w.lat.is_some() != w.lon.is_some() {
        let missing = if w.lat.is_none() { "personal.weather.lat" } else { "personal.weather.lon" };
        return Err(fe(missing, tr("personal.weather: vyplň zeměpisnou šířku i délku, nebo obě smaž.", "personal.weather: fill in both latitude and longitude, or clear both.")));
    }
    if w.lat.is_some_and(|v| !(-90.0..=90.0).contains(&v)) {
        return Err(fe("personal.weather.lat", tr("personal.weather.lat musí být −90 až 90.", "personal.weather.lat must be −90 to 90.")));
    }
    if w.lon.is_some_and(|v| !(-180.0..=180.0).contains(&v)) {
        return Err(fe("personal.weather.lon", tr("personal.weather.lon musí být −180 až 180.", "personal.weather.lon must be −180 to 180.")));
    }
    let links = &cfg.personal.links;
    if links.len() > MAX_LINKS {
        return Err(fe("personal.links", trf!("personal.links: nejvýš {MAX_LINKS} odkazů (je jich {}).", "personal.links: at most {MAX_LINKS} links (there are {}).", links.len())));
    }
    for (i, l) in links.iter().enumerate() {
        let n = i + 1;
        if l.name.trim().is_empty() {
            return Err(fe(format!("personal.links.{i}.name"), trf!("Odkaz č. {n} nemá název.", "Link no. {n} has no name.")));
        }
        if l.target.trim().is_empty() {
            return Err(fe(format!("personal.links.{i}.target"), trf!("Odkaz „{}“ nemá cíl.", "Link “{}” has no target.", l.name)));
        }
        if l.target.chars().count() > MAX_LINK_TARGET {
            return Err(fe(format!("personal.links.{i}.target"), trf!("Odkaz „{}“: cíl je delší než {MAX_LINK_TARGET} znaků.", "Link “{}”: the target is longer than {MAX_LINK_TARGET} characters.", l.name)));
        }
    }
    validate_note_kinds(&cfg.notes.kinds)?;
    let sync = &cfg.notes.sync;
    if !(1..=120).contains(&sync.interval_min) {
        return Err(fe("notes.sync.intervalMin", tr("notes.sync.intervalMin musí být 1–120 minut.", "notes.sync.intervalMin must be 1–120 minutes.")));
    }
    if (sync.enabled || !sync.remote.trim().is_empty()) && !valid_notes_remote(&sync.remote) {
        return Err(fe(
            "notes.sync.remote",
            trf!(
                "notes.sync.remote „{}“: chce https://…, git@…, ssh://… (nebo absolutní cestu / file:// k testovacímu repu).",
                "notes.sync.remote “{}”: needs https://…, git@…, ssh://… (or an absolute path / file:// to a test repo).",
                sync.remote
            ),
        ));
    }
    if !(1..=48).contains(&cfg.update.interval_h) {
        return Err(fe("update.intervalH", tr("update.intervalH musí být 1–48 (hodin).", "update.intervalH must be 1–48 (hours).")));
    }
    if !valid_repo(&cfg.update.repo) {
        return Err(fe("update.repo", trf!("update.repo „{}“ musí mít tvar owner/name (např. jiri-hyan/ledge-app).", "update.repo “{}” must have the form owner/name (e.g. jiri-hyan/ledge-app).", cfg.update.repo)));
    }
    if !(0.0..=1.0).contains(&cfg.strip.opacity) {
        return Err(fe("strip.opacity", tr("strip.opacity musí být 0–1.", "strip.opacity must be 0–1.")));
    }
    // bez myši i bez zkratky by šuplík nešel otevřít vůbec
    if cfg.drawer.trigger == Trigger::Off && (!cfg.hotkeys.enabled || cfg.hotkeys.toggle.trim().is_empty()) {
        return Err(fe("drawer.trigger", tr(
                "drawer.trigger „off“ jde jen se zapnutou zkratkou (hotkeys.enabled + hotkeys.toggle) — bez zkratek musí jít šuplík otevřít myší.",
                "drawer.trigger “off” only works with the shortcut on (hotkeys.enabled + hotkeys.toggle) — without shortcuts the drawer must open with the mouse.",
            )));
    }
    hotkey_collision(cfg)?;
    crate::actions::validate(&cfg.actions)?;
    crate::appearance::validate(&cfg.appearance)?;
    Ok(())
}

/// Tvar zkratky pro porovnání: malá písmena, sjednocené aliasy modifikátorů, modifikátory seřazené
/// (`Alt+Ctrl+n` == `ctrl + alt + N`).
pub fn normalize_hotkey(s: &str) -> String {
    let mut mods = Vec::new();
    let mut key = String::new();
    for part in s.split('+').map(|p| p.trim().to_lowercase()).filter(|p| !p.is_empty()) {
        match part.as_str() {
            "ctrl" | "control" | "cmdorctrl" | "commandorcontrol" | "cmdorcontrol" | "commandorctrl" => mods.push("ctrl"),
            "alt" | "option" => mods.push("alt"),
            "shift" => mods.push("shift"),
            "super" | "win" | "meta" | "cmd" | "command" => mods.push("super"),
            _ => key = part,
        }
    }
    mods.sort_unstable();
    mods.dedup();
    mods.push(&key);
    mods.join("+")
}

/// Akce, které můžou mít zkratku zaregistrovanou: s vypnutými zkratkami žádné, poznámka jen se
/// zapnutými poznámkami.
fn hotkey_actions(cfg: &Config) -> Vec<(&'static str, &str)> {
    let h = &cfg.hotkeys;
    if !h.enabled {
        return Vec::new();
    }
    let mut v = vec![("toggle", h.toggle.as_str()), ("approve", h.approve.as_str()), ("deny", h.deny.as_str())];
    if cfg.notes.enabled {
        v.push(("note", h.note.as_str()));
    }
    v.push(("gameMode", h.game_mode.as_str()));
    v
}

/// Dvě akce na stejné zkratce → chyba (druhá by se tiše nezaregistrovala nebo spouštěla obě).
fn hotkey_collision(cfg: &Config) -> Result<(), FieldError> {
    let acts = hotkey_actions(cfg);
    for (i, (a, ka)) in acts.iter().enumerate() {
        for (b, kb) in &acts[i + 1..] {
            if !ka.trim().is_empty() && normalize_hotkey(ka) == normalize_hotkey(kb) {
                return Err(fe(format!("hotkeys.{b}"), trf!("Zkratka „{ka}“ je u dvou akcí: hotkeys.{a} a hotkeys.{b}.", "Shortcut “{ka}” is used by two actions: hotkeys.{a} and hotkeys.{b}.")));
            }
        }
    }
    Ok(())
}

/// Zkratky, které mají být právě zaregistrované: s `hotkeys.enabled = false` žádné; jinak toggle
/// vždy, note se zapnutými poznámkami, approve/deny jen když čeká žádost Claude (`approve`).
pub fn active_hotkeys(cfg: &Config, approve: bool) -> Vec<(&'static str, String)> {
    hotkey_actions(cfg)
        .into_iter()
        .filter(|(a, _)| approve || !matches!(*a, "approve" | "deny"))
        .filter(|(_, k)| !k.trim().is_empty())
        .map(|(a, k)| (a, k.to_string()))
        .collect()
}

/// Plán přeregistrace: beze změny nic; jinak odregistrovat VŠECHNY staré a pak zaregistrovat celou
/// novou sadu — prohození dvou zkratek (A↔B) tak nikdy neskončí odregistrovanou novou zkratkou.
pub fn rebind_plan(old: &[(&'static str, String)], new: &[(&'static str, String)]) -> (Vec<String>, Vec<String>) {
    if old == new {
        return (Vec::new(), Vec::new());
    }
    let uniq = |v: &[(&'static str, String)]| {
        let mut out: Vec<String> = Vec::new();
        for (_, k) in v {
            if !out.iter().any(|o| normalize_hotkey(o) == normalize_hotkey(k)) {
                out.push(k.clone());
            }
        }
        out
    };
    (uniq(old), uniq(new))
}

/// Atomický zápis: tmp soubor + rename, ať watcher ani šéf nikdy neuvidí půlku souboru. Zápisy jsou
/// sériové (Nastavení i průvodce běží na různých vláknech) a každý má vlastní tmp. Nečitelný
/// soubor v cíli (rozbitý JSON — v paměti jsou pak jen výchozí hodnoty) se nejdřív zazálohuje
/// (`configio::keep_unreadable`), takže uložení z Nastavení nikdy potichu nesmaže šéfovy projekty.
pub fn save(path: &Path, cfg: &Config) -> Result<(), String> {
    static SAVING: parking_lot::Mutex<u64> = parking_lot::Mutex::new(0);
    validate(cfg)?;
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    let text = serde_json::to_string_pretty(cfg).map_err(|e| e.to_string())?;
    let mut n = SAVING.lock();
    *n += 1;
    crate::configio::keep_unreadable(path)?;
    let tmp = path.with_extension(format!("json.{}-{}.tmp", std::process::id(), *n));
    let res = std::fs::write(&tmp, text + "\n").and_then(|_| std::fs::rename(&tmp, path));
    if res.is_err() {
        let _ = std::fs::remove_file(&tmp);
    }
    res.map_err(|e| trf!("{} nejde zapsat: {e}", "can't write {}: {e}", path.display()))
}

pub fn write_schema(dir: &Path) {
    let _ = std::fs::create_dir_all(dir);
    let _ = std::fs::write(dir.join("config.schema.json"), schema_json());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_object_gives_defaults() {
        let cfg: Config = serde_json::from_str("{}").unwrap();
        assert_eq!(cfg.drawer.trigger, Trigger::Hover);
        assert!(cfg.modes.personal_enabled);
        assert_eq!(cfg.claude.port, 47800);
        assert_eq!(cfg.claude.enforce_commit, EnforceCommit::Notify);
    }

    #[test]
    fn validation_messages_follow_language() {
        let mut cfg = Config::default();
        cfg.strip.opacity = 2.0;
        assert_eq!(validate(&cfg).unwrap_err(), "strip.opacity musí být 0–1.");
        crate::i18n::with_lang(crate::uiconf::Lang::En, || {
            let e = validate_field(&cfg).unwrap_err();
            assert_eq!((e.field.as_deref(), e.message.as_str()), (Some("strip.opacity"), "strip.opacity must be 0–1."));
        });
        let en: Vec<String> = default_note_kinds_in(crate::uiconf::Lang::En).into_iter().map(|k| k.label).collect();
        assert_eq!(en, ["Idea", "Feedback", "To work on", "Note"]);
        assert_eq!(default_note_kinds(), default_note_kinds_in(crate::uiconf::Lang::Cs));
    }

    #[test]
    fn enforce_commit_old_config_and_values() {
        // starý config bez pole → notify
        let cfg: Config = serde_json::from_str(r#"{"claude":{"enabled":true,"port":47800,"approvalTimeoutS":45}}"#).unwrap();
        assert_eq!(cfg.claude.enforce_commit, EnforceCommit::Notify);
        for (j, e) in [("off", EnforceCommit::Off), ("notify", EnforceCommit::Notify), ("block", EnforceCommit::Block)] {
            let cfg: Config = serde_json::from_str(&format!(r#"{{"claude":{{"enforceCommit":"{j}"}}}}"#)).unwrap();
            assert_eq!(cfg.claude.enforce_commit, e);
        }
        assert!(serde_json::from_str::<Config>(r#"{"claude":{"enforceCommit":"nevim"}}"#).is_err());
        assert!(schema_json().contains("enforceCommit"));
    }

    #[test]
    fn roundtrip_and_atomic_save() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.json");
        let mut cfg = Config::default();
        cfg.projects.push(Project {
            id: "acme-app".into(),
            name: "Acme App".into(),
            enabled: true,
            kind: ProjectKind::Dev,
            path: None,
            remote: Some("acme/app".into()),
            subdir: None,
            run: Some("npm run dev:staging".into()),
            port: Some(5210),
            pass_port: true,
            open_browser: true,
            exe: None,
            app_name: None,
            args: vec![],
            accent: Some("#e0a13a".into()),
            monogram: None,
            logo: None,
            links: vec![],
        });
        save(&path, &cfg).unwrap();
        let names: Vec<String> = std::fs::read_dir(dir.path()).unwrap().map(|e| e.unwrap().file_name().to_string_lossy().into_owned()).collect();
        assert_eq!(names, ["config.json"], "žádný tmp ani záloha");
        let back = load(&path).unwrap().unwrap();
        assert_eq!(back, cfg);
    }

    #[test]
    fn rejects_duplicate_ids_and_bad_accent() {
        let mut cfg = Config::default();
        let p = Project {
            id: "a".into(),
            name: "A".into(),
            enabled: true,
            kind: ProjectKind::App,
            path: None,
            remote: None,
            subdir: None,
            run: None,
            port: None,
            pass_port: true,
            open_browser: true,
            exe: Some("a.exe".into()),
            app_name: None,
            args: vec![],
            accent: None,
            monogram: None,
            logo: None,
            links: vec![],
        };
        cfg.projects = vec![p.clone(), p.clone()];
        assert!(validate(&cfg).unwrap_err().contains("dvakrát"));
        let mut q = p.clone();
        q.accent = Some("red".into());
        cfg.projects = vec![q];
        assert!(validate(&cfg).is_err());
    }

    #[test]
    fn missing_file_is_none_and_bom_is_tolerated() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.json");
        assert!(load(&path).unwrap().is_none());
        std::fs::write(&path, "\u{feff}{}").unwrap();
        assert!(load(&path).unwrap().is_some());
    }

    #[test]
    fn old_config_without_bar_windows_gets_browser_defaults() {
        let cfg: Config = serde_json::from_str(r#"{"drawer":{"trigger":"hover","hoverDelayMs":140}}"#).unwrap();
        let b = &cfg.drawer.bar_windows;
        assert_eq!(b.trigger, BarTrigger::Push);
        assert_eq!(b.push_ms, 300);
        assert_eq!(b.push_distance, 60);
        assert!(b.processes.iter().any(|p| p == "opera.exe"));
        assert!(!b.processes.iter().any(|p| p.eq_ignore_ascii_case("code.exe")));
        // částečný blok: chybějící pole doplní výchozí
        let cfg: Config = serde_json::from_str(r#"{"drawer":{"barWindows":{"pushMs":500}}}"#).unwrap();
        assert_eq!(cfg.drawer.bar_windows.push_ms, 500);
        assert_eq!(cfg.drawer.bar_windows.processes.len(), 7);
        let cfg: Config = serde_json::from_str(r#"{"drawer":{"barWindows":{"trigger":"hover","processes":["Code.exe"]}}}"#).unwrap();
        assert_eq!(cfg.drawer.bar_windows.trigger, BarTrigger::Hover);
        assert_eq!(cfg.drawer.bar_windows.processes, vec!["Code.exe".to_string()]);
    }

    #[test]
    fn old_config_without_notes_gets_defaults() {
        let old = r#"{"drawer":{"sections":{"projects":true,"running":false,"claude":true}},
            "hotkeys":{"toggle":"Ctrl+Alt+Space","approve":"Ctrl+Alt+Y","deny":"Ctrl+Alt+D"}}"#;
        let cfg: Config = serde_json::from_str(old).unwrap();
        assert!(cfg.notes.enabled);
        assert_eq!(cfg.notes.root, "");
        assert_eq!(cfg.notes.kinds, default_note_kinds());
        assert_eq!(cfg.hotkeys.note, "Ctrl+Alt+N");
        assert!(cfg.drawer.sections.notes);
        assert!(!cfg.drawer.sections.running);
        let cfg: Config =
            serde_json::from_str(r#"{"notes":{"enabled":false},"hotkeys":{"note":"Ctrl+Alt+M"},"drawer":{"sections":{"notes":false}}}"#).unwrap();
        assert!(!cfg.notes.enabled);
        assert_eq!(cfg.hotkeys.note, "Ctrl+Alt+M");
        assert_eq!(cfg.hotkeys.toggle, "Ctrl+Alt+Space");
        assert!(!cfg.drawer.sections.notes);
        let s = schema_json();
        assert!(s.contains("\"notes\"") && s.contains("\"note\""));
        // bez notes.sync → vypnuto, bez remote (starý config dorovná `onboarding::upgrade_loaded` v `load`)
        assert_eq!(cfg.notes.sync, NotesSyncConfig::default());
        assert!(!cfg.notes.sync.enabled && cfg.notes.sync.interval_min == 5 && cfg.notes.sync.remote.is_empty());
        assert!(s.contains("intervalMin"));
    }

    #[test]
    fn notes_sync_validation() {
        const NOTES: &str = "https://github.com/acme/notes.git";
        let with = |enabled: bool, remote: &str, interval_min: u32| {
            let mut c = Config::default();
            c.notes.sync = NotesSyncConfig { enabled, remote: remote.into(), interval_min };
            validate(&c)
        };
        assert!(with(true, NOTES, 5).is_ok());
        for ok in ["git@github.com:acme/notes.git", "ssh://git@github.com/x/y.git", "file:///C:/tmp/bare", r"C:\tmp\bare.git", "D:/x/notes.git"] {
            assert!(with(true, ok, 5).is_ok(), "{ok}");
        }
        for bad in ["", "http://github.com/x/y.git", "github.com/x/y.git", "-uexec", "https://", "https://x/ y", "relativni/cesta", "ext::sh -c x"] {
            assert!(with(true, bad, 5).is_err(), "{bad}");
        }
        // vypnutý sync s prázdným remote je v pořádku, s nesmyslem ne
        assert!(with(false, "", 5).is_ok());
        assert!(with(false, "http://x/y", 5).is_err());
        assert!(with(true, NOTES, 0).is_err());
        assert!(with(true, NOTES, 121).is_err());
        assert!(with(true, NOTES, 120).is_ok() && with(true, NOTES, 1).is_ok());
        let cfg: Config = serde_json::from_str(r#"{"notes":{"sync":{"enabled":false}}}"#).unwrap();
        assert!(!cfg.notes.sync.enabled && cfg.notes.sync.remote.is_empty() && cfg.notes.sync.interval_min == 5);
    }

    #[test]
    fn hotkeys_off_by_default_even_in_old_config() {
        assert!(!Config::default().hotkeys.enabled);
        assert!(active_hotkeys(&Config::default(), true).is_empty());
        // šéfův config: zkratky vypsané výslovně, pole `enabled` chybí → nic se neregistruje
        let old = r#"{"hotkeys":{"toggle":"Ctrl+Alt+Space","approve":"Ctrl+Alt+Y","deny":"Ctrl+Alt+D","note":"Ctrl+Alt+N"}}"#;
        let cfg = load_str(old).unwrap();
        assert!(!cfg.hotkeys.enabled);
        assert_eq!(cfg.hotkeys.toggle, "Ctrl+Alt+Space");
        assert!(active_hotkeys(&cfg, false).is_empty());
        assert!(active_hotkeys(&cfg, true).is_empty());
        // vypnutí za běhu = odregistrovat všechno, nic nového
        let was = vec![("toggle", "Ctrl+Alt+Space".to_string()), ("note", "Ctrl+Alt+N".to_string())];
        assert_eq!(rebind_plan(&was, &active_hotkeys(&cfg, true)), (vec!["Ctrl+Alt+Space".to_string(), "Ctrl+Alt+N".into()], vec![]));
        // výslovně zapnuté se registrují jako dřív
        let on = load_str(r#"{"hotkeys":{"enabled":true}}"#).unwrap();
        assert_eq!(active_hotkeys(&on, false).len(), 2);
        assert!(schema_json().contains("\"enabled\""));
    }

    #[test]
    fn trigger_off_needs_hotkeys() {
        // šuplík by nešel otevřít vůbec
        let e = load_str(r#"{"drawer":{"trigger":"off"}}"#).unwrap_err();
        assert!(e.contains("myší"), "{e}");
        assert!(load_str(r#"{"drawer":{"trigger":"off"},"hotkeys":{"toggle":"Ctrl+Alt+Space"}}"#).is_err());
        assert!(load_str(r#"{"drawer":{"trigger":"off"},"hotkeys":{"enabled":true,"toggle":" "}}"#).is_err());
        // se zapnutou zkratkou to jde, myší ovládané triggery vždy
        assert!(load_str(r#"{"drawer":{"trigger":"off"},"hotkeys":{"enabled":true}}"#).is_ok());
        assert!(load_str(r#"{"drawer":{"trigger":"edge"}}"#).is_ok());
    }

    #[test]
    fn hotkey_normalize_and_collisions() {
        assert_eq!(normalize_hotkey("Alt+Ctrl+N"), normalize_hotkey(" ctrl + alt + n "));
        assert_eq!(normalize_hotkey("Control+Option+Space"), "alt+ctrl+space");
        assert_ne!(normalize_hotkey("Ctrl+Alt+N"), normalize_hotkey("Ctrl+Shift+N"));
        let mut cfg = Config::default();
        // vypnuté zkratky se nekontrolují (nikdy se neregistrují)
        cfg.hotkeys.note = "alt+ctrl+d".into();
        assert!(validate(&cfg).is_ok());
        cfg.hotkeys = HotkeyConfig { enabled: true, ..HotkeyConfig::default() };
        assert!(validate(&cfg).is_ok());
        cfg.hotkeys.note = "alt+ctrl+d".into(); // = deny
        let e = validate(&cfg).unwrap_err();
        assert!(e.contains("hotkeys.deny") && e.contains("hotkeys.note"), "{e}");
        // s vypnutými poznámkami note nekoliduje (nikdy se neregistruje)
        cfg.notes.enabled = false;
        assert!(validate(&cfg).is_ok());
        cfg.hotkeys.toggle = "Ctrl+Alt+Y".into(); // = approve
        assert!(validate(&cfg).is_err());
        // starý config s kolizí se nenačte (zůstane poslední dobrý) — jen se zapnutými zkratkami
        assert!(load_str(r#"{"hotkeys":{"enabled":true,"toggle":"Ctrl+Alt+N"}}"#).is_err());
        assert!(load_str(r#"{"hotkeys":{"toggle":"Ctrl+Alt+N"}}"#).is_ok());
    }

    fn load_str(s: &str) -> Result<Config, String> {
        let cfg: Config = serde_json::from_str(s).map_err(|e| e.to_string())?;
        validate(&cfg)?;
        Ok(cfg)
    }

    #[test]
    fn active_set_and_rebind_plan() {
        let mut cfg = Config::default();
        cfg.hotkeys.enabled = true;
        let keys = |v: &[(&'static str, String)]| v.iter().map(|(a, k)| format!("{a}={k}")).collect::<Vec<_>>();
        assert_eq!(keys(&active_hotkeys(&cfg, false)), ["toggle=Ctrl+Alt+Space", "note=Ctrl+Alt+N"]);
        assert_eq!(keys(&active_hotkeys(&cfg, true)), ["toggle=Ctrl+Alt+Space", "approve=Ctrl+Alt+Y", "deny=Ctrl+Alt+D", "note=Ctrl+Alt+N"]);
        cfg.notes.enabled = false;
        assert_eq!(keys(&active_hotkeys(&cfg, false)), ["toggle=Ctrl+Alt+Space"]);

        // beze změny nic
        let a = active_hotkeys(&cfg, false);
        assert_eq!(rebind_plan(&a, &a), (vec![], vec![]));
        // prohození A↔B: nejdřív pryč obě staré, pak obě nové
        let old = vec![("toggle", "Ctrl+Alt+A".to_string()), ("note", "Ctrl+Alt+B".to_string())];
        let new = vec![("toggle", "Ctrl+Alt+B".to_string()), ("note", "Ctrl+Alt+A".to_string())];
        assert_eq!(rebind_plan(&old, &new), (vec!["Ctrl+Alt+A".to_string(), "Ctrl+Alt+B".into()], vec!["Ctrl+Alt+B".to_string(), "Ctrl+Alt+A".into()]));
        // vypnutí poznámek: odregistrovat i note
        let off = vec![("toggle", "Ctrl+Alt+A".to_string())];
        assert_eq!(rebind_plan(&old, &off), (vec!["Ctrl+Alt+A".to_string(), "Ctrl+Alt+B".into()], vec!["Ctrl+Alt+A".to_string()]));
    }

    #[test]
    fn personal_defaults_and_old_config() {
        // výchozí: záložka zapnutá, místo nenastavené, žádné odkazy
        let cfg = Config::default();
        assert!(cfg.modes.personal_enabled);
        assert_eq!(cfg.personal.weather, WeatherPlace { lat: None, lon: None, label: String::new() });
        assert!(cfg.personal.weather.coords().is_none());
        assert!(cfg.personal.links.is_empty());
        // nenastavené místo se zapíše jako null (šéf vidí, co doplnit)
        let v = serde_json::to_value(&cfg).unwrap();
        assert_eq!(v["personal"]["weather"], serde_json::json!({ "lat": null, "lon": null, "label": "" }));
        assert_eq!(v["personal"]["links"], serde_json::json!([]));
        // starý config s výslovným false zůstane vypnutý, bez bloku personal dostane výchozí
        let cfg: Config = serde_json::from_str(r#"{"modes":{"personalEnabled":false}}"#).unwrap();
        assert!(!cfg.modes.personal_enabled);
        assert!(cfg.personal.links.is_empty());
        let cfg: Config = serde_json::from_str(r#"{"modes":{}}"#).unwrap();
        assert!(cfg.modes.personal_enabled);
        // vyplněný blok
        let cfg: Config = serde_json::from_str(
            r#"{"personal":{"weather":{"lat":50.08,"lon":14.42,"label":"Praha"},"links":[{"name":"Mapy","target":"https://mapy.cz"}]}}"#,
        )
        .unwrap();
        assert_eq!(cfg.personal.weather.coords(), Some((50.08, 14.42)));
        assert_eq!(cfg.personal.weather.label, "Praha");
        assert_eq!(cfg.personal.links[0].target, "https://mapy.cz");
        assert!(validate(&cfg).is_ok());
        // jen jedna souřadnice = chyba validace (ne tiché „nenastaveno“); mimo rozsah = chyba
        let cfg: Config = serde_json::from_str(r#"{"personal":{"weather":{"lat":50.0}}}"#).unwrap();
        assert!(cfg.personal.weather.coords().is_none());
        assert!(validate(&cfg).unwrap_err().contains("šířku i délku"));
        let cfg: Config = serde_json::from_str(r#"{"personal":{"weather":{"lon":14.0}}}"#).unwrap();
        assert!(validate(&cfg).is_err());
        let cfg: Config = serde_json::from_str(r#"{"personal":{"weather":{"lat":91,"lon":0}}}"#).unwrap();
        assert!(validate(&cfg).is_err());
        let cfg: Config = serde_json::from_str(r#"{"personal":{"weather":{"lat":0,"lon":-181}}}"#).unwrap();
        assert!(validate(&cfg).is_err());
        assert!(schema_json().contains("\"personal\""));
    }

    #[test]
    fn personal_links_validation() {
        let link = |name: &str, target: &str| PersonalLink { name: name.into(), target: target.into() };
        let mut cfg = Config::default();
        cfg.personal.links = vec![link("Mapy", "https://mapy.cz")];
        assert!(validate(&cfg).is_ok());
        cfg.personal.links = vec![link(" ", "https://mapy.cz")];
        assert!(validate(&cfg).unwrap_err().contains("č. 1 nemá název"));
        cfg.personal.links = vec![link("Mapy", "https://mapy.cz"), link("Fotky", "  ")];
        assert!(validate(&cfg).unwrap_err().contains("„Fotky“ nemá cíl"));
        cfg.personal.links = vec![link("Dlouhý", &format!("https://x.cz/{}", "a".repeat(MAX_LINK_TARGET)))];
        assert!(validate(&cfg).unwrap_err().contains("1024"));
        cfg.personal.links = vec![link("Akorát", &"a".repeat(MAX_LINK_TARGET))];
        assert!(validate(&cfg).is_ok());
        cfg.personal.links = (0..=MAX_LINKS).map(|i| link(&format!("L{i}"), "https://x.cz")).collect();
        assert!(validate(&cfg).unwrap_err().contains("nejvýš 24"));
        cfg.personal.links.pop();
        assert!(validate(&cfg).is_ok());
    }

    #[test]
    fn note_kinds_validation() {
        let mut c = Config::default();
        assert!(validate(&c).is_ok());
        assert!(schema_json().contains("\"kinds\""));
        let bad = |f: &dyn Fn(&mut NoteKind)| {
            let mut k = default_note_kinds();
            f(&mut k[0]);
            k
        };
        for kinds in [
            bad(&|k| k.id = "a b".into()),
            bad(&|k| k.id = "feedback".into()),
            bad(&|k| k.label = " ".into()),
            bad(&|k| k.color = "red".into()),
            bad(&|k| k.icon = "<svg>".into()),
        ] {
            c.notes.kinds = kinds;
            assert!(validate(&c).is_err(), "{:?}", c.notes.kinds[0]);
        }
        c.notes.kinds = bad(&|k| {
            k.color = String::new();
            k.icon = String::new();
        });
        assert!(validate(&c).is_ok());
        // vlastní sada z configu (bez výchozích) projde
        let cfg: Config = serde_json::from_str(r#"{"notes":{"kinds":[{"id":"bug","label":"Chyba"}]}}"#).unwrap();
        assert_eq!(cfg.notes.kinds.len(), 1);
        assert!(validate(&cfg).is_ok());
    }

    #[test]
    fn schema_mentions_projects() {
        assert!(schema_json().contains("projects"));
    }
}
