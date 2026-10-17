// Osobní: karta „Časovač“ — odpočet (předvolby + vlastní minuty, pauza, +5 min, zrušit) a stopky (start,
// pauza, reset). Stav drží Rust (`timer.rs`, přežije zavření šuplíku i restart); karta jen posílá akce
// (`api.timer`) a sekundy počítá z časů a tiku `now` skořápky (jen při otevřeném šuplíku). Nic nebere fokus
// kromě pole vlastních minut, do kterého uživatel sám klikne.
import { createEffect, createSignal, For, on, Show, type JSX } from "solid-js";
import { api, type Snapshot } from "../api";
import { t } from "../i18n";
import { sayError } from "../ui/toast";
import { elapsedMs, fmtDuration, leftMs, parseMinutes, presetsOf, progress, type TimerOp, type TimerState } from "./model";
import "./timer.css";

/** Akce časovače (karta i paleta). Vrátí nový stav, chyba = hláška. */
export async function timerDo(op: TimerOp): Promise<TimerState | undefined> {
  try {
    return await api.timer(op);
  } catch (e) {
    sayError(e);
    return undefined;
  }
}

export default function TimerCard(p: { s: Snapshot; now: number; menu?: JSX.Element }) {
  // odpověď příkazu hned, než dorazí nový snapshot (ten ji pak přepíše)
  const [local, setLocal] = createSignal<TimerState | null>(null);
  createEffect(on(() => p.s.timer, () => setLocal(null), { defer: true }));
  const st = (): TimerState => local() ?? p.s.timer ?? { countdown: null, stopwatch: null };
  const cd = () => st().countdown;
  const sw = () => st().stopwatch;
  const run = async (op: TimerOp) => {
    const s = await timerDo(op);
    if (s) setLocal(s);
  };
  const [custom, setCustom] = createSignal("");
  const customMin = () => parseMinutes(custom());
  const startCustom = () => {
    const m = customMin();
    if (m == null) return;
    setCustom("");
    void run({ op: "start", minutes: m });
  };
  const idle = () => !cd() || cd()!.state === "done";

  return (
    <div class="card" data-anim>
      <h4><span class="ic" style="background:var(--violet)" />{t("cards.timer.title", "Časovač")}{p.menu}</h4>

      <Show when={cd()}>
        {(c) => (
          <div class="tm-cd" classList={{ "tm-done": c().state === "done" }}>
            <div class="tm-big" role="timer" aria-live="off">
              <Show when={c().state !== "done"} fallback={t("cards.timer.done", "Čas vypršel")}>
                {fmtDuration(leftMs(c(), p.now), true)}
                <Show when={c().state === "paused"}><span class="tm-tag">{t("cards.timer.paused", "pauza")}</span></Show>
              </Show>
            </div>
            <div class="tm-bar"><span style={{ width: `${progress(c(), p.now) * 100}%` }} /></div>
            <div class="tm-row">
              <Show when={c().state === "running"}><button class="mini" onClick={() => void run({ op: "pause" })}>{t("cards.timer.pause", "Pauza")}</button></Show>
              <Show when={c().state === "paused"}><button class="mini ok" onClick={() => void run({ op: "resume" })}>{t("cards.timer.resume", "Pokračovat")}</button></Show>
              <button class="mini" onClick={() => void run({ op: "add", minutes: 5 })}>{t("cards.timer.plus5", "+5 min")}</button>
              <button class="mini tm-end" onClick={() => void run({ op: "cancel" })}>{c().state === "done" ? t("cards.timer.dismiss", "Zavřít") : t("cards.timer.cancel", "Zrušit")}</button>
            </div>
          </div>
        )}
      </Show>

      <Show when={idle()}>
        <div class="tm-row">
          <For each={presetsOf(p.s.timer)}>
            {(m) => <button class="mini" onClick={() => void run({ op: "start", minutes: m })}>{t("cards.timer.preset", "{m} min", { m })}</button>}
          </For>
        </div>
        <div class="tm-row">
          <input
            class="tm-min"
            type="number"
            min="1"
            max="600"
            inputmode="numeric"
            placeholder={t("cards.timer.custom.ph", "min")}
            aria-label={t("cards.timer.custom", "Vlastní délka v minutách")}
            value={custom()}
            onInput={(e) => setCustom(e.currentTarget.value)}
            onKeyDown={(e) => e.key === "Enter" && startCustom()}
          />
          <button class="mini" disabled={customMin() == null} onClick={startCustom}>{t("cards.timer.start", "Spustit")}</button>
        </div>
      </Show>

      <div class="tm-sw">
        <span class="tm-label">{t("cards.timer.stopwatch", "Stopky")}</span>
        <span class="tm-sw-time">{sw() ? fmtDuration(elapsedMs(sw()!, p.now)) : "0:00"}</span>
        <Show
          when={sw()?.state === "running"}
          fallback={<button class="mini" onClick={() => void run({ op: "swStart" })}>{sw() ? t("cards.timer.resume", "Pokračovat") : t("cards.timer.swStart", "Start")}</button>}
        >
          <button class="mini" onClick={() => void run({ op: "swPause" })}>{t("cards.timer.pause", "Pauza")}</button>
        </Show>
        <button class="mini" disabled={!sw()} onClick={() => void run({ op: "swReset" })}>{t("cards.timer.reset", "Reset")}</button>
      </div>
    </div>
  );
}
