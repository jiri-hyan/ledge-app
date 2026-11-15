// Zrcadlo `ext::ExtConfig` (Rust, config `extensions`): ID povolených rozšíření (`ledge.exe --ext <id>`).
// Výchozí `[]` = nic neběží; neznámé ID Ledge ignoruje. Nastavení rozšíření samotných je v `ext/<id>.json` (jen PC).
export type ExtensionsConfig = string[];

/** Výchozí stav (= `ExtConfig::default` v Rustu). */
export const EXTENSIONS_DEFAULT: ExtensionsConfig = [];
