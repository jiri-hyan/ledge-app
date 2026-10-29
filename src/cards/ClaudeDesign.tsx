// Vzhledy karty Claude (kolo 25, `look.design`, registr `layout/designs.ts`): prstence, Tempomat, hrdina (5 h / týden /
// útrata), Útrata, Sezení, Palivoměr. „Klasický“ je dál `ClaudeSessions.tsx` — MainPage sem pustí jen jiný vzhled a jen
// když na tebe nic nečeká (schválení / otázka mají tlačítka jen v klasické kartě). Data jsou jen skutečná
// (`Snapshot.limits` / `costs` / `sessions`, logika `claudeLooks.ts`); chybí-li, prázdný stav s důvodem, ne jiné číslo.
// Šířka: `container-type` na těle karty, úzká = podstatné, široká = detail (`claudeLooks.css`). Nic nepoběží v klidu.
import LimitsSources from "../limits/LimitsSources";
import { For, Match, Show, Switch, type JSX } from "solid-js";
import { api, type SessionView, type Snapshot } from "../api";
import { ago, fmtClock, num } from "../format";
import { t } from "../i18n";
import { say } from "../ui/toast";
import Bars from "../charts/Bars";
import Gauge from "../charts/Gauge";
import Rings from "../charts/Rings";
import { compact, usdDigits } from "../costs/model";
import { leftText, resetClock } from "../limits/LimitsRow";
import { bar, pctText } from "../limits/model";
import { gotoSettingsPanel } from "../palette/goto";
import HeroCard from "../layout/HeroCard";
import { heroFacts, limitsWhy, type ClaudePick } from "../layout/hero";
import { cardName, tx } from "../layout/text";
import { paceLine, sessionTiles, spendModel, winInfo, type PaceLine, type SpendModel, type WinInfo } from "./claudeLooks";
import "./claudeLooks.css";

const HERO_PICK: Record<string, ClaudePick> = { hero: "5h", heroWeek: "week", heroSpend: "spend" };
/** Vzhledy Claude, které kreslí tahle komponenta (klasický ne). */
export const isClaudeDesign = (id: string | undefined): boolean => !!id && id !== "classic";

type P = { s: Snapshot; now: number; design: string; menu?: JSX.Element; onSettings: () => void };

const money = (v: number, priced: boolean): string => {
  if (priced) return `${num(v, usdDigits(v))} $`;
  const c = compact(v);
  return `${num(c.v, c.digits)}${c.unit} ${t("cards.cd.tokens", "tokenů")}`;
};

/** Čas obnovy okna: „14:30 · za 2 h 10 min“. */
const resetText = (i: WinInfo, now: number): string => (i.resetsAt != null && i.kind !== "reset" ? `${resetClock(i.resetsAt, now)} · ${leftText(i.resetsAt, now)}` : "");
const tipOf = (name: string, i: WinInfo, now: number): string => {
  if (i.pct == null) return i.kind === "reset" ? t("cards.cd.tip.reset", "{name}: obnoveno, nové číslo přijde s další odpovědí", { name }) : t("cards.cd.tip.none", "{name}: zatím bez údaje", { name });
  const r = resetText(i, now);
  const old = i.kind === "stale" ? ` · ${t("sessions.limits.stale", "staré (před {n} min)", { n: i.ageMin })}` : "";
  return `${name}: ${pctText(i.pct)} %${r ? ` · ${t("cards.cd.tip.reset.at", "obnova {when}", { when: r })}` : ""}${old}`;
};

function LimitsEmpty(p: { s: Snapshot; onSettings: () => void }) {
  const lv = () => p.s.limits;
  const connect = () => lv()?.statusLine === "none" || lv()?.statusLine === "foreign";
  return (
    <div class="cd-empty" role="status">
      <b>{t("cards.hero.noLimits", "Limity zatím nejsou")}</b>
      <small>{tx(limitsWhy(lv()))}</small>
      <Show when={connect()}>
        <button class="hint-link" onClick={() => { p.onSettings(); gotoSettingsPanel("f-claude.statusLine"); }}>{t("sessions.limits.connect", "Zapojit stavový řádek pro limity 5h / týden")}</button>
      </Show>
      <Show when={lv()}>{(l) => <LimitsSources l={l()} />}</Show>
    </div>
  );
}

/** Věta Tempomatu z `paceLine` (čas dojití a obnovy místně). */
function paceSentence(pl: PaceLine | null, five: WinInfo): string {
  const reset = five.resetsAt != null ? fmtClock(new Date(five.resetsAt)) : "";
  if (!pl) return five.kind === "stale" ? t("sessions.limits.stale", "staré (před {n} min)", { n: five.ageMin }) : reset ? t("cards.cd.pace.reset", "Obnova {reset}", { reset }) : "";
  switch (pl.kind) {
    case "runs-out":
      return t("cards.cd.pace.runsOut", "Při tomhle tempu dojde v {at}, obnova {reset}", { at: fmtClock(new Date(pl.at!)), reset });
    case "lasts":
      return t("cards.cd.pace.lasts", "Při tomhle tempu limit vydrží do obnovy {reset}", { reset });
    case "idle":
      return t("cards.cd.pace.idle", "Bez čerpání, obnova {reset}", { reset });
    default:
      return t("cards.cd.pace.reset", "Obnova {reset}", { reset });
  }
}

