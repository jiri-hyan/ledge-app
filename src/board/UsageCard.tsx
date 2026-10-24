// Karta „Claude dnes“ (kolo 22, id `usage`): útrata a tokeny dnes a za 7 dní jako sloupce + časová osa dneška
// po hodinách a projektech (co Claude dnes dělal). Zdroj = tentýž přehled jako sekce útraty v kartě Claude
// (`costslive` nad lokálními transcripty, ODHAD v cenách API — štítek „odhad“). Přepočet se žádá jen s kartou
// na otevřené záložce a otevřeným šuplíkem (Rust ho pustí nejvýš 1× / 60 s; na ploše 1× / 15 min sám).
import { createEffect, For, on, Show, type JSX } from "solid-js";
import { api, type Snapshot } from "../api";
import { ago } from "../format";
import { t } from "../i18n";
import { tok, usd } from "../costs/CostsSection";
import { progressPct } from "../costs/model";
import { barHeights, heat, hourRows, hourSpan, hoursMax, isToday, lastDays, sumDays, usageState } from "./model";
import { originText } from "./text";
import Bars from "../charts/Bars";
import "./board.css";

export function OriginTag(p: { o: "official" | "derived" }) {
  const x = () => originText(p.o);
  return <span class="bd-origin" classList={{ "bd-origin-off": p.o === "official" }} title={x().tip}>{x().text}</span>;
}

export default function UsageCard(p: { s: Snapshot; now: number; menu?: JSX.Element; onSettings?: () => void; desk?: boolean }) {
  const cv = () => p.s.costs;
  // v šuplíku: přepočet při zobrazení (plocha si ho žádá v Rustu sama, z okna widgetu se nevolá nic)
  createEffect(on(() => p.s.view, (v) => {
    if (!p.desk && v === "full") void api.costsRefresh().catch(() => {});
  }));
  const state = () => usageState(cv());
  const week = () => lastDays(cv()?.summary?.days, 7);
  const today = () => week()[week().length - 1] ?? null;
  const fresh = () => isToday(cv(), p.now);
  const weekSum = () => sumDays(week());
  const heights = () => barHeights(week());
  const rows = () => (fresh() ? hourRows(cv()) : []);
  const max = () => hoursMax(rows());
  const span = () => hourSpan(rows(), new Date(p.now).getHours());
  const hours = () => {
    const [a, b] = span();
    return Array.from({ length: b - a + 1 }, (_, i) => a + i);
  };
  const pct = () => progressPct(cv()?.progress);
  const name = (r: { name: string; other: boolean }) => (r.other ? t("sessions.costs.other", "ostatní") : r.name || t("sessions.costs.noDir", "bez složky"));
  return (
    <div class="card bd-usage" data-anim>
      <h4>
        <span class="ic" style="background:var(--violet)" />
        {t("cards.board.usage.title", "Claude dnes")}
        <Show when={state() === "ok"}><OriginTag o="derived" /></Show>
        {p.menu}
      </h4>
      <Show when={state() !== "off"} fallback={
        <div class="empty">
          {t("cards.board.usage.off", "Útrata Claude je vypnutá (Nastavení → Claude → Útrata).")}
          <Show when={p.onSettings}><button class="mini" onClick={() => p.onSettings?.()}>{t("cards.board.settings", "Nastavení")}</button></Show>
        </div>
      }>
        <Show when={state() === "ok"} fallback={
          <div class="empty">
            {state() === "error"
              ? cv()?.error ?? t("cards.board.usage.error", "Přepočet se nepovedl.")
              : pct() != null ? t("sessions.costs.computingPct", "počítám… {n} %", { n: pct()! }) : t("sessions.costs.computing", "počítám…")}
          </div>
        }>
          <div class="bd-hero">
            <div class="bd-hero-main">
              <span class="bd-big" title={t("sessions.costs.tip", "Odhad podle ceníku API — při předplatném ji neplatíš.")}>{fresh() ? usd(today()?.cost) : "—"}</span>
              <span class="bd-sub">{fresh() ? t("cards.board.usage.todayTok", "dnes · {n} tokenů", { n: tok(today()?.tokens ?? 0) }) : t("cards.board.usage.stale", "dnešek ještě nepočítán")}</span>
            </div>
            <div class="bd-hero-side">
              <span class="bd-mid">{usd(weekSum().cost)}</span>
              <span class="bd-sub">{t("cards.board.usage.weekTok", "7 dní · {n}", { n: tok(weekSum().tokens) })}</span>
            </div>
          </div>
          <div class="bd-bars">
            <Bars
              values={heights()}
              max={1}
              h={32}
              fluid
              hi={fresh() ? week().length - 1 : -99}
              label={t("sessions.costs.barsLabel", "Útrata po dnech")}
              title={(i) => { const d = week()[i]; return d ? `${d.date}: ${usd(d.cost)} · ${tok(d.tokens)}` : ""; }}
            />
          </div>
          <Show when={fresh()}>
            <div class="bd-h">{t("cards.board.usage.timeline", "Co Claude dnes dělal")}</div>
            <Show when={rows().length} fallback={<div class="bd-note">{t("cards.board.usage.none", "Dnes zatím žádná odpověď Clauda.")}</div>}>
              <div class="bd-tl" style={{ "--bd-cols": String(hours().length) }}>
                <For each={rows()}>
                  {(r) => (
                    <div class="bd-tl-row">
                      <span class="bd-tl-name" title={name(r)}>{name(r)}</span>
                      <span class="bd-tl-cells">
                        <For each={hours()}>
                          {(h) => (
                            <i class="bd-cell" data-l={heat(r.tokens[h], max())} title={t("cards.board.usage.cell", "{h}:00 · {n} tokenů", { h, n: tok(r.tokens[h]) })} />
                          )}
                        </For>
                      </span>
                    </div>
                  )}
                </For>
                <div class="bd-tl-row bd-tl-axis">
                  <span class="bd-tl-name" />
                  <span class="bd-tl-cells">
                    <span>{span()[0]}:00</span>
                    <span>{span()[1]}:00</span>
                  </span>
                </div>
              </div>
            </Show>
          </Show>
          <Show when={cv()?.computedAt}>
            <div class="bd-foot">
              {t("sessions.costs.computed", "spočítáno {when}", { when: ago(cv()!.computedAt!, p.now) })}
              <Show when={state() === "ok" && cv()?.state === "computing"}> · {t("sessions.costs.computing", "počítám…")}</Show>
            </div>
          </Show>
        </Show>
      </Show>
    </div>
  );
}
