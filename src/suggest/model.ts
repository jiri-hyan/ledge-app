// Kontextové návrhy: čistá logika pruhu v šuplíku (bez DOM, Solidu a `t()`), test `model.test.mjs`.
// `pick(input, dismissed, now)` vybere nejvýš 3 chipy VÝHRADNĚ z reálných událostí — žádný zdroj, žádný chip:
// připomínka po termínu, schůzka s odkazem Připojit (10 min předem) a poznámka ze schůzky (15 min po konci),
// spadlé CI na mém PR, PR čekající na mou revizi, právě naběhlý dev server, adresa ve schránce, kterou ještě
// nemám mezi odkazy. Zavřený chip se pamatuje podle klíče 24 h (jen v paměti).
// Počítá se při změně snapshotu — žádný vlastní časovač ani vlákno.
import type { PrRef, SuggestOn } from "../types/suggest";

export type SuggestKind = "reminder" | "meeting" | "meetingNote" | "ghCi" | "ghReview" | "dev" | "clip";

/** Jeden chip. Texty skládá UI (`t()`), tady jsou jen data. */
export interface Suggestion {
  /** stabilní klíč události (zavření se pamatuje podle něj) */
  key: string;
  kind: SuggestKind;
  /** hlavní údaj: název poznámky, `repo#N`, `localhost:PORT`, adresa bez schématu */
  title: string;
  /** vedlejší údaj: titul PR, projekt dev serveru */
  detail: string;
  /** kolik dalších téhož druhu čeká (PR, připomínky) */
  more: number;
  /** adresa k otevření / uložení (PR, dev server, schránka) */
  url?: string;
  /** cesta poznámky (připomínka) */
  path?: string;
  /** číslo PR (`ghCi`, `ghReview`) */
  num?: number;
  /** schůzka (`meeting`, `meetingNote`): začátek v ms (titulek poznámky) a služba odkazu */
  start?: number;
  provider?: string;
}

/** Schůzka s ověřeným odkazem Připojit (z `personal.calendar`, jen dnešek, bez celodenních). */
export interface Meeting {
  title: string;
  start: number;
  end: number;
  join: { url: string; provider: string };
}

export interface SuggestInput {
  on: SuggestOn;
  /** připomínky po termínu (`Snapshot.reminders`) */
  reminders: { path: string; title: string; remindMs: number | null }[];
  github: { review: PrRef[]; failing: PrRef[]; fetchedMs: number | null };
  /** poznámky jsou zapnuté (bez nich se poznámka ze schůzky nenabízí) */
  notes: boolean;
  /** dnešní schůzky s odkazem, i ty, které kalendář už skryl jako skončené (`trackMeetings`) */
  meetings: Meeting[];
  /** procesy spuštěné z Ledge (`Snapshot.procs`) */
  procs: { id: string; phase: string; port: number | null; url: string | null; readyAt: number | null }[];
  projects: { id: string; name: string }[];
  /** nejnovější položka historie schránky, pokud je to adresa (`clipUrl`) */
  clip: { url: string; atMs: number } | null;
  /** cíle odkazů, které už mám (`knownLinks`) */
  known: string[];
}

export const MAX_CHIPS = 3;
/** Zavřený návrh se nevrací 24 hodin. */
export const DISMISS_MS = 24 * 3600_000;
/** Dev server nabídneme jen krátce po naběhnutí. */
export const DEV_FRESH_MS = 10 * 60_000;
/** Data GitHubu starší než hodinu už nejsou důvod něco nabízet. */
export const GH_FRESH_MS = 60 * 60_000;
/** Adresa ve schránce: nabídka jen 30 minut po zkopírování. */
export const CLIP_FRESH_MS = 30 * 60_000;

/** Odkaz Připojit se nabídne 10 minut před začátkem. */
export const MEET_LEAD_MS = 10 * 60_000;
/** Poznámka ze schůzky se nabídne do 15 minut po konci. */
export const MEET_NOTE_MS = 15 * 60_000;

export type Dismissed = ReadonlyMap<string, number>;

/** Je klíč zavřený (a ještě ne po 24 h)? */
export const isDismissed = (d: Dismissed, key: string, now: number): boolean => (d.get(key) ?? 0) > now;

/** Nová mapa se zavřeným klíčem; prošlé záznamy odpadnou (mapa nenaroste). */
export function dismiss(d: Dismissed, key: string, now: number): Map<string, number> {
  const next = new Map<string, number>();
  for (const [k, until] of d) if (until > now) next.set(k, until);
  next.set(key, now + DISMISS_MS);
  return next;
}

