// Kolečko myši nad záložkami šuplíku přepíná záložky (kolo 20, `smart.tabWheel`). ČISTÉ funkce — DOM a událost
// řeší `Tabs.tsx`. Kolečko se sčítá (jemné posuvy touchpadu), po přepnutí chvíli nic (jedno otočení = jedna záložka,
// ne tři), po pauze se součet nuluje; na kraji se nezalamuje. Hlavička šuplíku se neposouvá, takže kolečko nad
// záložkami dřív nedělalo nic — scroll obsahu (kolečko nad kartami) tím dotčený není.

/** Kolik px kolečka stačí na jedno přepnutí (jedno zářez myši ≈ 100, jemný touchpad se sčítá). */
export const STEP_PX = 40;
/** Po pauze delší než tohle se nasčítané zapomene. */
export const IDLE_MS = 250;
/** Mezi dvěma přepnutími nejméně tohle. */
export const COOLDOWN_MS = 220;

export type WheelState = { acc: number; at: number; lastSwitch: number };
export const WHEEL_INIT: WheelState = { acc: 0, at: 0, lastSwitch: -1e9 };

/** Posun v px z `deltaY` a `deltaMode` (0 px, 1 řádky, 2 stránky). */
export const pxOf = (dy: number, mode: number) => (mode === 1 ? dy * 16 : mode === 2 ? dy * 400 : dy);

/** Jeden krok kolečka. `dir` = -1 předchozí záložka, 1 další, 0 nic; `state` je nový součet. */
export function wheelStep(st: WheelState, dy: number, now: number): { state: WheelState; dir: -1 | 0 | 1 } {
  let acc = now - st.at > IDLE_MS ? 0 : st.acc;
  if (acc !== 0 && Math.sign(acc) !== Math.sign(dy)) acc = 0; // obrácený směr = nový začátek
  acc += dy;
  if (Math.abs(acc) >= STEP_PX && now - st.lastSwitch >= COOLDOWN_MS) {
    return { state: { acc: 0, at: now, lastSwitch: now }, dir: acc > 0 ? 1 : -1 };
  }
  return { state: { acc: Math.abs(acc) >= STEP_PX ? Math.sign(acc) * STEP_PX : acc, at: now, lastSwitch: st.lastSwitch }, dir: 0 };
}

/** Index záložky po otočení; na kraji zůstane (`null` = není kam). */
export function nextTab(cur: number, count: number, dir: -1 | 1): number | null {
  const to = cur + dir;
  return cur < 0 || to < 0 || to >= count ? null : to;
}

/** Smí se vůbec přepínat? Volba zapnutá, mimo úpravy rozložení, aspoň dvě záložky, nic s Ctrl (to je zoom). */
export function wheelAllowed(o: { enabled: boolean; editing: boolean; tabs: number; ctrl: boolean }): boolean {
  return o.enabled && !o.editing && o.tabs > 1 && !o.ctrl;
}
