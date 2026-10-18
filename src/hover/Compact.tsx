// Kompaktní náhled po najetí (`drawer.hover: "compact"`, Rust `View::Compact`): nahoře čekající schválení
// nebo otázka (stejné UI jako peek — `cards/Peek.tsx` `Approval`), session Clauda (nejvýš 3: název, stav, čas),
// Právě hraje (obal, název, ⏮⏯⏭ z `media/Player.tsx`), servery z Ledge se ■ Stop (proužek sám kliknout
// nejde — je průchozí; po najetí na něj se zastaví tady), dole čas, limit 5h a „▾ celý šuplík“. Jen data ze
// snapshotu (nic se nestahuje). Klik mimo tlačítka nebo setrvání (`useExpand`) = celý šuplík.
import { For, Show } from "solid-js";
import { api, type SessionView, type Snapshot } from "../api";
import { Approval } from "../cards/Peek";
import { Controls, Cover, useCover } from "../media/Player";
import { ago, fmtTime, uptime } from "../format";
import { pctText, level, winState } from "../limits/model";
import { t } from "../i18n";
import { compactPlan } from "./model";
import { useExpand } from "./expand";
import { isLive } from "../projects/model";
import { stopProject } from "../projects/state";
import "../media/media.css";
import "./hover.css";

/** stav → popisek (stejné klíče jako karta Claude, `ClaudeSessions.tsx`) */
const STATUS: Record<SessionView["status"], () => string> = {
  waiting: () => t("sessions.status.waiting", "čeká na tebe"),
  working: () => t("sessions.status.working", "pracuje"),
  idle: () => t("sessions.status.idle", "nečinná"),
  ended: () => t("sessions.status.ended", "skončila"),
};

const lastSeg = (p: string) => p.split(/[\\/]/).filter(Boolean).pop() ?? p;

/** „projekt · název“ (bez opakování, když se shodují). */
export const sessionLabel = (ss: SessionView) => {
  const proj = ss.project ?? lastSeg(ss.cwd);
  return proj && proj !== ss.name ? `${proj} · ${ss.name}` : ss.name;
};

/** Čas u session: nečinná = před kolika, jinak jak dlouho ve stavu; neznámé = nic. */
export const sessionTime = (ss: SessionView, now: number) =>
  ss.status === "idle" ? ago(ss.since ?? ss.updatedAt, now) : ss.since ? uptime(ss.since, now) : "";

/** Limit 5h jako „5h 42 %“; bez vzorku nic. */
export function Limit5h(p: { s: Snapshot; now: number }) {
  const st = () => {
    const l = p.s.limits;
    return l ? winState(l.fiveHour, l.at, p.now, l.staleMs) : { kind: "none" as const };
  };
  return (
    <Show when={(() => { const x = st(); return x.kind === "fresh" || x.kind === "stale" ? x : null; })()}>
      {(x) => (
        <span class="hov-lim" classList={{ "hov-lim-warn": level(x().pct) === "warn", "hov-lim-bad": level(x().pct) === "bad", "hov-lim-old": x().kind === "stale" }} title={t("cards.hover.limitHint", "Limit Clauda za 5 hodin")}>
          {t("cards.hover.limit5h", "5h {pct} %", { pct: pctText(x().pct) })}
        </span>
      )}
    </Show>
  );
}

export default function Compact(p: { s: Snapshot; now: number; expandMs: number }) {
  const plan = () => compactPlan(p.s);
  const cover = useCover(() => plan().media?.coverKey ?? null);
  const ex = useExpand(() => p.expandMs, () => plan().approval);
  return (
    <div class="hov" {...ex}>
      <Show when={plan().approval}>
        <div class="hov-approval"><Approval s={p.s} stack /></div>
      </Show>
      <For each={plan().sessions}>
        {(ss) => (
          <div class="hov-row" data-anim>
            <span class="hov-dot" classList={{ "hov-dot-run": ss.status === "working", "hov-dot-wait": ss.status === "waiting", "hov-pulse": ss.status === "working" || ss.status === "waiting" }} />
            <span class="grow">{sessionLabel(ss)}</span>
            <span class="pill" classList={{ a: ss.status === "waiting", v: ss.status === "working", n: ss.status === "idle", r: ss.status === "ended" }}>{STATUS[ss.status]()}</span>
            <span class="meta">{sessionTime(ss, p.now)}</span>
          </div>
        )}
      </For>
      <Show when={plan().media}>
        {(m) => (
          <div class="hov-row hov-media" data-anim>
            <Cover url={cover()} size="sm" />
            <span class="grow">
              <span class="hov-title">{m().title || t("cards.hover.unknownTrack", "Neznámá skladba")}</span>
              <Show when={m().artist}><span class="hov-sub">{m().artist}</span></Show>
            </span>
            <Controls m={m()} tight />
          </div>
        )}
      </Show>
      <For each={plan().servers}>
        {(pc) => {
          const name = () => p.s.projects.find((x) => x.id === pc.id)?.name ?? pc.id;
          return (
            <div class="hov-row" data-anim>
              <span class="hov-dot" classList={{ "hov-dot-ok": pc.phase === "ready" || pc.phase === "running", "hov-dot-wait": pc.phase === "starting" || pc.phase === "preparing", "hov-dot-bad": pc.phase === "failed" }} />
              <span class="grow">{name()}</span>
              <Show when={pc.phase === "failed"} fallback={<span class="meta">{pc.port ? `:${pc.port}` : uptime(pc.readyAt ?? pc.startedAt, p.now)}</span>}>
                <span class="pill r">{t("cards.running.failed", "pád")}</span>
              </Show>
              <Show when={isLive(pc.phase)}>
                <button class="hov-stop" title={t("cards.proj.stopOf", "Zastavit {name}", { name: name() })} aria-label={t("cards.proj.stopOf", "Zastavit {name}", { name: name() })} onClick={() => void stopProject(pc.id, name())}>■</button>
              </Show>
            </div>
          );
        }}
      </For>
      <Show when={!plan().approval && !plan().sessions.length && !plan().media && !plan().servers.length}>
        <div class="hov-empty" data-anim>{t("cards.hover.calm", "Nic se neděje.")}</div>
      </Show>
      <div class="hov-foot" data-anim>
        <span class="hov-clock">{fmtTime(new Date(p.now))}</span>
        <Limit5h s={p.s} now={p.now} />
        <button class="hov-more" onClick={() => void api.expandDrawer()}>{t("cards.hover.more", "▾ celý šuplík")}</button>
      </div>
    </div>
  );
}
