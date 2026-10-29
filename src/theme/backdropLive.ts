// Živá pozadí otevřeného šuplíku (kolo 25): šest kreslicích jednotek pro `<canvas>` za kartami. Kreslí se v ½
// rozlišení, každé s vlastním stropem snímků (`LIVE_FPS`, nejvýš 30) a jen z `Backdrop.tsx`, když je šuplík
// otevřený a viditelný (`backdrop.ts` `drawMode`). Jednotka = `frame(t, dt)` bez vlastních časovačů — smyčku
// (rAF) řídí `backdropLoop.ts`. Vše deterministické (náhodná čísla ze semínka), takže statický snímek při
// „omezit pohyb“ vypadá pokaždé stejně. Žádné DOM mimo canvas, žádná síť.
import type { FlameColor, LiveKind } from "../types.ts";
import { FLAME_COLORS } from "./backdropPresets.ts";

export type LiveOpts = { intensity: number; speed: number; flame: FlameColor };
export interface Live {
  frame(t: number, dt: number): void;
}

/** Snímků za vteřinu podle druhu (pomalé děje stačí méně; nad 30 se nejde). */
export const LIVE_FPS: Record<LiveKind, number> = { fire: 30, aurora: 20, lava: 15, stars: 20, rain: 30, grain: 12 };

/** Kolik kroků simulace je potřeba pro věrný statický snímek (oheň se musí „rozhořet“). */
export const STILL_WARMUP: Record<LiveKind, number> = { fire: 90, aurora: 0, lava: 0, stars: 0, rain: 0, grain: 0 };

/** Deterministický generátor 0–1 (mulberry32). */
export function rng(seed: number): () => number {
  let a = seed >>> 0;
  return () => {
    a = (a + 0x6d2b79f5) >>> 0;
    let t = a;
    t = Math.imul(t ^ (t >>> 15), t | 1);
    t ^= t + Math.imul(t ^ (t >>> 7), t | 61);
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
  };
}

const rgb = (h: string): [number, number, number] => {
  const n = parseInt(h.slice(1), 16);
  return [(n >> 16) & 255, (n >> 8) & 255, n & 255];
};
const lerp = (a: number, b: number, t: number) => a + (b - a) * t;
const smooth = (a: number, b: number, x: number) => {
  const t = Math.max(0, Math.min(1, (x - a) / (b - a)));
  return t * t * (3 - 2 * t);
};

type Ctx = CanvasRenderingContext2D;

/** Malé plátno pro simulace v mřížce, které se roztáhne na celou plochu (hladké zvětšení = měkké hrany). */
function lowRes(gw: number, gh: number) {
  const c = document.createElement("canvas");
  c.width = gw;
  c.height = gh;
  const x = c.getContext("2d")!;
  return { c, x, img: x.createImageData(gw, gh) };
}

// ---------- oheň ----------
export function fire(ctx: Ctx, w: number, h: number, o: LiveOpts): Live {
  const GW = 72, GH = 40;
  const heat = new Float32Array(GW * GH);
  const lr = lowRes(GW, GH);
  const r = rng(11);
  const pal = FLAME_COLORS[o.flame].map(rgb);
  // výška plamenů: intenzita 0.1–1 → 18–60 % výšky; chlazení na řádek tomu odpovídá
  const cool = 1 / (GH * lerp(0.18, 0.6, o.intensity)) * 1.15;
  const sparks: { x: number; y: number; vy: number; ph: number; life: number }[] = [];
  let acc = 1;
  const maxSparks = Math.round(10 + 22 * o.intensity);
  return {
    frame(t, dt) {
      acc += dt * 30 * o.speed;
      const steps = Math.floor(acc);
      acc -= steps;
      for (let s = 0; s < steps; s++) {
        for (let x = 0; x < GW; x++) heat[(GH - 1) * GW + x] = 0.75 + 0.25 * r();
        for (let y = 0; y < GH - 1; y++) {
          for (let x = 0; x < GW; x++) {
            const below = (y + 1) * GW;
            const sum = heat[below + x] * 2 + heat[below + ((x + GW - 1) % GW)] + heat[below + ((x + 1) % GW)];
            heat[y * GW + x] = Math.max(0, sum / 4 - cool * (0.6 + 0.8 * r()));
          }
        }
      }
      const d = lr.img.data;
      for (let i = 0; i < GW * GH; i++) {
        const v = Math.min(1, heat[i] * 1.25);
        const k = v < 0.5 ? v * 2 : (v - 0.5) * 2;
        const [a, b] = v < 0.5 ? [pal[0], pal[1]] : [pal[1], pal[2]];
        d[i * 4] = lerp(a[0], b[0], k);
        d[i * 4 + 1] = lerp(a[1], b[1], k);
        d[i * 4 + 2] = lerp(a[2], b[2], k);
        d[i * 4 + 3] = 255 * Math.min(1, smooth(0.04, 0.5, v) * (0.55 + 0.4 * o.intensity));
      }
      lr.x.putImageData(lr.img, 0, 0);
      ctx.clearRect(0, 0, w, h);
      ctx.imageSmoothingEnabled = true;
      ctx.drawImage(lr.c, 0, 0, w, h);
      // jiskry: vznášejí se nahoru, vlní se a hasnou
      if (sparks.length < maxSparks && r() < 0.35 * o.speed) sparks.push({ x: r() * w, y: h, vy: lerp(0.25, 0.7, r()) * h, ph: r() * 6.28, life: 1 });
      ctx.globalCompositeOperation = "lighter";
      ctx.fillStyle = FLAME_COLORS[o.flame][2];
      for (let i = sparks.length - 1; i >= 0; i--) {
        const s = sparks[i];
        s.y -= s.vy * dt * o.speed;
        s.life -= dt * 0.5 * o.speed;
        if (s.life <= 0 || s.y < 0) {
          sparks.splice(i, 1);
          continue;
        }
        ctx.globalAlpha = s.life * 0.9;
        const sz = 1 + s.life * 1.2;
        ctx.fillRect(s.x + Math.sin(t * 2.2 + s.ph) * 6, s.y, sz, sz);
      }
      ctx.globalAlpha = 1;
      ctx.globalCompositeOperation = "source-over";
    },
  };
}

