// Náhled po najetí (`drawer.hover`: ostrůvek / dok) — čistá logika bez DOM a bez `t()`:
// co ukázat (jen z dat, která snapshot už má — nic se kvůli náhledu nestahuje ani nepočítá), priorita
// dění pro ostrůvek a kdy se náhled sám rozbalí do celého šuplíku. Test `model.test.mjs`.
import type { MediaView, SessionView, Snapshot, View, VolumeView } from "../types";
import { positionAt, type MediaTimeline } from "../media/timeline.ts";
import { shownStatus } from "../media/model.ts";
import type { TimerView } from "../timer/model";
import { WX_OLD_MS, weatherAge } from "../cards/weatherAge.ts";
import { limitNear, meetingNow, topOf, type LimitNear, type MeetingNow, type Prio } from "./priority.ts";

/** Šířky náhledů (logické px) — hlásí je skořápka (`App.tsx` `report`), Rust je ořízne na monitor. */
export const PEEK_W = 680;
export const ISLAND_W = 480;
/** Dok je širší než ostrůvek: poznámka, hudba s názvem skladby, hotovo a čas se vedle sebe vejdou (L148). */
export const DOCK_W = 640;
/** Střední panel: šířku určuje jádro (`hovermode::mid_width`, třetina monitoru 520–760); tohle je jen hlášení skořápky. */
export const MID_W = 640;
/** „Hotovo“ = session přešla do klidu nejvýš před tolika ms (pak už je to obyčejná nečinná). */
export const DONE_FRESH_MS = 5 * 60_000;
/** Doběhlý odpočet ukazuje jádro „hotovo“ 10 min (`timer.rs` `DONE_SHOW_MS`) — tady stejně. */
export const TIMER_DONE_MS = 10 * 60_000;

export const isHoverView = (v: View): v is "island" => v === "island";

/** Data náhledu — podmnožina snapshotu (testy staví jen tohle). */
export type HoverData = Pick<Snapshot, "pending" | "questions" | "sessions"> & {
  personal: { media: MediaView | null; volume?: VolumeView | null; weather?: Snapshot["personal"]["weather"]; calendar?: Snapshot["personal"]["calendar"] };
  timer?: TimerView;
  limits?: Snapshot["limits"];
};

/** Potvrzená „Hotovo“ (L146): klíč → čas dokončení (`since`); klíč = session + čas dokončení, další dokončení téže session je nový klíč. */
export type DoneAcked = ReadonlyMap<string, number>;
export const doneKey = (s: Pick<SessionView, "id" | "since">): string => `${s.id}@${s.since ?? 0}`;

/** Po potvrzení: přidá klíč a zahodí ty, které už `doneSession` stejně nevrátí (starší než `DONE_FRESH_MS`) — mapa zůstává malá. */
export function ackDone(acked: DoneAcked, s: Pick<SessionView, "id" | "since">, now: number): DoneAcked {
  const out = new Map<string, number>();
  for (const [k, at] of acked) if (now - at <= DONE_FRESH_MS) out.set(k, at);
  if (s.since != null) out.set(doneKey(s), s.since);
  return out;
}

