// Styly lístku (Lístky 2.0) — čistá logika bez DOM (testy `styles.test.mjs`): galerie stylů a CSS proměnné
// lístku z (styl, pozadí %, text %, barva). `classic` = původní tmavé sklo (`pinColors`), výchozí vzhled.
// Rust `pins::PinStyle` má stejná id; neznámé = classic.
import { pinColors, TEXT_MIN } from "./model.ts";

export type PinStyleId = "classic" | "paper" | "glass" | "stripe" | "minimal" | "dark";
export const PIN_STYLES: PinStyleId[] = ["classic", "paper", "glass", "stripe", "minimal", "dark"];
export const isPinStyle = (s: unknown): s is PinStyleId => PIN_STYLES.includes(s as PinStyleId);

const clamp = (v: number, lo: number, hi: number) => Math.min(hi, Math.max(lo, Number.isFinite(v) ? v : hi));
const r2 = (v: number) => Math.round(v * 100) / 100;

/** Výchozí barva stylu bez vlastní barvy lístku (papír = žlutý, proužek = modrý). */
const BASE_TINT: Partial<Record<PinStyleId, string>> = { paper: "#f5c542", stripe: "#7ab8ff" };

/** CSS proměnné lístku: `--pin-bg`, `--pin-bar`, `--pin-fg`, `--pin-shadow`, `--pin-tint` (barva / prázdné),
 *  `--pin-line`. Pozadí a text (%) platí ve všech stylech. */
export function pinLook(style: string | undefined, bg: number, text: number, color: string | null): Record<string, string> {
  const s: PinStyleId = isPinStyle(style) ? style : "classic";
  const a = clamp(bg, 0, 100) / 100;
  const tx = clamp(text, TEXT_MIN, 100) / 100;
  const tint = color ?? BASE_TINT[s] ?? "";
  const base = pinColors(bg, text);
  // barva: přimíchá se do pozadí lišty / plochy (`color-mix`, WebView2 = Chromium)
  const mix = (pct: number, over: string) => (tint ? `color-mix(in srgb, ${tint} ${pct}%, ${over})` : over);
  switch (s) {
    case "paper": {
      const paper = `rgba(255, 252, 240, ${r2(Math.max(a, 0.25))})`;
      return { "--pin-bg": mix(45, paper), "--pin-bar": mix(65, paper), "--pin-fg": `rgba(28, 26, 22, ${r2(tx)})`, "--pin-shadow": "none", "--pin-tint": tint, "--pin-line": "rgba(0, 0, 0, 0.12)" };
    }
    case "glass": {
      const glass = `rgba(255, 255, 255, ${r2(a * 0.18)})`;
      return { "--pin-bg": mix(18, glass), "--pin-bar": mix(30, `rgba(255, 255, 255, ${r2(a * 0.26)})`), "--pin-fg": base.color, "--pin-shadow": "0 1px 2px rgba(0, 0, 0, 0.9), 0 0 3px rgba(0, 0, 0, 0.8)", "--pin-tint": tint, "--pin-line": "rgba(255, 255, 255, 0.35)" };
    }
    case "stripe":
      return { "--pin-bg": base.background, "--pin-bar": base.bar, "--pin-fg": base.color, "--pin-shadow": base.shadow, "--pin-tint": tint, "--pin-line": "rgba(255, 255, 255, 0.14)" };
    case "minimal":
      return { "--pin-bg": base.background, "--pin-bar": "transparent", "--pin-fg": base.color, "--pin-shadow": base.shadow, "--pin-tint": tint, "--pin-line": "transparent" };
    case "dark": {
      const dark = `rgba(8, 9, 12, ${r2(Math.max(a, 0.6))})`;
      return { "--pin-bg": dark, "--pin-bar": mix(22, dark), "--pin-fg": `rgba(255, 255, 255, ${r2(tx)})`, "--pin-shadow": "none", "--pin-tint": tint, "--pin-line": "rgba(255, 255, 255, 0.22)" };
    }
    default:
      return { "--pin-bg": mix(14, base.background), "--pin-bar": mix(40, base.bar), "--pin-fg": base.color, "--pin-shadow": base.shadow, "--pin-tint": tint, "--pin-line": "rgba(255, 255, 255, 0.14)" };
  }
}