// ---------- polární záře ----------
export function aurora(ctx: Ctx, w: number, h: number, o: LiveOpts): Live {
  const mk = (stops: [number, string][]) => {
    const c = document.createElement("canvas");
    c.width = 1;
    c.height = 64;
    const x = c.getContext("2d")!;
    const g = x.createLinearGradient(0, 0, 0, 64);
    for (const [p, col] of stops) g.addColorStop(p, col);
    x.fillStyle = g;
    x.fillRect(0, 0, 1, 64);
    return c;
  };
  const layers = [
    { strip: mk([[0, "rgba(52,211,153,0)"], [0.35, "rgba(52,211,153,0.55)"], [1, "rgba(20,184,166,0)"]]), y: 0.1, ph: 0, f: 0.011 },
    { strip: mk([[0, "rgba(129,140,248,0)"], [0.4, "rgba(129,140,248,0.45)"], [1, "rgba(99,102,241,0)"]]), y: 0.2, ph: 2.1, f: 0.016 },
    { strip: mk([[0, "rgba(192,132,252,0)"], [0.45, "rgba(192,132,252,0.35)"], [1, "rgba(168,85,247,0)"]]), y: 0.3, ph: 4.2, f: 0.009 },
  ];
  const step = 3;
  return {
    frame(t) {
      const s = t * 0.35 * o.speed;
      ctx.clearRect(0, 0, w, h);
      ctx.globalCompositeOperation = "lighter";
      ctx.globalAlpha = 0.35 + 0.65 * o.intensity;
      for (const L of layers) {
        for (let x = 0; x < w; x += step) {
          const top = h * L.y + Math.sin(x * L.f + s + L.ph) * h * 0.09 + Math.sin(x * L.f * 2.7 - s * 0.7) * h * 0.035;
          const len = h * (0.42 + 0.14 * Math.sin(x * 0.019 + s * 0.6 + L.ph));
          ctx.drawImage(L.strip, x, top, step, len);
        }
      }
      ctx.globalAlpha = 1;
      ctx.globalCompositeOperation = "source-over";
    },
  };
}

// ---------- lávová lampa ----------
export function lava(ctx: Ctx, w: number, h: number, o: LiveOpts): Live {
  const GW = 56, GH = 32;
  const lr = lowRes(GW, GH);
  const r = rng(7);
  const blobs = Array.from({ length: 7 }, () => ({ fx: lerp(0.05, 0.14, r()), fy: lerp(0.07, 0.17, r()), px: r() * 6.28, py: r() * 6.28, rad: lerp(0.12, 0.22, r()), x0: r(), hot: r() }));
  const [c1, c2] = [rgb("#ff4d2e"), rgb("#ffb347")];
  return {
    frame(t) {
      const s = t * o.speed;
      const pos = blobs.map((b) => ({ x: (b.x0 * 0.6 + 0.2 + 0.2 * Math.sin(s * b.fx * 6.28 + b.px)) * GW, y: (0.5 + 0.42 * Math.sin(s * b.fy * 6.28 + b.py)) * GH, r2: (b.rad * GH) ** 2, hot: b.hot }));
      const d = lr.img.data;
      for (let y = 0; y < GH; y++) {
        for (let x = 0; x < GW; x++) {
          let f = 0, hot = 0;
          for (const b of pos) {
            const dx = (x - b.x) * 0.62, dy = y - b.y;
            const v = b.r2 / (dx * dx + dy * dy + 1);
            f += v;
            hot += v * b.hot;
          }
          const i = (y * GW + x) * 4;
          const k = hot / Math.max(f, 1e-6);
          d[i] = lerp(c1[0], c2[0], k);
          d[i + 1] = lerp(c1[1], c2[1], k);
          d[i + 2] = lerp(c1[2], c2[2], k);
          d[i + 3] = 255 * smooth(0.85, 1.5, f) * (0.3 + 0.5 * o.intensity);
        }
      }
      lr.x.putImageData(lr.img, 0, 0);
      ctx.clearRect(0, 0, w, h);
      ctx.imageSmoothingEnabled = true;
      ctx.drawImage(lr.c, 0, 0, w, h);
    },
  };
}

