// Režim úprav přímo v šuplíku: rozpracované rozložení (`draft`), záložka, na kterou se v úpravách dívám
// (`editPage` — nová záložka v Rustu ještě není, proto lokálně), katalog, lišta „… · Vrátit“ a cíl tažení
// na záložku. „Hotovo“ uloží JEN pole `layout` (`get_config` → `withLayout` → `save_config`, jako ⋯ na
// kartě); „Zrušit“ / Esc zahodí. Vstup jen výslovným klikem (✎, paleta, ⋯) → `focusDrawer` (klávesy
// Esc a Alt+šipky, šuplík nezavře odjetí myši); konec → `releaseFocus`.
import { createSignal } from "solid-js";
import { api } from "../api";
import { t } from "../i18n";
import { say, sayError } from "../ui/toast";
import { clone, normalizeLayout, pageOf, sameLayout, withLayout, type Layout } from "./model";

const UNDO_MS = 6000;
const [draft, setDraft] = createSignal<Layout | null>(null);
const [editPage, setEditPage] = createSignal("");
const [catalogOpen, setCatalogOpen] = createSignal(false);
const [saving, setSaving] = createSignal(false);
const [undoBar, setUndoBar] = createSignal<{ text: string; restore: () => void } | null>(null);
/** během tažení karty: záložka pod kurzorem (pustit = přesunout kartu tam) */
const [dropTab, setDropTab] = createSignal<string | null>(null);
let before: Layout | null = null;
let undoTimer: number | undefined;

export { draft, editPage, setEditPage, catalogOpen, setCatalogOpen, saving, undoBar, dropTab, setDropTab };
export const editing = () => draft() !== null;

type Src = { layout: unknown; mode: string; cfgError: string | null };

/** ✎ / paleta / ⋯ → úpravy (s chybou configu ne — uložení by přepsalo uživatelův soubor výchozím). */
export function beginEdit(s: Src, openCatalog = false) {
  if (s.cfgError) {
    say(t("cards.layout.cfgError", "Config má chybu — šuplík teď upravíš jen v Nastavení."), true);
    return;
  }
  if (editing()) {
    if (openCatalog) setCatalogOpen(true);
    return;
  }
  const l = normalizeLayout(s.layout);
  before = l;
  setDraft(clone(l));
  setEditPage(pageOf(l, s.mode).id);
  setCatalogOpen(openCatalog);
  setUndoBar(null);
  api.focusDrawer();
}

/** Změna rozpracovaného rozložení (čistou funkcí z `model.ts`). */
export function change(f: (l: Layout) => Layout) {
  const d = draft();
  if (d) setDraft(f(d));
}

/** Lišta „… · Vrátit“ na pár sekund (odebraná karta, smazaná záložka, přesun na jinou záložku). */
export function offerUndo(text: string, restore: () => void) {
  clearTimeout(undoTimer);
  setUndoBar({ text, restore });
  undoTimer = window.setTimeout(() => setUndoBar(null), UNDO_MS);
}
export function runUndo() {
  const u = undoBar();
  clearTimeout(undoTimer);
  setUndoBar(null);
  u?.restore();
}

function end(release: boolean) {
  setDraft(null);
  setCatalogOpen(false);
  setDropTab(null);
  clearTimeout(undoTimer);
  setUndoBar(null);
  before = null;
  if (release) api.releaseFocus();
}

/** Zrušit / Esc. `release` = false při zavření šuplíku (fokus vrací Rust sám). */
export const cancelEdit = (release = true) => editing() && end(release);

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
    say(t("cards.hide.cfgError", "Config má chybu — karty teď skryješ jen v Nastavení."), true);
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

/** „Upravit přímo v šuplíku“ z Nastavení: skořápka (`App`) přepne na hlavní stránku a spustí úpravy. */
const [editAsk, setEditAsk] = createSignal(0);
export { editAsk };
export const askEdit = () => setEditAsk((n) => n + 1);