/** Adresa pro porovnání: bez `www.`, bez koncového lomítka a bez `#`; velikost jen u hostu. Neadresa = null. */
export function normUrl(u: string): string | null {
  try {
    const x = new URL(u.trim());
    if (x.protocol !== "http:" && x.protocol !== "https:") return null;
    return `${x.hostname.toLowerCase().replace(/^www\./, "")}${x.port ? `:${x.port}` : ""}${x.pathname.replace(/\/+$/, "")}${x.search}`;
  } catch {
    return null;
  }
}

/** Je adresa už mezi odkazy? */
export function isKnown(url: string, known: readonly string[]): boolean {
  const n = normUrl(url);
  return n !== null && known.some((k) => normUrl(k) === n);
}

/** Cíle odkazů, které už mám: akce typu web (`url` / `web`) a starší `personal.links` s adresou. */
export function knownLinks(actions: readonly { kind: string; target: string }[] | undefined, links: readonly { kind: string; target: string }[] | undefined): string[] {
  return [...(actions ?? []).filter((a) => a.kind === "url" || a.kind === "web"), ...(links ?? []).filter((l) => l.kind === "web")].map((x) => x.target);
}

/** Adresa ze schránky: rozhoduje NEJNOVĚJŠÍ položka historie (to je ta ve schránce) a jen když je to jedna celá
 *  adresa http(s), ne zkrácený náhled a ne starší než `CLIP_FRESH_MS`. */
export function clipUrl(items: readonly { preview: string; chars: number; atMs: number }[] | null | undefined, now: number): { url: string; atMs: number } | null {
  let top: { preview: string; chars: number; atMs: number } | null = null;
  for (const it of items ?? []) if (!top || it.atMs > top.atMs) top = it;
  if (!top || now - top.atMs > CLIP_FRESH_MS) return null;
  const text = top.preview.trim();
  if (top.chars !== top.preview.length || /\s/.test(text) || normUrl(text) === null) return null;
  return { url: text, atMs: top.atMs };
}

/** Adresa dev serveru: z výstupu procesu jen loopback, jinak `http://localhost:PORT`. */
export function devUrl(url: string | null, port: number): string {
  return url && /^https?:\/\/(localhost|127\.0\.0\.1|\[::1\])(:\d+)?(\/|$)/i.test(url) ? url : `http://localhost:${port}`;
}

/** Host bez `www.` pro název odkazu / chipu; neadresa = "". */
export function hostOf(url: string): string {
  try {
    return new URL(url).hostname.replace(/^www\./, "");
  } catch {
    return "";
  }
}

const meetKey = (kind: "join" | "note", m: Meeting) => `meet-${kind}:${m.start}|${m.title}`;

/** Konec schůzky pro rozhodování: nulová délka se bere jako minuta (jako `joinable` v kalendáři). */
const endOf = (m: Meeting) => Math.max(m.end, m.start + 60_000);

/**
 * Paměť dnešních schůzek s odkazem. Kalendář skončené události ze zobrazení hned vyřadí, ale poznámku ze schůzky
 * chceme nabídnout až PO konci — proto se schůzky pamatují (jen v paměti) od chvíle, kdy je snapshot ukáže, a
 * zapomenou se 15 minut po konci. Nová mapa; čistá funkce.
 */
export function trackMeetings(mem: ReadonlyMap<string, Meeting>, current: readonly Meeting[], now: number): Map<string, Meeting> {
  const next = new Map<string, Meeting>();
  for (const [k, m] of mem) if (now - endOf(m) <= MEET_NOTE_MS) next.set(k, m);
  for (const m of current) next.set(`${m.start}|${m.title}`, m);
  return next;
}

/** Dnešní schůzky s ověřeným odkazem, které ještě neskončily (kalendář skončené sám nevrací); bez celodenních. */
export function currentMeetings(cal: { days: { kind: string; items: { title: string; start: number; end: number; allDay: boolean; join?: { url: string; provider: string } | null }[] }[] } | null | undefined): Meeting[] {
  const out: Meeting[] = [];
  for (const it of cal?.days.find((d) => d.kind === "today")?.items ?? []) if (!it.allDay && it.join?.url) out.push({ title: it.title, start: it.start, end: it.end, join: { url: it.join.url, provider: it.join.provider } });
  return out;
}

/** Text nové poznámky ze schůzky: první řádek = titulek „{název} — {datum}“ (datum místně `d. m. yyyy`). */
export function meetingNoteText(m: Pick<Meeting, "title" | "start">): string {
  const d = new Date(m.start);
  return `${m.title.trim() || "—"} — ${d.getDate()}. ${d.getMonth() + 1}. ${d.getFullYear()}\n\n`;
}

const prKey = (kind: "ci" | "review", p: PrRef) => `gh-${kind}:${p.repo}#${p.number}`;

