//! Poznámky: kategorie (`notes.kinds`), výchozí sady a synchronizace přes git remote.

use crate::configerr::{FieldError, fe};
use crate::i18n::tr;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::Path;

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
    /// Lístky na plochu: výchozí vzhled nového lístku (`pins/`).
    pub pins: PinDefaults,
    /// Barva lístku podle štítku (`štítek` → id barvy z palety, např. `{"qa": "red"}`); platí, když
    /// poznámka nemá vlastní `color:`. Prázdné = nic.
    pub tag_colors: BTreeMap<String, String>,
    /// Propojení poznámek se složkou `.md` souborů, kterou umí jiné aplikace (Obsidian, Logseq, Joplin…).
    pub folder_link: FolderLinkConfig,
}

impl Default for NotesConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            root: String::new(),
            sync: NotesSyncConfig::default(),
            kinds: default_note_kinds(),
            pins: PinDefaults::default(),
            tag_colors: BTreeMap::new(),
            folder_link: FolderLinkConfig::default(),
        }
    }
}

/// Směr propojení se složkou: co se kam zapisuje.
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum LinkDirection {
    /// Ze složky do Ledge (složka se nikdy nemění).
    Import,
    /// Z Ledge do složky (poznámky ve složce se nikdy nenačítají zpět).
    Export,
    /// Oběma směry.
    #[default]
    Both,
}

/// Propojení poznámek Ledge se složkou `.md` souborů (`notes::folder_sync`). Výchozí vypnuto; bez
/// vybrané složky se nic neděje. Žádná síť, žádné sledování v klidu (události systému souborů).
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(default, rename_all = "camelCase")]
pub struct FolderLinkConfig {
    /// Zapnuto. Vypnutí složku a stav pamatuje, jen se přestane synchronizovat.
    pub enabled: bool,
    /// Absolutní cesta ke složce (Obsidian vault, `pages` Logsequ, export Joplinu…). Prázdné = nic.
    pub path: String,
    /// `import` | `export` | `both` (výchozí).
    pub direction: LinkDirection,
    /// Propagovat i mazání na druhou stranu (výchozí `false` = smazaná poznámka se na druhé straně nechá
    /// být). Smazaný soubor ve složce jde do její podsložky `.trash`, smazaná poznámka Ledge do Archivu.
    pub propagate_deletes: bool,
}

impl Default for FolderLinkConfig {
    fn default() -> Self {
        Self { enabled: false, path: String::new(), direction: LinkDirection::Both, propagate_deletes: false }
    }
}

/// Výchozí vzhled nového lístku (Nastavení → Poznámky → Lístky). Výchozí hodnoty = původní vzhled.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(default, rename_all = "camelCase")]
pub struct PinDefaults {
    /// Styl: `classic` (tmavé sklo), `paper`, `glass`, `stripe`, `minimal`, `dark`. Neznámý = `classic`.
    pub style: String,
    /// Neprůhlednost pozadí 0–100 %.
    pub bg: u8,
    /// Neprůhlednost textu 50–100 %.
    pub text: u8,
    /// Nový lístek převezme naposledy použitý vzhled (jinak vždy tenhle výchozí).
    pub remember_last: bool,
}

impl Default for PinDefaults {
    fn default() -> Self {
        Self { style: "classic".into(), bg: 88, text: 100, remember_last: true }
    }
}

pub(super) fn validate_pin_defaults(p: &PinDefaults, tag_colors: &BTreeMap<String, String>) -> Result<(), FieldError> {
    if crate::pins::PinStyle::from_id(&p.style).is_none() {
        return Err(fe("notes.pins.style", trf!("notes.pins.style: styl „{}“ neexistuje.", "notes.pins.style: style “{}” doesn't exist.", p.style)));
    }
    if p.bg > 100 {
        return Err(fe("notes.pins.bg", tr("notes.pins.bg: 0–100 %.", "notes.pins.bg: 0–100 %.")));
    }
    if !(crate::pins::TEXT_MIN..=100).contains(&p.text) {
        return Err(fe("notes.pins.text", tr("notes.pins.text: 50–100 % (text musí zůstat čitelný).", "notes.pins.text: 50–100 % (text must stay readable).")));
    }
    for (tag, color) in tag_colors {
        if crate::notes::meta::norm_tag(tag).as_deref() != Some(tag.as_str()) {
            return Err(fe("notes.tagColors", trf!("notes.tagColors: štítek „{tag}“ piš malými písmeny bez #.", "notes.tagColors: write tag “{tag}” in lower case without #.")));
        }
        if !crate::notes::meta::valid_color(color) {
            return Err(fe("notes.tagColors", trf!("notes.tagColors: barva „{color}“ u „{tag}“ neexistuje.", "notes.tagColors: color “{color}” of “{tag}” doesn't exist.")));
        }
    }
    Ok(())
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

pub(super) fn validate_note_kinds(kinds: &[NoteKind]) -> Result<(), FieldError> {
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
