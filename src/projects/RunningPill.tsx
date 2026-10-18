// Pilulka „N běží“ v hlavičce šuplíku (vlna 9 Z): klik rozbalí seznam serverů, které spustil Ledge —
// u každého Stop / Restart / Otevřít / Log (selhaný: Zkusit znovu / Zavřít), dole „Zastavit všechny“.
// Při otevření se jádra jednou zeptá, jestli porty projektů nedrží něco, co Ledge NESPUSTIL (`port_holders`)
// — to jen ukáže a ukončí až po výslovném potvrzení (jen ten jeden proces).
import { createResource, createSignal, For, onCleanup, onMount, Show } from "solid-js";
import { Portal } from "solid-js/web";
import { api, type PortHolder, type ProcView, type Snapshot } from "../api";
import { uptime } from "../format";
import { t } from "../i18n";
import { rich } from "../ui/rich";
import { say, sayError } from "../ui/toast";
import { isLive, liveProcs } from "./model";
import { restartProject, stopAll, stopProject } from "./state";
import "./projpanel.css";

export default function RunningPill(p: { s: Snapshot; now: number }) {
  const [open, setOpen] = createSignal<{ x: number; y: number } | null>(null);
  const live = () => liveProcs(p.s.procs);
  const toggle = (e: MouseEvent) => {
    if (open()) return setOpen(null);
    const r = (e.currentTarget as HTMLElement).getBoundingClientRect();
    setOpen({ x: Math.max(8, Math.min(r.left, window.innerWidth - 440)), y: r.bottom + 6 });
  };
  return (
    <>
      <button
        class={`chip pz-pill ${live().length ? "green" : "dim"}`}
        aria-expanded={!!open()}
        title={t("cards.proj.pill.title", "Běžící servery — klik ukáže Stop, Restart, Otevřít a log")}
        onClick={toggle}
      >
        <span class="d" />{rich(t("cards.head.live", "<b>{n}</b> běží", { n: live().length }))} <span class="pz-caret">▾</span>
      </button>
      <Show when={open()}>{(at) => <Drop s={p.s} now={p.now} at={at()} onClose={() => setOpen(null)} />}</Show>
    </>
  );
}

