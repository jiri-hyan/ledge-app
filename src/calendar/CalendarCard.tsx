// Osobní: karta Kalendář — Dnes / Zítra (nebo nejbližší další den) z odkazů ICS. Data stahuje a vybírá
// Rust (`calendar/live.rs`, `view.rs`); karta jen kreslí. Co Rust nemá, karta poctivě řekne (žádný
// kalendář / načítám / nedostupné / stará data) — nic se nevymýšlí. Kalendář jde jedním klikem vypnout
// (a zase zapnout) přímo v patičce karty; adresa se nikde neukazuje, jen název. U schůzky s odkazem
// (Teams, Meet, Zoom, Webex, Jitsi — Rust `calendar/meeting.rs`) tlačítko Připojit; Outlook v tomto PC
// se zavřeným Outlookem nabídne „Načíst z Outlooku“ (jen na klik — Ledge ho sám nespouští).
import { createSignal, For, Show, type JSX } from "solid-js";
import { api, type CalendarDay, type CalendarItem, type Snapshot } from "../api";
import { ago, fmtDate } from "../format";
import { t } from "../i18n";
import { settingsPath } from "../settings/path";
import { gotoSettingsPanel } from "../palette/goto";
import { say, sayError } from "../ui/toast";
import { avatarRow, calColor, joinable, meetingSpan, minLabel, oldestFetchedMs, placeLabel, providerName, timeLabel, withCalendar } from "./model";
import "./calendar.css";

/** Nastavení → Obsah karet → Kalendáře (přidání, úpravy). */
export function openCalendarSettings(onSettings: () => void) {
  onSettings();
  gotoSettingsPanel("f-personal.calendars");
}

/** w8: Q — zdroj z účtu (Microsoft / Google) se vypíná v Nastavení → Obsah karet → Pokročilé, ne v `personal.calendars`. */
function openAccountSettings(onSettings: () => void, account: "microsoft" | "google") {
  onSettings();
  gotoSettingsPanel(account === "microsoft" ? "f-accounts.microsoft" : "f-accounts.google");
}

