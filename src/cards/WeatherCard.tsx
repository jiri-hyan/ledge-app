// Osobní: počasí z Open-Meteo (stahuje Rust `weather.rs`). Co Rust nemá, karta poctivě řekne
// (nenastaveno / nedostupné / stará data s časem) — nic se nedopočítává. Vzhledy (kolo 25, `weatherDesigns.ts`):
// klasické (výchozí, beze změny), Živá obloha, Teploměr dne, Plakát, Křivka, Pět dní, Oblouk slunce.
import { Match, Show, Switch, type JSX } from "solid-js";
import { type Snapshot } from "../api";
import { ago, num } from "../format";
import { t } from "../i18n";
import { drawerShown } from "../media/shown";
import { gotoSettingsPanel } from "../palette/goto";
import type { Weather } from "../types/personal";
import { tempIn } from "./temp";
import { weatherDesign } from "./weatherDesigns";
import { PosterBody, SkyBg, SkyBody, ThermoBody } from "./weatherViews";
import { ArcBody, CurveBody, WeekBody } from "./weatherViews2";
import "./personal.css";
import "./weather.css";

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

/** Široká karta: vpravo předpověď na další dny (jen když ji zdroj poslal; úzká karta ji schová CSS). */
function Aside(p: { d: Weather; f: boolean | undefined }) {
  return <Show when={(p.d.days?.length ?? 0) > 0}><div class="wx-aside"><WeekBody d={p.d} f={p.f} /></div></Show>;
}

export default function WeatherCard(p: { s: Snapshot; now: number; onSettings: () => void; menu?: JSX.Element; design?: string }) {
  const w = () => p.s.personal.weather;
  const design = () => weatherDesign(p.design);
  // Rust posílá stav jen při změně — stáří dopočítat i z tiku, ať „ok“ nezůstane viset jako čerstvé
  const old = () => w().status === "stale" || (w().fetchedMs != null && p.now - w().fetchedMs! >= WX_MAX_AGE_MS);
  return (
    <div class="card wx-card" data-anim data-wx={design()} classList={{ "wx-live": drawerShown() }}>
      <Show when={design() === "sky" && w().data}>{(d) => <SkyBg code={d().code} isDay={d().isDay} />}</Show>
      <h4><span class="ic" style="background:var(--blue)" />{t("cards.weather.title", "Počasí")} <span class="n">{w().label}</span>{p.menu}</h4>
      <Show when={w().data} fallback={
        <Show when={w().status !== "unset"} fallback={
          <div class="empty">{t("cards.weather.unset", "Vyber město v Nastavení → Šuplík a karty → Obsah karet. Bez něj se nic nestahuje.")} <button class="mini" onClick={() => { p.onSettings(); gotoSettingsPanel("f-personal.weather"); }}>{t("cards.weather.setPlace", "Nastavit místo…")}</button></div>
        }>
          <Show when={w().status === "unavailable"} fallback={<div class="empty">{w().loading ? t("cards.weather.loading", "Načítám počasí…") : t("cards.weather.notYet", "Počasí zatím nenačteno — stáhne se při otevření.")}</div>}>
            <div class="empty">{t("cards.weather.unavailable", "Počasí je nedostupné.")}</div>
            <Show when={w().error}>{(e) => <div class="sub err">{e()}</div>}</Show>
          </Show>
        </Show>
      }>
        {(d) => (
          <>
            <Switch fallback={
              <div class="wx">
                <span class="wx-ico" aria-hidden="true">{d().icon}</span>
                <span class="wx-temp">{num(tempIn(d().temp, w().fahrenheit))}°</span>
                <div class="grow">
                  <div class="name">{d().desc}</div>
                  <div class="sub">
                    <Show when={d().min != null && d().max != null} fallback={t("cards.weather.noToday", "dnešek bez údajů")}>{num(tempIn(d().min!, w().fahrenheit))}° / {num(tempIn(d().max!, w().fahrenheit))}°</Show>
                    <Show when={d().precipSum != null}> · {num(d().precipSum!, 1)} mm</Show>
                    <Show when={d().precipProb != null}>{t("cards.weather.precipProb", " · srážky {p} %", { p: num(d().precipProb!) })}</Show>
                  </div>
                </div>
              </div>
            }>
              <Match when={design() === "sky"}><SkyBody d={d()} f={w().fahrenheit} /></Match>
              <Match when={design() === "thermo"}><ThermoBody d={d()} f={w().fahrenheit} /></Match>
              <Match when={design() === "poster"}><div class="wx-split"><PosterBody d={d()} f={w().fahrenheit} /><Aside d={d()} f={w().fahrenheit} /></div></Match>
              <Match when={design() === "curve"}><CurveBody d={d()} f={w().fahrenheit} /></Match>
              <Match when={design() === "week"}><WeekBody d={d()} f={w().fahrenheit} /></Match>
              <Match when={design() === "arc"}><div class="wx-split"><ArcBody d={d()} f={w().fahrenheit} /><Aside d={d()} f={w().fahrenheit} /></div></Match>
            </Switch>
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
