//! Motiv „Podle Windows“ (kolo 34, `appearance.auto`): světlý / tmavý motiv podle `AppsUseLightTheme` a
//! volitelně akcent Windows. Čistá logika bez registru (stav Windows dodává `winlook.rs`): `resolve` z uloženého
//! vzhledu a stavu Windows složí EFEKTIVNÍ vzhled — z něj čte nativní proužek (`stripview`). Zrcadlo
//! `src/theme/auto.ts` (stejná pravidla, vlastní tabulkový test).

use super::{AppearanceConfig, Theme};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::borrow::Cow;

/// Volba „Podle Windows“: dvojice motivů (světlý / tmavý) a akcent Windows. Výchozí = vypnuto (nic se nezapisuje).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(default, rename_all = "camelCase")]
pub struct AutoLook {
    /// Motiv se přepíná podle Windows (aplikace: Barva → Světlý / Tmavý).
    pub on: bool,
    /// Motiv při světlém Windows.
    pub light: Theme,
    /// Motiv při tmavém Windows.
    pub dark: Theme,
    /// Akcent se bere z akcentové barvy Windows (přebije `accent`).
    pub accent: bool,
}

impl Default for AutoLook {
    fn default() -> Self {
        Self { on: false, light: Theme::Light, dark: Theme::Dark, accent: false }
    }
}

impl AutoLook {
    pub fn is_default(&self) -> bool {
        *self == Self::default()
    }

    /// Je potřeba sledovat změny Windows (watcher v `winlook.rs`)?
    pub fn watching(&self) -> bool {
        self.on || self.accent
    }
}

/// Stav Windows: `light` = `AppsUseLightTheme`, `accent` = akcentová barva `#rrggbb`; `None` = nezjištěno.
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WinLook {
    pub light: Option<bool>,
    pub accent: Option<String>,
}

/// Efektivní vzhled: u zapnutého `auto` vybraný motiv podle Windows (vlastní motiv se přeskočí), u `auto.accent`
/// akcent Windows (bez druhé barvy a akcentu z pozadí). Co Windows neřekly, zůstává jak je. Bez `auto` nic nekopíruje.
pub fn resolve<'a>(a: &'a AppearanceConfig, win: &WinLook) -> Cow<'a, AppearanceConfig> {
    let theme = win.light.filter(|_| a.auto.on).map(|l| if l { a.auto.light } else { a.auto.dark });
    let accent = win.accent.as_deref().filter(|_| a.auto.accent);
    if theme.is_none() && accent.is_none() {
        return Cow::Borrowed(a);
    }
    let mut e = a.clone();
    if let Some(t) = theme {
        e.theme = t;
        e.custom_theme.clear();
    }
    if let Some(h) = accent {
        e.accent = h.to_string();
        e.accent2.clear();
        e.accent_from_backdrop = false;
    }
    Cow::Owned(e)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cfg(on: bool, accent: bool) -> AppearanceConfig {
        AppearanceConfig { auto: AutoLook { on, light: Theme::Paper, dark: Theme::Nord, accent }, custom_theme: "my".into(), ..Default::default() }
    }

    #[test]
    fn resolve_table() {
        let win = |l: Option<bool>, c: Option<&str>| WinLook { light: l, accent: c.map(String::from) };
        // (auto.on, auto.accent, Windows, očekávaný motiv, očekávaný akcent, vlastní motiv zrušen)
        let cases = [
            (false, false, win(Some(true), Some("#112233")), Theme::Dark, "", false),
            (true, false, win(Some(true), Some("#112233")), Theme::Paper, "", true),
            (true, false, win(Some(false), None), Theme::Nord, "", true),
            (true, false, win(None, None), Theme::Dark, "", false),
            (false, true, win(Some(true), Some("#112233")), Theme::Dark, "#112233", false),
            (true, true, win(Some(false), Some("#112233")), Theme::Nord, "#112233", true),
            (true, true, win(Some(false), None), Theme::Nord, "", true),
        ];
        for (i, (on, ac, w, theme, accent, cleared)) in cases.into_iter().enumerate() {
            let a = cfg(on, ac);
            let e = resolve(&a, &w);
            assert_eq!((e.theme, e.accent.as_str(), e.custom_theme.is_empty()), (theme, accent, cleared), "případ {i}");
        }
    }

    #[test]
    fn off_borrows_and_old_config_unchanged() {
        let a = AppearanceConfig::default();
        assert!(a.auto.is_default() && !a.auto.watching());
        assert!(matches!(resolve(&a, &WinLook { light: Some(false), accent: Some("#ffffff".into()) }), Cow::Borrowed(_)));
        // starý config bez `auto` se načte a znovu nezapíše pole `auto`
        let old: AppearanceConfig = serde_json::from_str(r##"{"theme":"nord","accent":"#22c55e"}"##).unwrap();
        assert!(old.auto.is_default());
        assert!(!serde_json::to_string(&old).unwrap().contains("\"auto\""));
    }

    #[test]
    fn accent_gradient_cleared() {
        let mut a = cfg(false, true);
        a.accent2 = "#000000".into();
        a.accent_from_backdrop = true;
        let e = resolve(&a, &WinLook { light: None, accent: Some("#abcdef".into()) });
        assert_eq!((e.accent.as_str(), e.accent2.as_str(), e.accent_from_backdrop), ("#abcdef", "", false));
    }
}