function Drop(p: { s: Snapshot; now: number; at: { x: number; y: number }; onClose: () => void }) {
  let box!: HTMLDivElement;
  const [logOf, setLogOf] = createSignal<string | null>(null);
  const [lines, setLines] = createSignal<string[]>([]);
  const [ask, setAsk] = createSignal<PortHolder | null>(null);
  const [holders, { refetch }] = createResource(() => api.portHolders().then((x) => x ?? []));
  const name = (id: string) => p.s.projects.find((x) => x.id === id)?.name ?? id;
  onMount(() => {
    const down = (e: PointerEvent) => {
      const el = e.target as HTMLElement | null;
      if (el && !box.contains(el) && !el.closest(".pz-pill")) p.onClose();
    };
    const key = (e: KeyboardEvent) => e.key === "Escape" && (e.stopPropagation(), p.onClose());
    document.addEventListener("pointerdown", down, true);
    window.addEventListener("keydown", key, true);
    onCleanup(() => {
      document.removeEventListener("pointerdown", down, true);
      window.removeEventListener("keydown", key, true);
    });
  });
  const showLog = async (id: string) => {
    if (logOf() === id) return setLogOf(null);
    setLogOf(id);
    setLines(((await api.logs(id)) ?? []).slice(-60));
  };
  const kill = async (h: PortHolder) => {
    setAsk(null);
    try {
      await api.killPortHolder(h.port, h.pid);
      say(t("cards.proj.foreign.killed", "Ukončeno: {proc} na portu {port}", { proc: h.process, port: h.port }));
      void refetch();
    } catch (e) {
      sayError(e);
    }
  };
  const sub = (pc: ProcView) =>
    pc.phase === "failed" || pc.phase === "exited"
      ? pc.message ?? ""
      : [pc.port ? `:${pc.port}` : "", pc.phase === "ready" || pc.phase === "running" ? t("cards.proj.pill.up", "běží {time}", { time: uptime(pc.readyAt ?? pc.startedAt, p.now) }) : t("cards.running.starting", "startuje"), pc.cwd ?? ""].filter(Boolean).join(" · ");
  return (
    <Portal mount={document.querySelector(".shell") ?? document.body}>
      <div class="pz-drop" ref={box} role="dialog" aria-label={t("cards.proj.pill.aria", "Běžící servery")} style={{ left: `${p.at.x}px`, top: `${p.at.y}px` }}>
        <For each={p.s.procs} fallback={<div class="pz-note">{t("cards.proj.pill.empty", "Nic z Ledge neběží. Server spustíš klikem na projekt.")}</div>}>
          {(pc) => (
            <>
              <div class="pz-srv">
                <span class="pz-dot" classList={{ "pz-dot-ok": pc.phase === "ready" || pc.phase === "running", "pz-dot-wait": pc.phase === "starting" || pc.phase === "preparing", "pz-dot-bad": pc.phase === "failed" }} />
                <span class="grow">
                  <b>{name(pc.id)}</b>
                  <small classList={{ "pz-bad": pc.phase === "failed" }}>{sub(pc)}</small>
                </span>
                <Show when={pc.url}>{(u) => <button class="mini" onClick={() => api.openUrl(u())}>{t("cards.proj.open", "Otevřít")}</button>}</Show>
                <button class="mini" aria-expanded={logOf() === pc.id} onClick={() => void showLog(pc.id)}>{t("cards.running.log", "Log")}</button>
                <Show when={isLive(pc.phase)} fallback={
                  <>
                    <button class="mini" onClick={() => void restartProject(pc.id, name(pc.id))}>{t("cards.proj.fail.retry", "Zkusit znovu")}</button>
                    <button class="mini" onClick={() => api.dismiss(pc.id)}>{t("cards.proj.fail.dismiss", "Zavřít")}</button>
                  </>
                }>
                  <button class="mini" onClick={() => void restartProject(pc.id, name(pc.id))}>{t("cards.proj.restart", "Restart")}</button>
                  <button class="mini ok" onClick={() => void stopProject(pc.id, name(pc.id))}>■ {t("cards.proj.stop", "Stop")}</button>
                </Show>
              </div>
              <Show when={logOf() === pc.id}>
                <pre class="log" ref={(el) => queueMicrotask(() => (el.scrollTop = el.scrollHeight))}>{lines().join("\n")}</pre>
              </Show>
            </>
          )}
        </For>
        <Show when={liveProcs(p.s.procs).length > 1}>
          <div class="pz-row pz-btns pz-foot">
            <span class="grow" />
            <button class="mini" onClick={() => void stopAll()}>■ {t("cards.proj.stopAll", "Zastavit všechny servery")}</button>
          </div>
        </Show>
        <Show when={holders()?.length}>
          <div class="pz-sep">{t("cards.proj.foreign.title", "Na portech projektů běží i něco, co Ledge nespustil:")}</div>
          <For each={holders()}>
            {(h) => (
              <>
                <div class="pz-srv">
                  <span class="pz-dot" />
                  <span class="grow">
                    <b>{t("cards.proj.foreign.port", "Port {port}", { port: h.port })}</b>
                    <small>{t("cards.proj.foreign.who", "{proc} · port projektu {name}", { proc: h.process, name: name(h.id) })}</small>
                  </span>
                  <button class="mini" onClick={() => setAsk(h)}>{t("cards.proj.foreign.kill", "Ukončit…")}</button>
                </div>
                <Show when={ask()?.pid === h.pid && ask()?.port === h.port}>
                  <div class="pz-ask" role="alertdialog">
                    <span class="grow">{t("cards.proj.foreign.ask", "Ukončit {proc}? Ledge ho nespustil — může to být server, který běží v terminálu, a rozdělaná práce v něm skončí.", { proc: h.process })}</span>
                    <button class="mini ok" onClick={() => void kill(h)}>{t("cards.proj.foreign.yes", "Ukončit")}</button>
                    <button class="mini" onClick={() => setAsk(null)}>{t("cards.proj.foreign.no", "Nechat běžet")}</button>
                  </div>
                </Show>
              </>
            )}
          </For>
        </Show>
      </div>
    </Portal>
  );
}
