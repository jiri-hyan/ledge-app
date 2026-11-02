// Nové sekce lišty notche (kolo 27) z EXISTUJÍCÍCH dat, bez nové sítě: Limit Claude (stejný zdroj a prázdný stav
// jako hrdina karty Claude — `layout/hero.ts`, „Limity zatím nejsou“ s důvodem), Počasí (snapshot jako karta
// Počasí) a Příští schůzka (kalendář v paměti jako karta Dnes / Týden). Co zdroj nedal, ukáže prázdný stav.
import { For, Match, Show, Switch } from "solid-js";
import { api, type Snapshot } from "../api";
import { sayError } from "../ui/toast";
import type { CalendarItem, CalendarView } from "../types/calendar";
import { t } from "../i18n";
import { num } from "../format";
import { heroFacts, type HeroFacts } from "../layout/hero";
import { tx } from "../layout/text";
import { ring } from "../charts/geom";
import { tempIn } from "../cards/temp";
import { calState } from "../board/model";
import { countdown } from "../board/weekModel";
import { cdText } from "../board/WeekViews";
import { calColor, hm, providerName } from "../calendar/model";
import { eqLit, nextMeeting, paintOf, solidOf, weatherNow, GRADIENT_CSS } from "./look";
import type { BarSection } from "./bar";

const SEGS = 28;

/** Limit z hrdiny karty Claude; čekající schválení na liště nevadí (tlačítka jsou v kartě, ne tady). */
const limitOf = (s: Snapshot, now: number, pick: "5h" | "week"): HeroFacts | null => heroFacts("claude", { ...s, pending: [], questions: [] }, now, pick);

function Empty(p: { f: HeroFacts }) {
  return (
    <div class="nb-emptybox" role="status">
      <b>{tx(p.f.empty!.title)}</b>
      <small>{tx(p.f.empty!.why)}</small>
    </div>
  );
}

export function Limits(p: { s: Snapshot; now: number; sec: BarSection; design: string }) {
  const five = () => limitOf(p.s, p.now, "5h");
  const week = () => limitOf(p.s, p.now, "week");
  const fill = (pct: number) => {
    const x = paintOf(p.sec.color ?? "load", pct);
    return x.kind === "gradient" ? GRADIENT_CSS : x.css;
  };
  /** Segmentový pruh („čárky“): rozsvícené podle procent, barva podle procent (výchozí zátěž) — reset jen, když ho zdroj zná. */
  const Bar = (q: { f: HeroFacts | null; name: string }) => (
    <Show when={q.f && !q.f.empty ? q.f : null}>
      {(f) => (
        <div class="nb-lim">
          <div class="nb-limhead">
            <b>{q.name}</b>
            <Show when={f().sub}>{(sub) => <small>{tx(sub().text, sub().vars)}</small>}</Show>
            <b class="nb-limv" style={{ color: solidOf(p.sec.color ?? "load", f().value) }}>{f().value} %</b>
          </div>
          <span class="nb-segs" aria-hidden="true">
            <For each={Array.from({ length: SEGS }, (_, i) => i)}>{(i) => <i style={{ background: i < eqLit(f().value, SEGS) ? fill(f().value) : undefined }} />}</For>
          </span>
        </div>
      )}
    </Show>
  );
  return (
    <section class="nb-sec nb-limits" data-design={p.design} aria-label={t("cards.notch.sec.limits", "Limit Claude")}>
      <Show when={five()} fallback={<small class="nb-empty">{t("cards.notch.limitsBusy", "Limity teď nejdou ukázat.")}</small>}>
        {(f) => (
          <Show when={!f().empty} fallback={<Empty f={f()} />}>
            <Switch>
              <Match when={p.design === "ring"}>
                <div class="nb-limring">
                  <svg class="nb-ring" viewBox="0 0 52 52" aria-hidden="true">
                    <circle cx="26" cy="26" r={ring(1, 52, 5)!.r} fill="none" class="nb-track" stroke-width="5" />
                    <circle cx="26" cy="26" r={ring(f().value / 100, 52, 5)!.r} fill="none" style={{ stroke: solidOf(p.sec.color ?? "load", f().value) }} stroke-width="5" stroke-dasharray={ring(f().value / 100, 52, 5)!.dash} stroke-linecap="round" transform="rotate(-90 26 26)" />
                    <text x="26" y="30" text-anchor="middle" class="nb-ringv">{f().value}</text>
                  </svg>
                  <div class="nb-dcol">
                    <b>{tx(f().label)}</b>
                    <Show when={f().sub}>{(sub) => <small>{tx(sub().text, sub().vars)}</small>}</Show>
                  </div>
                </div>
              </Match>
              <Match when={true}>
                <Bar f={f()} name={t("cards.notch.lim5h", "5 h")} />
                <Bar f={week()} name={t("cards.notch.limWeek", "Týden")} />
              </Match>
            </Switch>
          </Show>
        )}
      </Show>
    </section>
  );
}

