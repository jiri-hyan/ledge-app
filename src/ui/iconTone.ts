// Tón ikony (favicon akce, logo projektu v Nastavení): tmavá / světlá kresba na průhledném pozadí se na
// dlaždici stejného tónu ztratí (černý GitHub na tmavém motivu). `toneOf` (čistá) z pixelů pozná tmavou /
// světlou kresbu; `measureTone` ji změří jednou na URL (16×16 canvas, jen při načtení obrázku, žádné
// časovače) a CSS pak podloží jen ikonu opačného tónu než motiv (`:root[data-scheme]` z `theme/apply.ts`)
// podkladem barvy textu motivu — obecně, ne výjimka pro GitHub. Ikona s vlastním pozadím (skoro plná) =
// `mid`, nic se nemění. Testy `iconTone.test.mjs`.

import { createEffect, createSignal } from "solid-js";

export type Tone = "dark" | "light" | "mid";

/** Pod tímto podílem krytí je ikona „prázdná“ (nic k posouzení). */
const MIN_COVER = 0.03;
/** Nad tímto podílem krytí má ikona vlastní pozadí — podklad nepotřebuje. */
const OWN_BG = 0.85;
/** Relativní jas (sRGB → lineární, WCAG) kresby: pod `DARK` tmavá, nad `LIGHT` světlá. */
const DARK = 0.12;
const LIGHT = 0.6;

const lin = (c: number) => {
  const s = c / 255;
  return s <= 0.04045 ? s / 12.92 : ((s + 0.055) / 1.055) ** 2.4;
};

/** RGBA pixely (jako `ImageData.data`) → tón kresby; jas se váží alfou. */
export function toneOf(px: ArrayLike<number>): Tone {
  const n = Math.floor(px.length / 4);
  if (!n) return "mid";
  let cover = 0;
  let lum = 0;
  for (let i = 0; i < n; i++) {
    const a = px[i * 4 + 3] / 255;
    if (a <= 0) continue;
    cover += a;
    lum += a * (0.2126 * lin(px[i * 4]) + 0.7152 * lin(px[i * 4 + 1]) + 0.0722 * lin(px[i * 4 + 2]));
  }
  const share = cover / n;
  if (share < MIN_COVER || share > OWN_BG) return "mid";
  const l = lum / cover;
  return l < DARK ? "dark" : l > LIGHT ? "light" : "mid";
}

const cache = new Map<string, Promise<Tone>>();

/** Solid: signál tónu pro měnící se URL (bez `createResource` — žádné Suspense, žádné blikání). */
export function useTone(src: () => string | undefined): () => Tone {
  const [tone, setTone] = createSignal<Tone>("mid");
  createEffect(() => {
    const s = src();
    setTone("mid");
    if (s) void measureTone(s).then((t) => src() === s && setTone(t));
  });
  return tone;
}

/** Tón obrázku z URL (data: z jádra) — jednou na URL; chyba / cizí původ = `mid` (beze změny). */
export function measureTone(src: string): Promise<Tone> {
  let p = cache.get(src);
  if (!p) {
    p = new Promise<Tone>((resolve) => {
      const img = new Image();
      img.onload = () => {
        try {
          const c = document.createElement("canvas");
          c.width = c.height = 16;
          const g = c.getContext("2d", { willReadFrequently: true });
          if (!g) return resolve("mid");
          g.drawImage(img, 0, 0, 16, 16);
          resolve(toneOf(g.getImageData(0, 0, 16, 16).data));
        } catch {
          resolve("mid");
        }
      };
      img.onerror = () => resolve("mid");
      img.src = src;
    });
    cache.set(src, p);
  }
  return p;
}
