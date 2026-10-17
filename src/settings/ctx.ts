// Stav Nastavení: rozpracovaný config (store na úrovni modulu — přežije zavření stránky i šuplíku,
// rozdělaná práce se neztratí), uložená verze `base` (pro „Neuložené změny“ a Zahodit), poslední
// chyba validace z Rustu, zvýrazněný řádek z hledání a lišta „Vrátit“. Pole čtou/mění config přes
// `useSettings()` cestou (`"drawer.trigger"`, `"projects.2.run"`).
import { createContext, createSignal, useContext } from "solid-js";
import { createStore, reconcile, unwrap } from "solid-js/store";
import { setLang } from "../i18n";
import type { FieldError, Snapshot } from "../types";
import { getPath, pathKeys } from "./model";

const [cfg, setCfgRaw] = createStore<any>({});
/** JSON uložené verze (tak, jak ji vrací jádro) */
const [base, setBaseSig] = createSignal("");
const [rev, setRev] = createSignal(0);
const [fieldError, setFieldError] = createSignal<FieldError | null>(null);
const [hit, setHit] = createSignal<string | null>(null);
const [active, setActive] = createSignal("projekty");
/** rozpracovaná verze se liší od uložené (počítá shell po každé změně, s debounce) */
const [dirty, setDirty] = createSignal(false);
const [undoMsg, setUndoMsg] = createSignal<{ text: string; restore: () => void } | null>(null);
let undoTimer: number | undefined;

export { cfg, base, rev, dirty, setDirty, fieldError, setFieldError, hit, setHit, active, setActive, undoMsg };

/** Každá změna konfigu jde tudy — `rev` spouští ověření a výpočet „neuloženo“. */
export const touch = () => setRev((r) => r + 1);

/** Setter pro panely z registru (`PanelProps.setCfg`): stejné API jako store, navíc označí změnu. */
export const setCfg: any = (...args: any[]) => {
  (setCfgRaw as any)(...args);
  touch();
};

/** Načtená / právě uložená verze z jádra → draft i základ. Jazyk rozhraní jde podle ULOŽENÉ verze —
 *  přepínač v Nastavení (`ui.lang`) tak platí hned po Uložit, bez restartu. */
export function loadBase(c: any) {
  const l = c?.ui?.lang;
  if (l === "cs" || l === "en") setLang(l);
  setCfgRaw(reconcile(structuredClone(c)));
  setBaseSig(JSON.stringify(c));
  touch();
}

export const isLoaded = () => base() !== "";
export const snapshotDraft = () => structuredClone(unwrap(cfg));
export const discard = () => base() && loadBase(JSON.parse(base()));

/** Lišta „Vrátit“ (smazání, obnova výchozích…) na 8 s. */
export function offerUndo(text: string, restore: () => void) {
  clearTimeout(undoTimer);
  setUndoMsg({ text, restore });
  undoTimer = window.setTimeout(() => setUndoMsg(null), 8000);
}
export function runUndo() {
  const u = undoMsg();
  clearTimeout(undoTimer);
  setUndoMsg(null);
  u?.restore();
}

export type Ctx = {
  cfg: any;
  s: Snapshot;
  get: (path: string) => any;
  set: (path: string, v: any) => void;
  update: (path: string, fn: (v: any) => any) => void;
  /** chyba validace pro pole; `nested` = i chyby vnořených polí (`personal.weather` ← `.lat`) */
  error: (path: string, nested?: boolean) => string | null;
  save: () => Promise<void>;
};

export function makeCtx(s: Snapshot, save: () => Promise<void>): Ctx {
  return {
    cfg,
    s,
    get: (path) => getPath(cfg, path),
    // objekty vždy jako čistá kopie (ne proxy ze stejného store — přesuny v seznamech)
    set: (path, v) => setCfg(...pathKeys(path), v !== null && typeof v === "object" ? structuredClone(unwrap(v)) : v),
    update: (path, fn) => setCfg(...pathKeys(path), (old: any) => fn(structuredClone(unwrap(old)))),
    error: (path, nested = false) => {
      const e = fieldError();
      if (!e?.field) return null;
      return e.field === path || (nested && e.field.startsWith(path + ".")) ? e.message : null;
    },
    save,
  };
}

export const SettingsCtx = createContext<Ctx>();
export const useSettings = () => useContext(SettingsCtx)!;
