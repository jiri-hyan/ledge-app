// Widgetkit — čistý resolver `*.kit.json`: `resolve(spec, snapshot, now) → WidgetData` (tvar, který umí `WidgetCard`).
// Šablona = text s `{zdroj}` / `{zdroj|formát}` (5 formátovačů, žádné výrazy); tón z prahů `over`; akce jen z bílé listiny
// (`open`, `url`, `copy`, `action:<id>`, `drawer:<karta>`). Neznámý zdroj = řádek vynechán + poznámka; null = „—“.
// Žádný čas ani I/O mimo argumenty → tabulkový test `model.test.mjs`.
import type { KitAct, KitButton, KitRow, KitSpec, KitTone, Snapshot, WidgetData, WidgetItem, WidgetTone } from "../types";
import { fmtDate, num, pad2, uptime } from "../format.ts";
import { usdDigits } from "../costs/model.ts";
import { t } from "../i18n.ts";
import { sourceValue, type SrcValue } from "./sources.ts";

export const DASH = "—";
const MAX_TEXT = 120;
const MAX_ROWS = 12;
const MAX_BUTTONS = 6;
const has = (o: object, k: string) => Object.prototype.hasOwnProperty.call(o, k);

/** Tón widgetkitu → předvolba barvy widgetu. */
export const KIT_TONE: Record<KitTone, WidgetTone> = { ok: "green", amber: "amber", red: "red", dim: "muted" };

/** Tón podle prahů `[[práh, tón], …]`: nejvyšší práh, který hodnota dosáhla (≥); bez hodnoty nebo neplatný tón = žádný. */
export function toneOf(over: unknown, value: SrcValue | undefined): WidgetTone | undefined {
  if (typeof value !== "number" || !Number.isFinite(value) || !Array.isArray(over)) return undefined;
  let best: [number, WidgetTone] | null = null;
  for (const e of over) {
    if (!Array.isArray(e) || typeof e[0] !== "number" || typeof e[1] !== "string" || !has(KIT_TONE, e[1])) continue;
    if (value >= e[0] && (!best || e[0] >= best[0])) best = [e[0], KIT_TONE[e[1] as KitTone]];
  }
  return best?.[1];
}

/** Pevná sada formátovačů (`{zdroj|formát}`); text zdroje projde beze změny. */
export const FORMATS: Record<string, (v: number | string) => string> = {
  usd: (v) => (typeof v === "number" ? t("sessions.costs.usd", "{v} $", { v: num(v, usdDigits(v)) }) : v),
  dur: (v) => (typeof v === "number" ? uptime(0, Math.max(0, v) * 60_000) : v),
  pct: (v) => (typeof v === "number" ? `${Math.floor(v)} %` : v),
  hm: (v) => (typeof v === "number" ? `${Math.floor(v / 60)}:${pad2(Math.floor(v % 60))}` : v),
  date: (v) => (typeof v === "number" ? fmtDate(new Date(v)) : v),
};

/** Hodnota bez formátu: celé číslo bez desetin, jinak 1 desetina; text beze změny. */
export const plain = (v: number | string): string => (typeof v === "number" ? num(v, Number.isInteger(v) ? 0 : 1) : v);

export const fmtValue = (v: SrcValue, format?: string): string => (v === null ? DASH : format ? FORMATS[format](v) : plain(v));

type Part = { lit: string } | { src: string; format?: string };
/** Rozloží text na kusy; chyba syntaxe (nepárová závorka, prázdný zdroj, víc `|`) = null. */
export function parseTemplate(text: string): Part[] | null {
  const out: Part[] = [];
  const re = /\{([^{}]*)\}/g;
  let last = 0;
  for (let m = re.exec(text); m; m = re.exec(text)) {
    if (m.index > last) out.push({ lit: text.slice(last, m.index) });
    const [src, format, ...rest] = m[1].split("|").map((x) => x.trim());
    if (!src || rest.length) return null;
    out.push({ src, format });
    last = m.index + m[0].length;
  }
  const tail = text.slice(last);
  if (/[{}]/.test(tail)) return null;
  if (tail) out.push({ lit: tail });
  return out;
}

/** Důvod vynechání řádku (už přeložený text poznámky). */
class Bad extends Error {}
const badSource = (src: string) => new Bad(t("cards.widgetkit.note.source", "Neznámý zdroj „{src}“.", { src }));

