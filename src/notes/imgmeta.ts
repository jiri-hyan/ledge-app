// Popisky a kotvy čerstvě vložených snímků (ještě neuložených): klíč = data URL snímku. Nepřežívají restart
// (koncept na disku nese jen PNG); při uložení se kotva přepočítá na řádek těla (`resolveAnchor`).
import { createSignal } from "solid-js";
import type { ImageIn } from "../types";
import { cleanCaption, imageIn, resolveAnchor, type Anchor } from "./brief";
import { b64Of } from "./draft";

interface Entry { caption: string; anchor: Anchor | null }
const [meta, setMeta] = createSignal<Record<string, Entry>>({});

export const entryOf = (url: string): Entry | undefined => meta()[url];
export const setCaption = (url: string, caption: string) => setMeta({ ...meta(), [url]: { ...(meta()[url] ?? { anchor: null }), caption } });
/** Nové snímky (ty, co nebyly v `before`) dostanou kotvu na řádek s kurzorem. */
export function anchorNew(before: string[], after: string[], anchor: Anchor | null) {
  const add: Record<string, Entry> = {};
  for (const u of after) if (!before.includes(u)) add[u] = { caption: "", anchor };
  if (Object.keys(add).length) setMeta({ ...meta(), ...add });
}
/** `images` pro `note_save` / `note_create` v tvaru Rustu; `body` = přesně odesílaný text (kotva = jeho řádek). */
export function imagesFor(urls: string[], body: string): ImageIn[] {
  return urls.map((u) => {
    const e = meta()[u];
    return imageIn(b64Of([u])[0], e ? cleanCaption(e.caption) : "", e ? resolveAnchor(body, e.anchor) : undefined);
  });
}
export const clearMeta = () => setMeta({});
