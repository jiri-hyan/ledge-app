// Limity Claude 5h / týden v kartě Claude: pruh, procenta, čas resetu (místní) a odpočet. Data jen
// z `Snapshot.limits` (Rust `limitslive.rs`: stavový řádek Ledge nebo `usage-log/latest.json`).
// Bez dat „—“; starý vzorek „staré (před X min)“; po resetu procenta neznáme → „obnoveno“.
import { For, Show } from "solid-js";
import type { LimitsView, Snapshot } from "../api";
import { dayNames, fmtTime } from "../format";
import { t } from "../i18n";
import { bar, left, level, otherDay, pctText, winState, type LimitWindow } from "./model";
import "./limits.css";
import { gotoSettingsPanel } from "../palette/goto";

/** „za 2 h 10 min“ / „za 3 d 4 h“ / „za 5 min“ */
export function leftText(resetsAt: number | null | undefined, now: number): string {
  const l = left(resetsAt, now);
  if (!l) return "";
  if (l.d > 0) return l.h > 0 ? t("sessions.limits.inDH", "za {d} d {h} h", l) : t("sessions.limits.inD", "za {d} d", l);
  if (l.h > 0) return l.m > 0 ? t("sessions.limits.inHM", "za {h} h {m} min", l) : t("sessions.limits.inH", "za {h} h", l);
  return t("sessions.limits.inM", "za {m} min", l);
}

/** Čas resetu místně: „14:30“, jiný den „pátek 14:30“. */
export function resetClock(ms: number, now: number): string {
  const d = new Date(ms);
  return otherDay(ms, now) ? `${dayNames()[d.getDay()]} ${fmtTime(d)}` : fmtTime(d);
}

function LimitBar(p: { label: string; w: LimitWindow | null; lv: LimitsView; now: number }) {
  const st = () => winState(p.w, p.lv.at, p.now, p.lv.staleMs);
  const pct = () => {
    const s = st();
    return s.kind === "fresh" || s.kind === "stale" ? s.pct : null;
  };
  const tip = () => {
    const r = p.w?.resetsAt;
    return r != null && st().kind !== "reset" ? t("sessions.limits.resetsTip", "Reset {when}", { when: resetClock(r, p.now) }) : undefined;
  };
  return (
    <div class="lim-row" classList={{ "lim-old": st().kind === "stale" }} title={tip()}>
      <span class="lim-name">{p.label}</span>
      <span class="lim-track">
        <Show when={pct() !== null}>
          <span class="lim-fill" classList={{ "lim-ok": level(pct()!) === "ok", "lim-warn": level(pct()!) === "warn", "lim-bad": level(pct()!) === "bad" }} style={{ width: `${bar(pct()!)}%` }} />
        </Show>
      </span>
      <span class="lim-pct">{pct() !== null ? `${pctText(pct()!)} %` : st().kind === "reset" ? t("sessions.limits.reset", "obnoveno") : "—"}</span>
      <span class="lim-when">
        <Show when={st().kind === "stale"} fallback={
          <Show when={p.w?.resetsAt != null && st().kind === "fresh"}>
            {resetClock(p.w!.resetsAt!, p.now)} · {leftText(p.w!.resetsAt, p.now)}
          </Show>
        }>
          {t("sessions.limits.stale", "staré (před {n} min)", { n: (st() as { ageMin: number }).ageMin })}
        </Show>
      </span>
    </div>
  );
}

export default function LimitsRow(p: { s: Snapshot; now: number; onSettings: () => void }) {
  const lv = () => p.s.limits;
  const rows = () => {
    const l = lv();
    return l ? [{ label: "5h", w: l.fiveHour }, { label: t("sessions.limits.week", "týden"), w: l.sevenDay }] : [];
  };
  return (
    <Show when={lv()}>
      {(l) => (
        <Show when={l().at != null} fallback={
          <Show when={l().statusLine === "none" || l().statusLine === "foreign"} fallback={
            <Show when={l().statusLine === "full"}>
              <div class="lim-empty">{t("sessions.limits.waiting", "Limity 5h / týden: — (čekám na první odpověď Clauda)")}</div>
            </Show>
          }>
            <button class="hint-link" onClick={() => { p.onSettings(); gotoSettingsPanel("f-claude.statusLine"); }}>{t("sessions.limits.connect", "Zapojit stavový řádek pro limity 5h / týden")}</button>
          </Show>
        }>
          <div class="lim">
            <For each={rows()}>{(r) => <LimitBar label={r.label} w={r.w} lv={l()} now={p.now} />}</For>
          </div>
        </Show>
      )}
    </Show>
  );
}
