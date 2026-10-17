// Přeložený text s tučnou částí: `rich(t("k", "<b>{n}</b> běží", { n }))` → uzly bez innerHTML, ať
// překlad může tučné slovo přesunout („<b>3</b> running“). Jiné značky než `<b>` nezná.
import type { JSX } from "solid-js";

export function rich(s: string): JSX.Element {
  return s
    .split(/(<b>.*?<\/b>)/)
    .filter(Boolean)
    .map((part) => (part.startsWith("<b>") && part.endsWith("</b>") ? <b>{part.slice(3, -4)}</b> : part));
}
