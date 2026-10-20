// Prohlídka při prvním otevření šuplíku (nový uživatel, nebo po aktualizaci jednou): 3 bubliny u ✎, ⋯ a
// 🔍 / Ctrl+K. Čistá logika (test `tips.test.mjs`): kdy se ukáže, zápis „už viděno“ do `ui.tips`, pozice
// bubliny vůči cíli. Vykreslí `Tour.tsx`. Jen typy, žádné importy za běhu; texty jako páry klíč + čeština.

/** Id prohlídky v `ui.tips` (nová prohlídka po další aktualizaci = nové id). */
export const TOUR_ID = "tour1";

export type Text = readonly [string, string];
/** Krok: cíl (`data-tip` v hlavičce, `menu` = ⋯ první karty), nadpis a text. */
export type Step = { target: "edit" | "menu" | "palette"; title: Text; body: Text };
export const STEPS: readonly Step[] = [
  {
    target: "edit",
    title: ["cards.tour.edit.title", "Uprav si šuplík"],
    body: ["cards.tour.edit.body", "Přesuň karty, tahem za okraj jim změň šířku a výšku, přidej další z katalogu nebo založ vlastní záložku."],
  },
  {
    target: "menu",
    title: ["cards.tour.menu.title", "Každá karta má menu"],
    body: ["cards.tour.menu.body", "Najeď na kartu a klikni na ⋯ — velikost, výška, skrytí a u Projektů i velikost dlaždic."],
  },
  {
    target: "palette",
    title: ["cards.tour.palette.title", "Najdi cokoli"],
    body: ["cards.tour.palette.body", "Ctrl+K nebo 🔍: projekty, akce, poznámky, nastavení i výpočty na jednom místě."],
  },
];

/** Viděl uživatel prohlídku `id`? (`ui.tips` z configu, obranně) */
export function seen(cfg: unknown, id = TOUR_ID): boolean {
  const tips = (cfg as { ui?: { tips?: unknown } } | null | undefined)?.ui?.tips;
  return Array.isArray(tips) && tips.includes(id);
}

/** Ukázat prohlídku? Ne při chybě configu (uložení „viděno“ by neprošlo), ne s průvodcem a ne podruhé. */
export const shouldShow = (cfg: unknown, wizard: boolean, cfgError: string | null): boolean => !!cfg && !cfgError && !wizard && !seen(cfg);

/** Id jednorázového tipu „Podrž kartu a přetáhni ji“ v `ui.tips` (kolo 10). */
export const HOLD_TIP = "hold1";
/** Ukázat tip o podržení karty? Až když uživatel prohlídku už viděl (bubliny se nepletou), ne s průvodcem,
 *  ne při chybě configu (uložení „viděno“ by neprošlo) a ne podruhé. */
export const shouldShowHold = (cfg: unknown, wizard: boolean, cfgError: string | null): boolean =>
  !!cfg && !cfgError && !wizard && seen(cfg, TOUR_ID) && !seen(cfg, HOLD_TIP);
/** Texty tipu (klíč + čeština jako u prohlídky). */
export const HOLD_TEXT: { title: Text; body: Text; ok: Text } = {
  title: ["cards.tip.hold.title", "Karty přesuneš tahem"],
  body: ["cards.tip.hold.body", "Podrž kartu za hlavičku a přetáhni ji — jinam, na jinou záložku nebo dolů na „Skrýt kartu“. Esc tah zruší."],
  ok: ["cards.tip.hold.ok", "Rozumím"],
};

/** Config se značkou „viděno“ v `ui.tips` (vstup zůstane netknutý; jen jedno pole). */
export function withTip<T extends Record<string, any>>(cfg: T, id = TOUR_ID): T {
  const ui = { ...(cfg.ui ?? {}) };
  const tips: string[] = Array.isArray(ui.tips) ? ui.tips.filter((x: unknown) => typeof x === "string") : [];
  ui.tips = tips.includes(id) ? tips : [...tips, id];
  return { ...cfg, ui };
}

export type Box = { left: number; top: number; width: number; height: number };
/** Pozice bubliny šířky `bw` a výšky `bh` u cíle `r` v okně `vw × vh`: pod cílem (šipka nahoru), když se
 *  tam nevejde, nad ním; vodorovně na střed cíle, ale celá v okně (okraj 8 px). */
export function bubblePos(r: Box, bw: number, bh: number, vw: number, vh: number): { left: number; top: number; below: boolean; arrow: number } {
  const M = 8;
  const below = r.top + r.height + 10 + bh <= vh - M || r.top - 10 - bh < M;
  const top = below ? r.top + r.height + 10 : r.top - 10 - bh;
  const cx = r.left + r.width / 2;
  const left = Math.max(M, Math.min(vw - M - bw, cx - bw / 2));
  return { left, top: Math.max(M, top), below, arrow: Math.max(12, Math.min(bw - 12, cx - left)) };
}
