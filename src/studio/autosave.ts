// Ledge Studio — „Platí hned“ (kolo 24): každá změna draftu (`settings/ctx.ts` `rev`) se po `SAVE_DELAY_MS`
// ověří v Rustu (`check_config`) a — je-li platná a nepíše se zrovna do textového pole — zapíše
// (`save_config`); skutečný Ledge se změní hned. Textová pole se zapíšou při Enter / opuštění (`typing`).
// Zápis jde do časové osy relace (`history.ts`: slučování, Vrátit, vše do stavu při otevření). Config
// změněný jinde (Claude přes MCP): bez rozpracované změny převzít tiše, jinak sloučit, a teprve když to nejde,
// hláška konfliktu (`mcp/merge.ts` `external`). Žádný časovač v klidu — jen jednorázový po změně.
import { createEffect, createSignal, on, onCleanup } from "solid-js";
import { api, on as onEvent } from "../api";
import { say, sayError } from "../ui/toast";
import { t } from "../i18n";
import {
  adoptBase, base, fieldError, isLoaded, loadBase, replaceDraft, rev, runAfterSave, setDirty, setFieldError, snapshotDraft,
} from "../settings/ctx";
import { canon, cleanDraft } from "../settings/model";
import { external } from "../mcp/merge";
import { SAVE_DELAY_MS, canSave, emptyHistory, isTextEntry, record, undoEntry, undoLast, type History } from "./history";

export type SaveState = "idle" | "saving" | "saved" | "invalid" | "conflict";

const [state, setState] = createSignal<SaveState>("idle");
const [history, setHistory] = createSignal<History>(emptyHistory());
const [conflict, setConflict] = createSignal(false);
/** píše se do textového pole — zápis počká na Enter / opuštění */
let typing = false;
let saving = false;
/** příští zápis je Vrátit — do časové osy nejde (položka už z ní zmizela) */
let quiet = false;
/** config při otevření Studia („Vrátit vše do stavu při otevření“) */
let origin: any = null;
let timer: number | undefined;

export { state as saveState, history, conflict };

function schedule(ms = SAVE_DELAY_MS) {
  clearTimeout(timer);
  timer = window.setTimeout(() => void run(), ms);
}

async function run() {
  if (!isLoaded() || saving) return;
  const draft = cleanDraft(snapshotDraft());
  const before = JSON.parse(base());
  const changed = canon(draft) !== canon(before);
  setDirty(changed);
  if (!changed) return void (state() === "invalid" && setState("idle"));
  let err = null;
  try {
    err = (await api.checkConfig(draft)) ?? null;
  } catch {
    // ověření je jen nápověda — rozhodne až zápis
  }
  setFieldError(err);
  if (err) return void setState("invalid");
  if (!canSave({ changed, typing, invalid: false, conflict: conflict(), saving })) return;
  saving = true;
  setState("saving");
  try {
    // někdo config mezitím změnil (Claude, druhé okno, ruční úprava) → sloučit, nepřepsat potichu
    const live = await api.getConfig();
    if (live && canon(live) !== canon(before)) {
      const r = external(before, draft, live, canon);
      if (r.action === "conflict") return void (setConflict(true), setState("conflict"));
      if (r.action === "merge") adoptBase(live, r.draft);
      else loadBase(live);
      return;
    }
    await api.saveConfig(draft);
    for (const e of await runAfterSave()) sayError(e);
    if (!quiet) setHistory((h) => record(h, before, draft, Date.now()));
    quiet = false;
    const fresh = await api.getConfig();
    const now = snapshotDraft();
    // mezitím další změna → nový základ, rozpracované zůstane (autosave ji vezme v dalším kole)
    if (fresh) canon(cleanDraft(now)) === canon(draft) ? loadBase(fresh) : adoptBase(fresh, now);
    setFieldError(null);
    setState("saved");
  } catch (e) {
    sayError(e);
    setState("idle");
  } finally {
    saving = false;
  }
}

