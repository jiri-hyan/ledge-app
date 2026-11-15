// Motiv „Podle Windows“ (kolo 34, `appearance.auto`; zrcadlo `appearance/auto.rs`): čistá logika bez DOM.
// Stav Windows (`WinLook`: tón z `AppsUseLightTheme`, akcent z DWM) dodává Rust (`win_look` + událost `winlook`,
// `winlook.rs`, žádný polling). `resolveAuto` z uloženého vzhledu a stavu Windows složí EFEKTIVNÍ vzhled:
// u zapnutého `auto.on` vybraný motiv dvojice (vlastní motiv se přeskočí), u `auto.accent` akcent Windows
// doladěný na čitelnost proti motivu (`fitAccent`). Uložený vzhled se nemění (editor ukazuje volbu uživatele).
import type { Appearance, AutoLook, ThemeId } from "../types.ts";
import { THEME_IDS, THEMES } from "./themes.ts";
import { fitAccent } from "./backdrop.ts";

/** Stav Windows z Rustu; `null` = nezjištěno. */
export type WinLook = { light: boolean | null; accent: string | null };
export const NO_WIN: WinLook = { light: null, accent: null };

export const DEFAULT_AUTO: AutoLook = { on: false, light: "light", dark: "dark", accent: false };

const hex6 = (v: unknown): v is string => typeof v === "string" && /^#[0-9a-fA-F]{6}$/.test(v);
const theme = (v: unknown, d: ThemeId): ThemeId => ((THEME_IDS as readonly string[]).includes(v as string) ? (v as ThemeId) : d);

/** Config → platná volba; neplatné = výchozí. */
export function normalizeAuto(raw: any): AutoLook {
  return {
    on: raw?.on === true,
    light: theme(raw?.light, DEFAULT_AUTO.light),
    dark: theme(raw?.dark, DEFAULT_AUTO.dark),
    accent: raw?.accent === true,
  };
}

export const isDefaultAuto = (a: AutoLook): boolean => a.on === DEFAULT_AUTO.on && a.light === DEFAULT_AUTO.light && a.dark === DEFAULT_AUTO.dark && a.accent === DEFAULT_AUTO.accent;

/** Efektivní vzhled podle Windows (bez `auto` / bez údajů Windows vrací tentýž objekt). */
export function resolveAuto(a: Appearance, win: WinLook): Appearance {
  const auto = a.auto ?? DEFAULT_AUTO;
  const id = auto.on && win.light !== null ? (win.light ? auto.light : auto.dark) : null;
  const acc = auto.accent && hex6(win.accent) ? win.accent.toLowerCase() : null;
  if (!id && !acc) return a;
  const out: Appearance = { ...a };
  if (id) {
    out.theme = id;
    out.customTheme = "";
  }
  if (acc) {
    out.accent = fitAccent(acc, THEMES[out.theme].vars);
    delete out.accent2;
    delete out.accentFromBackdrop;
  }
  return out;
}
