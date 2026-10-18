// Karta Kalendář — čistá logika bez DOM a bez importů za běhu (test `model.test.mjs`): barvy kalendářů,
// úprava vloženého odkazu (zrcadlo Rust `calendar/conf.rs` `normalize_url`, aby editor hned řekl, co je
// špatně), maskování adresy (jen doména — celá adresa je tajemství), čas řádku a změna jednoho kalendáře
// v configu. Texty si skládá komponenta přes t().

/** Barvy přiřazované novým kalendářům postupně (stejná paleta jako vzorky v Nastavení). */
export const CAL_COLORS = ["#60a5fa", "#4ade80", "#fbbf24", "#f472b6", "#a78bfa", "#2dd4bf", "#fb923c", "#f87171"] as const;

/** Barva kalendáře: z configu, jinak podle pořadí. */
export const calColor = (color: string | null | undefined, index: number): string =>
  color && /^#[0-9a-fA-F]{6}$/.test(color) ? color : CAL_COLORS[((index % CAL_COLORS.length) + CAL_COLORS.length) % CAL_COLORS.length];

/** První barva palety, kterou ještě žádný kalendář nemá (jinak podle počtu). */
export function nextColor(used: readonly (string | null | undefined)[]): string {
  const taken = new Set(used.filter(Boolean).map((c) => c!.toLowerCase()));
  return CAL_COLORS.find((c) => !taken.has(c)) ?? CAL_COLORS[used.length % CAL_COLORS.length];
}

export type UrlProblem = "empty" | "http" | "scheme" | "invalid";

/** Vložený odkaz → https adresa, nebo důvod, proč ne. `webcal://` = `https://`. */
export function normalizeUrl(raw: string): { url: string } | { problem: UrlProblem } {
  const t = raw.trim();
  if (!t) return { problem: "empty" };
  const lower = t.toLowerCase();
  const pre = ["webcal://", "webcals://", "https://"].find((p) => lower.startsWith(p));
  if (!pre) return { problem: lower.startsWith("http://") ? "http" : "scheme" };
  const rest = t.slice(pre.length);
  const host = rest.split(/[/?#]/)[0];
  if (!host || host.includes("@") || /[\s\u0000-\u001f]/.test(t)) return { problem: "invalid" };
  return { url: `https://${rest}` };
}

/** Jen doména (bez cesty, dotazu, portu, přihlášení). */
export function domainOf(raw: string): string {
  const t = raw.trim();
  const i = t.indexOf("://");
  const rest = i >= 0 ? t.slice(i + 3) : t;
  const host = rest.split(/[/?#]/)[0];
  return (host.split("@").pop() ?? "").split(":")[0].toLowerCase();
}

/** Odkaz z Googlu/Outlooku? (pro nápovědu u chyby) */
export const provider = (raw: string): "google" | "outlook" | "other" => {
  const d = domainOf(raw);
  if (d.endsWith("google.com")) return "google";
  if (d.includes("outlook") || d.includes("office365") || d.includes("live.com")) return "outlook";
  return "other";
};

const pad = (n: number) => String(n).padStart(2, "0");
/** „09:15“ v místním čase. */
export const hm = (ms: number) => {
  const d = new Date(ms);
  return `${pad(d.getHours())}:${pad(d.getMinutes())}`;
};

/** Místní datum `YYYY-MM-DD` času `ms`. */
export const ymd = (ms: number) => {
  const d = new Date(ms);
  return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}`;
};

/** Čas řádku v rámci dne `day` (YYYY-MM-DD): „09:15–09:30“; začátek/konec jiný den = šipka
 *  („→ 10:00“ pokračuje ze včerejška, „22:00 →“ končí zítra). Celodenní = null. */
export function timeLabel(it: { start: number; end: number; allDay: boolean }, day: string): string | null {
  if (it.allDay) return null;
  const sameStart = ymd(it.start) === day;
  const sameEnd = ymd(it.end) === day || (it.end > it.start && ymd(it.end - 1) === day && hm(it.end) === "00:00");
  if (it.end <= it.start) return hm(it.start);
  if (sameStart && sameEnd) return `${hm(it.start)}–${hm(it.end)}`;
  if (sameStart) return `${hm(it.start)} →`;
  if (sameEnd) return `→ ${hm(it.end)}`;
  return "↔";
}

/** Config se změnou jednoho pole kalendáře `i` (vstup netknutý). */
export function withCalendar<T extends Record<string, any>>(cfg: T, i: number, patch: Record<string, unknown>): T {
  const personal = cfg.personal ?? {};
  const list = [...(personal.calendars ?? [])];
  if (!list[i]) return cfg;
  list[i] = { ...list[i], ...patch };
  return { ...cfg, personal: { ...personal, calendars: list } };
}