/** Text šablony → hotový text + první číselný zdroj (pro tón). Vyhodí `Bad` na neznámý zdroj / formát / syntaxi. */
function fill(text: string, snap: Snapshot, now: number): { text: string; value: SrcValue | undefined } {
  const parts = parseTemplate(text);
  if (!parts) throw new Bad(t("cards.widgetkit.note.syntax", "Špatná šablona „{text}“ (chybí nebo přebývá závorka).", { text: text.slice(0, 40) }));
  let value: SrcValue | undefined;
  let s = "";
  for (const p of parts) {
    if ("lit" in p) {
      s += p.lit;
      continue;
    }
    if (p.format && !has(FORMATS, p.format)) throw new Bad(t("cards.widgetkit.note.format", "Neznámý formát „{f}“ ({src}).", { f: p.format, src: p.src }));
    const v = sourceValue(p.src, snap, now);
    if (v === undefined) throw badSource(p.src);
    if (value === undefined && typeof v === "number") value = v;
    s += fmtValue(v, p.format);
  }
  return { text: s.slice(0, MAX_TEXT), value };
}

/** Akce řádku / tlačítka: jedna, přednost `action` > `url` (jen http/https) > `open` > `copy` (šablona). */
function actOf(r: Pick<KitRow, "open" | "url" | "copy" | "action">, snap: Snapshot, now: number): KitAct | undefined {
  if (typeof r.action === "string" && r.action) return { type: "action", id: r.action };
  if (typeof r.url === "string" && /^https?:\/\//i.test(r.url)) return { type: "url", url: r.url };
  if (typeof r.open === "string" && r.open) return r.open.startsWith("drawer:") ? { type: "drawer", card: r.open.slice(7) } : { type: "open", path: r.open };
  if (typeof r.copy === "string" && r.copy) return { type: "copy", text: fill(r.copy, snap, now).text };
  return undefined;
}

function rowItem(r: KitRow, snap: Snapshot, now: number): WidgetItem {
  const label = typeof r.label === "string" ? r.label.slice(0, MAX_TEXT) : undefined;
  let item: WidgetItem;
  if (typeof r.bar === "string") {
    const v = sourceValue(r.bar, snap, now);
    if (v === undefined) throw badSource(r.bar);
    const pct = typeof v === "number" ? v : null;
    item = { label: label ?? r.bar, value: pct === null ? DASH : FORMATS.pct(pct), color: toneOf(r.tone?.over, pct) };
    if (pct !== null) item.bar = Math.max(0, Math.min(1, pct / 100));
  } else if (typeof r.text === "string") {
    const f = fill(r.text, snap, now);
    item = label ? { label, value: f.text } : { label: f.text };
    item.color = toneOf(r.tone?.over, f.value);
  } else {
    throw new Bad(t("cards.widgetkit.note.empty", "Řádek nemá ani text, ani průběh."));
  }
  if (item.color === undefined) delete item.color;
  const kit = actOf(r, snap, now);
  if (kit) item.kit = kit;
  return item;
}

const why = (e: unknown) => (e instanceof Bad ? e.message : t("cards.widgetkit.note.broken", "rozbitý zdroj"));

/** Spec + snapshot + čas → data karty. Rozbitý vstup (špatný tvar specu / snapshotu) nikdy nevyhodí: řádek se vynechá s poznámkou. */
export function resolve(spec: KitSpec, snap: Snapshot | null | undefined, now: number): WidgetData {
  const s = (snap ?? {}) as Snapshot;
  const notes: string[] = [];
  const items: WidgetItem[] = [];
  const rows: unknown[] = Array.isArray(spec?.rows) ? spec.rows : [];
  for (const [i, r] of rows.slice(0, MAX_ROWS).entries()) {
    try {
      if (!r || typeof r !== "object") throw new Bad(t("cards.widgetkit.note.empty", "Řádek nemá ani text, ani průběh."));
      items.push(rowItem(r as KitRow, s, now));
    } catch (e) {
      notes.push(t("cards.widgetkit.note.row", "Řádek {n} vynechán: {why}", { n: i + 1, why: why(e) }));
    }
  }
  const buttons: unknown[] = Array.isArray(spec?.buttons) ? spec.buttons : [];
  const btns: WidgetItem[] = [];
  for (const [i, b] of buttons.slice(0, MAX_BUTTONS).entries()) {
    const btn = b as KitButton;
    try {
      const kit = btn && typeof btn.label === "string" ? actOf(btn, s, now) : undefined;
      if (!kit) throw new Bad(t("cards.widgetkit.note.noAct", "Tlačítko nemá platnou akci."));
      btns.push({ label: btn.label.slice(0, MAX_TEXT), icon: "▸", kit });
    } catch (e) {
      notes.push(t("cards.widgetkit.note.btn", "Tlačítko {n} vynecháno: {why}", { n: i + 1, why: why(e) }));
    }
  }
  if (btns.length) items.push(...(items.length ? [{ label: "", sep: true }] : []), ...btns);
  return { title: typeof spec?.title === "string" ? spec.title : null, text: null, items, badge: null, progress: null, strip: null, color: null, notes: notes.length ? notes : undefined };
}