/** Zapne / vypne jeden kalendář (uloží jen to pole). S chybou configu neukládá — přepsal by se. */
async function setEnabled(i: number, on: boolean, name: string, cfgError: string | null) {
  if (cfgError) {
    say(t("cards.calendar.cfgError", "Nastavení obsahuje chybu — kalendář teď vypneš jen v Nastavení."), true);
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

/** Připojit: otevře ověřený odkaz na schůzku (Rust ho zkontroluje znovu). */
async function join(url: string) {
  try {
    await api.calendarJoin(url);
  } catch (e) {
    sayError(e);
  }
}

/** Načíst z Outlooku (výslovný klik — smí Outlook spustit na pozadí). */
async function loadOutlook(setBusy: (b: boolean) => void) {
  setBusy(true);
  try {
    const n = await api.calendarOutlookLoad();
    say(t("cards.calendar.outlookLoaded", "Načteno z Outlooku — nadcházejících událostí: {n}.", { n: n ?? 0 }));
  } catch (e) {
    sayError(e);
  } finally {
    setBusy(false);
  }
}

function Row(p: { it: CalendarItem; day: string; color: string; now: number }) {
  const time = () => timeLabel(p.it, p.day);
  const place = () => placeLabel(p.it);
  /** délka: před schůzkou „za 8 min · 30 min“ (do hodiny), během „běží 35 min · zbývá 12 min“; z hodin aplikace, bez časovače */
  const span = () => meetingSpan(p.it, p.now);
  const crowd = () => avatarRow(p.it.people);
  const running = () => {
    const s = span();
    return s?.phase === "running" ? s : null;
  };
  const before = () => {
    const s = span();
    return s?.phase === "before" ? s : null;
  };
  return (
    <div class="row cal-row">
      <span class="cal-dot" style={{ background: p.color }} aria-hidden="true" />
      <span class="cal-time" classList={{ "cal-time-now": p.it.ongoing }} title={p.it.tzGuess ? t("cards.calendar.tzGuess", "Časové pásmo události Ledge nezná — čas je brán jako místní.") : undefined}>
        {time() ?? t("cards.calendar.allDay", "celý den")}
        <Show when={p.it.tzGuess}>≈</Show>
      </span>
      <div class="grow">
        <div class="name" classList={{ "cal-now": p.it.ongoing }}>{p.it.title || t("cards.calendar.untitled", "(bez názvu)")}</div>
        <Show when={place()}><div class="sub" title={place()}>{place()}</div></Show>
        <Show when={running() || crowd().shown.length > 0}>
          <div class="cal-span">
            <Show when={running()}>{(r) => <span class="cal-span-run">{t("cards.calendar.spanRunning", "běží {run} · zbývá {left}", { run: minLabel(r().run), left: minLabel(r().left) })}</span>}</Show>
            <Show when={crowd().shown.length > 0}>
              <span class="cal-people" title={crowd().names}>
                <For each={crowd().shown}>{(a) => <span class="cal-av" classList={{ "cal-av-tent": a.tentative }} style={{ background: a.color }} aria-label={a.label}>{a.initials}</span>}</For>
                <Show when={crowd().more > 0}><span class="cal-av cal-av-more">{t("cards.calendar.morePeople", "+{n}", { n: crowd().more })}</span></Show>
              </span>
            </Show>
          </div>
        </Show>
      </div>
      <Show when={before()}>{(b) => <span class="pill b">{t("cards.calendar.spanBefore", "za {in} · {total}", { in: minLabel(b().startsIn), total: minLabel(b().total) })}</span>}</Show>
      <Show when={p.it.ongoing}><span class="pill g">{t("cards.calendar.ongoing", "teď")}</span></Show>
      <Show when={joinable(p.it, p.now) ? p.it.join : null}>
        {(j) => (
          <button class="mini ok cal-join" title={t("cards.calendar.joinTitle", "Připojit se přes {app}", { app: providerName(j().provider) })} onClick={() => join(j().url)}>
            {t("cards.calendar.join", "Připojit")}
          </button>
        )}
      </Show>
    </div>
  );
}

/** `readOnly` = okno widgetu na ploše: config tam zapisovat nesmí (capability `desk`), klik na kalendář otevře Nastavení. */
export default function CalendarCard(p: { s: Snapshot; now: number; onSettings: () => void; menu?: JSX.Element; readOnly?: boolean }) {
  const v = () => p.s.personal.calendar;
  const cals = () => v()?.calendars ?? [];
  const color = (i: number) => calColor(cals()[i]?.color, i);
  const days = () => v()?.days ?? [];
  const status = () => v()?.status ?? "unset";
  const fetched = () => cals().some((c) => c.enabled && c.fetchedMs != null);
  const problems = () => cals().filter((c) => c.enabled && c.error);
  const add = () => openCalendarSettings(p.onSettings);
  const [olBusy, setOlBusy] = createSignal(false);
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
            <Show when={status() === "unavailable"}>
              <Show when={problems()[0]?.error}>{(e) => <div class="sub err">{e()}</div>}</Show>
              <Show when={!p.readOnly}><button class="mini" onClick={() => void api.retryOnline().catch(sayError)}>{t("ui.boundary.retry", "Zkusit znovu")}</button></Show>
            </Show>
          </div>
        }>
          <Show when={lead()}>{(l) => <div class="cal-lead">{l()}</div>}</Show>
          <Show when={days().length > 0} fallback={<div class="empty">{t("cards.calendar.nothing", "Dnes ani v příštích dvou týdnech nic.")}</div>}>
            <div class="cal-days">
              <For each={days()}>
                {(d) => (
                  <div class="cal-day">
                    <div class="cal-day-h">{dayTitle(d)}</div>
                    <For each={d.items}>{(it) => <Row it={it} day={d.date} color={color(it.cal)} now={p.now} />}</For>
                  </div>
                )}
              </For>
            </div>
          </Show>
          <Show when={(v()?.more ?? 0) > 0}><div class="cal-more">{t("cards.calendar.more", "+ další {n}", { n: v()!.more })}</div></Show>
        </Show>
        <For each={problems()}>
          {(c) => (
            <div class="cal-problem">
              <span class="sub err">{t("cards.calendar.problem", "{name}: {error}", { name: c.name, error: c.error! })}</span>
              <Show when={c.hint === "outlook-closed"}>
                <button class="mini" disabled={olBusy() || v()?.loading} onClick={() => loadOutlook(setOlBusy)}>
                  {olBusy() ? t("cards.calendar.outlookLoading", "Načítám z Outlooku…") : t("cards.calendar.outlookLoad", "Načíst z Outlooku")}
                </button>
              </Show>
            </div>
          )}
        </For>
        <div class="cal-foot">
          <For each={cals()}>
            {(c, i) => (
              <button
                class="cal-chip"
                classList={{ "cal-chip-off": !c.enabled, "cal-chip-bad": !!c.error }}
                title={p.readOnly
                  ? t("cards.calendar.chipDesk", "{name} — zapneš nebo vypneš v Nastavení Ledge", { name: c.name })
                  : c.account
                  ? t("cards.calendar.chipAccount", "{name} ({domain}) — účet nastavíš v {path}", { name: c.name, domain: c.domain, path: settingsPath("osobni", "calendars", "accounts") })
                  : c.enabled
                  ? c.domain
                    ? t("cards.calendar.chipOff", "{name} ({domain}) — kliknutím vypneš", { name: c.name, domain: c.domain })
                    : t("cards.calendar.chipOffLocal", "{name} — kliknutím vypneš", { name: c.name })
                  : t("cards.calendar.chipOn", "{name} je vypnutý — kliknutím zapneš", { name: c.name })}
                onClick={() => (p.readOnly ? p.onSettings() : c.account ? openAccountSettings(p.onSettings, c.account) : setEnabled(i(), !c.enabled, c.name, p.s.cfgError))}
              >
                <span class="cal-dot" style={{ background: color(i()) }} />
                {c.name}
              </button>
            )}
          </For>
          <Show when={status() === "stale" && fetched()}>
            <span class="pill a" title={problems().length ? "" : t("cards.calendar.oldTitle", "Data jsou starší než 15 minut.")}>{t("cards.calendar.old", "stará data")}</span>
          </Show>
          <Show when={oldestFetchedMs(cals())}>
            {(f) => <span class="cal-meta">{v()?.loading ? t("cards.calendar.refreshing", "obnovuji…") : t("cards.calendar.fetched", "staženo {ago}", { ago: ago(f(), p.now) })}</span>}
          </Show>
        </div>
      </Show>
    </div>
  );
}
