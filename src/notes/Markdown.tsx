// Náhled Markdownu poznámky. HTML dělá Rust (`note_render` → `notes::md::to_html`: vše escapované,
// žádné raw HTML ze vstupu); tady se jen vloží, doplní obrázky (data URL z `note_read`) a obslouží
// kliky: odkaz → `open_url` (Rust znovu ověří http/https), úkol → `onToggle(řádek, stav)`, vazba `[[…]]`
// → `onWiki(název)`, štítek `#…` → `onTag(štítek)` (jen data atributy, žádné href), odkaz `@cesta[:řádek]` → `onRef` (čip dělá `atChips`).

import { createEffect, createSignal, on } from "solid-js";
import { api } from "../api";
import { t } from "../i18n";
import { splitAtRefs } from "./brief";
import "./mdtask.css";

export default function Markdown(p: {
  body: string;
  images: { src: string; data: string }[];
  onToggle: (line: number, checked: boolean) => void;
  onWiki?: (name: string) => void;
  onTag?: (tag: string) => void;
  /** klik na `@cesta[:řádek]` */
  onRef?: (path: string, line: number | null) => void;
}) {
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
      // popisek snímku (`![popisek](…)`) pod obrázkem
      if (img.isConnected && img.alt) img.after(Object.assign(document.createElement("span"), { className: "md-cap", textContent: img.alt }));
    });
    atChips(box);
  }));
  function click(e: MouseEvent) {
    const t = e.target as HTMLElement;
    const link = t.closest<HTMLElement>(".md-link");
    if (link?.dataset.href) {
      e.preventDefault();
      api.openUrl(link.dataset.href);
      return;
    }
    const wiki = t.closest<HTMLElement>(".md-wiki");
    if (wiki?.dataset.wiki) {
      e.preventDefault();
      p.onWiki?.(wiki.dataset.wiki);
      return;
    }
    const tag = t.closest<HTMLElement>(".md-tag");
    if (tag?.dataset.tag) {
      p.onTag?.(tag.dataset.tag);
      return;
    }
    const ref = t.closest<HTMLElement>(".md-at");
    if (ref?.dataset.path) {
      e.preventDefault();
      p.onRef?.(ref.dataset.path, ref.dataset.line ? Number(ref.dataset.line) : null);
      return;
    }
    const box = t.closest<HTMLElement>(".md-check");
    // `[~]` (navržený jako hotový) potvrzuje jen člověk tlačítkem Potvrdit / Vrátit — klik ho nepřepíná
    if (box?.dataset.line && !box.classList.contains("proposed")) p.onToggle(Number(box.dataset.line), !box.classList.contains("on"));
  }
  return <div class="md" ref={box} onClick={click} />;
}

/** `@rel/cesta[:řádek]` v textu náhledu → čip `.md-at` (mimo odkazy a kód). */
function atChips(root: HTMLElement) {
  const nodes: Text[] = [];
  const w = document.createTreeWalker(root, NodeFilter.SHOW_TEXT);
  for (let n = w.nextNode(); n; n = w.nextNode()) if (n.nodeValue?.includes("@") && !(n.parentElement?.closest("a, code, pre, .md-link"))) nodes.push(n as Text);
  for (const n of nodes) {
    const parts = splitAtRefs(n.nodeValue ?? "");
    if (!parts.some((x) => x.ref)) continue;
    const frag = document.createDocumentFragment();
    for (const x of parts) {
      if (!x.ref) frag.append(x.text);
      else {
        const s = document.createElement("span");
        s.className = "md-at";
        s.dataset.path = x.ref.path;
        if (x.ref.line !== null) s.dataset.line = String(x.ref.line);
        s.title = t("notes.md.atTitle", "Klik zkopíruje cestu");
        s.textContent = x.text;
        frag.append(s);
      }
    }
    n.replaceWith(frag);
  }
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
