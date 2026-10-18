// Čistá simulace pružiny → CSS `linear()` (bez DOM; testuje `theme.test.mjs`). Používá ji `motion.ts`
// `spring()` (styl a rychlost z vzhledu, cache) i plán otevírání `openAnim.ts` (styl zadaný výslovně —
// náhled animace v Nastavení přehraje jiný styl, než je zrovna uložený).
import type { MotionStyle } from "../types.ts";
import { springPreset } from "./appearance.ts";

export interface Spring {
  easing: string;
  duration: number;
}

/**
 * Tlumená pružina → CSS `linear()` easing. Simulace pohybové rovnice (m·x'' = −k(x−1) − c·x'),
 * vzorkovaná, dokud se neusadí. Overshoot vzniká přirozeně z fyziky, ne ručně laděnou bezierkou.
 * Styl mění jen parametry (`springPreset`), `none` = délka 0; `speed` dělí délku.
 */
export function simulateSpring(style: MotionStyle, speed: number, stiffness = 300, damping = 24, mass = 1): Spring {
  const p = springPreset(style, stiffness, damping, mass);
  if (!p) return { easing: "linear", duration: 0 };
  ({ stiffness, damping } = p);
  const k = Number.isFinite(speed) && speed > 0 ? speed : 1;
  const dt = 1 / 240;
  let x = 0;
  let v = 0;
  let t = 0;
  const pts: number[] = [0];
  let still = 0;
  while (t < 2) {
    const a = (-stiffness * (x - 1) - damping * v) / mass;
    v += a * dt;
    x += v * dt;
    t += dt;
    if (Math.round(t * 240) % 4 === 0) pts.push(x);
    if (Math.abs(x - 1) < 0.0015 && Math.abs(v) < 0.02) {
      if (++still > 12) break;
    } else still = 0;
  }
  pts.push(1);
  return { easing: `linear(${pts.map((v) => +v.toFixed(4)).join(", ")})`, duration: Math.round((t * 1000) / k) };
}
