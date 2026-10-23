//! Poznámky: kategorie (`notes.kinds`), výchozí sady a synchronizace přes git remote.

use crate::configerr::{FieldError, fe};
use crate::i18n::tr;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
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
