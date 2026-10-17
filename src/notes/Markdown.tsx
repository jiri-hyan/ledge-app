// Náhled Markdownu poznámky. HTML dělá Rust (`note_render` → `notes::md::to_html`: vše escapované,
// žádné raw HTML ze vstupu); tady se jen vloží, doplní obrázky (data URL z `note_read`) a obslouží
// kliky: odkaz → `open_url` (Rust znovu ověří http/https), úkol → `onToggle(řádek, stav)`.

import { createEffect, createSignal, on } from "solid-js";
import { api } from "../api";
import { t } from "../i18n";

export default function Markdown(p: { body: string; images: { src: string; data: string }[]; onToggle: (line: number, checked: boolean) => void }) {
  const [html, setHtml] = createSignal("");
  let box!: HTMLDivElement;
  let seq = 0;
  createEffect(on(() => p.body, async (body) => {
    const mine = ++seq;
    const out = (await api.noteRender(body)) ?? "";
    if (mine === seq) setHtml(out);
  }));
  createEffect(on([html, () => p.images], () => {
    box.innerHTML = html();
    const map = new Map(p.images.map((i) => [i.src, i.data]));
    box.querySelectorAll<HTMLImageElement>("img[data-src]").forEach((img) => {
      const data = map.get(img.dataset.src ?? "");
      if (data?.startsWith("data:image/")) img.src = data;
      else img.replaceWith(Object.assign(document.createElement("span"), { className: "md-alt", textContent: t("notes.md.image", "[obrázek: {name}]", { name: img.alt || img.dataset.src || "" }) }));
    });
  }));
  function click(e: MouseEvent) {
    const t = e.target as HTMLElement;
    const link = t.closest<HTMLElement>(".md-link");
    if (link?.dataset.href) {
      e.preventDefault();
      api.openUrl(link.dataset.href);
      return;
    }
    const box = t.closest<HTMLElement>(".md-check");
    if (box?.dataset.line) p.onToggle(Number(box.dataset.line), !box.classList.contains("on"));
  }
  return <div class="md" ref={box} onClick={click} />;
}

/** `- [ ]` ↔ `- [x]` na řádku těla (od 0) — stejné počítání řádků jako Rust `md::toggle_line`.
 *  Pro rozepsaný text (neuložené změny se nepíšou na disk). */
export function toggleLocal(body: string, line: number, checked: boolean): string {
  const lines = body.split("\n");
  const l = lines[line];
  const i = l?.indexOf("[") ?? -1;
  if (i < 0 || l[i + 2] !== "]" || !" xX".includes(l[i + 1])) return body;
  lines[line] = `${l.slice(0, i + 1)}${checked ? "x" : " "}${l.slice(i + 2)}`;
  return lines.join("\n");
}
