// Režim úprav přímo v šuplíku: rozpracované rozložení (`draft`), záložka, na kterou se v úpravách dívám
// (`editPage` — nová záložka v Rustu ještě není, proto lokálně), katalog, lišta „… · Vrátit“ a cíl tažení
// na záložku. „Hotovo“ uloží JEN pole `layout` (`get_config` → `withLayout` → `save_config`, jako ⋯ na
// kartě); „Zrušit“ / Esc zahodí. Vstup jen výslovným klikem (✎, paleta, ⋯) → `focusDrawer` (klávesy
// Esc a Alt+šipky, šuplík nezavře odjetí myši); konec → `releaseFocus`.
// Zavření šuplíku (blur, skrytí) rozpracované úpravy NEzahodí (`keep.ts`): draft i Zpět žijí v modulu
// (WebView zůstává teplý), po znovuotevření `resumeEdit` vrátí fokus a lišta připomene neuložené změny.
// Přežití restartu / zničení WebView (úsporný režim) NENÍ — sessionStorage by ho nepřenesl.
// Vlna 9: každá změna jde vrátit (`history.ts` — Zpět / Ctrl+Z v liště, i po Obnovit výchozí), Hotovo
// nejdřív ověří rozložení stejně jako Rust (`validate.ts`) a s chybou úpravy nezavře (nic se neztratí).
// `applyLayout` = jedna změna odkudkoli (⋯ na kartě): v úpravách do draftu, jinak rovnou do configu.
import { createSignal } from "solid-js";
import { api } from "../api";
import { t } from "../i18n";
import { say, sayError } from "../ui/toast";
import { clone, normalizeLayout, pageOf, sameLayout, withLayout, type Layout } from "./model";
import { popUndo, pushUndo, resetLayout } from "./history";
import { layoutProblem } from "./validate";
import { escAction, showResume } from "./keep";
import { endSizeEdit, rememberSize, revertSize } from "./drawerSize";

const UNDO_MS = 6000;
const [draft, setDraft] = createSignal<Layout | null>(null);
const [editPage, setEditPage] = createSignal("");
const [catalogOpen, setCatalogOpen] = createSignal(false);
const [saving, setSaving] = createSignal(false);
/** lišta „… · Vrátit“ (bez `restore` = jen nápověda, třeba proč velikost nemá efekt) */
const [undoBar, setUndoBar] = createSignal<{ text: string; restore?: () => void } | null>(null);
/** během tažení karty: záložka pod kurzorem (pustit = přesunout kartu tam) */
const [dropTab, setDropTab] = createSignal<string | null>(null);
/** záložka, jejíž název se má hned začít přepisovat (karta přetažená na „+“ vytvoří „Nová záložka“) */
const [renameAsk, setRenameAsk] = createSignal<string | null>(null);
/** předchozí rozložení v úpravách (Zpět) */
const [undoStack, setUndoStack] = createSignal<Layout[]>([]);
export const canUndo = () => undoStack().length > 0;
/** připomínka po znovuotevření šuplíku s neuloženými úpravami (zmizí po 10 s nebo první další změně) */
const [resumed, setResumed] = createSignal(false);
/** Esc se změnami: lišta se ptá „Zahodit úpravy?“ */
const [askDiscard, setAskDiscard] = createSignal(false);
let before: Layout | null = null;
let resumeTimer: number | undefined;
let undoTimer: number | undefined;

/** Kolikrát uživatel zahájil tah podržením karty (tip „Podrž kartu“ se pak už nepotřebuje ukazovat). */
const [holdDrags, setHoldDrags] = createSignal(0);
export const noteHoldDrag = () => setHoldDrags((n) => n + 1);

export { renameAsk, setRenameAsk, draft, editPage, setEditPage, catalogOpen, setCatalogOpen, saving, undoBar, dropTab, setDropTab, resumed, askDiscard, setAskDiscard, holdDrags };
export const editing = () => draft() !== null;

type Src = { layout: unknown; mode: string; cfgError: string | null };

/** ✎ / paleta / ⋯ → úpravy (s chybou configu ne — uložení by přepsalo uživatelův soubor výchozím). */
export function beginEdit(s: Src, openCatalog = false) {
  if (s.cfgError) {
    say(t("cards.layout.cfgError", "Nastavení obsahuje chybu — šuplík teď upravíš jen v Nastavení."), true);
    return;
  }
  if (editing()) {
    if (openCatalog) setCatalogOpen(true);
    return;
  }
  const l = normalizeLayout(s.layout);
  before = l;
  rememberSize();
  setDraft(clone(l));
  setEditPage(pageOf(l, s.mode).id);
  setCatalogOpen(openCatalog);
  setUndoBar(null);
  setUndoStack([]);
  api.focusDrawer();
}

/** Změna rozpracovaného rozložení (čistou funkcí z `model.ts`). */
export function change(f: (l: Layout) => Layout) {
  const d = draft();
  if (!d) return;
  const next = f(d);
  setUndoStack((st) => pushUndo(st, d, next));
  setDraft(next);
  setResumed(false);
}

