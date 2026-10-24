// Klávesnice v otevřeném šuplíku — DOM část (`keys.ts` je čistá). Jedno posluchač `keydown` na `window` šuplíku:
// přijde mu jen událost, když okno šuplíku má fokus od uživatele (Ledge fokus nebere), takže nic není globální.
// Zástupci = tlačítka dlaždic (`.proj-hit`), akcí (`.act-tile`) a odkazů (`.link-row`) na viditelné záložce v pořadí
// na obrazovce. Alt+N klikne na N-tého (jeho obvyklý `onClick` — tytéž kontroly a potvrzení jako myší), šipky
// přesunou fokus. Při držení Alt dostane prvních devět zástupců číslo (`data-qn`, CSS `quick.css`); odebere se
// po puštění Alt, ztrátě fokusu nebo spuštění — žádný časovač, žádný render.
import { badges, keyPlan, type Rect } from "./keys.ts";

/** Zástupci k číslování a šipkám — jedno místo pravdy (test `quick.test.mjs` hlídá, že třídy v kartách jsou). */
export const QUICK_SELECTOR = ".deck :is(.proj-hit, .act-tile, .link-row):not(:disabled)";

export type QuickState = { enabled: boolean; page: string; editing: boolean; paletteOpen: boolean };

const visible = (el: HTMLElement) => el.getClientRects().length > 0;
const items = () => Array.from(document.querySelectorAll<HTMLElement>(QUICK_SELECTOR)).filter(visible);
const rectOf = (el: HTMLElement): Rect => {
  const r = el.getBoundingClientRect();
  return { x: r.x, y: r.y, w: r.width, h: r.height };
};
const editable = (t: EventTarget | null) => {
  const el = t as HTMLElement | null;
  return !!el && (["INPUT", "TEXTAREA", "SELECT"].includes(el.tagName) || el.isContentEditable);
};

function clearBadges() {
  document.querySelectorAll("[data-qn]").forEach((el) => el.removeAttribute("data-qn"));
}

function showBadges() {
  const list = items();
  badges(list.length).forEach((n) => list[n - 1].setAttribute("data-qn", String(n)));
}

/** Zapojí klávesy; vrací odpojení. `state()` se čte při každé události (čerstvé signály skořápky). */
export function installQuickKeys(state: () => QuickState): () => void {
  const active = () => {
    const s = state();
    return s.enabled && s.page === "main" && !s.editing && !s.paletteOpen;
  };
  const down = (e: KeyboardEvent) => {
    const s = state();
    if (e.key === "Alt" && !e.repeat) {
      if (active()) showBadges();
      return;
    }
    if (!s.enabled) return;
    const list = items();
    const focused = list.indexOf(document.activeElement as HTMLElement);
    const t = e.target as HTMLElement | null;
    const plan = keyPlan(e, {
      enabled: s.enabled,
      page: s.page,
      editing: s.editing,
      paletteOpen: s.paletteOpen,
      editable: editable(t),
      onBody: t === document.body || t === document.documentElement,
      rects: list.map(rectOf),
      focused,
    });
    if (!plan) return;
    e.preventDefault();
    if (plan.do === "launch") {
      clearBadges();
      list[plan.index]?.click();
    } else {
      list[plan.index]?.focus();
      list[plan.index]?.scrollIntoView({ block: "nearest" });
    }
  };
  const up = (e: KeyboardEvent) => e.key === "Alt" && clearBadges();
  const off = () => clearBadges();
  window.addEventListener("keydown", down);
  window.addEventListener("keyup", up);
  window.addEventListener("blur", off);
  return () => {
    window.removeEventListener("keydown", down);
    window.removeEventListener("keyup", up);
    window.removeEventListener("blur", off);
    clearBadges();
  };
}
