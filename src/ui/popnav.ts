// Čistá logika menu (`Popover.tsx` `PopMenu`): kam skočí fokus po šipkách / Home / End a jak se menu zavře
// po klávese. Bez DOM, tabulkový test `popover.test.mjs`.

/** Klávesy, které menu obsluhuje samo (šipky, Home, End). */
export const NAV_KEYS = ["ArrowDown", "ArrowUp", "Home", "End"] as const;

/**
 * Index položky, na kterou se po klávese posune fokus. `cur` = dnešní položka (-1 = fokus mimo položky),
 * `count` = kolik je položek. ↓ z poslední skočí na první, ↑ z první na poslední (kruhově); ↑ bez fokusu na
 * poslední, ↓ na první. Prázdné menu nebo jiná klávesa = null (nic se nedělá).
 */
export function navTarget(key: string, cur: number, count: number): number | null {
  if (count <= 0) return null;
  switch (key) {
    case "ArrowDown":
      return cur < 0 ? 0 : (cur + 1) % count;
    case "ArrowUp":
      return cur <= 0 ? count - 1 : cur - 1;
    case "Home":
      return 0;
    case "End":
      return count - 1;
    default:
      return null;
  }
}

/** Co s klávesou, když je menu otevřené: zavřít (Esc, Tab) / posun fokusu / nic. */
export type MenuKey = "close" | "nav" | null;

/**
 * `inside` = fokus je v menu; `free` = fokus nikde (tělo stránky) nebo na spouštěči; `typing` = fokus v poli
 * pro psaní (pole uvnitř menu, jezdec: šipky patří jemu). Esc a Tab zavírají vždy; šipky jen když patří menu.
 */
export function menuKey(key: string, f: { inside: boolean; free: boolean; typing: boolean }): MenuKey {
  if (key === "Escape" || key === "Tab") return "close";
  if ((NAV_KEYS as readonly string[]).includes(key) && !f.typing && (f.inside || f.free)) return "nav";
  return null;
}