/** Zpět o jednu změnu (tlačítko v liště, Ctrl+Z). */
export function undoLast() {
  const r = popUndo(undoStack());
  if (!r.layout || !draft()) return;
  setUndoStack(r.stack);
  setDraft(r.layout);
  clearTimeout(undoTimer);
  setUndoBar(null);
  // záložka z úprav mohla zmizet (Zpět po přidání záložky)
  if (!r.layout.pages.some((p) => p.id === editPage())) setEditPage(r.layout.pages[0].id);
}

/** Obnovit výchozí rozložení (po potvrzení v liště); jde vrátit Zpět. */
export function resetToDefault() {
  change((l) => resetLayout(l));
  const d = draft();
  if (d && !d.pages.some((p) => p.id === editPage())) setEditPage(d.pages[0].id);
}

/** Lišta „… · Vrátit“ na pár sekund (odebraná karta, smazaná záložka, přesun na jinou záložku). */
export function offerUndo(text: string, restore?: () => void) {
  clearTimeout(undoTimer);
  setUndoBar({ text, restore });
  undoTimer = window.setTimeout(() => setUndoBar(null), UNDO_MS);
}
export function runUndo() {
  const u = undoBar();
  clearTimeout(undoTimer);
  setUndoBar(null);
  u?.restore?.();
}

function end(release: boolean) {
  setDraft(null);
  setResumed(false);
  setAskDiscard(false);
  clearTimeout(resumeTimer);
  setCatalogOpen(false);
  setDropTab(null);
  clearTimeout(undoTimer);
  setUndoBar(null);
  setUndoStack([]);
  before = null;
  endSizeEdit();
  if (release) api.releaseFocus();
}

/** Šuplík se znovu otevřel s rozpracovanými úpravami: fokus zpět (odjetí myši ho nezavře) + připomínka. */
export function resumeEdit() {
  if (!editing()) return;
  api.focusDrawer();
  if (!showResume("reopen", before, draft())) return;
  setResumed(true);
  clearTimeout(resumeTimer);
  resumeTimer = window.setTimeout(() => setResumed(false), 10000);
}

/** Esc mimo tah: beze změn zrušit, se změnami se nejdřív zeptat (nic se nezahodí omylem). */
export function escEdit() {
  if (!editing()) return;
  if (escAction(before, draft()) === "cancel") return void end(true);
  setAskDiscard((a) => !a);
}

/** Zrušit / Esc. `release` = false při zavření šuplíku (fokus vrací Rust sám). */
// Zrušit vrátí i velikost šuplíku taženou v úpravách (`drawerSize.ts`, kolo 23)
export const cancelEdit = (release = true) => editing() && (void revertSize(), end(release));

/** Hotovo: uloží `layout` a přepne Rust na záložku, na kterou se uživatel díval. */
export async function finishEdit() {
  const d = draft();
  if (!d || saving()) return;
  const page = editPage();
  if (before && sameLayout(before, d)) {
    end(true);
    void api.setMode(page);
    return;
  }
  const bad = layoutProblem(d);
  if (bad) {
    say(t("cards.layout.invalid", "Rozložení teď nejde uložit ({field}) — oprav to nebo dej Zpět. Úpravy zůstávají otevřené.", { field: bad }), true);
    return;
  }
  setSaving(true);
  try {
    const cfg = await api.getConfig();
    if (!cfg) return;
    await api.saveConfig(withLayout(cfg, d));
    // nejdřív Rust na záložku z úprav (nová záložka už v configu je), pak konec draftu
    await api.setMode(page);
    end(true);
  } catch (e) {
    sayError(e);
  } finally {
    setSaving(false);
  }
}

/** Jedna změna rozložení mimo režim úprav (⋯ na kartě): uloží jen pole `layout`. */
export async function saveLayout(f: (l: Layout) => Layout, cfgError: string | null): Promise<boolean> {
  if (cfgError) {
    say(t("cards.hide.cfgError", "Nastavení obsahuje chybu — karty teď skryješ jen v Nastavení."), true);
    return false;
  }
  try {
    const cfg = await api.getConfig();
    if (!cfg) return false;
    await api.saveConfig(withLayout(cfg, f(normalizeLayout(cfg.layout))));
    return true;
  } catch (e) {
    sayError(e);
    return false;
  }
}

/** Jedna změna rozložení odkudkoli (⋯ na kartě): v režimu úprav do draftu (s Zpět), jinak hned do
 *  configu — dřív ⋯ v úpravách zapsal config a Hotovo ho draftem přepsal (změna se ztratila). */
export async function applyLayout(f: (l: Layout) => Layout, cfgError: string | null): Promise<boolean> {
  if (editing()) {
    change(f);
    return true;
  }
  return saveLayout(f, cfgError);
}

/** „Upravit přímo v šuplíku“ z Nastavení: skořápka (`App`) přepne na hlavní stránku a spustí úpravy. */
const [editAsk, setEditAsk] = createSignal(0);
export { editAsk };
export const askEdit = () => setEditAsk((n) => n + 1);
