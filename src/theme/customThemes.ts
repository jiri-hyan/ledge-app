// Vlastní motivy jako čistá logika bez DOM (testuje `theme.test.mjs`; zrcadlo `appearance/pages.rs`):
//  - vlastní motiv = vestavěný motiv `base` (vrstvy karet a linek) + 11 upravitelných barev `CUSTOM_KEYS`;
//    `resolveCustom` z toho složí úplnou sadu tokenů `COLOR_KEYS` (stejný tvar jako `THEMES[x].vars`),
//    u opačného tónu pozadí než `base` se vrstvy vezmou z `dark` / `light`; odvozené barvy (fialová,
//    tyrkysová, „inkousty“ a text tlačítka) se dotáhnou na kontrast ≥ 4.5:1;
//  - `customProblems` = kontrola čitelnosti textu (stejné funkce jako test motivů, `contrast.ts`); editor
//    uložení s problémem zablokuje;
//  - uložení z aktuálních barev, jména, normalizace configu a export / import motivu jako JSON;
//  - `themeOf` = tokeny a tón motivu pro `Appearance` (vestavěný nebo vlastní) — jediné místo, které to řeší.
import type { CustomTheme, ThemeId } from "../types.ts";
import { THEMES, THEME_IDS, type ColorKey } from "./themes.ts";
import { MIN_CONTRAST, contrast, lum, parseColor, surfaces, textProblems, toHex, type Rgba } from "./contrast.ts";

/** Upravitelné barvy vlastního motivu (zrcadlo `EDITABLE` v `appearance/pages.rs`; všechny povinné). */
export const CUSTOM_KEYS = ["bg", "raised", "t1", "t2", "t3", "accent", "accent-2", "green", "amber", "red", "blue"] as const;
export type CustomKey = (typeof CUSTOM_KEYS)[number];
/** Z nich texty / barvy stavů, které musí na pozadí mít kontrast ≥ 4.5:1 (zrcadlo `TEXT_KEYS`). */
export const CUSTOM_TEXT_KEYS: readonly CustomKey[] = ["t1", "t2", "t3", "green", "amber", "red", "blue"];
export const MAX_CUSTOM_THEMES = 20;
export const MAX_NAME = 32;
/** Svítivost pozadí, nad kterou je motiv světlý (zrcadlo `LIGHT_LUM`). */
export const LIGHT_LUM = 0.18;

const HEX = /^#[0-9a-fA-F]{6}$/;
const isHex6 = (v: unknown): v is string => typeof v === "string" && HEX.test(v);
const isBuiltin = (v: unknown): v is ThemeId => (THEME_IDS as readonly string[]).includes(v as string);
export const isLightBg = (hex: string): boolean => lum(parseColor(hex)) > LIGHT_LUM;
/** Jméno (id) vlastního motivu: 1–24 znaků `a–z 0–9 -`, nezačíná pomlčkou (zrcadlo `custom_id_ok`). */
export const customIdOk = (s: unknown): s is string => typeof s === "string" && /^[a-z0-9][a-z0-9-]{0,23}$/.test(s);

export type Colors = Record<CustomKey, string>;

/** Upravitelné barvy vestavěného motivu jako `#rrggbb` (výchozí bod nového vlastního motivu). */
export function colorsOf(id: ThemeId): Colors {
  const v = THEMES[id]?.vars ?? THEMES.dark.vars;
  return Object.fromEntries(CUSTOM_KEYS.map((k) => [k, toHex(parseColor(v[k as ColorKey]))])) as Colors;
}

/** Nejbližší čitelná varianta barvy: míchá ji k černé (světlý motiv) / bílé, dokud nemá ≥ 4.5:1 na všech plochách. */
function readableOn(hex: string, planes: Rgba[], light: boolean): string {
  const c = parseColor(hex);
  const target = light ? [0, 0, 0] : [255, 255, 255];
  for (let i = 0; i <= 10; i++) {
    const x = [0, 1, 2].map((j) => c[j] + (target[j] - c[j]) * (i / 10));
    if (planes.every((p) => contrast([x[0], x[1], x[2], 1], p) >= MIN_CONTRAST)) return toHex(x);
  }
  return toHex(target);
}

