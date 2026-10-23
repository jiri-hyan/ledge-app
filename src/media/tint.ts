// Podbarvení karty Právě hraje barvou obalu (config `media.tint`, výchozí vypnuté).
// `pickTint` = čistá logika nad pixely RGBA (8×8 zmenšenina obalu): nejsytější „rozumná“ barva (ne skoro černá
// ani bílá); když je obal málo sytý, průměr těch rozumných; jinak null (karta zůstane bez podbarvení).
// `coverTint` ji spočítá jednou na obal přes malý canvas — jen když je volba zapnutá a karta vidět.
// Původ: obal posílá jádro jako `data:image/png;base64,…` (`media::cover`, CSP `img-src 'self' data:`) — data URL
// canvas neznečistí, `getImageData` projde. Jiný zdroj (cizí http, asset protokol) by canvas znečistil a čtení by
// tiše spadlo do null, proto `canTint` takový zdroj odmítne předem a výslovně.

const hex = (r: number, g: number, b: number) => `#${[r, g, b].map((v) => Math.round(v).toString(16).padStart(2, "0")).join("")}`;

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
        c.width = c.height = 8;
        const ctx = c.getContext("2d", { willReadFrequently: true });
        if (!ctx) return done(null);
        ctx.drawImage(img, 0, 0, 8, 8);
        done(pickTint(ctx.getImageData(0, 0, 8, 8).data));
      } catch {
        done(null);
      }
    };
    img.onerror = () => done(null);
    img.src = url;
  });
}
