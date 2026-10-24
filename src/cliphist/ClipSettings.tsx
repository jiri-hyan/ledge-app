// Nastavení → Šuplík a karty → Obsah karet → Historie schránky (`clipboard.*`): lidsky popsané volby (co se
// ukládá, kde, co ne), vyloučené aplikace s lidskými názvy a „Přidat běžící program“ (výběr, ne psaní exe),
// „Vymazat historii teď“. Jen řádky — nadpis a větu skupiny dává `settings/panels/Personal.tsx`. Popisky voleb
// jsou v `settings/labels.ts` (hledání v Nastavení).
import { createEffect, createSignal, For, Show } from "solid-js";
import { api } from "../api";
import { t } from "../i18n";
import { useSettings } from "../settings/ctx";
import { Slider, Toggle } from "../settings/fields/controls";
import { Row } from "../settings/fields/Row";
import { fieldLabel } from "../settings/labels";
import { say, sayError } from "../ui/toast";
import { clipApi, type AppChoice } from "./api";
import "./cliphist.css";

const same = (a: string, b: string) => a.trim().toLowerCase() === b.trim().toLowerCase();

function ExcludeApps() {
  const c = useSettings();
  const list = (): string[] => c.get("clipboard.excludeApps") ?? [];
  const [labels, setLabels] = createSignal<Record<string, string>>({});
  const [running, setRunning] = createSignal<AppChoice[] | null>(null);
  const [busy, setBusy] = createSignal(false);
  createEffect(() => {
    const exes = list();
    clipApi.apps(exes, false).then((r) => r && setLabels(Object.fromEntries(r.listed.map((a) => [a.exe, a.label]))), () => {});
  });
  const add = (exe: string) => !list().some((x) => same(x, exe)) && c.set("clipboard.excludeApps", [...list(), exe]);
  const remove = (i: number) => c.set("clipboard.excludeApps", list().filter((_, j) => j !== i));
  async function pick() {
    if (running()) return setRunning(null);
    setBusy(true);
    try {
      setRunning((await clipApi.apps([], true))?.running ?? []);
    } catch (e) {
      sayError(e);
    } finally {
      setBusy(false);
    }
  }
  async function defaults() {
    const d: string[] = (await api.configDefaults())?.clipboard?.excludeApps ?? [];
    const missing = d.filter((x) => !list().some((y) => same(x, y)));
    if (missing.length) c.set("clipboard.excludeApps", [...list(), ...missing]);
  }
  const offer = () => (running() ?? []).filter((a) => !list().some((x) => same(x, a.exe)));
  return (
    <div class="ch-apps">
      <div class="ch-list">
        <For each={list()} fallback={<span class="ch-none">{t("settings.clip.apps.none", "Žádné — ukládá se ze všech aplikací")}</span>}>
          {(exe, i) => {
            const label = () => labels()[exe.split(/[\/]/).pop()?.trim() ?? exe] ?? exe;
            return (
              <span class="ch-app" title={exe}>
                {label()}
                <Show when={label() !== exe}><small>{exe}</small></Show>
                <button aria-label={t("settings.ui.chip.remove", "Odebrat {x}", { x: label() })} onClick={() => remove(i())}>×</button>
              </span>
            );
          }}
        </For>
      </div>
      <div class="ch-bar">
        <button class="mini" disabled={busy()} onClick={pick}>{running() ? t("settings.clip.apps.done", "Hotovo") : t("settings.clip.apps.pick", "Přidat běžící program…")}</button>
        <button class="mini" onClick={() => void defaults()}>{t("settings.clip.apps.defaults", "Přidat běžné správce hesel")}</button>
      </div>
      <Show when={running()}>
        <div class="ch-pick" role="group" aria-label={t("settings.clip.apps.pick", "Přidat běžící program…")}>
          <For each={offer()} fallback={<span class="ch-none">{t("settings.clip.apps.noneRunning", "Žádný další program s oknem neběží")}</span>}>
            {(a) => (
              <button classList={{ "ch-known": a.known }} title={a.exe} onClick={() => add(a.exe)}>
                {a.label}
              </button>
            )}
          </For>
        </div>
      </Show>
    </div>
  );
}

export function ClipSettings() {
  const c = useSettings();
  const on = () => !!c.cfg.clipboard?.enabled;
  async function clearNow() {
    try {
      await clipApi.clear();
      say(t("settings.clip.cleared", "Historie schránky vymazaná (připnuté zůstaly)"));
    } catch (e) {
      sayError(e);
    }
  }
  return (
    <>
      <Row id="clipboard.enabled"><Toggle path="clipboard.enabled" label={fieldLabel("clipboard.enabled")} /></Row>
      <Show when={on()}>
        <Row id="clipboard.max"><Slider path="clipboard.max" min={5} max={200} step={5} label={fieldLabel("clipboard.max")} /></Row>
        <Row id="clipboard.skipSecrets"><Toggle path="clipboard.skipSecrets" label={fieldLabel("clipboard.skipSecrets")} /></Row>
        <Row id="clipboard.clearOnLock"><Toggle path="clipboard.clearOnLock" label={fieldLabel("clipboard.clearOnLock")} /></Row>
        <Row id="clipboard.savePins"><Toggle path="clipboard.savePins" label={fieldLabel("clipboard.savePins")} /></Row>
        <Row id="clipboard.excludeApps" stack><ExcludeApps /></Row>
        <Row id="clipboard.clear">
          <button class="mini" onClick={() => void clearNow()}>{t("settings.clip.clearBtn", "Vymazat")}</button>
        </Row>
      </Show>
    </>
  );
}
