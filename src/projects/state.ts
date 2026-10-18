// Zástupci projektů — akce z dlaždice, pilulky a palety (vlna 9 Z): spustit podle místa (víc složek = nejdřív
// výběr, chybějící složka = panel s chybou), Stop / Restart / Zastavit vše, Změnit složku… (nativní dialog
// `pick_path`) a Změnit příkaz (pole v panelu) — obojí uloží jen `path` / `run` toho projektu v configu.
// Který panel je otevřený (jeden na šuplík): `projectPanel`.
import { createSignal } from "solid-js";
import { api, type ProcView, type ProjectView } from "../api";
import { t } from "../i18n";
import { say, sayError } from "../ui/toast";
import { closeLook } from "../tilelook/state";
import { clickOf, placeProblem, withProjectFields, type ProjectPatch } from "./model";

export type PanelMode = "edit" | "choose";
export type PanelTarget = { id: string; mode: PanelMode; focus?: "run" | "log" };

const [panel, setPanel] = createSignal<PanelTarget | null>(null);
export { panel as projectPanel };
/** pole panelu si vzalo klávesnici → při zavření ji vrátit */
let tookFocus = false;

export function openProjectPanel(id: string, mode: PanelMode, focus?: "run" | "log") {
  void closeLook(true); // jeden panel u dlaždic naráz
  tipLeave();
  setPanel({ id, mode, focus });
}

export function closeProjectPanel() {
  setPanel(null);
  if (tookFocus) void api.releaseFocus();
  tookFocus = false;
}

// ---------- náhled po najetí (jen informace; akce jsou v menu dlaždice) ----------

const [tip, setTip] = createSignal<{ id: string; el: HTMLElement } | null>(null);
export { tip };
/** prodleva náhledu — přejetí myší přes řadu dlaždic nic neukáže */
export const TIP_DELAY_MS = 450;
let tipTimer: number | undefined;
/** Najetí / fokus na dlaždici: náhled po prodlevě (jeden časovač jen během najetí, v klidu nic). */
export function tipEnter(id: string, el: HTMLElement) {
  clearTimeout(tipTimer);
  tipTimer = window.setTimeout(() => setTip({ id, el }), TIP_DELAY_MS);
}
export function tipLeave() {
  clearTimeout(tipTimer);
  tipTimer = undefined;
  if (tip()) setTip(null);
}

/** Psaní do pole (příkaz) potřebuje klávesnici — jen po kliku do pole. */
export function wantKeys() {
  if (tookFocus) return;
  tookFocus = true;
  void api.focusDrawer();
}

/** Uloží `path` / `run` jednoho projektu (get_config → změna → save_config, jako vzhled zástupce). */
export async function saveProjectFields(id: string, patch: ProjectPatch): Promise<boolean> {
  try {
    const cfg = await api.getConfig();
    if (!cfg) return false;
    await api.saveConfig(withProjectFields(cfg, id, patch));
    return true;
  } catch (e) {
    sayError(e);
    return false;
  }
}

/** „Změnit složku…“: dialog Windows (patří šuplíku, fokus jen na tenhle klik), uloží do `path`. */
export async function changeFolder(pr: ProjectView): Promise<string | null> {
  try {
    await api.focusDrawer(); // dialog patří šuplíku: odjetí myší ani blur ho pak nezavřou
    const dir = await api.pickPath("folder", t("cards.proj.pick.title", "Složka, ze které se {name} spouští", { name: pr.name }), pr.path ?? undefined);
    if (!dir) return null;
    if (await saveProjectFields(pr.id, { path: dir })) {
      say(t("cards.proj.pick.saved", "{name} se teď spouští z {dir}", { name: pr.name, dir }));
      return dir;
    }
    return null;
  } catch (e) {
    sayError(e);
    return null;
  } finally {
    if (!tookFocus) void api.releaseFocus();
  }
}

export async function launch(pr: ProjectView) {
  try {
    const r = await api.launch(pr.id);
    if (r?.outcome === "alreadyRunning") {
      if (r.url) api.openUrl(r.url);
      else say(t("cards.projects.alreadyRunning", "{name} už běží.", { name: pr.name }));
    } else {
      say(pr.kind === "dev" ? t("cards.projects.starting", "Spouštím {run} · {name}", { run: pr.run ?? "server", name: pr.name }) : t("cards.projects.opening", "Otevírám {name}…", { name: pr.name }));
    }
  } catch (e) {
    sayError(e);
  }
}

/** Klik na dlaždici: víc složek → výběr, chybí složka → panel s chybou a „Změnit složku…“, jinak spustit. */
export function startProject(pr: ProjectView, proc?: ProcView | null) {
  switch (clickOf(pr, proc)) {
    case "choose":
      return openProjectPanel(pr.id, "choose");
    case "fix":
      say(placeProblem(pr) ?? t("cards.proj.problem.none", "Složka projektu na tomhle PC není."), true);
      return openProjectPanel(pr.id, "edit");
    default:
      return void launch(pr);
  }
}

export async function stopProject(id: string, name: string) {
  try {
    await api.stop(id);
    say(t("cards.proj.stopped", "Zastaveno: {name}", { name }));
  } catch (e) {
    sayError(e);
  }
}

export async function restartProject(id: string, name: string) {
  say(t("cards.proj.restarting", "Restartuji {name}…", { name }));
  try {
    await api.restart(id);
  } catch (e) {
    sayError(e);
  }
}

export async function stopAll() {
  try {
    const n = (await api.stopAllServers()) ?? 0;
    say(n ? t("cards.proj.stoppedAll", "Zastaveno serverů: {n}", { n }) : t("cards.proj.nothingRunning", "Nic z Ledge neběží."));
  } catch (e) {
    sayError(e);
  }
}
