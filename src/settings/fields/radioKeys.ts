// Klávesnice pro skupiny `role="radio"` (WAI-ARIA radiogroup): jedna zastávka na Tab (roving tabindex), uvnitř šipky
// a Home/End kruhově. Čistá část (`radioTarget`, `tabStop`) je bez DOM a má tabulkový test (`radioKeys.test.mjs`);
// `radioKeyDown` je tenký DOM obal, který se věší na kontejner `radiogroup` (event delegation, jeden handler).
// Cíl pohybu počítá `navTarget` z `ui/popnav.ts` (sdílená logika menu), tady jen přidává ←/→.
import { navTarget } from "../../ui/popnav.ts";

/** Klávesy, které skupina obsluhuje. */
export const RADIO_KEYS = ["ArrowRight", "ArrowDown", "ArrowLeft", "ArrowUp", "Home", "End"] as const;

/** Index radia, na které se posune fokus (`null` = klávesa se skupiny netýká). →/↓ dál, ←/↑ zpět, kruhově. */
export function radioTarget(key: string, cur: number, count: number): number | null {
  const k = key === "ArrowRight" ? "ArrowDown" : key === "ArrowLeft" ? "ArrowUp" : key;
  return navTarget(k, cur, count);
}

/** `tabIndex` radia `i`: zastávku má vybrané (`checked`), bez výběru první — skupina je vždy dosažitelná Tabem. */
export function tabStop(i: number, checked: number): 0 | -1 {
  return (checked >= 0 ? i === checked : i === 0) ? 0 : -1;
}

/**
 * Handler `onKeyDown` pro kontejner `role="radiogroup"`. Posune fokus na sousední `[role=radio]` (bez `disabled`);
 * `select` (výchozí ano) ho zároveň vybere kliknutím — vypni u drahých voleb (náhled motivu), kde se vybírá
 * Enterem / mezerníkem. Vrací, zda klávesu obsloužil.
 */
export function radioKeyDown(e: KeyboardEvent & { currentTarget: HTMLElement }, select = true): boolean {
  if (!(RADIO_KEYS as readonly string[]).includes(e.key) || e.altKey || e.ctrlKey || e.metaKey) return false;
  const from = (e.target as HTMLElement | null)?.closest<HTMLElement>('[role="radio"]');
  if (!from) return false;
  const items = [...e.currentTarget.querySelectorAll<HTMLElement>('[role="radio"]')].filter((x) => !(x as HTMLButtonElement).disabled);
  const to = radioTarget(e.key, items.indexOf(from), items.length);
  if (to === null) return false;
  e.preventDefault();
  items[to].focus();
  if (select) items[to].click();
  return true;
}