function Track(p: { label: string; i: WinInfo; now: number; plan?: number | null }) {
  return (
    <div class="cd-track-row" title={tipOf(p.label, p.i, p.now) + (p.plan != null ? ` · ${t("cards.cd.tip.plan", "plán podle času {n} %", { n: pctText(p.plan) })}` : "")}>
      <span class="cd-track-name">{p.label}</span>
      <span class="cd-track">
        <Show when={p.i.pct != null}>
          <i class="cd-fill" data-tone={p.i.tone} style={{ width: `${bar(p.i.pct!)}%` }} />
        </Show>
        <Show when={p.plan != null}>
          <i class="cd-plan" style={{ left: `${bar(p.plan!)}%` }} />
        </Show>
      </span>
      <b class="cd-track-pct">{p.i.pct != null ? `${pctText(p.i.pct)} %` : p.i.kind === "reset" ? t("sessions.limits.reset", "obnoveno") : "—"}</b>
    </div>
  );
}

function SpendBody(p: { m: SpendModel }) {
  const m = () => p.m;
  return (
    <Switch>
      <Match when={m().state === "ok"}>
        <div class="cd-spend">
          <div class="cd-spend-top">
            <span class="cd-big">{m().today != null ? money(m().today!, m().priced) : "—"}</span>
            <span class="cd-cap">{m().today != null ? t("cards.hero.costToday", "Útrata dnes (odhad v cenách API)") : t("cards.cd.spend.noToday", "Dnešek zatím bez údaje (odhad v cenách API)")}</span>
          </div>
          <div class="cd-chart">
            <Bars values={m().values} fluid h={44} hi={m().today != null ? -1 : -2} title={(i) => `${m().dates[i] ?? ""}: ${money(m().values[i] ?? 0, m().priced)}`} label={t("cards.cd.spend.chart", "Útrata po dnech, posledních 14 dní")} />
          </div>
          <div class="cd-wide cd-stats">
            <span>{t("cards.cd.spend.avg", "průměr {v}", { v: money(m().avg ?? 0, m().priced) })}</span>
            <span>{t("cards.cd.spend.peak", "špička {v}", { v: money(m().peak, m().priced) })}</span>
            <span>{t("cards.cd.spend.days", "{n} dní", { n: m().values.length })}</span>
          </div>
        </div>
      </Match>
      <Match when={true}>
        <div class="cd-empty" role="status">
          <b>{m().state === "off" ? t("cards.cd.spend.off", "Útrata je vypnutá") : m().state === "computing" ? t("cards.cd.spend.computing", "Počítám útratu…") : t("cards.hero.noSpend", "Útrata dnes zatím není")}</b>
          <small>{m().state === "off" ? t("cards.cd.spend.offWhy", "Zapni ji v Nastavení → Claude → Útrata.") : t("cards.hero.why.spend", "Přehled útraty ještě nemá dnešek s cenou — zapni a nech dopočítat v Nastavení → Claude → Útrata.")}</small>
        </div>
      </Match>
    </Switch>
  );
}

const STATE: Record<SessionView["status"], () => string> = {
  waiting: () => t("sessions.status.waiting", "čeká na tebe"),
  working: () => t("sessions.status.working", "pracuje"),
  idle: () => t("sessions.status.idle", "nečinná"),
  ended: () => t("sessions.status.ended", "skončila"),
};

function Tile(p: { ss: SessionView; now: number }) {
  const open = async () => {
    if (!p.ss.canOpen) return say(p.ss.openHint, true);
    try {
      await api.sessionOpen(p.ss.id);
    } catch (e) {
      say(String(e), true);
    }
  };
  const state = () => STATE[p.ss.status]();
  return (
    <button class="cd-tile" data-st={p.ss.status} onClick={open} title={`${p.ss.cwd}\n${p.ss.openHint}`} disabled={!p.ss.canOpen}>
      <span class="cd-dot" aria-hidden="true" />
      <b class="cd-tile-name">{p.ss.project ?? p.ss.name}</b>
      <small class="cd-tile-st">{state()}{p.ss.since != null ? ` · ${ago(p.ss.since, p.now)}` : ""}</small>
    </button>
  );
}

