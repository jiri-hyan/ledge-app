// Líně načítané pohledy mimo první obrazovku šuplíku (paměť WebView2 + velikost hlavního chunku): Nastavení
// (s editorem rozložení a přehledem vzhledů), průvodce, paleta, stránka Poznámky, panel „Nová verze“, katalog karet
// a panel „Vzhled karty“. Každý je vlastní chunk i se svým CSS — Vite počká na styly dřív, než `import()` dokončí,
// takže se nic nevykreslí bez nich. Fallback = nic (žádné blikání). `preloadViews` je přednačte v klidu po prvním
// zobrazení šuplíku (`App.tsx`), aby první klik na Nastavení / Ctrl+K nebyl pomalý.
import { lazy, Suspense, type Component } from "solid-js";
import { loadDict } from "./i18n";

/** Lazy komponenta bez fallbacku: dokud se chunk nenačte, nekreslí se nic. */
function view<P extends object>(load: () => Promise<{ default: Component<P> }>): Component<P> {
  const C = lazy(load);
  return (p) => (
    <Suspense>
      <C {...p} />
    </Suspense>
  );
}

export const Settings = view(() => import("./Settings"));
export const Wizard = view(() => import("./onboarding/Wizard"));
export const NotesPage = view(() => import("./notes/NotesPage"));
export const UpdatePanel = view(() => import("./cards/UpdatePanel"));
export const Catalog = view(() => import("./layout/Catalog"));
export const CardLookPanel = view(() => import("./layout/CardLook"));
// Paleta hledá i anglicky při české UI (`tIn("en", …)` u Nastavení Windows, jednotek, emoji) → před ní počká na slovník.
export const Palette = view(async () => {
  await loadDict("en").catch(() => {});
  return import("./palette/Palette");
});

/** Chunky k přednačtení (paleta bez slovníku — ten se stáhne až při jejím otevření). */
const PRELOAD: (() => Promise<unknown>)[] = [
  () => import("./Settings"),
  () => import("./palette/Palette"),
  () => import("./notes/NotesPage"),
  () => import("./cards/UpdatePanel"),
  () => import("./layout/Catalog"),
  () => import("./layout/CardLook"),
  () => import("./onboarding/Wizard"),
];

let scheduled = false;
/** Jednou za běh: po prvním zobrazení šuplíku načte lazy chunky, každý v samostatném volném okamžiku. */
export function preloadViews() {
  if (scheduled) return;
  scheduled = true;
  const idle = (fn: () => void) => (typeof requestIdleCallback === "function" ? requestIdleCallback(fn, { timeout: 5000 }) : setTimeout(fn, 1500));
  const next = (i: number) => {
    if (i >= PRELOAD.length) return;
    idle(() => {
      PRELOAD[i]().catch(() => {}).finally(() => next(i + 1));
    });
  };
  next(0);
}