export type Resolved = { vars: Record<ColorKey, string>; light: boolean };

/** Úplná sada tokenů vlastního motivu a jeho tón (světlý podle svítivosti pozadí). */
export function resolveCustom(c: CustomTheme): Resolved {
  const col = c.colors;
  const light = isLightBg(col.bg);
  const baseDef = THEMES[c.base] ?? THEMES.dark;
  const base = baseDef.light === light ? baseDef : THEMES[light ? "light" : "dark"];
  const [r, g, b] = parseColor(col.bg);
  const [pr, pg, pb] = parseColor(col.raised);
  const v: Record<ColorKey, string> = {
    ...base.vars,
    bg: `${r} ${g} ${b}`,
    raised: col.raised,
    pop: `rgba(${pr}, ${pg}, ${pb}, 0.98)`,
    t1: col.t1, t2: col.t2, t3: col.t3,
    accent: col.accent, "accent-2": col["accent-2"], focus: col["accent-2"],
    green: col.green, amber: col.amber, red: col.red, blue: col.blue,
  };
  const planes = surfaces(v);
  for (const k of ["violet", "teal", "amber-ink", "red-ink"] as const) v[k] = readableOn(v[k], planes, light);
  const t1 = parseColor(v.t1);
  v["on-light"] = contrast(parseColor("#111111"), t1) >= contrast(parseColor("#ffffff"), t1) ? "#111111" : "#ffffff";
  return { vars: v, light };
}

/** Tokeny a tón motivu vzhledu: vybraný vlastní motiv (existuje-li), jinak vestavěný `theme`. */
export function themeOf(a: { theme: ThemeId; customTheme: string; customThemes: readonly CustomTheme[] }): Resolved {
  const c = a.customTheme ? a.customThemes.find((x) => x.id === a.customTheme) : undefined;
  if (c) return resolveCustom(c);
  const d = THEMES[a.theme] ?? THEMES.dark;
  return { vars: d.vars, light: d.light };
}

export type Problem = { key: CustomKey; on: "bg" | "raised" | "card"; ratio: number };

/** Texty vlastního motivu s kontrastem pod 4.5:1 (na pozadí, na `raised` a na kartách nad tmavou i světlou
 *  plochou — stejná kontrola jako test motivů). Prázdné = lze uložit. Nejhorší poměr na klíč. */
export function customProblems(c: CustomTheme): Problem[] {
  const { vars } = resolveCustom(c);
  const worst = new Map<CustomKey, Problem>();
  const add = (key: CustomKey, on: Problem["on"], ratio: number) => {
    if (ratio < MIN_CONTRAST && (worst.get(key)?.ratio ?? Infinity) > ratio) worst.set(key, { key, on, ratio });
  };
  for (const p of textProblems(vars, CUSTOM_TEXT_KEYS)) add(p.key === "on-light" ? "t1" : (p.key as CustomKey), "card", p.ratio);
  for (const k of CUSTOM_TEXT_KEYS) {
    const fg = parseColor(c.colors[k]);
    add(k, "bg", contrast(fg, parseColor(c.colors.bg)));
    add(k, "raised", contrast(fg, parseColor(c.colors.raised)));
  }
  return CUSTOM_TEXT_KEYS.flatMap((k) => (worst.has(k) ? [worst.get(k)!] : []));
}

/** Platné vlastní motivy z configu: neplatné záznamy pryč, chybějící / vadné barvy z `base`, max 20. */
export function normalizeCustomThemes(raw: unknown): CustomTheme[] {
  if (!Array.isArray(raw)) return [];
  const out: CustomTheme[] = [];
  for (const r of raw as any[]) {
    if (out.length >= MAX_CUSTOM_THEMES) break;
    if (!r || !customIdOk(r.id) || isBuiltin(r.id) || out.some((x) => x.id === r.id)) continue;
    const base: ThemeId = isBuiltin(r.base) ? r.base : "dark";
    const fb = colorsOf(base);
    const colors = Object.fromEntries(CUSTOM_KEYS.map((k) => [k, isHex6(r.colors?.[k]) ? r.colors[k].toLowerCase() : fb[k]])) as Colors;
    const name = typeof r.name === "string" && r.name.trim() ? r.name.trim().slice(0, MAX_NAME) : r.id;
    out.push({ id: r.id, name, base, colors });
  }
  return out;
}

