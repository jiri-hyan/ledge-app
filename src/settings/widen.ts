// Šířka, o kterou si stránka šuplíku řekne (`wantWidth` → `report_size`); Rust ji ořízne na min(žádost, 90 % monitoru)
// (`drawerfit.rs`). Šířku potřebuje už jen záložní Nastavení v šuplíku (`setStudioFallback`); běžně je Nastavení
// samostatné okno Studio.

/** Šířka Nastavení (kolo 21 větší: postranní sekce + stránka + náhled dole vedle sebe). */
export const SETTINGS_WIDE = 1280;

/** Šířka, o kterou si stránka řekne (0 = běžný šuplík podle configu). */
export const wantWidth = (page: string) => (page === "settings" ? SETTINGS_WIDE : 0);
