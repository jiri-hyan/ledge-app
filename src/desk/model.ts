// Widgety na ploše — čistá logika UI (bez DOM, bez t() při importu; test `model.test.mjs`): druhy widgetů a
// jejich názvy (páry klíč + čeština), co jde připnout z které karty, stav připnutí ze snapshotu, vrstvy,
// průhlednost, data z `desk-data` přeložená na tvar snapshotu (karty šuplíku se tak dají znovupoužít).
import type { DeskData, DeskLayer, DeskView, DeskWidget, Snapshot } from "../types";

/** Pár [klíč překladu, čeština] — přeloží se až při vykreslení. */
export type Pair = readonly [string, string];

export const WIDGET_PREFIX = "widget:";
/** Vestavěné druhy (pořadí = pořadí v Nastavení a v Ctrl+K). */
export const BUILTIN_KINDS = ["limits", "waiting", "dev", "agenda"] as const;
export type BuiltinKind = (typeof BUILTIN_KINDS)[number];

export const KIND_NAME: Record<BuiltinKind, Pair> = {
  limits: ["cards.desk.kind.limits", "Limity Clauda"],
  waiting: ["cards.desk.kind.waiting", "Čeká na tebe"],
  dev: ["cards.desk.kind.dev", "Dev servery"],
  agenda: ["cards.desk.kind.agenda", "Agenda dne"],
};

export const KIND_HINT: Record<BuiltinKind, Pair> = {
  limits: ["cards.desk.kind.limits.hint", "5h okno a týden, reset a odpočet"],
  waiting: ["cards.desk.kind.waiting.hint", "schválení a otázky Clauda — klik otevře šuplík"],
  dev: ["cards.desk.kind.dev.hint", "co běží, porty, Otevřít a Stop"],
  agenda: ["cards.desk.kind.agenda.hint", "dnešek a zítřek z kalendáře"],
};

export const LAYERS: readonly { v: DeskLayer; name: Pair; hint: Pair }[] = [
  { v: "top", name: ["cards.desk.layer.top", "Navrchu"], hint: ["cards.desk.layer.top.hint", "Přes všechna okna (hře vždy ustoupí)"] },
  { v: "normal", name: ["cards.desk.layer.normal", "Normální"], hint: ["cards.desk.layer.normal.hint", "Jako běžné okno — schová se za aktivní okno"] },
  { v: "desktop", name: ["cards.desk.layer.desktop", "Jen na ploše"], hint: ["cards.desk.layer.desktop.hint", "Pod všemi okny, vidět jen na ploše"] },
];

export const OPACITY_MIN = 30;

export const isScriptKind = (k: string): boolean => k.startsWith(WIDGET_PREFIX) && k.length > WIDGET_PREFIX.length;
export const isBuiltinKind = (k: string): k is BuiltinKind => (BUILTIN_KINDS as readonly string[]).includes(k);
/** Platný druh (zrcadlo Rust `desk::Kind::parse`, bez kontroly jména souboru). */
export const isKind = (k: unknown): k is string => typeof k === "string" && (isBuiltinKind(k) || isScriptKind(k));

/** Co jde připnout na plochu z karty šuplíku (⋯ → Připnout na plochu). Ostatní karty zatím nic. */
export function kindsOfCard(card: string): string[] {
  switch (card) {
    case "claude":
      return ["limits", "waiting"];
    case "running":
      return ["dev"];
    case "calendar":
      return ["agenda"];
    default:
      return isScriptKind(card) ? [card] : [];
  }
}

/** Název druhu: vestavěný = pár, skript = jméno souboru (bez překladu). */
export function kindName(kind: string): Pair | string {
  if (isBuiltinKind(kind)) return KIND_NAME[kind];
  return isScriptKind(kind) ? kind.slice(WIDGET_PREFIX.length) : kind;
}

export function widgetOf(v: DeskView | undefined, kind: string): DeskWidget | null {
  return v?.widgets.find((w) => w.kind === kind) ?? null;
}

/** Jde připnout další (není plno)? Už připnutý druh jde vždy (jen se ukáže). */
export function canPin(v: DeskView | undefined, kind: string): boolean {
  if (!v) return true;
  return !!widgetOf(v, kind) || v.widgets.length < v.max;
}

/** Neprůhlednost do rozsahu 30–100 (zrcadlo Rust `sanitized`). */
export function clampOpacity(o: number): number {
  return Number.isFinite(o) ? Math.min(100, Math.max(OPACITY_MIN, Math.round(o))) : 100;
}

/** Jméno profilu jako v Rustu (`clean_name`): mezery srovnané, nejvýš 40 znaků; prázdné = neplatné. */
export function cleanProfileName(name: string): string {
  return name.split(/\s+/).filter(Boolean).join(" ").slice(0, 40);
}

export const sameName = (a: string, b: string) => a.trim().toLowerCase() === b.trim().toLowerCase();

/** Kolik minut čeká (ne záporné). */
export function waitMinutes(since: number, now: number): number {
  return Math.max(0, Math.floor((now - since) / 60_000));
}

/** Data z `desk-data` ve tvaru snapshotu — jen pole, která čtou znovupoužité karty (Limity, Běží, Kalendář,
 *  widget ze skriptu). Co v datech není, je prázdné (karta pak ukáže svůj prázdný stav). */
export function deskSnapshot(d: DeskData | null): Snapshot {
  const s = {
    mode: "",
    cfgError: d?.cfgError ?? null,
    claudeEnabled: d?.claudeEnabled ?? false,
    limits: d?.limits,
    sessions: d?.working ? [{ status: "working" }] : [],
    procs: d?.procs ?? [],
    projects: (d?.projects ?? []).map((p) => ({ id: p.id, name: p.name })),
    personal: { calendar: d?.calendar },
    widgets: d?.widgets ?? [],
  };
  return s as unknown as Snapshot;
}

/** Běží aspoň jeden dev server (pro hlavičku widgetu)? */
export function liveCount(d: DeskData | null): number {
  return (d?.procs ?? []).filter((p) => p.phase !== "exited" && p.phase !== "failed").length;
}
