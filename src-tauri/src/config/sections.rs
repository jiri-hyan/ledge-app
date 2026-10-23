//! `drawer.sections` — plochý seznam viditelnosti karet, odvozený z `layout`.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Plochý seznam viditelnosti (`drawer.sections`): každé pole je `bool`. **Odvozené z `layout`**
/// (`layout::derive` při načtení, uložení i aplikaci) — čtou ho moduly, které podle karty zapínají
/// stahování/odběry, a starší Ledge; ruční změna se přepíše. Starý config bez `layout` se z něj převede.
/// Nová karta / část = řádek v `sections!` níže + `layout::CARDS` / `PARTS` (+ zrcadla v `src/cards/sections.ts`
/// `SECTION_KEYS` a `src/layout/model.ts`).
/// Výchozí hodnota = `layout::section_in_default`, tj. podle výchozího rozložení (`layout::clean_default`):
/// karta, která v něm není (katalog ji jen nabídne), je `false`.
macro_rules! sections {
    ($($(#[$doc:meta])* $field:ident,)*) => {
        #[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
        #[serde(default, rename_all = "camelCase")]
        pub struct Sections {
            $($(#[$doc])* pub $field: bool,)*
        }

        impl Default for Sections {
            fn default() -> Self {
                // odvozené pole musí sedět s výchozím `layout` (jinak by se `Config::default()` lišil od
                // načteného): karta mimo čisté výchozí rozložení (`layout::clean_default`) je vypnutá
                Self { $($field: crate::layout::section_in_default(stringify!($field)),)* }
            }
        }

        #[cfg(test)]
        impl Sections {
            /// Názvy polí v Rustu (snake_case) — jen pro testy.
            pub const FIELDS: &'static [&'static str] = &[$(stringify!($field)),*];
        }
    };
}

sections! {
    /// Práce: dlaždice projektů nahoře.
    projects,
    /// Práce: karta „Běží“ (jen když Ledge něco spustil).
    running,
    /// Práce: karta Claude (session, schvalování). Vypnutá = čekající žádosti ukáže „Claude čeká na tebe“.
    claude,
    /// Karta „Poznámky“ (poslední poznámky + připomínky na dnes), Práce i Osobní.
    notes,
    /// Práce: karta „Repa“ (neuložená / nepushnutá práce, pozadu za remote).
    repos,
    /// Karta „Rychlé akce“, Práce i Osobní.
    actions,
    /// Karta „Přihrádka“ (soubory přetažené na proužek), Práce i Osobní. Přihrádku samu vypíná `shelf.enabled`.
    shelf,
    /// Osobní: karta „Čas“.
    clock,
    /// Osobní: karta „Časovač“ (odpočet a stopky). Skrytá karta časovač nevypíná (paleta, proužek).
    timer,
    /// Osobní: karta „Počasí“. Skrytá = nic se nestahuje.
    weather,
    /// Osobní: karta „Kalendář“ (odkazy ICS z `personal.calendars`). Skrytá = nic se nestahuje.
    calendar,
    /// Osobní: karta „Odkazy“.
    links,
    /// Karta Claude: řádek limitů 5h / týden.
    claude_limits,
    /// Karta Claude: sekce „Útrata (odhad)“. Skrytá = transcripty se neindexují (jako `claude.costs: false`).
    claude_costs,
    /// Karta Claude: panel Git u session.
    claude_git,
    /// Karta Claude: řádek subagentů u session.
    claude_subagents,
    /// Karta „Právě hraje“ (`layout` id `media`): skladba z médií Windows — název, interpret, ovládání (`media.rs`).
    now_playing,
    /// Hlasitost výchozího výstupu (posuvník + ztlumit; `volume.rs`), část karty Právě hraje (`media`).
    sound,
    /// Oblíbené playlisty a stanice (`media.favorites`) — část karty Právě hraje (`media`); jen otevírá odkazy, nic nespouští na pozadí.
    favorites,
    /// Osobní: stav baterie v kartě Čas — jen na stroji s baterií (`battery.rs`).
    battery,
    /// Karta „GitHub“ (PR k revizi, vlastní PR s CI, notifikace; `github/`). Výchozí na žádné záložce; skrytá = nic se nestahuje.
    github,
    /// Karta „Systém“ (CPU, paměť, disky, síť; `sysmon/`). Výchozí na žádné záložce; skrytá = nic se nevzorkuje.
    system,
    /// Karta „Pošta“ (účet Microsoft: nepřečtené a 5 nejnovějších předmětů, `accounts/mail.rs`). Výchozí na žádné záložce (`layout::OPT_IN`).
    mail,
}
