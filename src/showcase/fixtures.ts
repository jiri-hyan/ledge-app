// Doplňková ukázková data pro přehlídku vzhledů karet (kolo 25, `scripts/showcase/designs.mjs`): útrata Claude za 14 dní,
// předpověď počasí (hodiny, dny, východ a západ) a odběr systému s historií. Všechno smyšlené, čas relativní k `now`.
// Čisté funkce bez DOM (test `sets.test.mjs`).
const MIN = 60_000;
const DAY = 86_400_000;
const pad = (n: number) => String(n).padStart(2, "0");
/** Místní čas jako „2026-10-16T06:58“ (tak ho posílá počasí). */
export const localIso = (t: number): string => {
  const d = new Date(t);
  return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}T${pad(d.getHours())}:${pad(d.getMinutes())}`;
};
const ymd = (t: number) => localIso(t).slice(0, 10);

export type WxKind = "day" | "night" | "rain";

/** Počasí s předpovědí: `day` polojasno, `night` jasná noc, `rain` déšť. */
export function weatherFull(label: string, now: number, kind: WxKind = "day") {
  const d = new Date(now);
  const midnight = new Date(d.getFullYear(), d.getMonth(), d.getDate()).getTime();
  const rain = kind === "rain";
  const night = kind === "night";
  const code = rain ? 63 : night ? 0 : 2;
  const base = rain ? 12 : night ? 9 : 17;
  const hours = Array.from({ length: 48 }, (_, i) => {
    const h = i % 24;
    const wave = Math.round(4.5 * Math.sin(((h - 9) / 24) * Math.PI * 2));
    return { time: localIso(midnight + i * 60 * MIN), temp: base + wave, precipProb: rain ? 60 + ((i * 7) % 35) : (i * 13) % 30, code: rain && h > 6 ? 63 : h >= 7 && h <= 19 ? 2 : 0 };
  });
  const codes = [2, 3, 61, 1, 0];
  const days = codes.map((c, i) => ({ date: ymd(midnight + (i + 1) * DAY), min: base - 6 + (i % 3), max: base + 3 + ((i * 2) % 5), code: c }));
  return {
    status: "ok", label, fetchedMs: now, error: null, loading: false,
    data: {
      temp: base + 1, code, desc: rain ? "Déšť" : night ? "Jasno" : "Polojasno", icon: rain ? "🌧️" : night ? "🌙" : "⛅", isDay: !night,
      min: base - 6, max: base + 3, precipSum: rain ? 8.4 : 0.4, precipProb: rain ? 85 : 20, observed: localIso(now - 15 * MIN),
      feels: base - 1, wind: rain ? 22 : 11, windDir: 250,
      sunrise: localIso(midnight + 6 * 60 * MIN + 41 * MIN), sunset: localIso(midnight + 18 * 60 * MIN + 12 * MIN),
      hours, days,
    },
  };
}

/** Útrata za 14 dní (poslední den = dnešek) ve tvaru `CostsView`. */
export function costsFull(now: number) {
  const vals = [3.1, 4.8, 2.2, 0, 0, 6.4, 5.1, 7.9, 4.3, 3.6, 0.9, 0, 5.8, 6.7];
  const days = vals.map((cost, i) => ({ date: ymd(now - (vals.length - 1 - i) * DAY), tokens: Math.round(cost * 410_000), cost }));
  const total = vals.reduce((a, b) => a + b, 0);
  const line = (name: string, k: number) => ({ name, tokens: Math.round(total * 410_000 * k), cost: Math.round(total * k * 100) / 100, unpriced: 0, other: false });
  return {
    state: "ok", progress: null, computedAt: now, error: null, windowDays: 30,
    summary: {
      ranges: [{ days: 14, split: { input: 1.2e6, output: 0.4e6, cacheWrite: 2.1e6, cacheRead: 9.8e6 }, total: line("Celkem", 1), requests: 640, projects: [line("atlas-web", 0.6), line("harbor-api", 0.4)], models: [line("Model A", 0.7), line("Model B", 0.3)] }],
      days,
    },
    pricesVerified: "2026-10-01", pricesSource: "example.com/ceny",
  };
}

const wave = (n: number, base: number, amp: number, seed: number) =>
  Array.from({ length: n }, (_, i) => Math.max(1, Math.min(99, Math.round(base + amp * Math.sin(i / 3 + seed) + (amp / 2) * Math.sin(i * 1.7 + seed * 2)))));

/** Odběr systému (`SystemView`) s historií 60 vzorků po 2 s. */
export function systemFull(now: number) {
  const GB = 1024 ** 3;
  const rxHist = wave(60, 500, 380, 1).map((v) => v * 4_000);
  const txHist = wave(60, 120, 90, 2).map((v) => v * 3_000);
  return {
    atMs: now, cpu: 38, memUsed: 11.2 * GB, memTotal: 32 * GB, memPct: 35,
    disks: [{ letter: "C", total: 953 * GB, free: 311 * GB, system: true }, { letter: "D", total: 1862 * GB, free: 1204 * GB, system: false }],
    rx: rxHist[59], tx: txHist[59], cpuHist: wave(60, 40, 22, 0), memHist: wave(60, 35, 6, 3), rxHist, txHist, netUnit: "bytes", allDisks: ["C", "D"],
  };
}