/** Nové volné `id` z názvu: `my-` + písmena a číslice názvu, při shodě s číslem na konci. */
export function newThemeId(existing: readonly string[], name: string): string {
  const slug = name.normalize("NFD").replace(/\p{M}/gu, "").toLowerCase().replace(/[^a-z0-9]+/g, "-").replace(/^-+|-+$/g, "").slice(0, 16);
  const stem = `my-${slug.replace(/-+$/, "") || "motiv"}`;
  let id = stem;
  for (let i = 2; existing.includes(id); i++) id = `${stem}-${i}`;
  return id;
}

/** Přidá motiv z `colors` (např. z aktuálního vzhledu); `null` = už je jich 20. */
export function addCustomTheme(list: readonly CustomTheme[], name: string, base: ThemeId, colors: Colors): { list: CustomTheme[]; id: string } | null {
  if (list.length >= MAX_CUSTOM_THEMES) return null;
  const clean = name.trim().slice(0, MAX_NAME) || "Motiv";
  const id = newThemeId(list.map((c) => c.id), clean);
  return { list: [...list, { id, name: clean, base, colors: { ...colors } }], id };
}

/** Změna jednoho motivu v seznamu (barvy se čistí na malá písmena, vadné hodnoty se ignorují). */
export function updateCustomTheme(list: readonly CustomTheme[], id: string, patch: { name?: string; colors?: Partial<Colors> }): CustomTheme[] {
  return list.map((c) => {
    if (c.id !== id) return c;
    const name = patch.name === undefined ? c.name : patch.name.trim().slice(0, MAX_NAME) || c.name;
    const colors = { ...c.colors };
    for (const [k, v] of Object.entries(patch.colors ?? {})) if ((CUSTOM_KEYS as readonly string[]).includes(k) && isHex6(v)) colors[k as CustomKey] = v.toLowerCase();
    return { ...c, name, colors };
  });
}

// ---------- export / import (schránka) ----------
export const THEME_FILE_TAG = "ledgeTheme";

/** JSON motivu pro schránku (jméno, základ a barvy — `id` se při importu přidělí nové). */
export const exportTheme = (c: CustomTheme): string => JSON.stringify({ [THEME_FILE_TAG]: 1, name: c.name, base: c.base, colors: c.colors }, null, 2);

export type ImportResult =
  | { ok: true; theme: CustomTheme }
  | { ok: false; error: "json" | "format" | "full" | "contrast"; keys?: CustomKey[] };

/** Motiv ze schránky: musí být JSON ve tvaru exportu, mít všech 11 barev `#rrggbb` a čitelný text. */
export function importTheme(text: string, existing: readonly CustomTheme[]): ImportResult {
  let raw: any;
  try {
    raw = JSON.parse(text);
  } catch {
    return { ok: false, error: "json" };
  }
  if (!raw || typeof raw !== "object" || raw[THEME_FILE_TAG] !== 1 || typeof raw.colors !== "object" || !raw.colors) return { ok: false, error: "format" };
  if (CUSTOM_KEYS.some((k) => !isHex6(raw.colors[k]))) return { ok: false, error: "format" };
  if (existing.length >= MAX_CUSTOM_THEMES) return { ok: false, error: "full" };
  const base: ThemeId = isBuiltin(raw.base) ? raw.base : "dark";
  const name = (typeof raw.name === "string" && raw.name.trim() ? raw.name.trim() : "Motiv").slice(0, MAX_NAME);
  const colors = Object.fromEntries(CUSTOM_KEYS.map((k) => [k, (raw.colors[k] as string).toLowerCase()])) as Colors;
  const theme: CustomTheme = { id: newThemeId(existing.map((c) => c.id), name), name, base, colors };
  const bad = customProblems(theme);
  return bad.length ? { ok: false, error: "contrast", keys: bad.map((p) => p.key) } : { ok: true, theme };
}