// ---------- hvězdy s paralaxou ----------
export function stars(ctx: Ctx, w: number, h: number, o: LiveOpts): Live {
  const r = rng(3);
  const area = (w * h) / 40000;
  const depth = [
    { n: Math.round((22 + 40 * o.intensity) * area), v: 3, s: 0.8, a: 0.55 },
    { n: Math.round((14 + 26 * o.intensity) * area), v: 8, s: 1.2, a: 0.75 },
    { n: Math.round((6 + 14 * o.intensity) * area), v: 18, s: 1.8, a: 0.95 },
  ].map((d) => ({ ...d, p: Array.from({ length: Math.max(3, d.n) }, () => ({ x: r() * w, y: r() * h, ph: r() * 6.28, tw: lerp(0.6, 2.2, r()) })) }));
  return {
    frame(t) {
      ctx.clearRect(0, 0, w, h);
      ctx.fillStyle = "#e6eeff";
      for (const L of depth) {
        for (const s of L.p) {
          const x = (((s.x + t * L.v * o.speed) % w) + w) % w;
          const y = s.y + Math.sin(t * 0.15 * o.speed + s.ph) * L.s * 2;
          ctx.globalAlpha = L.a * (0.55 + 0.45 * Math.sin(t * s.tw * o.speed + s.ph));
          ctx.fillRect(x, y, L.s, L.s);
        }
      }
      ctx.globalAlpha = 1;
    },
  };
}

// ---------- déšť na skle ----------
export function rain(ctx: Ctx, w: number, h: number, o: LiveOpts): Live {
  const r = rng(5);
  const n = Math.round((24 + 70 * o.intensity) * (w * h) / 60000);
  const drops = Array.from({ length: Math.max(6, n) }, () => ({ x: r() * w, y: r() * h, len: lerp(6, 16, r()), v: lerp(0.8, 1.6, r()) }));
  const beads = Array.from({ length: Math.max(5, Math.round(12 + 20 * o.intensity)) }, () => ({ x: r() * w, y: r() * h, rad: lerp(1.2, 3, r()), v: lerp(0.02, 0.12, r()) }));
  return {
    frame(_t, dt) {
      ctx.clearRect(0, 0, w, h);
      const slant = 0.22;
      ctx.lineWidth = 1;
      ctx.strokeStyle = "rgba(170,195,235,0.38)";
      ctx.beginPath();
      for (const d of drops) {
        d.y += d.v * h * 0.9 * dt * o.speed;
        d.x -= d.v * h * 0.9 * dt * o.speed * slant;
        if (d.y > h + d.len) ((d.y = -d.len), (d.x = r() * (w + h * slant)));
        ctx.moveTo(d.x, d.y);
        ctx.lineTo(d.x + d.len * slant, d.y - d.len);
      }
      ctx.stroke();
      // kapky na skle: pomalu stékají a občas zmizí dole
      for (const b of beads) {
        b.y += b.v * h * 0.05 * dt * o.speed;
        if (b.y > h + 4) ((b.y = -4), (b.x = r() * w));
        ctx.fillStyle = "rgba(190,210,245,0.16)";
        ctx.beginPath();
        ctx.arc(b.x, b.y, b.rad, 0, 6.2832);
        ctx.fill();
        ctx.fillStyle = "rgba(255,255,255,0.35)";
        ctx.fillRect(b.x - b.rad * 0.35, b.y - b.rad * 0.5, 1, 1);
      }
    },
  };
}

// ---------- filmové zrno ----------
export function grain(ctx: Ctx, w: number, h: number, o: LiveOpts): Live {
  const r = rng(9);
  const tiles = Array.from({ length: 4 }, () => {
    const lr = lowRes(96, 96);
    const d = lr.img.data;
    for (let i = 0; i < 96 * 96; i++) {
      const v = r() < 0.5 ? 0 : 255;
      d[i * 4] = d[i * 4 + 1] = d[i * 4 + 2] = v;
      d[i * 4 + 3] = r() * 255;
    }
    lr.x.putImageData(lr.img, 0, 0);
    return ctx.createPattern(lr.c, "repeat")!;
  });
  let n = 0;
  return {
    frame() {
      ctx.clearRect(0, 0, w, h);
      ctx.globalAlpha = 0.05 + 0.2 * o.intensity;
      ctx.save();
      ctx.translate(Math.floor(r() * 96), Math.floor(r() * 96));
      ctx.fillStyle = tiles[n++ % tiles.length];
      ctx.fillRect(-96, -96, w + 96, h + 96);
      ctx.restore();
      ctx.globalAlpha = 1;
    },
  };
}

const MAKE: Record<LiveKind, (ctx: Ctx, w: number, h: number, o: LiveOpts) => Live> = { fire, aurora, lava, stars, rain, grain };
export const createLive = (kind: LiveKind, ctx: Ctx, w: number, h: number, o: LiveOpts): Live => MAKE[kind](ctx, w, h, o);
