// Podbarvení karty Právě hraje barvou obalu (config `media.tint`, výchozí vypnuté).
// `pickTint` = čistá logika nad pixely RGBA (32×32 zmenšenina obalu): nejsytější „rozumná“ barva (ne skoro černá
// ani bílá); když je obal málo sytý, průměr těch rozumných; jinak null (karta zůstane bez podbarvení).
// `coverTint` ji spočítá jednou na obal přes malý canvas — jen když je volba zapnutá a karta vidět.
// Původ: obal posílá jádro jako `data:image/png;base64,…` (`media::cover`, CSP `img-src 'self' data:`) — data URL
// canvas neznečistí, `getImageData` projde. Jiný zdroj (cizí http, asset protokol) by canvas znečistil a čtení by
// tiše spadlo do null, proto `canTint` takový zdroj odmítne předem a výslovně.

const hex = (r: number, g: number, b: number) => `#${[r, g, b].map((v) => Math.round(v).toString(16).padStart(2, "0")).join("")}`;

/** Zmenšenina obalu pro výběr barvy: 32×32 (1024 pixelů, jednou na skladbu). */
export const TINT_SIZE = 32;

const rgb = (h: string): [number, number, number] => [1, 3, 5].map((i) => parseInt(h.slice(i, i + 2), 16)) as [number, number, number];
const chan = (v: number) => ((v /= 255) <= 0.03928 ? v / 12.92 : ((v + 0.055) / 1.055) ** 2.4);
/** Relativní jas podle WCAG (0 černá … 1 bílá). */
export const luminance = (h: string): number => {
  const [r, g, b] = rgb(h);
  return 0.2126 * chan(r) + 0.7152 * chan(g) + 0.0722 * chan(b);
};
/** Poměr kontrastu WCAG dvou barev `#rrggbb` (1 … 21). */
export const contrast = (a: string, b: string): number => {
  const [x, y] = [luminance(a), luminance(b)].sort((m, n) => n - m);
  return (x + 0.05) / (y + 0.05);
};
/** Barva přitažená k černé/bílé, až má proti pozadí `bg` kontrast ≥ `min` (výchozí 3:1); odstín zůstává. */
export function withContrast(h: string, bg: string, min = 3): string {
  if (contrast(h, bg) >= min) return h;
  const to = luminance(bg) < 0.5 ? 255 : 0;
  const [r, g, b] = rgb(h);
  for (let k = 1; k <= 20; k++) {
    const f = k / 20;
    const c = hex(r + (to - r) * f, g + (to - g) * f, b + (to - b) * f);
    if (contrast(c, bg) >= min) return c;
  }
  return hex(to, to, to);
}

export function pickTint(px: ArrayLike<number>): string | null {
  let best: string | null = null;
  let bestSat = -1;
  let sum = [0, 0, 0];
  let n = 0;
  for (let i = 0; i + 3 < px.length; i += 4) {
    const [r, g, b, a] = [px[i], px[i + 1], px[i + 2], px[i + 3]];
    if (a < 128) continue;
    const max = Math.max(r, g, b);
    const sat = max === 0 ? 0 : (max - Math.min(r, g, b)) / max;
    const lum = 0.2126 * r + 0.7152 * g + 0.0722 * b;
    if (lum <= 25 || lum >= 230 || sat <= 0.15) continue;
    sum = [sum[0] + r, sum[1] + g, sum[2] + b];
    n++;
    if (sat > bestSat) {
      bestSat = sat;
      best = hex(r, g, b);
    }
  }
  if (best && bestSat >= 0.22) return best;
  return n ? hex(sum[0] / n, sum[1] / n, sum[2] / n) : null;
}

/** Jde z obrázku číst pixely bez znečištění canvasu? Jen rastrový data URL z jádra. */
export const canTint = (url: string | null | undefined): boolean => !!url && /^data:image\/(png|jpeg|webp|bmp|gif);base64,/i.test(url);

/** Barva obalu (`url` = data URL z `useCover`); jiný zdroj, chyba nebo nečitelný obrázek = null. */
export function coverTint(url: string): Promise<string | null> {
  if (!canTint(url)) return Promise.resolve(null);
  return new Promise((done) => {
    const img = new Image();
    img.onload = () => {
      try {
        const c = document.createElement("canvas");
        c.width = c.height = TINT_SIZE;
        const ctx = c.getContext("2d", { willReadFrequently: true });
        if (!ctx) return done(null);
        ctx.drawImage(img, 0, 0, TINT_SIZE, TINT_SIZE);
        done(pickTint(ctx.getImageData(0, 0, TINT_SIZE, TINT_SIZE).data));
      } catch {
        done(null);
      }
    };
    img.onerror = () => done(null);
    img.src = url;
  });
}

/** Klíč cache `coverAccent`: obal + pozadí karty. */
export const accentKey = (url: string, bg: string) => `${url}|${bg.toLowerCase()}`;
const done = new Map<string, string | null>();
/** Pozadí karty pro kontrast: `--card` z motivu, když je to #rrggbb; jinak tmavé. */
function cardBg(): string {
  const v = typeof document === "undefined" ? "" : getComputedStyle(document.documentElement).getPropertyValue("--card").trim();
  return /^#[0-9a-f]{6}$/i.test(v) ? v : "#1c1c20";
}
/** Barva obalu s kontrastem ≥ 3:1 proti kartě; výsledek se pamatuje podle obalu (nejvýš 4 kusy), takže
 *  se počítá jen při změně skladby, nikdy v klidu. */
export async function coverAccent(url: string): Promise<string | null> {
  const bg = cardBg();
  const key = accentKey(url, bg); // po přepnutí motivu (jiné `--card`) se kontrast přepočítá
  if (done.has(key)) return done.get(key)!;
  const raw = await coverTint(url);
  const out = raw ? withContrast(raw, bg) : null;
  if (done.size >= 4) done.delete(done.keys().next().value!);
  done.set(key, out);
  return out;
}
