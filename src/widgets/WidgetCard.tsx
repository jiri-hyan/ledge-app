// Karta widgetu ze skriptu (`widget:<soubor>` na libovolné záložce): co skript vypsal (název, odznak,
// text, průběh, položky s barvou a klikem), poctivý stav (souhlas, chyba s výstupem, chybějící soubor,
// stará data) a ⋯ s „Obnovit teď“, „Upravit skript“, „Zobrazit výstup“ a „V proužku“. Data jsou jen
// ze snapshotu (`Snapshot.widgets`) — spouští je Rust (`widgets/live.rs`), jen když je karta vidět.
import { createSignal, For, Show, type JSX } from "solid-js";
import { api, type Snapshot } from "../api";
import { ago } from "../format";
import { t } from "../i18n";
import { say, sayError } from "../ui/toast";
import type { CardMenuItem } from "../cards/CardMenu";
import type { WidgetItem, WidgetView } from "../types";
import { widgetsApi } from "./api";
import { intervalParts, isStale, itemKind, progressPct, toneVar, widgetFile, withStripItem } from "./model";
import "./widgets.css";

/** Zapne / vypne položku proužku `id` (jedno pole configu, jako ⋯ Skrýt kartu). */
export async function toggleStrip(id: string, on: boolean, cfgError: string | null): Promise<void> {
  if (cfgError) {
    say(t("cards.widget.cfgError", "Nastavení obsahuje chybu — proužek teď upravíš jen v Nastavení."), true);
    return;
  }
  try {
    const cfg = await api.getConfig();
    if (!cfg) return;
    await api.saveConfig({ ...cfg, strip: { ...cfg.strip, items: withStripItem(cfg.strip?.items, id, on) } });
    say(on ? t("cards.widget.stripOn", "Widget je v proužku.") : t("cards.widget.stripOff", "Widget už v proužku není."));
  } catch (e) {
    sayError(e);
  }
}

const KIND_HINT: Record<string, () => string> = {
  url: () => t("cards.widget.item.url", "Otevřít v prohlížeči"),
  open: () => t("cards.widget.item.open", "Otevřít složku nebo soubor"),
  copy: () => t("cards.widget.item.copy", "Zkopírovat do schránky"),
  run: () => t("cards.widget.item.run", "Spustit skript"),
};

export function intervalText(s: number): string {
  const x = intervalParts(s);
  return t("cards.widget.every", "každých {n} {unit}", { n: x.n, unit: x.unit });
}

/** Povolení widgetu: věta, ukázka začátku skriptu, co přesně se povoluje, jedním klikem. */
export function WidgetConsent(p: { v: WidgetView }) {
  const [busy, setBusy] = createSignal(false);
  const allow = async () => {
    const c = p.v.consent;
    if (!c) return;
    setBusy(true);
    try {
      await widgetsApi.consent(p.v.id, c.confirmText);
      say(t("cards.widget.consent.done", "Povoleno — widget se spouští."));
    } catch (e) {
      sayError(e);
    } finally {
      setBusy(false);
    }
  };
  return (
    <Show when={p.v.consent}>
      {(c) => (
        <div class="wg-consent">
          <div class="wg-note" classList={{ "wg-changed": c().changed }}>{c().note}</div>
          <pre class="wg-pre">{c().preview}</pre>
          <div class="sub wg-fp">{c().confirmText}</div>
          <div class="wg-actions">
            <button class="mini ok" disabled={busy()} onClick={() => void allow()}>{c().changed ? t("cards.widget.consent.again", "Povolit znovu a spustit") : t("cards.widget.consent.allow", "Povolit a spustit")}</button>
            <button class="mini" onClick={() => void widgetsApi.edit(p.v.id).catch(sayError)}>{t("cards.widget.menu.edit", "Upravit skript")}</button>
          </div>
        </div>
      )}
    </Show>
  );
}

