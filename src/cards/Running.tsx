// Karta Běží: procesy spuštěné Ledge (`Snapshot.procs`) — fáze, uptime, log (posledních 60 řádků
// přes `api.logs`, jen u rozbaleného), Otevřít / Stop / Zavřít.
import { createEffect, createSignal, For, Show, type JSX } from "solid-js";
import { api, type ProcView, type Snapshot } from "../api";
import { uptime } from "../format";
import { t, tp } from "../i18n";

/** fáze → barva pilulky a popisek (funkce, ať se popisek přeloží až při vykreslení) */
const PHASE: Record<string, [string, () => string]> = {
  preparing: ["a", () => "build"],
  starting: ["a", () => t("cards.running.starting", "startuje")],
  ready: ["g", () => ""],
  running: ["g", () => t("cards.running.running", "běží")],
  exited: ["n", () => t("cards.running.exited", "skončil")],
  failed: ["r", () => t("cards.running.failed", "pád")],
};

export default function Running(p: { s: Snapshot; now: number; menu?: JSX.Element }) {
  const [open, setOpen] = createSignal<string | null>(null);
  const [lines, setLines] = createSignal<string[]>([]);
  const name = (id: string) => p.s.projects.find((x) => x.id === id)?.name ?? id;
  createEffect(() => {
    const id = open();
    void p.s.procs.length;
    void p.s.procs.find((x) => x.id === id)?.lastLine;
    if (id) api.logs(id).then((l) => setLines(l?.slice(-60) ?? []));
  });
  return (
    <div class="card" data-anim>
      <h4><span class="ic" style="background:var(--green)" />{t("cards.running.title", "Běží")} <span class="n">{p.s.procs.filter((x) => x.phase !== "exited" && x.phase !== "failed").length}</span>{p.menu}</h4>
      <For each={p.s.procs} fallback={<div class="empty">{t("cards.running.empty", "Nic neběží. Klikni na projekt nahoře.")}</div>}>
        {(pc: ProcView) => {
          const ph = (): [string, () => string] => PHASE[pc.phase] ?? ["n", () => pc.phase];
          return (
            <>
              <div class="row">
                <span class={`pill ${ph()[0]}`}>{pc.phase === "ready" ? `:${pc.port}` : ph()[1]()}</span>
                <div class="grow">
                  <div class="name">{name(pc.id)}</div>
                  <Show when={pc.phase === "failed" || pc.phase === "exited"} fallback={
                    <div class="sub">{pc.phase === "ready" || pc.phase === "running" ? `up ${uptime(pc.readyAt ?? pc.startedAt, p.now)}` : pc.lastLine ?? "…"}{pc.errors > 0 ? tp("cards.running.errors", pc.errors, " · {n}× chyba") : ""}</div>
                  }>
                    <div class="sub err">{pc.message}</div>
                  </Show>
                </div>
                <span class="r">
                  <Show when={pc.url}>{(u) => <button class="mini" onClick={() => api.openUrl(u())}>{t("cards.running.open", "Otevřít")}</button>}</Show>
                  <button class="mini" aria-expanded={open() === pc.id} onClick={() => setOpen(open() === pc.id ? null : pc.id)}>{open() === pc.id ? t("cards.running.hide", "Skrýt") : t("cards.running.log", "Log")}</button>
                  <Show when={pc.phase === "failed" || pc.phase === "exited"} fallback={<button class="mini" onClick={() => api.stop(pc.id)}>{t("cards.running.stop", "Stop")}</button>}>
                    <button class="mini" onClick={() => api.dismiss(pc.id)}>{t("cards.running.dismiss", "Zavřít")}</button>
                  </Show>
                </span>
              </div>
              <Show when={open() === pc.id}>
                <pre class="log" ref={(el) => queueMicrotask(() => (el.scrollTop = el.scrollHeight))}>{lines().join("\n")}</pre>
              </Show>
            </>
          );
        }}
      </For>
    </div>
  );
}
