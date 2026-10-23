// Čistá logika UI aktualizací (bez DOM, testy `model.test.mjs`): bezpečný rozbor poznámek k vydání na
// bloky (vykreslí se jako textové uzly — žádné HTML z vydání, odkazy jen jako text), průběh tichého
// stažení jen ze skutečných bajtů a co nabídnout v panelu „Nová verze“. Stav dělá Rust (`updater.rs`).
import type { UpdateFetch } from "../types/settings";

export type Inline = { k: "t" | "code" | "b"; s: string };
export type Block = { k: "h" | "li" | "p"; parts: Inline[] };

/** Nejvýš tolik bloků — dlouhé poznámky se v panelu stejně posouvají, ať rozbor nic nezahltí. */
const MAX_BLOCKS = 200;

/** `**tučně**` a `` `kód` `` → části; odkazy `[text](url)` → jen text; HTML značky pryč (zbytek je text). */
export function inline(s: string): Inline[] {
  const clean = s.replace(/<[^>]*>/g, "").replace(/!?\[([^\]]*)\]\([^)]*\)/g, "$1");
  const out: Inline[] = [];
  const re = /`([^`]+)`|\*\*([^*]+)\*\*/g;
  let at = 0;
  for (let m = re.exec(clean); m; m = re.exec(clean)) {
    if (m.index > at) out.push({ k: "t", s: clean.slice(at, m.index) });
    out.push(m[1] !== undefined ? { k: "code", s: m[1] } : { k: "b", s: m[2] });
    at = m.index + m[0].length;
  }
  if (at < clean.length) out.push({ k: "t", s: clean.slice(at) });
  return out.filter((p) => p.s.length > 0);
}

/** Markdown poznámek (nadpisy, odrážky, odstavce) → bloky. Prázdné řádky a čistě HTML řádky zmizí. */
export function parseNotes(md: string): Block[] {
  const out: Block[] = [];
  for (const raw of md.replace(/\r/g, "").split("\n")) {
    if (out.length >= MAX_BLOCKS) break;
    const line = raw.trim();
    if (!line) continue;
    const h = /^#{1,6}\s+(.*)$/.exec(line);
    const li = /^(?:[-*+]|\d+[.)])\s+(.*)$/.exec(line);
    const [k, text]: [Block["k"], string] = h ? ["h", h[1]] : li ? ["li", li[1]] : ["p", line];
    const parts = inline(text);
    if (parts.length) out.push({ k, parts });
  }
  return out;
}

/** Celá procenta staženého, `null` = velikost neznámá (neurčitý průběh) nebo se nestahuje. */
export function progressPct(f: UpdateFetch): number | null {
  if (f.state !== "downloading" || !f.total || f.total <= 0) return null;
  return Math.floor((Math.min(f.got, f.total) * 100) / f.total);
}

/** Staženo v MB s jedním desetinným místem (z bajtů, které opravdu přišly). */
export function mb(bytes: number): string {
  return (Math.max(0, bytes) / (1024 * 1024)).toFixed(1);
}

/** Hlavní tlačítko panelu „Nová verze“. */
export type Action = "restart" | "downloading" | "retry" | "install";

/** Připraveno → „Restartovat a aktualizovat“; stahuje → zamčeno s průběhem; selhalo → „Zkusit znovu“
 *  (a jako záloha i stažení na klik); jinak (tiché stažení vypnuté / čeká) → „Stáhnout a nainstalovat“. */
export function action(f: UpdateFetch): Action {
  switch (f.state) {
    case "ready": return "restart";
    case "downloading": return "downloading";
    case "failed": return "retry";
    default: return "install";
  }
}