export default function WidgetCard(p: { s: Snapshot; id: string; now: number; menu: (extra: CardMenuItem[]) => JSX.Element }) {
  const v = (): WidgetView | null => p.s.widgets?.find((w) => w.id === p.id) ?? null;
  const [out, setOut] = createSignal<string | null>(null);
  const inStrip = () => !!v()?.inStrip;

  const showOutput = async () => {
    if (out() !== null) return setOut(null);
    try {
      const o = await widgetsApi.output(p.id);
      setOut(o ?? t("cards.widget.noOutput", "Widget zatím nic nevypsal."));
    } catch (e) {
      sayError(e);
    }
  };
  const refresh = () => void widgetsApi.refresh(p.id).catch(sayError);
  const edit = () => void widgetsApi.edit(p.id).catch(sayError);
  const click = async (i: WidgetItem, index: number) => {
    if (!itemKind(i)) return;
    try {
      const msg = await widgetsApi.click(p.id, index, i.label);
      if (msg) say(msg);
    } catch (e) {
      sayError(e);
    }
  };
  const extra = (): CardMenuItem[] => {
    const w = v();
    const items: CardMenuItem[] = [
      { label: t("cards.widget.menu.refresh", "Obnovit teď"), run: refresh, disabled: !w || w.running || w.status === "consent" || w.status === "problem" },
      { label: t("cards.widget.menu.edit", "Upravit skript"), run: edit, disabled: !w || w.status === "missing" },
      { label: out() !== null ? t("cards.widget.menu.hideOutput", "Skrýt výstup") : t("cards.widget.menu.output", "Zobrazit výstup"), run: () => void showOutput(), disabled: !w?.hasOutput && out() === null },
    ];
    if (w && w.show !== "card") {
      items.push(inStrip()
        ? { label: t("cards.widget.menu.stripOff", "Odebrat z proužku"), run: () => void toggleStrip(p.id, false, p.s.cfgError) }
        : { label: t("cards.widget.menu.stripOn", "Ukázat i v proužku"), run: () => void toggleStrip(p.id, true, p.s.cfgError) });
    }
    return items;
  };

  return (
    <div class="card wg" data-anim data-status={v()?.status ?? "idle"}>
      <h4>
        <Show when={v()?.icon}>{(ic) => <span class="wg-ico" aria-hidden="true">{ic()}</span>}</Show>
        {v()?.data?.title ?? v()?.title ?? widgetFile(p.id) ?? p.id}
        <Show when={v()?.data?.badge}>{(b) => <span class="pill b wg-badge" style={{ color: toneVar(v()?.data?.color) }}>{b()}</span>}</Show>
        <Show when={v()?.running}><span class="n wg-spin" title={t("cards.widget.running", "Běží…")}>⟳</span></Show>
        {p.menu(extra())}
      </h4>
      <Show when={v()} fallback={<div class="empty">{t("cards.widget.waiting", "Načítám widget…")}</div>}>
        {(w) => (
          <>
            <Show when={w().status === "off"}>
              <div class="empty">{t("cards.widget.off", "Widgety jsou vypnuté (Nastavení → Projekty, akce a odkazy → Widgety).")}</div>
            </Show>
            <Show when={w().status === "missing"}>
              <div class="empty">{t("cards.widget.missing", "Soubor {file} ve složce widgetů není — přejmenovaný nebo smazaný. Kartu odebereš přes ⋯.", { file: w().file })}</div>
            </Show>
            <Show when={w().status === "problem"}>
              <div class="sub err wg-msg">{w().problem}</div>
              <div class="wg-actions"><button class="mini" onClick={edit}>{t("cards.widget.menu.edit", "Upravit skript")}</button></div>
            </Show>
            <Show when={w().status === "consent"}>
              <WidgetConsent v={w()} />
            </Show>
            <Show when={w().error && w().status === "error"}>
              <div class="wg-err">
                <span class="grow">{w().error}</span>
                <Show when={w().hasOutput}><button class="mini" onClick={() => void showOutput()}>{out() !== null ? t("cards.widget.menu.hideOutput", "Skrýt výstup") : t("cards.widget.menu.output", "Zobrazit výstup")}</button></Show>
              </div>
            </Show>
            <Show when={!w().data && (w().status === "idle" || w().status === "loading")}>
              <div class="empty">{w().running ? t("cards.widget.loading", "Spouštím skript…") : t("cards.widget.idle", "Zatím neběžel — spustí se, když je karta vidět.")}</div>
            </Show>
            <Show when={w().status !== "consent" && w().status !== "problem" ? w().data : null}>
              {(d) => (
                <>
                  <Show when={d().text}>{(x) => <div class="wg-text">{x()}</div>}</Show>
                  <Show when={progressPct(d().progress) !== null}>
                    <div class="wg-bar" role="progressbar" aria-valuenow={progressPct(d().progress)!} aria-valuemin={0} aria-valuemax={100}>
                      <span style={{ width: `${progressPct(d().progress)}%`, background: toneVar(d().color) }} />
                    </div>
                  </Show>
                  <Show when={d().items.length}>
                    <div class="wg-items">
                      <For each={d().items}>
                        {(i, idx) => (
                          <Show when={!i.sep} fallback={<hr class="wg-sep" />}>
                            <button class="row wg-item" classList={{ "wg-click": !!itemKind(i) }} disabled={!itemKind(i)} title={itemKind(i) ? KIND_HINT[itemKind(i)!]() : ""} onClick={() => void click(i, idx())}>
                              <Show when={i.icon} fallback={<span class="wg-dot" style={{ background: toneVar(i.color) ?? "transparent" }} />}>
                                <span class="wg-ico" aria-hidden="true">{i.icon}</span>
                              </Show>
                              <span class="grow wg-label">{i.label}</span>
                              <Show when={i.value}><span class="wg-value" style={{ color: toneVar(i.color) }}>{i.value}</span></Show>
                            </button>
                          </Show>
                        )}
                      </For>
                    </div>
                  </Show>
                  <Show when={!d().text && !d().items.length && progressPct(d().progress) === null}>
                    <div class="empty">{d().strip ?? t("cards.widget.empty", "Skript nic nevypsal.")}</div>
                  </Show>
                  <Show when={d().notes?.length}>
                    <details class="wg-notes">
                      <summary>{t("cards.widget.notes", "Něco z výstupu nešlo použít")}</summary>
                      <For each={d().notes}>{(n) => <div class="sub">{n}</div>}</For>
                    </details>
                  </Show>
                </>
              )}
            </Show>
            <Show when={out() !== null}>
              <pre class="log wg-out">{out()}</pre>
            </Show>
            <Show when={w().updatedMs || w().intervalS}>
              <div class="wg-foot">
                <Show when={isStale(w().updatedMs, w().intervalS, p.now)}><span class="pill a">{t("cards.widget.stale", "stará data")}</span></Show>
                <span class="wg-meta">
                  {w().updatedMs ? t("cards.widget.updated", "obnoveno {ago}", { ago: ago(w().updatedMs!, p.now) }) : ""}
                  {w().updatedMs && w().intervalS ? " · " : ""}
                  {w().intervalS ? intervalText(w().intervalS) : ""}
                </span>
              </div>
            </Show>
          </>
        )}
      </Show>
    </div>
  );
}
