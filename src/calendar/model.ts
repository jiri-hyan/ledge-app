// Karta Kalendář — čistá logika bez DOM a bez importů za běhu (test `model.test.mjs`): barvy kalendářů,
// úprava vloženého odkazu (zrcadlo Rust `calendar/conf.rs` `normalize_url`, aby editor hned řekl, co je
// špatně), maskování adresy (jen doména — celá adresa je tajemství), čas řádku a změna jednoho kalendáře
// v configu. Texty si skládá komponenta přes t().

/** Barvy přiřazované novým kalendářům postupně (stejná paleta jako vzorky v Nastavení). */
export const CAL_COLORS = ["#60a5fa", "#4ade80", "#fbbf24", "#f472b6", "#a78bfa", "#2dd4bf", "#fb923c", "#f87171"] as const;

/** Nejstarší stažení mezi zapnutými kalendáři (ms) — stáří dat karty je stáří nejméně čerstvého; žádné = null. */
export function oldestFetchedMs(cals: readonly { enabled: boolean; fetchedMs: number | null }[] | null | undefined): number | null {
  let old: number | null = null;
  for (const c of cals ?? []) if (c.enabled && c.fetchedMs != null && (old === null || c.fetchedMs < old)) old = c.fetchedMs;
  return old;
}

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

import { pad2 as pad } from "../pad.ts";
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

// --- vlna 8: zdroje, Připojit, připomínky ---

export type Source = "ics" | "outlook-local" | "graph" | "google";

/** Zdroj kalendáře z configu (starý config bez pole = odkaz ICS). */
export const sourceOf = (f: { source?: string | null } | null | undefined): Source =>
  f?.source === "outlook-local" || f?.source === "graph" || f?.source === "google" ? f.source : "ics";

/** Název služby u tlačítka Připojit (vlastní jména, nepřekládají se). */
export const PROVIDER_NAME: Record<string, string> = { teams: "Teams", meet: "Google Meet", zoom: "Zoom", webex: "Webex", jitsi: "Jitsi" };
export const providerName = (p: string | null | undefined): string => (p && PROVIDER_NAME[p]) || "";

/** Místo pod názvem: když je to jen odkaz na schůzku (Zoom v místě), neukazovat ho — je na tlačítku. */
export function placeLabel(it: { location: string; join?: { url: string } | null }): string {
  const l = it.location.trim();
  if (it.join && /^https?:\/\//i.test(l)) return "";
  return l;
}

/** Tlačítko Připojit: událost má odkaz a ještě neskončila (nulová „trvá“ minutu). */
export const joinable = (it: { end: number; start: number; join?: { url: string } | null }, now: number): boolean =>
  !!it.join?.url && now < Math.max(it.end, it.start + 60_000);

/** Volby připomínky (minuty předem; 0 = nepřipomínat). Hodnota z configu mimo nabídku se přidá. */
export const REMIND_MINUTES = [0, 1, 5, 10, 15, 30] as const;
export function remindOptions(cur: number | null | undefined): number[] {
  const set = new Set<number>(REMIND_MINUTES);
  if (typeof cur === "number" && Number.isInteger(cur) && cur >= 0 && cur <= 120) set.add(cur);
  return [...set].sort((a, b) => a - b);
}

/** Výchozí připomínka nového i starého kalendáře (Rust `conf::DEFAULT_REMIND_MIN`). */
export const DEFAULT_REMIND = 5;
export const remindOf = (f: { remindMin?: number | null } | null | undefined): number =>
  typeof f?.remindMin === "number" ? f.remindMin : DEFAULT_REMIND;

/** Nový kalendář „Outlook v tomto PC“ (bez adresy). */
export const outlookFeed = (color: string) => ({ name: "", url: "", color, enabled: true, source: "outlook-local" as const, remindMin: DEFAULT_REMIND, remindToast: true });

// --- kolo 33 (L85): délka schůzky a účastníci ---

/** Minuty čitelně: „45 min“, „2 h“, „2 h 10 min“ (jednotky jsou stejné česky i anglicky). */
export function minLabel(n: number): string {
  const m = Math.max(0, Math.round(n));
  if (m < 60) return `${m} min`;
  return m % 60 ? `${Math.floor(m / 60)} h ${m % 60} min` : `${Math.floor(m / 60)} h`;
}

/** Jak dlouho schůzka trvá / za jak dlouho začne, spočítané z `start/end/now` (zrcadlo Rust `calendar/span.rs`, tabulka hranic je stejná; žádný vlastní časovač —
 *  překresluje se s hodinami aplikace). `before` jen do `within` minut před začátkem; celodenní, nulová
 *  a skončená = null. Konec přes půlnoc nevadí (absolutní ms). */
export type MeetingSpan = { phase: "before"; startsIn: number; total: number } | { phase: "running"; run: number; left: number; total: number };
export function meetingSpan(it: { start: number; end: number; allDay: boolean }, now: number, within = 60): MeetingSpan | null {
  if (it.allDay || it.end <= it.start || now >= it.end) return null;
  const total = Math.round((it.end - it.start) / 60_000);
  if (now < it.start) {
    const startsIn = Math.ceil((it.start - now) / 60_000);
    return startsIn <= within ? { phase: "before", startsIn, total } : null;
  }
  return { phase: "running", run: Math.floor((now - it.start) / 60_000), left: Math.max(1, Math.ceil((it.end - now) / 60_000)), total };
}

export interface CalendarPerson {
  name: string;
  email: string;
  resp: "pending" | "accepted" | "tentative";
}

/** Písmena jména bez závorek („Jan Novák (Externí)“) a titulů v čárce; „Příjmení, Jméno“ se otočí. */
function nameWords(name: string): string[] {
  let n = name.replace(/\([^)]*\)/g, " ").trim();
  const comma = n.split(",");
  if (comma.length === 2 && comma[0].trim() && comma[1].trim()) n = `${comma[1]} ${comma[0]}`;
  return n.split(/[\s ]+/).filter((w) => /\p{L}/u.test(w));
}
const firstLetter = (w: string): string => (w.match(/\p{L}/u)?.[0] ?? "").toLocaleUpperCase("cs");

