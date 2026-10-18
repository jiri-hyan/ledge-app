// Čistá logika bubliny MSIX pro UI (bez DOM, testy `model.test.mjs`).
import type { PkgView } from "./pkgctx";

export interface Bubble {
  /** jméno aplikace, ve které Ledge běží („Claude“) */
  app: string;
  /** rodina balíčku (`Claude_pzs8sxrjxfjjc`) */
  family: string;
  /** kde config z bubliny fyzicky leží; null = nevíme (neukazovat) */
  bubbleDir: string | null;
  /** kam se config ukládá mimo bublinu (jak ho vidí Ledge) */
  configDir: string;
}

/** Stav z Rustu → co ukázat; `null` = mimo bublinu nebo ještě nenačteno (nic neukazovat). */
export function bubbleOf(v: PkgView | null | undefined): Bubble | null {
  if (!v?.package) return null;
  const p = v.package;
  const family = (p.family || p.full || "").trim();
  const app = (p.name || "").trim() || family.replace(/_[^_]*$/, "") || family;
  if (!app) return null;
  return { app, family, bubbleDir: v.bubbleConfigDir?.trim() || null, configDir: v.configDir };
}
