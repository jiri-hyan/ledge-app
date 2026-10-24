// „Schovat do…“ (Lístky 2.0) — čistá logika rychlých voleb (testy `hide.test.mjs`): lístek zmizí a v daný čas
// se sám vrátí (Rust `pins::win::hide`, jeden termín ve vlákně `emit`). Hodnota = lokální `YYYY-MM-DDTHH:MM`.

const pad2 = (n: number) => String(n).padStart(2, "0");
export const localValue = (d: Date) => `${d.getFullYear()}-${pad2(d.getMonth() + 1)}-${pad2(d.getDate())}T${pad2(d.getHours())}:${pad2(d.getMinutes())}`;
const at = (d: Date, days: number, h: number) => new Date(d.getFullYear(), d.getMonth(), d.getDate() + days, h, 0);

export type HidePick = { id: "hour" | "evening" | "tomorrow" | "monday" | "week"; value: string };

/** Rychlé volby: za hodinu, dnes večer 18:00 (jen do 17:00), zítra ráno 8:00, v pondělí 8:00 (není-li zítra),
 *  za týden 8:00. */
export function hidePicks(now: Date): HidePick[] {
  const out: HidePick[] = [];
  const hour = new Date(Math.ceil((now.getTime() + 60 * 60_000) / 300_000) * 300_000);
  out.push({ id: "hour", value: localValue(hour) });
  if (now.getHours() < 17) out.push({ id: "evening", value: localValue(at(now, 0, 18)) });
  out.push({ id: "tomorrow", value: localValue(at(now, 1, 8)) });
  const toMonday = (8 - now.getDay()) % 7 || 7;
  if (toMonday !== 1) out.push({ id: "monday", value: localValue(at(now, toMonday, 8)) });
  out.push({ id: "week", value: localValue(at(now, 7, 8)) });
  return out;
}

/** Vlastní čas z pole: jen budoucnost (jinak null — Rust by ho stejně odmítl). */
export function validHide(v: string, now: Date): string | null {
  const m = /^(\d{4})-(\d{2})-(\d{2})T(\d{2}):(\d{2})$/.exec(v.trim());
  if (!m) return null;
  const d = new Date(+m[1], +m[2] - 1, +m[3], +m[4], +m[5]);
  return Number.isNaN(d.getTime()) || d.getTime() <= now.getTime() ? null : localValue(d);
}
