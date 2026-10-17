// Výběr projektu místo nativního <select>: logo / monogram v barvě projektu, psaním filtruje,
// šipky + Enter + Esc. Hodnota "" = bez projektu; `all` přidá volbu „Všechny projekty“ (hodnota "*").
// Panel je v toku lišty (viz popup.ts) — klávesnice funguje, až má šuplík fokus (klik).

import { createSignal, createMemo, For, Show } from "solid-js";
import type { ProjectView } from "../api";
import { usePopup, swallow, scrollWithin } from "./popup";
import { logoSrc } from "../logos";
import { t } from "../i18n";

type Opt = { id: string; name: string; accent: string; monogram: string; logoKey?: string | null };
// funkce, ať se název přeloží až při vykreslení
const none = (): Opt => ({ id: "", name: t("notes.picker.none", "Bez projektu"), accent: "#475569", monogram: "–" });
const all = (): Opt => ({ id: "*", name: t("notes.picker.all", "Všechny projekty"), accent: "#334155", monogram: "∗" });

function Logo(p: { o: Opt }) {
  const src = () => (p.o.logoKey ? logoSrc({ id: p.o.id, logoKey: p.o.logoKey ?? null }) : undefined);
  return (
    <span class="pp-logo" classList={{ "has-img": !!src() }} style={{ background: `linear-gradient(135deg, ${p.o.accent}, color-mix(in srgb, ${p.o.accent} 45%, #000))` }}>
      <Show when={src()} fallback={p.o.monogram}>{(src) => <img src={src()} alt="" draggable={false} />}</Show>
    </span>
  );
}

const fold = (s: string) => s.normalize("NFD").replace(/[̀-ͯ]/g, "").toLowerCase();

export default function ProjectPicker(p: { projects: ProjectView[]; value: string; onChange: (id: string) => void; all?: boolean; label?: string }) {
  const pop = usePopup();
  const [q, setQ] = createSignal("");
  const [hi, setHi] = createSignal(0);
  let trigger!: HTMLButtonElement;
  const opts = createMemo<Opt[]>(() => [...(p.all ? [all()] : []), none(), ...p.projects]);
  const shown = createMemo(() => {
    const f = fold(q().trim());
    return f ? opts().filter((o) => fold(o.name).includes(f) || fold(o.id).includes(f)) : opts();
  });
  const cur = () => opts().find((o) => o.id === p.value) ?? { ...none(), id: p.value, name: p.value || none().name };

  function openPanel() {
    setQ("");
    setHi(Math.max(0, opts().findIndex((o) => o.id === p.value)));
    pop.toggle();
  }
  function pick(o: Opt | undefined) {
    if (!o) return;
    p.onChange(o.id);
    pop.close();
    trigger?.focus();
  }
  function key(e: KeyboardEvent) {
    swallow(e);
    const n = shown().length;
    if (e.key === "ArrowDown") setHi((hi() + 1) % Math.max(n, 1));
    else if (e.key === "ArrowUp") setHi((hi() - 1 + n) % Math.max(n, 1));
    else if (e.key === "Enter") pick(shown()[hi()]);
    else if (e.key === "Escape") {
      pop.close();
      trigger?.focus();
    } else return;
    e.preventDefault();
    queueMicrotask(() => scrollWithin(document.querySelector(`[data-pop="${pop.id}"] .pp-list`), document.querySelector(`[data-pop="${pop.id}"] .pp-opt.hi`)));
  }

  return (
    <>
      <span class="pick-field">
        <Show when={p.label}><span class="pick-lbl">{p.label}</span></Show>
        <button ref={trigger} class="pick-btn" classList={{ open: pop.open() }} data-pop={pop.id} onClick={openPanel} aria-haspopup="listbox" aria-expanded={pop.open()}>
          <Logo o={cur()} />
          <span class="pick-val">{cur().name}</span>
          <span class="pick-caret" />
        </button>
      </span>
      <Show when={pop.open()}>
        <div class="pick-pop" data-pop={pop.id} onKeyDown={key}>
          <input
            type="text"
            class="pp-q"
            placeholder={t("notes.picker.search", "Hledat projekt…")}
            value={q()}
            ref={(el) => queueMicrotask(() => el.focus())}
            onInput={(e) => {
              setQ(e.currentTarget.value);
              setHi(0);
            }}
          />
          <div class="pp-list" role="listbox">
            <For each={shown()} fallback={<div class="empty">{t("notes.picker.noMatch", "Nic neodpovídá.")}</div>}>
              {(o, i) => (
                <button class="pp-opt" classList={{ hi: i() === hi(), on: o.id === p.value }} role="option" aria-selected={o.id === p.value} onPointerEnter={() => setHi(i())} onClick={() => pick(o)}>
                  <Logo o={o} />
                  <span class="pp-name">{o.name}</span>
                </button>
              )}
            </For>
          </div>
        </div>
      </Show>
    </>
  );
}
