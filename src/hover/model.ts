// Náhled po najetí (`drawer.hover`: kompaktní náhled / ostrůvek) — čistá logika bez DOM a bez `t()`:
// co ukázat (jen z dat, která snapshot už má — nic se kvůli náhledu nestahuje ani nepočítá), priorita
// dění pro ostrůvek a kdy se náhled sám rozbalí do celého šuplíku. Test `model.test.mjs`.
import type { MediaView, ProcView, SessionView, Snapshot, View } from "../types";
import type { TimerView } from "../timer/model";

/** Šířky náhledů (logické px) — hlásí je skořápka (`App.tsx` `report`), Rust je ořízne na monitor. */
export const PEEK_W = 680;
export const COMPACT_W = 420;
export const ISLAND_W = 480;
/** Kompaktní náhled: nejvýš tolik session Clauda. */
export const SESSION_ROWS = 3;
/** „Hotovo“ = session přešla do klidu nejvýš před tolika ms (pak už je to obyčejná nečinná). */
export const DONE_FRESH_MS = 5 * 60_000;
/** Doběhlý odpočet ukazuje jádro „hotovo“ 10 min (`timer.rs` `DONE_SHOW_MS`) — tady stejně. */
export const TIMER_DONE_MS = 10 * 60_000;

export const isHoverView = (v: View): v is "compact" | "island" => v === "compact" || v === "island";

/** Data náhledu — podmnožina snapshotu (testy staví jen tohle). */
export type HoverData = Pick<Snapshot, "pending" | "questions" | "sessions"> & {
  /** servery z Ledge (kompaktní náhled: řádek se ■ Stop) — starší volající bez nich = žádné */
  procs?: ProcView[];
  personal: { media: MediaView | null; weather?: Snapshot["personal"]["weather"] };
  timer?: TimerView;
  limits?: Snapshot["limits"];
};

/** Session pro kompaktní náhled: bez skončených, v pořadí snapshotu (čeká > pracuje > nečinná), nejvýš `SESSION_ROWS`. */
export function compactSessions(sessions: SessionView[]): SessionView[] {
  return sessions.filter((s) => !s.ended && s.status !== "ended").slice(0, SESSION_ROWS);
}

/** Session, která právě dokončila práci (v klidu nejvýš `DONE_FRESH_MS`); nejčerstvější první. */
export function doneSession(sessions: SessionView[], now: number): SessionView | null {
  let best: SessionView | null = null;
  for (const s of sessions) {
    if (s.ended || s.status !== "idle" || s.since == null) continue;
    const age = now - s.since;
    if (age < 0 || age > DONE_FRESH_MS) continue;
    if (!best || s.since > best.since!) best = s;
  }
  return best;
}

/** Je časovač vidět (běží / pauza / před chvílí doběhl, nebo jdou stopky)? */
export function timerActive(tm: TimerView | undefined, now: number): "countdown" | "stopwatch" | null {
  const c = tm?.countdown;
  if (c && (c.state !== "done" || now - c.at < TIMER_DONE_MS)) return "countdown";
  if (tm?.stopwatch?.state === "running") return "stopwatch";
  return null;
}

/** Jedno nejdůležitější dění pro ostrůvek. */
export type IslandEvent =
  | { kind: "approval"; pendingId: number }
  | { kind: "question"; askId: number }
  | { kind: "done"; session: SessionView }
  | { kind: "timer"; what: "countdown" | "stopwatch" }
  | { kind: "music"; media: MediaView }
  | { kind: "working"; session: SessionView; count: number }
  | { kind: "idle" };

/**
 * Priorita: čeká na tebe (schválení, pak otázka) → hotovo → časovač → hraje hudba → Claude pracuje →
 * pozastavená hudba → klid (čas, datum, počasí). Bez dat = klid, nic se nevymýšlí.
 */
export function islandEvent(d: HoverData, now: number): IslandEvent {
  if (d.pending[0]) return { kind: "approval", pendingId: d.pending[0].id };
  if (d.questions[0]) return { kind: "question", askId: d.questions[0].id };
  const done = doneSession(d.sessions, now);
  if (done) return { kind: "done", session: done };
  const tm = timerActive(d.timer, now);
  if (tm) return { kind: "timer", what: tm };
  const m = d.personal.media;
  if (m && m.status === "playing") return { kind: "music", media: m };
  const working = d.sessions.filter((s) => !s.ended && s.status === "working");
  if (working[0]) return { kind: "working", session: working[0], count: working.length };
  if (m && m.status === "paused") return { kind: "music", media: m };
  return { kind: "idle" };
}

/** Co kompaktní náhled ukáže (prázdné části se vynechají). */
export interface CompactPlan {
  approval: boolean;
  sessions: SessionView[];
  media: MediaView | null;
  /** limit 5h — jen když je vzorek (procenta z reálného zdroje) */
  limit: boolean;
  /** servery, které spustil Ledge: běží / startují / selhaly (nejvýš `SERVER_ROWS`) — po najetí na proužek jde Stop */
  servers: ProcView[];
}

export const SERVER_ROWS = 3;
const SERVER_SHOWN = ["ready", "running", "starting", "preparing", "failed"];

export function compactPlan(d: HoverData): CompactPlan {
  return {
    approval: d.pending.length > 0 || d.questions.length > 0,
    sessions: compactSessions(d.sessions),
    media: d.personal.media && d.personal.media.status !== "stopped" ? d.personal.media : null,
    limit: !!(d.limits?.at != null && d.limits.fiveHour),
    servers: (d.procs ?? []).filter((x) => SERVER_SHOWN.includes(x.phase)).slice(0, SERVER_ROWS),
  };
}

/**
 * Za kolik ms se náhled sám rozbalí do celého šuplíku; null = neodpočítávat. `ms` 0 = jen klikem;
 * s čekajícím schválením / otázkou ne (člověk čte a rozhoduje), myš nad tlačítkem taky ne (⏯ nebo
 * Povolit by jinak rozbalení „předběhlo“ — odpočet začne znovu, až z tlačítka sjede).
 */
export function expandAfter(ms: number | undefined, waiting: boolean, overControl: boolean): number | null {
  if (!ms || ms <= 0 || waiting || overControl) return null;
  return ms;
}

/** Šířka malého pohledu, kterou skořápka hlásí jádru; null = celý šuplík (šířka podle stránky). */
export function smallWidth(v: View): number | null {
  return v === "peek" ? PEEK_W : v === "compact" ? COMPACT_W : v === "island" ? ISLAND_W : null;
}

/** Totéž jako `weather::MAX_AGE_MS` v Rustu — starší počasí ostrůvek neukáže (karta Počasí ho ukáže s časem). */
export const WX_MAX_AGE_MS = 30 * 60_000;

/** Počasí pro ostrůvek v klidu: jen čerstvá data, která už jádro má (kvůli ostrůvku se nic nestahuje). */
export function freshWeather(w: HoverData["personal"]["weather"], now: number): { temp: number; desc: string; icon: string } | null {
  const d = w?.data;
  if (!d || w.status === "stale" || w.fetchedMs == null || now - w.fetchedMs >= WX_MAX_AGE_MS) return null;
  return { temp: d.temp, desc: d.desc, icon: d.icon };
}

/** Klik do náhledu na prázdné místo (ne na tlačítko, odkaz, pole, lištu průběhu) = celý šuplík. */
export const EXPAND_IGNORE = "button, a, input, textarea, select, [role=slider], [data-noexpand]";