export function WeatherSec(p: { s: Snapshot; sec: BarSection; design: string }) {
  const w = () => weatherNow(p.s.personal.weather);
  const f = () => p.s.personal.weather?.fahrenheit;
  const deg = (c: number) => `${num(Math.round(tempIn(c, f())))}°`;
  return (
    <section class="nb-sec nb-wx" data-design={p.design} style={p.sec.color ? { "--nb-c": solidOf(p.sec.color) } : undefined} aria-label={t("cards.notch.sec.weather", "Počasí")}>
      <Show
        when={w()}
        fallback={
          <small class="nb-empty">
            {p.s.personal.weather?.status === "unset" ? t("cards.notch.wxUnset", "Místo pro počasí není nastavené (Nastavení → Osobní).") : t("cards.notch.wxNone", "Počasí zatím není.")}
          </small>
        }
      >
        {(x) => (
          <Switch>
            <Match when={p.design === "range"}>
              <div class="nb-wxhead"><span class="nb-wxico" aria-hidden="true">{x().icon}</span><b class="nb-wxt">{deg(x().temp)}</b></div>
              <Show when={x().pos != null} fallback={<small class="nb-by">{x().desc}</small>}>
                <div class="nb-wxrange">
                  <small>{deg(x().min!)}</small>
                  <span class="nb-bar nb-wxbar"><u style={{ left: `${x().pos! * 100}%` }} /></span>
                  <small>{deg(x().max!)}</small>
                </div>
              </Show>
            </Match>
            <Match when={true}>
              <div class="nb-wxhead"><span class="nb-wxico" aria-hidden="true">{x().icon}</span><b class="nb-wxt">{deg(x().temp)}</b></div>
              <small class="nb-by">{[x().desc, x().label].filter(Boolean).join(" · ")}</small>
            </Match>
          </Switch>
        )}
      </Show>
    </section>
  );
}

/** Připojit: ověřený odkaz na schůzku (Rust ho zkontroluje znovu, jako karta Kalendář). */
const join = (url: string) => void api.calendarJoin(url).catch(sayError);

/** Řádek příští schůzky („Nahoře“): proužek barvou kalendáře, odpočet, název, čas · kalendář, Připojit jen s odkazem. */
export function MeetLine(p: { it: CalendarItem; cal: CalendarView | null | undefined; now: number; compact?: boolean }) {
  const info = () => p.cal?.calendars[p.it.cal];
  return (
    <div class="nb-mline" classList={{ "nb-mline-c": p.compact }} style={{ "--nb-cal": calColor(info()?.color, p.it.cal) }}>
      <div class="nb-mtext">
        <Show when={!p.compact}><small class="nb-mcd">{p.it.ongoing || p.it.start <= p.now ? t("cards.notch.now", "Právě teď") : cdText(countdown(p.it.start, p.now))}</small></Show>
        <b class="nb-title nb-mtitle">{p.it.title}</b>
        <small class="nb-by">{p.compact ? `${cdText(countdown(p.it.start, p.now))} · ` : ""}{hm(p.it.start)}–{hm(p.it.end)}<Show when={!p.compact && (info()?.name || p.it.location)}> · {info()?.name || p.it.location}</Show></small>
      </div>
      <Show when={p.it.join}>
        {(j) => (
          <button class="nb-join" title={t("cards.calendar.joinTitle", "Připojit se přes {app}", { app: providerName(j().provider) })} onClick={() => join(j().url)}>
            <svg viewBox="0 0 24 24" aria-hidden="true"><path d="M3 7.5A1.5 1.5 0 0 1 4.5 6h9A1.5 1.5 0 0 1 15 7.5v9a1.5 1.5 0 0 1-1.5 1.5h-9A1.5 1.5 0 0 1 3 16.5zM15 10l6-3.5v11L15 14z" fill="currentColor" /></svg>
            {t("cards.notch.join", "Připojit")}
          </button>
        )}
      </Show>
    </div>
  );
}

/** Prázdný stav kalendáře (žádný zdroj / načítá / nedostupný / volno). */
export function calEmpty(cal: CalendarView | null | undefined): string {
  const st = calState(cal);
  if (st === "none") return t("cards.notch.calNone", "Kalendář není nastavený (Nastavení → Kalendář).");
  if (st === "loading") return t("cards.notch.calLoading", "Načítám kalendář…");
  if (st === "unavailable") return t("cards.notch.calErr", "Kalendář teď není dostupný.");
  return t("cards.notch.calFree", "Dnes ani zítra žádná schůzka.");
}

export function Meeting(p: { s: Snapshot; now: number; sec: BarSection; design: string }) {
  const cal = () => p.s.personal.calendar;
  const it = () => nextMeeting(cal(), p.now);
  return (
    <section class="nb-sec nb-meet" data-design={p.design} style={p.sec.color ? { "--nb-c": solidOf(p.sec.color) } : undefined} aria-label={t("cards.notch.sec.meeting", "Příští schůzka")}>
      <Show when={it()} fallback={<small class="nb-empty">{calEmpty(cal())}</small>}>
        {(m) => (
          <Show when={p.design === "countdown"} fallback={<MeetLine it={m()} cal={cal()} now={p.now} />}>
            <b class="nb-cd">{m().ongoing || m().start <= p.now ? t("cards.notch.now", "Právě teď") : cdText(countdown(m().start, p.now))}</b>
            <small class="nb-title">{m().title}</small>
            <small class="nb-by">{hm(m().start)}–{hm(m().end)}</small>
          </Show>
        )}
      </Show>
    </section>
  );
}