export default function ClaudeDesign(p: P) {
  const lv = () => p.s.limits;
  const five = () => winInfo(lv()?.fiveHour, lv(), p.now);
  const week = () => winInfo(lv()?.sevenDay, lv(), p.now);
  const anyLimit = () => five().pct != null || week().pct != null;
  const pace = () => paceLine(lv(), p.now);
  const spend = () => spendModel(p.s.costs, p.now);
  const n5 = () => t("cards.cd.name5h", "5 h");
  const nWeek = () => t("sessions.limits.week", "týden");
  const fix = () => {
    p.onSettings();
    gotoSettingsPanel("f-claude.statusLine");
  };
  const body = (): JSX.Element => (
    <Switch>
      <Match when={p.design === "spend"}>
        <SpendBody m={spend()} />
      </Match>
      <Match when={p.design === "sessions"}>
        <SessionsBody s={p.s} now={p.now} five={five()} />
      </Match>
      <Match when={!anyLimit()}>
        <LimitsEmpty s={p.s} onSettings={p.onSettings} />
      </Match>
      <Match when={p.design === "rings"}>
        <div class="cd-rings">
          <div class="cd-rings-fig">
            <Rings
              label={t("cards.cd.rings.label", "Limity Claude: 5 h a týden")}
              rings={[
                { frac: five().pct != null ? five().pct! / 100 : null, tone: five().tone, title: tipOf(n5(), five(), p.now) },
                { frac: week().pct != null ? week().pct! / 100 : null, tone: week().tone, title: tipOf(nWeek(), week(), p.now) },
              ]}
            >
              <b class="cd-big">{five().pct != null ? `${pctText(five().pct!)} %` : "—"}</b>
              <small class="cd-cap">{five().resetsAt != null && five().kind !== "reset" ? t("cards.cd.rings.reset", "obnova {when}", { when: fmtClock(new Date(five().resetsAt!)) }) : n5()}</small>
            </Rings>
          </div>
          <div class="cd-legend">
            <Track label={n5()} i={five()} now={p.now} />
            <Track label={nWeek()} i={week()} now={p.now} />
            <small class="cd-wide cd-cap">{resetText(five(), p.now) && `${n5()}: ${resetText(five(), p.now)}`}</small>
            <small class="cd-wide cd-cap">{resetText(week(), p.now) && `${nWeek()}: ${resetText(week(), p.now)}`}</small>
          </div>
        </div>
      </Match>
      <Match when={p.design === "pace"}>
        <div class="cd-pace">
          <Track label={n5()} i={five()} now={p.now} plan={pace()?.plan} />
          <p class="cd-sentence">{paceSentence(pace(), five())}</p>
          <small class="cd-wide cd-cap">
            <Show when={pace()?.plan != null}>{t("cards.cd.pace.plan", "ryska = kde bys měl být podle času ({n} %)", { n: pctText(pace()!.plan) })}</Show>
            <Show when={pace()?.kind === "runs-out"}> · {t("cards.cd.pace.slow", "do obnovy stačí tempo {n} %/h", { n: num(pace()!.slowTo ?? 0, 1) })}</Show>
          </small>
          <div class="cd-wide"><Track label={nWeek()} i={week()} now={p.now} /></div>
        </div>
      </Match>
      <Match when={p.design === "gauge"}>
        <div class="cd-gauge">
          <div class="cd-gauge-fig">
            <Gauge frac={five().pct != null ? five().pct! / 100 : null} plan={pace() ? pace()!.plan / 100 : null} tone={five().tone} title={tipOf(n5(), five(), p.now)} label={tipOf(n5(), five(), p.now)}>
              <b class="cd-big">{five().pct != null ? `${pctText(five().pct!)} %` : "—"}</b>
              <small class="cd-cap">{five().kind !== "reset" && five().resetsAt != null ? t("cards.cd.rings.reset", "obnova {when}", { when: fmtClock(new Date(five().resetsAt!)) }) : n5()}</small>
            </Gauge>
          </div>
          <div class="cd-gauge-side cd-wide">
            <p class="cd-sentence">{paceSentence(pace(), five())}</p>
            <Track label={nWeek()} i={week()} now={p.now} />
          </div>
        </div>
      </Match>
    </Switch>
  );
  return (
    <Show
      when={!HERO_PICK[p.design]}
      fallback={
        <Show when={heroFacts("claude", p.s, p.now, HERO_PICK[p.design])}>
          {(f) => <HeroCard id="claude" facts={f()} menu={p.menu} onFix={f().empty && f().label[0] !== "cards.hero.costToday" ? fix : undefined} />}
        </Show>
      }
    >
      <div class="card cd" data-anim data-design={p.design}>
        <h4>
          <span class="ic" style="background:var(--accent)" />
          {cardName("claude")}
          {p.menu}
        </h4>
        <div class="cd-body">{body()}</div>
      </div>
    </Show>
  );
}

function SessionsBody(p: { s: Snapshot; now: number; five: WinInfo }) {
  const tiles = () => sessionTiles(p.s.sessions, 6);
  return (
    <div class="cd-sessions">
      <Show when={tiles().shown.length} fallback={<div class="cd-empty" role="status"><b>{t("sessions.empty", "Žádná běžící session Claude Code.")}</b></div>}>
        <div class="cd-tiles">
          <For each={tiles().shown}>{(ss) => <Tile ss={ss} now={p.now} />}</For>
        </div>
        <Show when={tiles().more > 0}>
          <small class="cd-cap">{t("cards.cd.sessions.more", "+{n} dalších", { n: tiles().more })}</small>
        </Show>
      </Show>
      <Show when={p.five.pct != null} fallback={<small class="cd-cap">{t("cards.hero.noLimits", "Limity zatím nejsou")}</small>}>
        <Track label={t("cards.cd.name5h", "5 h")} i={p.five} now={p.now} />
      </Show>
    </div>
  );
}
