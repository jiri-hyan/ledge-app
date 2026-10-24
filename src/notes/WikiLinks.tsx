// Vazby `[[název poznámky]]` v editoru i lístku (Lístky 2.0): klik na vazbu v náhledu (`useWikiFollow` —
// existující poznámka se otevře, neexistující nabídne „Vytvořit“) a našeptávač při psaní `[[` (`useWikiSuggest`
// + `WikiList`): titulky jen z paměti jádra (`note_link_suggest`), zeptá se po krátké pauze, ne v klidu.
import { createSignal, For, Show } from "solid-js";
import { t } from "../i18n";
import { sayError } from "../ui/toast";
import { debounce } from "../util/time";
import { notesApi } from "./api";
import { applyWiki, wikiQuery } from "./wiki";

/** Klik na `[[název]]`: `open(cesta)` u existující, jinak `missing()` = název k nabídce „Vytvořit“. */
export function useWikiFollow(open: (path: string) => void) {
  const [missing, setMissing] = createSignal<string | null>(null);
  async function follow(name: string) {
    try {
      const path = await notesApi.linkResolve(name);
      if (path) {
        setMissing(null);
        open(path);
      } else setMissing(name);
    } catch (e) {
      sayError(e);
    }
  }
  async function create() {
    const name = missing();
    setMissing(null);
    if (!name) return;
    try {
      const path = await notesApi.linkCreate(name);
      if (path) open(path);
    } catch (e) {
      sayError(e);
    }
  }
  return { missing, follow, create, dismiss: () => setMissing(null) };
}

/** Lišta „Poznámka «X» zatím není — Vytvořit“ (`pin` = v okně lístku, jinak editor šuplíku). */
export function WikiMissing(p: { w: ReturnType<typeof useWikiFollow>; pin?: boolean }) {
  return (
    <Show when={p.w.missing()}>
      {(name) => (
        <div class={p.pin ? "pin-banner" : "banner warn"} role="status">
          <span>{t("notes.wiki.missing", "Poznámka „{name}“ zatím není.", { name: name() })}</span>
          <button class={p.pin ? "pin-btn pri" : "mini ok"} onClick={() => void p.w.create()}>{t("notes.wiki.create", "Vytvořit")}</button>
          <button class={p.pin ? "pin-btn" : "mini"} onClick={p.w.dismiss}>{t("notes.wiki.cancel", "Zrušit")}</button>
        </div>
      )}
    </Show>
  );
}

type Hit = { title: string; path: string };

/** Našeptávač `[[` nad textovým polem: `onInput` po každé změně, `onKey` PŘED vlastními klávesami pole
 *  (vrací true = klávesu zpracoval: šipky, Enter / Tab vloží, Esc zavře). */
export function useWikiSuggest(setText: (v: string) => void) {
  const [hits, setHits] = createSignal<Hit[]>([]);
  const [sel, setSel] = createSignal(0);
  let at: { start: number; el: HTMLTextAreaElement } | null = null;
  let seq = 0;
  const ask = debounce((q: string) => {
    const id = ++seq;
    notesApi.linkSuggest(q, 6).then(
      (r) => id === seq && (setHits(r ?? []), setSel(0)),
      () => id === seq && setHits([]),
    );
  }, 150);
  function close() {
    ask.cancel();
    seq++;
    at = null;
    setHits([]);
  }
  function onInput(el: HTMLTextAreaElement) {
    const q = wikiQuery(el.value, el.selectionStart);
    if (!q) return close();
    at = { start: q.start, el };
    ask(q.query);
  }
  function pick(h: Hit) {
    if (!at) return;
    const el = at.el;
    const r = applyWiki(el.value, at.start, el.selectionStart, h.title);
    setText(r.text);
    close();
    queueMicrotask(() => {
      el.focus();
      el.setSelectionRange(r.caret, r.caret);
    });
  }
  function onKey(e: KeyboardEvent): boolean {
    const list = hits();
    if (!list.length) return false;
    if (e.key === "ArrowDown" || e.key === "ArrowUp") {
      e.preventDefault();
      setSel((sel() + (e.key === "ArrowDown" ? 1 : list.length - 1)) % list.length);
      return true;
    }
    if ((e.key === "Enter" && !e.ctrlKey) || e.key === "Tab") {
      e.preventDefault();
      pick(list[sel()] ?? list[0]);
      return true;
    }
    if (e.key === "Escape") {
      e.preventDefault();
      e.stopPropagation();
      close();
      return true;
    }
    return false;
  }
  return { hits, sel, pick, onInput, onKey, close };
}

/** Seznam návrhů pod polem (`pin` = v okně lístku). */
export function WikiList(p: { s: ReturnType<typeof useWikiSuggest>; pin?: boolean }) {
  return (
    <Show when={p.s.hits().length}>
      <div class={p.pin ? "pin-wiki-list" : "note-wiki-list"} role="listbox" aria-label={t("notes.wiki.suggest", "Vazba na poznámku")}>
        <For each={p.s.hits()}>
          {(h, i) => (
            <button type="button" role="option" aria-selected={i() === p.s.sel()} classList={{ on: i() === p.s.sel() }} onMouseDown={(e) => e.preventDefault()} onClick={() => p.s.pick(h)}>
              [[{h.title}]]
            </button>
          )}
        </For>
      </div>
    </Show>
  );
}