/** Chip z prvního nezavřeného PR ze seznamu; `more` = kolik dalších nezavřených čeká. */
function fromPrs(list: readonly PrRef[], kind: "ci" | "review", d: Dismissed, now: number): Suggestion | null {
  const open = list.filter((p) => !isDismissed(d, prKey(kind, p), now));
  const first = open[0];
  if (!first) return null;
  return { key: prKey(kind, first), kind: kind === "ci" ? "ghCi" : "ghReview", title: `${first.repo}#${first.number}`, detail: first.title, more: open.length - 1, url: first.url, num: first.number };
}

/** Stejné chipy (podle obsahu)? `pick` se počítá s každým tikem `now`; bez téhle shody by se chipy každou
 *  sekundu překreslily znovu (objekty jsou pokaždé nové). */
export function sameSuggestions(a: readonly Suggestion[], b: readonly Suggestion[]): boolean {
  return a.length === b.length && a.every((x, i) => x.key === b[i].key && x.more === b[i].more && x.title === b[i].title && x.detail === b[i].detail && x.url === b[i].url && x.path === b[i].path && x.num === b[i].num && x.start === b[i].start);
}

/**
 * Nejvýš `MAX_CHIPS` návrhů v pořadí důležitosti: připomínka · schůzka · spadlé CI · PR k revizi · dev server · schránka.
 * Jeden chip na druh (další téhož druhu jen jako `more`). Čistá funkce nad snapshotem.
 */
export function pick(input: SuggestInput, dismissed: Dismissed, now: number): Suggestion[] {
  const out: Suggestion[] = [];
  const { on } = input;

  if (on.reminders) {
    const keyOf = (r: { path: string; remindMs: number | null }) => `rem:${r.path}|${r.remindMs ?? ""}`;
    const open = input.reminders.filter((r) => !isDismissed(dismissed, keyOf(r), now));
    const first = open[0];
    if (first) out.push({ key: keyOf(first), kind: "reminder", title: first.title, detail: "", more: open.length - 1, path: first.path });
  }

  if (on.reminders) {
    const live = input.meetings.filter((m) => now >= m.start - MEET_LEAD_MS && now < endOf(m) && !isDismissed(dismissed, meetKey("join", m), now)).sort((a, b) => a.start - b.start);
    const m = live[0];
    if (m) out.push({ key: meetKey("join", m), kind: "meeting", title: m.title, detail: m.join.provider, more: live.length - 1, url: m.join.url, start: m.start, provider: m.join.provider });
    const done = input.meetings.filter((x) => now >= endOf(x) && now - endOf(x) <= MEET_NOTE_MS && !isDismissed(dismissed, meetKey("note", x), now)).sort((a, b) => endOf(b) - endOf(a));
    const n = input.notes ? done[0] : undefined;
    if (n) out.push({ key: meetKey("note", n), kind: "meetingNote", title: n.title, detail: "", more: done.length - 1, start: n.start });
  }

  const ghFresh = input.github.fetchedMs !== null && now - input.github.fetchedMs <= GH_FRESH_MS && now >= input.github.fetchedMs - 60_000;
  if (on.github && ghFresh) {
    const ci = fromPrs(input.github.failing, "ci", dismissed, now);
    if (ci) out.push(ci);
    const review = fromPrs(input.github.review, "review", dismissed, now);
    if (review) out.push(review);
  }

  if (on.devServer) {
    const live = input.procs
      .filter((p) => (p.phase === "ready" || p.phase === "running") && p.port !== null && p.readyAt !== null && now - p.readyAt <= DEV_FRESH_MS)
      .map((p) => ({ p, key: `dev:${p.id}:${p.port}:${p.readyAt}` }))
      .filter((x) => !isDismissed(dismissed, x.key, now))
      .sort((a, b) => (b.p.readyAt ?? 0) - (a.p.readyAt ?? 0));
    const top = live[0];
    if (top && top.p.port !== null) {
      const name = input.projects.find((x) => x.id === top.p.id)?.name ?? top.p.id;
      out.push({ key: top.key, kind: "dev", title: `localhost:${top.p.port}`, detail: name, more: live.length - 1, url: devUrl(top.p.url, top.p.port) });
    }
  }

  const clip = input.clip;
  if (on.clipboard && clip && now - clip.atMs <= CLIP_FRESH_MS && !isKnown(clip.url, input.known)) {
    const key = `clip:${normUrl(clip.url)}`;
    if (!isDismissed(dismissed, key, now)) out.push({ key, kind: "clip", title: hostOf(clip.url) || clip.url, detail: clip.url, more: 0, url: clip.url });
  }

  return out.slice(0, MAX_CHIPS);
}
