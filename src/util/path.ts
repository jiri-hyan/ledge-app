// Porovnání cest souborů (Windows): bez ohledu na velikost písmen, směr lomítek, mezery kolem a koncové lomítko.
// Jediná implementace pro lístky (`pin/model.ts`), poznámky a zástupce projektů (`projects/model.ts`); shodné s
// Rust `pins::same_note`. Test `path.test.mjs`.

/** Cesta v podobě pro porovnání: zpětná lomítka, bez koncového lomítka, malá písmena. */
export const normPath = (p: string): string => p.trim().replace(/\//g, "\\").replace(/\\+$/, "").toLowerCase();

/** Jsou to tytéž cesty? (`C:/Work/App/` = `c:\work\app`) */
export const samePath = (a: string, b: string): boolean => normPath(a) === normPath(b);
