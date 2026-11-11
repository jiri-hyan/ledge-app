// Našeptávač souborů projektu po `@` v textu poznámky: token pod kurzorem (`atToken`), po 150 ms ticha
// `project_files`, max 20 položek, ↑↓ Enter / Tab vybere, Esc zavře. Bez projektu nic.
import { createSignal, For, Show } from "solid-js";
import { api } from "../api";
import { debounce } from "../util/time";
import { applyAt, atToken, type AtToken } from "./brief";

export const AT_MAX = 20;

export function createAt(o: { text: () => string; project: () => string; setText: (v: string) => void; ta: () => HTMLTextAreaElement | undefined }) {
  const [files, setFiles] = createSignal<string[]>([]);
  const [hi, setHi] = createSignal(0);
  const [tok, setTok] = createSignal<AtToken | null>(null);
  let seq = 0;
  const fetch = debounce(async (q: string) => {
    const mine = ++seq;
    const r = (await api.projectFiles(o.project(), q).catch(() => undefined)) ?? [];
    if (mine === seq) {
      setFiles(r.slice(0, AT_MAX));
      setHi(0);
    }
  }, 150);
  const close = () => {
    seq++;
    fetch.cancel();
    setTok(null);
    setFiles([]);
  };
  /** Po každé změně textu / kurzoru. */
  function update() {
    const ta = o.ta();
    const tk = ta && o.project() && ta.selectionStart === ta.selectionEnd ? atToken(ta.value, ta.selectionStart) : null;
    if (!tk) return close();
    if (tok()?.query !== tk.query || tok()?.start !== tk.start) fetch(tk.query);
    setTok(tk);
  }
  const open = () => tok() !== null && files().length > 0;
  function pick(path: string) {
    const ta = o.ta();
    const tk = tok();
    if (!ta || !tk) return;
    const r = applyAt(ta.value, tk, ta.selectionStart, path);
    o.setText(r.text);
    close();
    queueMicrotask(() => {
      ta.focus();
      ta.setSelectionRange(r.caret, r.caret);
    });
  }
  /** true = klávesu spolkl našeptávač. */
  function key(e: KeyboardEvent): boolean {
    if (!open()) return false;
    const n = files().length;
    if (e.key === "ArrowDown" || e.key === "ArrowUp") setHi((hi() + (e.key === "ArrowDown" ? 1 : n - 1)) % n);
    else if ((e.key === "Enter" && !e.ctrlKey) || e.key === "Tab") pick(files()[hi()]);
    else if (e.key === "Escape") close();
    else return false;
    e.preventDefault();
    e.stopPropagation();
    return true;
  }
  return { files, hi, setHi, open, pick, key, update, close };
}

export function AtList(p: { at: ReturnType<typeof createAt> }) {
  return (
    <Show when={p.at.open()}>
      <div class="at-list" role="listbox">
        <For each={p.at.files()}>
          {(f, i) => (
            <button class="at-item" role="option" classList={{ on: i() === p.at.hi() }} aria-selected={i() === p.at.hi()} onMouseDown={(e) => (e.preventDefault(), p.at.pick(f))} onMouseEnter={() => p.at.setHi(i())}>{f}</button>
          )}
        </For>
      </div>
    </Show>
  );
}