/** Zapsat hned, co čeká (rozepsané pole i změna < 400 ms) — před zavřením okna a před Vrátit. */
export async function flush() {
  clearTimeout(timer);
  typing = false;
  for (let i = 0; i < 20 && saving; i++) await new Promise((r) => setTimeout(r, 50));
  await run();
}

/** Zavřít okno Studia až po uložení (tlačítko ×; Alt+F4 přijde jako událost `studio-close-ask`). */
export async function closeStudio() {
  try {
    await flush();
  } catch {
    // zavřít se musí i tak
  }
  void api.studioWin("close").catch(() => {});
}

/** Studio: spustit autosave (jednou za život okna / stránky). Vrací akce pro titulek. */
export function useAutosave(root: () => HTMLElement | undefined) {
  createEffect(on(rev, () => {
    if (isLoaded() && origin === null) origin = JSON.parse(base());
    schedule();
  }, { defer: true }));
  onCleanup(() => clearTimeout(timer));
  const closeAsk = onEvent("studio-close-ask", () => void closeStudio());
  onCleanup(() => void closeAsk.then((off) => off()));

  // textová pole: psaní zápis odloží, Enter / opuštění ho spustí hned
  const isText = (el: EventTarget | null) => el instanceof HTMLElement && !el.closest("[data-nogate]") && isTextEntry(el.tagName, el.getAttribute("type"));
  const focusIn = (e: FocusEvent) => (typing = isText(e.target));
  const focusOut = (e: FocusEvent) => {
    if (!isText(e.target)) return;
    typing = false;
    schedule(0);
  };
  const enter = (e: KeyboardEvent) => {
    if (e.key === "Enter" && isText(e.target) && (e.target as HTMLElement).tagName === "INPUT") {
      typing = false;
      schedule(0);
      queueMicrotask(() => (typing = document.activeElement === e.target));
    }
  };
  queueMicrotask(() => {
    const el = root();
    if (!el) return;
    el.addEventListener("focusin", focusIn);
    el.addEventListener("focusout", focusOut);
    el.addEventListener("keydown", enter, true);
    onCleanup(() => {
      el.removeEventListener("focusin", focusIn);
      el.removeEventListener("focusout", focusOut);
      el.removeEventListener("keydown", enter, true);
    });
  });
}

/** Ctrl+Z / „Vrátit“ v titulku: poslední položka časové osy. */
export async function undo() {
  await flush();
  const r = undoLast(history(), snapshotDraft());
  if (!r) return;
  quiet = true;
  setHistory(r.history);
  replaceDraft(r.cfg);
}

/** „Vrátit“ u jedné položky časové osy (jen její volby, ostatní změny zůstanou). */
export async function undoOne(id: number) {
  await flush();
  const r = undoEntry(history(), id, snapshotDraft());
  if (!r) return;
  quiet = true;
  setHistory(r.history);
  replaceDraft(r.cfg);
}

/** „Vrátit vše do stavu při otevření“. */
export async function undoAll() {
  if (!origin) return;
  await flush();
  quiet = true;
  setHistory(emptyHistory());
  replaceDraft(origin);
}

/** Konflikt: vzít aktuální config (moje rozpracované zahodit), nebo přesto zapsat moje. */
export async function resolveConflict(mine: boolean) {
  setConflict(false);
  if (mine) {
    const live = await api.getConfig();
    if (live) adoptBase(live, snapshotDraft());
    return;
  }
  const live = await api.getConfig();
  if (live) loadBase(live);
  say(t("settings.reloaded", "Načteno aktuální nastavení"));
}

/** Změna configu zvenku (událost `config-changed`): bez rozpracované změny převzít tiše, jinak sloučit / konflikt. */
export async function adoptExternal() {
  if (!isLoaded() || saving) return;
  const fresh = await api.getConfig().catch(() => null);
  if (!fresh) return;
  const r = external(JSON.parse(base()), snapshotDraft(), fresh, canon);
  if (r.action === "load") loadBase(fresh);
  else if (r.action === "merge") adoptBase(fresh, r.draft);
  else if (r.action === "conflict") (setConflict(true), setState("conflict"));
}

/** Chyba ověření (pro titulek). */
export const saveError = () => fieldError();
