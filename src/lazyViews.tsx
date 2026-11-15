// Líně načítané pohledy mimo první obrazovku šuplíku (paměť WebView2 + velikost hlavního chunku): Nastavení
// (s editorem rozložení a přehledem vzhledů), průvodce, paleta, stránka Poznámky, editor poznámky, panel „Nová verze“, katalog karet
// a panel „Vzhled karty“. Každý je vlastní chunk i se svým CSS — Vite počká na styly dřív, než `import()` dokončí,
// takže se nic nevykreslí bez nich. Fallback = nic (žádné blikání). `preloadViews` je přednačte v klidu po prvním
// zobrazení šuplíku (`App.tsx`), aby první klik na Ctrl+K nebyl pomalý. Studio se nepřednačítá: Nastavení je samostatné okno, `Settings` je jen záloha (`setStudioFallback`).
import { lazy, Suspense, type Component } from "solid-js";
import { loadDict } from "./i18n";

/** Lazy komponenta bez fallbacku: dokud se chunk nenačte, nekreslí se nic. */
export function view<P extends object>(load: () => Promise<{ default: Component<P> }>): Component<P> {
  const C = lazy(load);
  return (p) => (
    <Suspense>
      <C {...p} />
    </Suspense>
  );
}

export const Settings = view(() => import("./studio/Studio"));
export const Wizard = view(() => import("./onboarding/Wizard"));
export const NotesPage = view(() => import("./notes/NotesPage"));
export const NoteEditor = view(() => import("./notes/NoteEditor"));
export const UpdatePanel = view(() => import("./cards/UpdatePanel"));
export const Catalog = view(() => import("./layout/Catalog"));
export const CardLookPanel = view(() => import("./layout/CardLook"));
export const HeadLookPanel = view(() => import("./layout/HeadLook"));
export const DropNew = view(() => import("./actions/DropNew"));
// Zřídka používané nebo těžké karty a nástroje úprav (`MainPage.tsx`): chunk se načte, až karta / úpravy poprvé přijdou na řadu
// (`preloadCards` je přednačte jen pro karty v rozložení, ať první otevření nebliká). Střední panel z docku (`hover/Mid.tsx`) kreslí
// týmiž `card()` — lazy karta je tam Suspense jako tady. Čisté pomocné exporty (`openCalendarSettings`…) zůstávají statické v modulech karet.
export const WeatherCard = view(() => import("./cards/WeatherCard"));
export const CalendarCard = view(() => import("./calendar/CalendarCard"));
export const MailCard = view(() => import("./accounts/MailCard"));
export const GithubCard = view(() => import("./integrations/GithubCard"));
export const SystemCard = view(() => import("./integrations/SystemCard"));
export const UsageCard = view(() => import("./board/UsageCard"));
export const TogglesCard = view(() => import("./board/TogglesCard"));
export const WinQuickCard = view(() => import("./winquick/WinQuickCard"));
export const WeekCard = view(() => import("./board/WeekCard"));
export const BigClockCard = view(() => import("./board/BigClockCard"));
export const ClaudeDesign = view(() => import("./cards/ClaudeDesign"));
export const EditBar = view(() => import("./layout/EditBar"));
export const ResizeHandles = view(() => import("./layout/ResizeHandles"));
export const Tour = view(() => import("./layout/Tour"));
export const HoldTip = view(() => import("./layout/HoldTip"));

/** Načtení chunku lazy karty podle id (jen ty, které jsou lazy). */
const CARD_CHUNKS: Record<string, () => Promise<unknown>> = {
  weather: () => import("./cards/WeatherCard"),
  calendar: () => import("./calendar/CalendarCard"),
  mail: () => import("./accounts/MailCard"),
  github: () => import("./integrations/GithubCard"),
  system: () => import("./integrations/SystemCard"),
  usage: () => import("./board/UsageCard"),
  toggles: () => import("./board/TogglesCard"),
  winquick: () => import("./winquick/WinQuickCard"),
  week: () => import("./board/WeekCard"),
  bigclock: () => import("./board/BigClockCard"),
};
/** Lazy karty, které rozložení opravdu obsahuje (čistá funkce; test `lazyViews.test.mjs`). */
export function lazyCardIds(ids: readonly string[]): string[] {
  return [...new Set(ids)].filter((id) => id in CARD_CHUNKS);
}
/** Přednačte chunky karet z rozložení (volá `App.tsx` po změně rozložení, v klidu) — první otevření pak nebliká. */
export function preloadCards(ids: readonly string[]) {
  for (const id of lazyCardIds(ids)) void CARD_CHUNKS[id]().catch(() => {});
}
// Paleta hledá i anglicky při české UI (`tIn("en", …)` u Nastavení Windows, jednotek, emoji) → před ní počká na slovník.
export const Palette = view(async () => {
  await loadDict("en").catch(() => {});
  return import("./palette/Palette");
});

/** Chunky k přednačtení v klidu po prvním otevření (paleta bez slovníku — ten se stáhne až při jejím otevření).
 *  Jen to, co uživatel potkává pořád; průvodce, katalog, vzhled karty a panel „Nová verze“ se táhnou až při potřebě. */
const PRELOAD: (() => Promise<unknown>)[] = [
  () => import("./palette/Palette"),
  () => import("./notes/NotesPage"),
  () => import("./notes/NoteEditor"),
];
/** Úpravy šuplíku (✎): katalog karet a panel vzhledu karty — načte se při vstupu do úprav, ne dřív. */
const EDIT_VIEWS: (() => Promise<unknown>)[] = [() => import("./layout/Catalog"), () => import("./layout/CardLook")];

/** Přednačtení pohledů úprav (volá `App.tsx` při `editing()`); idempotentní, chybu spolkne (načte se při použití). */
export function preloadEditViews() {
  EDIT_VIEWS.forEach((load) => void load().catch(() => {}));
}
/** Panel „Nová verze“ — až když je co ukázat (chip v hlavičce). */
export function preloadUpdatePanel() {
  void import("./cards/UpdatePanel").catch(() => {});
}

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
