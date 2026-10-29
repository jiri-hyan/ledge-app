//! Config `notify` (kolo 26): KDE a JAK se oznámení ukáže — proužek (Notch) nebo toast Windows, záložní
//! toast, doba a styl animace pro každou událost. ZDA se událost hlásí, rozhodují dál dnešní vypínače
//! (`claude.toast`, `claude.limitsAlert`, `timer.toast`, `personal.calendars[].remindToast`) — tady žádný
//! druhý vypínač téže věci.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Kam jde oznámení. `notch` = proužek nahoře se rozbalí (výchozí od kola 26), `toast` = jen Windows toast
/// (dřívější chování).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum Surface {
    #[default]
    Notch,
    Toast,
}

/// Choreografie Notche: `unfold` = rozbalení dolů do kartičky, `sides` = rozšíření do stran, `drop` = kapka
/// (bublina spadne pod proužek), `ticker` = text projede, `glow` = jen záře.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum NotchStyle {
    Unfold,
    Sides,
    Drop,
    Ticker,
    Glow,
}

impl NotchStyle {
    #[cfg(test)]
    pub const ALL: [NotchStyle; 5] = [NotchStyle::Unfold, NotchStyle::Sides, NotchStyle::Drop, NotchStyle::Ticker, NotchStyle::Glow];
    /// Má tvar místo na tlačítko (Otevřít / Připojit)? Ticker a záře ne — nic, co jede, se neklikne.
    pub fn has_button(self) -> bool {
        matches!(self, NotchStyle::Unfold | NotchStyle::Sides | NotchStyle::Drop)
    }
}

/// Styl pro každou událost; každá má svůj výchozí.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(default, rename_all = "camelCase")]
pub struct NotchStyles {
    pub waiting: NotchStyle,
    pub done: NotchStyle,
    pub meeting: NotchStyle,
    pub limit: NotchStyle,
    pub timer: NotchStyle,
}

impl Default for NotchStyles {
    fn default() -> Self {
        Self { waiting: NotchStyle::Unfold, done: NotchStyle::Sides, meeting: NotchStyle::Drop, limit: NotchStyle::Ticker, timer: NotchStyle::Glow }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(default, rename_all = "camelCase")]
pub struct NotifyConfig {
    /// `notch` (výchozí) nebo `toast` (jen Windows toast jako dřív).
    pub surface: Surface,
    /// Když proužek není vidět (vypnutý, šuplík přes něj, kurzor na jiném monitoru), pošli aspoň toast.
    pub fallback_toast: bool,
    /// Jak dlouho (s) zůstane Notch rozbalený: 2–30, výchozí 6 („Hotovo“ trvá 2/3 z toho).
    pub duration_s: u32,
    pub style: NotchStyles,
}

impl Default for NotifyConfig {
    fn default() -> Self {
        Self { surface: Surface::Notch, fallback_toast: true, duration_s: 6, style: NotchStyles::default() }
    }
}

pub const MIN_S: u32 = 2;
pub const MAX_S: u32 = 30;

impl NotifyConfig {
    /// Doba držení v ms pro událost (oříznutá na 2–30 s; „Hotovo“ 2/3, nejméně 2 s).
    pub fn hold_ms(&self, kind: super::Kind) -> u64 {
        let s = self.duration_s.clamp(MIN_S, MAX_S) as u64 * 1000;
        if kind == super::Kind::Done { (s * 2 / 3).max(MIN_S as u64 * 1000) } else { s }
    }
    pub fn style_of(&self, kind: super::Kind) -> NotchStyle {
        use super::Kind::*;
        match kind {
            Waiting => self.style.waiting,
            Done => self.style.done,
            Meeting => self.style.meeting,
            Limit => self.style.limit,
            Timer => self.style.timer,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::Kind;
    use super::*;

    #[test]
    fn old_config_without_notify_gets_defaults() {
        // config z kola 25 sekci `notify` nemá → výchozí (Notch, záložní toast, 6 s, výchozí styly)
        let c: crate::config::Config = serde_json::from_str(r#"{"claude":{"toast":true},"timer":{"toast":false}}"#).unwrap();
        assert_eq!(c.notify, NotifyConfig::default());
        assert!(c.claude.toast && !c.timer.toast, "dnešní vypínače beze změny");
        let part: NotifyConfig = serde_json::from_str(r#"{"surface":"toast","style":{"done":"glow"}}"#).unwrap();
        assert_eq!(part.surface, Surface::Toast);
        assert!(part.fallback_toast);
        assert_eq!(part.style.done, NotchStyle::Glow);
        assert_eq!(part.style.waiting, NotchStyle::Unfold);
    }

    #[test]
    fn round_trip_is_camel_case() {
        let s = serde_json::to_string(&NotifyConfig::default()).unwrap();
        assert!(s.contains(r#""fallbackToast":true"#) && s.contains(r#""durationS":6"#) && s.contains(r#""meeting":"drop""#), "{s}");
        let back: NotifyConfig = serde_json::from_str(&s).unwrap();
        assert_eq!(back, NotifyConfig::default());
    }

    #[test]
    fn hold_table() {
        let cases = [(6, Kind::Waiting, 6000), (6, Kind::Done, 4000), (0, Kind::Timer, 2000), (2, Kind::Done, 2000), (99, Kind::Limit, 30_000)];
        for (s, k, want) in cases {
            let c = NotifyConfig { duration_s: s, ..Default::default() };
            assert_eq!(c.hold_ms(k), want, "{s} s {k:?}");
        }
    }

    #[test]
    fn default_styles_differ_per_event() {
        let c = NotifyConfig::default();
        let styles: std::collections::HashSet<NotchStyle> = Kind::ALL.iter().map(|k| c.style_of(*k)).collect();
        assert_eq!(styles.len(), 5, "každá událost má jiný výchozí styl");
        assert!(c.style_of(Kind::Waiting).has_button() && c.style_of(Kind::Meeting).has_button(), "Čeká a Schůzka mají tlačítko");
    }
}