/** Iniciály: jméno → první písmeno prvního a posledního slova („Tomáš Hora“ → „JH“, „Žofie“ → „Ž“,
 *  „Hora, Tomáš“ → „JH“); bez jména e-mail → první písmeno před @; nic použitelného → "". */
export function initials(name: string, email = ""): string {
  const w = nameWords(name);
  if (w.length) return w.length === 1 ? firstLetter(w[0]) : firstLetter(w[0]) + firstLetter(w[w.length - 1]);
  const local = email.split("@")[0] ?? "";
  return firstLetter(local);
}

/** Stabilní barva avataru z klíče (e-mail, jinak jméno) — z palety akcentů kalendářů, stejný člověk = stejná barva. */
export function avatarColor(key: string): string {
  let h = 2166136261;
  for (const c of key.toLowerCase()) h = Math.imul(h ^ c.codePointAt(0)!, 16777619) >>> 0;
  return CAL_COLORS[h % CAL_COLORS.length];
}

export interface Avatar {
  initials: string;
  color: string;
  /** jméno (a e-mail) pro tooltip */
  label: string;
  tentative: boolean;
}

/** Avatary řady: nejvýš `max`, zbytek (včetně `more` z Rustu) jako „+N“; stejný člověk (e-mail / jméno) jen jednou,
 *  bez použitelných iniciál se nekreslí. Bez účastníků (zdroj je nevrací) = prázdné → nic se nekreslí. */
export function avatarRow(people: { list?: readonly CalendarPerson[]; more?: number } | null | undefined, max = 4): { shown: Avatar[]; more: number; names: string } {
  const seen = new Set<string>();
  const all: Avatar[] = [];
  for (const p of people?.list ?? []) {
    const key = (p.email || p.name).trim().toLowerCase();
    const ini = initials(p.name, p.email);
    if (!key || !ini || seen.has(key)) continue;
    seen.add(key);
    all.push({ initials: ini, color: avatarColor(key), label: p.name && p.email ? `${p.name} <${p.email}>` : p.name || p.email, tentative: p.resp === "tentative" });
  }
  const shown = all.slice(0, max);
  const more = all.length - shown.length + (people?.more ?? 0);
  return { shown, more, names: all.map((a) => a.label).join("\n") };
}
