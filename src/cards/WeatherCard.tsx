// Osobní: počasí z Open-Meteo (stahuje Rust `weather.rs`). Co Rust nemá, karta poctivě řekne
// (nenastaveno / nedostupné / stará data s časem) — nic se nedopočítává.
import { Show, type JSX } from "solid-js";
import { type Snapshot } from "../api";
import { ago, num } from "../format";
import { t } from "../i18n";
import { gotoSettingsPanel } from "../palette/goto";
import "./personal.css";

/** Totéž jako `weather::MAX_AGE_MS` v Rustu. */
const WX_MAX_AGE_MS = 30 * 60 * 1000;

/** `current.time` z Open-Meteo („2026-10-16T07:00“, místní čas místa) → „7:00“, jiný den „15. 10. 7:00“. */
function observedLabel(iso: string, now: number): string {
  const m = /^(\d{4})-(\d{2})-(\d{2})T(\d{2}):(\d{2})/.exec(iso);
  if (!m) return "—";
  const hm = `${+m[4]}:${m[5]}`;
  const d = new Date(now);
  const today = +m[1] === d.getFullYear() && +m[2] === d.getMonth() + 1 && +m[3] === d.getDate();
  return today ? hm : t("cards.weather.observedDay", "{d}. {m}. {time}", { d: +m[3], m: +m[2], time: hm });
}

export default function WeatherCard(p: { s: Snapshot; now: number; onSettings: () => void; menu?: JSX.Element }) {
  const w = () => p.s.personal.weather;
  // Rust posílá stav jen při změně — stáří dopočítat i z tiku, ať „ok“ nezůstane viset jako čerstvé
  const old = () => w().status === "stale" || (w().fetchedMs != null && p.now - w().fetchedMs! >= WX_MAX_AGE_MS);
  return (
    <div class="card" data-anim>
      <h4><span class="ic" style="background:var(--blue)" />{t("cards.weather.title", "Počasí")} <span class="n">{w().label}</span>{p.menu}</h4>
      <Show when={w().data} fallback={
        <Show when={w().status !== "unset"} fallback={
          <div class="empty">{t("cards.weather.unset", "Nastav místo v Nastavení (zeměpisná šířka a délka). Bez něj se nic nestahuje.")} <button class="mini" onClick={() => { p.onSettings(); gotoSettingsPanel("f-personal.weather"); }}>{t("cards.weather.setPlace", "Nastavit místo…")}</button></div>
        }>
          <Show when={w().status === "unavailable"} fallback={<div class="empty">{w().loading ? t("cards.weather.loading", "Načítám počasí…") : t("cards.weather.notYet", "Počasí zatím nenačteno — stáhne se při otevření.")}</div>}>
            <div class="empty">{t("cards.weather.unavailable", "Počasí je nedostupné.")}</div>
            <Show when={w().error}>{(e) => <div class="sub err">{e()}</div>}</Show>
          </Show>
        </Show>
      }>
        {(d) => (
          <>
            <div class="wx">
              <span class="wx-ico" aria-hidden="true">{d().icon}</span>
              <span class="wx-temp">{num(d().temp)}°</span>
              <div class="grow">
                <div class="name">{d().desc}</div>
                <div class="sub">
                  <Show when={d().min != null && d().max != null} fallback={t("cards.weather.noToday", "dnešek bez údajů")}>{num(d().min!)}° / {num(d().max!)}°</Show>
                  <Show when={d().precipSum != null}> · {num(d().precipSum!, 1)} mm</Show>
                  <Show when={d().precipProb != null}>{t("cards.weather.precipProb", " · srážky {p} %", { p: num(d().precipProb!) })}</Show>
                </div>
              </div>
            </div>
            <div class="wx-foot">
              <Show when={old()}>
                <span class="pill a" title={w().error ?? t("cards.weather.oldTitle", "Data jsou starší než 30 minut.")}>{w().error ? t("cards.weather.unavailableOld", "nedostupné · stará data") : t("cards.weather.old", "stará data")}</span>
              </Show>
              <span class="meta" title={w().fetchedMs ? t("cards.weather.fetchedTitle", "Čas měření je místní čas místa. Staženo {ago}.", { ago: ago(w().fetchedMs!, p.now) }) : ""}>
                {w().loading ? t("cards.weather.refreshing", "obnovuji… · ") : ""}{t("cards.weather.measured", "měřeno {at}", { at: observedLabel(d().observed, p.now) })}
              </span>
            </div>
          </>
        )}
      </Show>
    </div>
  );
}
