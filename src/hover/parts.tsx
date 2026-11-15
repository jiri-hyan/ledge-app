// Společné drobnosti náhledů po najetí (ostrůvek `Island.tsx`, dok `Dock.tsx`): popisek a čas session Clauda,
// limit 5h jako „5h 42 %“. Jen data ze snapshotu (nic se nestahuje).
import { Show } from "solid-js";
import type { SessionView, Snapshot } from "../api";
import { ago, uptime } from "../format";
import { pctText, level, winState } from "../limits/model";
import { t } from "../i18n";
import "./hover.css";

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
