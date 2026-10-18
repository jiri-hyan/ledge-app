// Osobní: karta Kalendář — Dnes / Zítra (nebo nejbližší další den) z odkazů ICS. Data stahuje a vybírá
// Rust (`calendar/live.rs`, `view.rs`); karta jen kreslí. Co Rust nemá, karta poctivě řekne (žádný
// kalendář / načítám / nedostupné / stará data) — nic se nevymýšlí. Kalendář jde jedním klikem vypnout
// (a zase zapnout) přímo v patičce karty; adresa se nikde neukazuje, jen název.
import { For, Show, type JSX } from "solid-js";
import { api, type CalendarDay, type CalendarItem, type Snapshot } from "../api";
import { ago, fmtDate } from "../format";
import { t } from "../i18n";
import { gotoSettingsPanel } from "../palette/goto";
import { say, sayError } from "../ui/toast";
import { calColor, timeLabel, withCalendar } from "./model";
import "./calendar.css";

/** Nastavení → Osobní → Kalendáře (přidání, úpravy). */
export function openCalendarSettings(onSettings: () => void) {
  onSettings();
  gotoSettingsPanel("osobni");
  requestAnimationFrame(() => requestAnimationFrame(() => document.getElementById("f-personal.calendars")?.scrollIntoView({ block: "start" })));
}

/** Zapne / vypne jeden kalendář (uloží jen to pole). S chybou configu neukládá — přepsal by se. */
async function setEnabled(i: number, on: boolean, name: string, cfgError: string | null) {
  if (cfgError) {
    say(t("cards.calendar.cfgError", "Config má chybu — kalendář teď vypneš jen v Nastavení."), true);
    return;
  }
  try {
    const cfg = await api.getConfig();
    if (!cfg) return;
    await api.saveConfig(withCalendar(cfg, i, { enabled: on }));
    if (!on) say(t("cards.calendar.offDone", "Kalendář {n} vypnutý — zapneš ho klikem dole v kartě.", { n: name }));
  } catch (e) {
    sayError(e);
  }
}

function dayTitle(d: CalendarDay): string {
  if (d.kind === "today") return t("cards.calendar.today", "Dnes");
  if (d.kind === "tomorrow") return t("cards.calendar.tomorrow", "Zítra");
  const [y, m, day] = d.date.split("-").map(Number);
  return t("cards.calendar.later", "Příští · {date}", { date: fmtDate(new Date(y, m - 1, day)) });
}

function Row(p: { it: CalendarItem; day: string; color: string }) {
  const time = () => timeLabel(p.it, p.day);
  return (
    <div class="row cal-row">
      <span class="cal-dot" style={{ background: p.color }} aria-hidden="true" />
      <span class="cal-time" classList={{ "cal-time-now": p.it.ongoing }} title={p.it.tzGuess ? t("cards.calendar.tzGuess", "Časové pásmo události Ledge nezná — čas je brán jako místní.") : undefined}>
        {time() ?? t("cards.calendar.allDay", "celý den")}
        <Show when={p.it.tzGuess}>≈</Show>
      </span>
      <div class="grow">
        <div class="name" classList={{ "cal-now": p.it.ongoing }}>{p.it.title || t("cards.calendar.untitled", "(bez názvu)")}</div>
        <Show when={p.it.location}><div class="sub" title={p.it.location}>{p.it.location}</div></Show>
      </div>
      <Show when={p.it.ongoing}><span class="pill g">{t("cards.calendar.ongoing", "teď")}</span></Show>
    </div>
  );
}

export default function CalendarCard(p: { s: Snapshot; now: number; onSettings: () => void; menu?: JSX.Element }) {
  const v = () => p.s.personal.calendar;
  const cals = () => v()?.calendars ?? [];
  const color = (i: number) => calColor(cals()[i]?.color, i);
  const days = () => v()?.days ?? [];
  const status = () => v()?.status ?? "unset";
  const fetched = () => cals().some((c) => c.enabled && c.fetchedMs != null);
  const problems = () => cals().filter((c) => c.enabled && c.error);
  const add = () => openCalendarSettings(p.onSettings);
  /** proč chybí dnešek: „Dnes už nic.“ / „Dnes ani zítra nic.“ */
  const lead = () => {
    const d = days();
    if (!d.length || d[0].kind === "today") return null;
    return d[0].kind === "tomorrow" ? t("cards.calendar.noToday", "Dnes už nic.") : t("cards.calendar.noTodayTomorrow", "Dnes ani zítra nic.");
  };
  return (
    <div class="card" data-anim>
      <h4><span class="ic" style="background:var(--green)" />{t("cards.calendar.title", "Kalendář")}{p.menu}</h4>
      <Show when={status() !== "unset"} fallback={
        <div class="empty cal-empty">
          {t("cards.calendar.unset", "Uvidíš tu dnešek a zítřek z Google nebo Outlook kalendáře — stačí vložit odkaz.")}
          <button class="mini ok" onClick={add}>{t("cards.calendar.add", "Přidat kalendář")}</button>
        </div>
      }>
        <Show when={fetched()} fallback={
          <div class="empty">
            {status() === "loading" ? t("cards.calendar.loading", "Načítám kalendář…")
              : status() === "unavailable" ? t("cards.calendar.unavailable", "Kalendář se nepodařilo načíst.")
              : t("cards.calendar.notYet", "Kalendář se načte při otevření záložky.")}
          </div>
        }>
          <Show when={lead()}>{(l) => <div class="cal-lead">{l()}</div>}</Show>
          <For each={days()} fallback={<div class="empty">{t("cards.calendar.nothing", "Dnes ani v příštích dvou týdnech nic.")}</div>}>
            {(d) => (
              <div class="cal-day">
                <div class="cal-day-h">{dayTitle(d)}</div>
                <For each={d.items}>{(it) => <Row it={it} day={d.date} color={color(it.cal)} />}</For>
              </div>
            )}
          </For>
          <Show when={(v()?.more ?? 0) > 0}><div class="cal-more">{t("cards.calendar.more", "+ další {n}", { n: v()!.more })}</div></Show>
        </Show>
        <For each={problems()}>{(c) => <div class="sub err">{t("cards.calendar.problem", "{name}: {error}", { name: c.name, error: c.error! })}</div>}</For>
        <div class="cal-foot">
          <For each={cals()}>
            {(c, i) => (
              <button
                class="cal-chip"
                classList={{ "cal-chip-off": !c.enabled, "cal-chip-bad": !!c.error }}
                title={c.enabled
                  ? t("cards.calendar.chipOff", "{name} ({domain}) — kliknutím vypneš", { name: c.name, domain: c.domain })
                  : t("cards.calendar.chipOn", "{name} je vypnutý — kliknutím zapneš", { name: c.name })}
                onClick={() => setEnabled(i(), !c.enabled, c.name, p.s.cfgError)}
              >
                <span class="cal-dot" style={{ background: color(i()) }} />
                {c.name}
              </button>
            )}
          </For>
          <Show when={status() === "stale" && fetched()}>
            <span class="pill a" title={problems().length ? "" : t("cards.calendar.oldTitle", "Data jsou starší než 15 minut.")}>{t("cards.calendar.old", "stará data")}</span>
          </Show>
          <Show when={cals().find((c) => c.fetchedMs)}>
            {(c) => <span class="cal-meta">{v()?.loading ? t("cards.calendar.refreshing", "obnovuji…") : t("cards.calendar.fetched", "staženo {ago}", { ago: ago(c().fetchedMs!, p.now) })}</span>}
          </Show>
        </div>
      </Show>
    </div>
  );
}