/** Session, která právě dokončila práci (v klidu nejvýš `DONE_FRESH_MS`, nepotvrzená); nejčerstvější první. */
export function doneSession(sessions: SessionView[], now: number, acked?: DoneAcked): SessionView | null {
  let best: SessionView | null = null;
  for (const s of sessions) {
    if (s.ended || s.status !== "idle" || s.since == null) continue;
    if (acked?.has(doneKey(s))) continue;
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
  | { kind: "meeting"; meeting: MeetingNow }
  | { kind: "limit"; limit: LimitNear }
  | { kind: "done"; session: SessionView }
  | { kind: "timer"; what: "countdown" | "stopwatch" }
  | { kind: "music"; media: MediaView }
  | { kind: "working"; session: SessionView; count: number }
  | { kind: "idle" };

/**
 * Priorita (jedna tabulka `priority.ts` `PRIORITY`): čeká na tebe (schválení, pak otázka) → schůzka za ≤ 10 min /
 * právě probíhá → limit 5h blízko vyčerpání → hotovo → časovač → hraje hudba → Claude pracuje → pozastavená hudba →
 * klid (čas, datum, počasí). Bez dat = klid, nic se nevymýšlí.
 */
export function islandEvent(d: HoverData, now: number, acked?: DoneAcked): IslandEvent {
  const c: ({ prio: Prio } & IslandEvent)[] = [];
  if (d.pending[0]) c.push({ prio: "approval", kind: "approval", pendingId: d.pending[0].id });
  if (d.questions[0]) c.push({ prio: "question", kind: "question", askId: d.questions[0].id });
  const meeting = meetingNow(d.personal.calendar, now);
  if (meeting) c.push({ prio: "meeting", kind: "meeting", meeting });
  const limit = limitNear(d.limits, now);
  if (limit) c.push({ prio: "limit", kind: "limit", limit });
  const done = doneSession(d.sessions, now, acked);
  if (done) c.push({ prio: "done", kind: "done", session: done });
  const tm = timerActive(d.timer, now);
  if (tm) c.push({ prio: "timer", kind: "timer", what: tm });
  const m = d.personal.media;
  const ms = shownStatus(m);
  if (m && ms) c.push({ prio: ms === "playing" ? "music" : "musicPaused", kind: "music", media: m });
  const working = d.sessions.filter((s) => !s.ended && s.status === "working");
  if (working[0]) c.push({ prio: "working", kind: "working", session: working[0], count: working.length });
  const top = topOf(c);
  if (!top) return { kind: "idle" };
  const { prio: _p, ...ev } = top;
  return ev as IslandEvent;
}

/**
 * Za kolik ms se náhled sám rozbalí do celého šuplíku; null = neodpočítávat. `ms` 0 = jen klikem;
 * s čekajícím schválením / otázkou ne (člověk čte a rozhoduje), myš nad tlačítkem taky ne (⏯ nebo
 * Povolit by jinak rozbalení „předběhlo“ — odpočet začne znovu, až z tlačítka sjede). Upevněný dok (`pinned`) se nerozbaluje nikdy.
 */
export function expandAfter(ms: number | undefined, waiting: boolean, overControl: boolean, pinned = false): number | null {
  if (!ms || ms <= 0 || waiting || overControl || pinned) return null;
  return ms;
}

/** Šířka malého pohledu, kterou skořápka hlásí jádru; null = celý šuplík (šířka podle stránky). `dock` = `view: island` kreslí dok. */
export function smallWidth(v: View, dock = false): number | null {
  return v === "peek" ? PEEK_W : v === "island" ? (dock ? DOCK_W : ISLAND_W) : v === "mid" ? MID_W : null;
}

/** Totéž jako `weather::MAX_AGE_MS` v Rustu (`cards/weatherAge.ts`) — starší počasí ostrůvek neukáže (karta Počasí ho ukáže s časem). */
export const WX_MAX_AGE_MS = WX_OLD_MS;

/** Počasí pro ostrůvek v klidu: jen čerstvá data, která už jádro má (kvůli ostrůvku se nic nestahuje). */
export function freshWeather(w: HoverData["personal"]["weather"], now: number): { temp: number; desc: string; icon: string } | null {
  const d = w?.data;
  if (!d || weatherAge(w, now) !== "fresh") return null;
  return { temp: d.temp, desc: d.desc, icon: d.icon };
}

/** Klik do náhledu na prázdné místo (ne na tlačítko, odkaz, pole, lištu průběhu) = celý šuplík. */
export const EXPAND_IGNORE = "button, a, input, textarea, select, [role=slider], [data-noexpand]";

// ---------- Dok (`drawer.hover = "dock"`, `Dock.tsx`) ----------

/** Krok hlasitosti kolečkem / šipkou (procentní body). */
export const VOLUME_STEP = 5;
/** Seek šipkami ← → po zaměření slotu hudby (ms). */
export const DOCK_SEEK_MS = 10_000;

/** Dok podle `ViewMsg.dock` (L130, jádro ho posílá vždy); `null` = jiný pohled než `island`. */
export const dockOfView = (m: { view: string; dock: boolean }): boolean | null => (m.view === "island" ? m.dock : null);

/** Stav 📌 v docku po události (L137) — zrcadlo `Opener::Pinned` v jádře: `view` nese `ViewMsg.pinned` (jádro po `show` ví, jestli upevnění
 *  zůstalo — rozbalení docku do středního panelu / celého šuplíku ho drží, nové `show` jinou cestou a skrytí ne; chybí = ne),
 *  `set` = výsledek `drawer_pin` po úspěšném zavolání. Událost, která dok přepne na ostrůvek a zpět, `view` nepošle →
 *  upevnění zůstane; zkratka na už upevněný dok ho taky nemění (jádro `show`: `Opener::Pinned` zůstává). */
export const dockPinAfter = (ev: { type: "view"; pinned?: boolean } | { type: "set"; on: boolean }): boolean => (ev.type === "set" ? ev.on : ev.pinned === true);

/** Sloty docku: poznámka a čas jsou vždy, hudba jen s daty (zastavená / žádná = bez slotu), časovač jen když je vidět,
 *  „hotovo“ (✓ jméno · Otevřít · ✕) jen nepotvrzené — schválení a otázka přebírají celé okno, hotovo je jen slot v řadě (L146). */
export interface DockSlots {
  done: SessionView | null;
  music: { media: MediaView; volume: VolumeView | null } | null;
  timer: "countdown" | "stopwatch" | null;
}

export function dockSlots(d: HoverData, now: number, acked?: DoneAcked): DockSlots {
  const m = d.personal.media;
  return {
    done: doneSession(d.sessions, now, acked),
    music: m && shownStatus(m) ? { media: m, volume: d.personal.volume ?? null } : null,
    timer: timerActive(d.timer, now),
  };
}

/** Kolečko myši → počet kroků hlasitosti: nahoru (deltaY < 0) +1, dolů −1, 0 / neplatné = 0. */
export function wheelToStep(deltaY: number): number {
  return Number.isFinite(deltaY) && deltaY !== 0 ? (deltaY < 0 ? 1 : -1) : 0;
}

/** Nová hlasitost po `steps` krocích po `VOLUME_STEP`, ořez 0–100 (neplatná úroveň = 0). */
export function volumeStep(level: number, steps: number): number {
  const base = Number.isFinite(level) ? Math.round(level) : 0;
  return Math.min(100, Math.max(0, base + steps * VOLUME_STEP));
}

/** Šipky po zaměření slotu hudby: ↑↓ hlasitost, ←→ seek ±10 s (jen když to zdroj umí); jiná klávesa / nejde = null. */
export type ArrowPlan = { kind: "volume"; steps: 1 | -1 } | { kind: "seek"; ms: number };

export function arrowPlan(key: string, tl: MediaTimeline | null | undefined, playing: boolean, now: number): ArrowPlan | null {
  if (key === "ArrowUp") return { kind: "volume", steps: 1 };
  if (key === "ArrowDown") return { kind: "volume", steps: -1 };
  if ((key === "ArrowLeft" || key === "ArrowRight") && tl?.canSeek && tl.durationMs > 0) {
    const to = positionAt(tl, now, playing) + (key === "ArrowRight" ? DOCK_SEEK_MS : -DOCK_SEEK_MS);
    return { kind: "seek", ms: Math.min(tl.durationMs, Math.max(0, to)) };
  }
  return null;
}
