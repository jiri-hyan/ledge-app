// Geometrie budíků a prstenců (kolo 25, karty Claude a Systém): půlkruhový budík s ručičkou a soustředné prstence.
// Čisté funkce bez DOM (test `dial.test.mjs`). Podíl 0–1 mimo rozsah se ořízne, nečíslo = null (nic se nekreslí).
const r2 = (n: number) => Math.round(n * 100) / 100;

/** Podíl 0–1 oříznutý do rozsahu; chybějící / nekonečné číslo = null. */
export const frac01 = (v: number | null | undefined): number | null => (typeof v === "number" && Number.isFinite(v) ? Math.max(0, Math.min(1, v)) : null);

/** Bod na kružnici; `deg` 0 = nahoře (12 hodin), po směru hodin. */
export function polar(cx: number, cy: number, r: number, deg: number): { x: number; y: number } {
  const a = (deg * Math.PI) / 180;
  return { x: r2(cx + r * Math.sin(a)), y: r2(cy - r * Math.cos(a)) };
}

/** Úhel budíku v stupních od svislice: 0 → −90° (vlevo), 1 → +90° (vpravo). */
export const dialDeg = (f: number): number => -90 + 180 * f;

/** Oblouk půlkruhového budíku od podílu `f0` po `f1` (SVG `d`); prázdný / obrácený rozsah = "". */
export function dialPath(cx: number, cy: number, r: number, f0: number, f1: number): string {
  const a = frac01(f0);
  const b = frac01(f1);
  if (a === null || b === null || b <= a) return "";
  const p0 = polar(cx, cy, r, dialDeg(a));
  const p1 = polar(cx, cy, r, dialDeg(b));
  return `M${p0.x} ${p0.y} A${r} ${r} 0 0 1 ${p1.x} ${p1.y}`;
}

/** Konec ručičky budíku. */
export const needleEnd = (cx: number, cy: number, len: number, f: number): { x: number; y: number } => polar(cx, cy, len, dialDeg(frac01(f) ?? 0));

/** Prstenec poloměru `r`: obvod a `stroke-dasharray` pro podíl (oblouk od 12 hodin po směru hodin); null = nic. */
export function ringDash(r: number, f: number | null | undefined): { c: number; dash: string } | null {
  const v = frac01(f);
  if (v === null) return null;
  const c = r2(2 * Math.PI * r);
  return { c, dash: `${r2(c * v)} ${c}` };
}

/** Poloměry `n` soustředných prstenců ve čtverci `size` (vnější první) s tloušťkou `stroke` a mezerou `gap`. */
export function concentric(n: number, size: number, stroke: number, gap: number): number[] {
  const out: number[] = [];
  for (let i = 0; i < n; i++) {
    const r = size / 2 - stroke / 2 - i * (stroke + gap);
    if (r > stroke / 2) out.push(r2(r));
  }
  return out;
}

/** Barevná úroveň podle procent a mezí (od `warn` pozor, od `bad` plno). */
export const toneOf = (pct: number | null | undefined, warn: number, bad: number): "ok" | "warn" | "bad" => (pct == null ? "ok" : pct >= bad ? "bad" : pct >= warn ? "warn" : "ok");
