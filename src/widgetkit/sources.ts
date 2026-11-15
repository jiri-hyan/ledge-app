// Widgetkit — pevná tabulka zdrojů (cesta → extraktor). Jen pole, která jsou ve `Snapshot` I v `DeskData`
// (`desk/model.ts` `deskSnapshot`), ať plocha i šuplík ukážou totéž (test `model.test.mjs`). ŽÁDNÉ čtení cesty
// nad celým snapshotem (`get(path)`) — to by prozradilo poznámky, cesty a další soukromá data (audit W, W2).
// Hodnota: číslo, text, nebo null = „nevíme“ (resolver ukáže „—“, nikdy vymyšlenou nulu).
import type { Snapshot } from "../types";
import { left, winState } from "../limits/model.ts";
import { fmtClock, fmtDate } from "../format.ts";

export type SrcValue = number | string | null;
type Extract = (s: Snapshot, now: number) => SrcValue;

/** Minuty do resetu okna (nahoru); neznámo nebo už proběhlo = null. */
const resetMin = (at: number | null | undefined, now: number): number | null => {
  const l = left(at, now);
  return l ? l.d * 1440 + l.h * 60 + l.m : null;
};
const num = (v: unknown): number | null => (typeof v === "number" && Number.isFinite(v) ? v : null);
/** Procento okna jen z čerstvého vzorku: starý vzorek, okno po resetu i žádná data = null („—“), ne starý údaj. */
const freshPct = (s: Snapshot, w: "fiveHour" | "sevenDay", now: number): number | null => {
  const l = s.limits;
  const st = winState(l?.[w], l?.at, now, l?.staleMs ?? Infinity);
  return st.kind === "fresh" ? num(st.pct) : null;
};
/** Útrata přesně za `days` dní (`pickRange` by při chybějícím rozsahu vzal jiný = lež); jen při `state` ok
 *  (po chybě / vypnuto / před přepočtem může `summary` zůstat starý). */
const costOf = (s: Snapshot, days: number): number | null => (s.costs?.state === "ok" ? num(s.costs.summary?.ranges?.find((r) => r.days === days)?.total.cost) : null);

/** Nejbližší (probíhající nebo budoucí) událost kalendáře, celodenní ne. */
function nextEvent(s: Snapshot, now: number) {
  const items = (s.personal?.calendar?.days ?? []).flatMap((d) => d.items ?? []).filter((i) => !i.allDay && num(i.end) !== null && i.end > now);
  return items.sort((a, b) => a.start - b.start)[0] ?? null;
}

export const SOURCES: Record<string, Extract> = {
  "limits.five.pct": (s, now) => freshPct(s, "fiveHour", now),
  "limits.five.resetMin": (s, now) => resetMin(s.limits?.fiveHour?.resetsAt, now),
  "limits.week.pct": (s, now) => freshPct(s, "sevenDay", now),
  "limits.week.resetMin": (s, now) => resetMin(s.limits?.sevenDay?.resetsAt, now),
  // čeká na člověka = schvalování + otázky Clauda (plocha je v `deskSnapshot` rozdělí zpět)
  // plocha dostává seznam ořezaný, skutečný počet je `waitingTotal` (`deskSnapshot`)
  "waiting.count": (s) => {
    const total = (s as { waitingTotal?: unknown }).waitingTotal;
    if (typeof total === "number") return total;
    return s.pending || s.questions ? (s.pending?.length ?? 0) + (s.questions?.length ?? 0) : null;
  },
  "procs.count": (s) => (s.procs ? s.procs.filter((p) => p.phase !== "exited" && p.phase !== "failed").length : null),
  "costs.today": (s) => costOf(s, 1),
  "costs.week": (s) => costOf(s, 7),
  "calendar.next.title": (s, now) => nextEvent(s, now)?.title || null,
  "calendar.next.inMin": (s, now) => {
    const e = nextEvent(s, now);
    return e ? Math.max(0, Math.ceil((e.start - now) / 60000)) : null;
  },
  "clock.hm": (_s, now) => fmtClock(new Date(now)),
  "clock.date": (_s, now) => fmtDate(new Date(now)),
};

/** Hodnota zdroje; neznámý zdroj = `undefined` (resolver řádek vynechá + poznámka), výjimka extraktoru = null. */
export function sourceValue(path: string, s: Snapshot, now: number): SrcValue | undefined {
  if (!Object.prototype.hasOwnProperty.call(SOURCES, path)) return undefined;
  try {
    return SOURCES[path](s, now);
  } catch {
    return null;
  }
}
