// Společné kousky kroků průvodce: dlaždice náhledu, řádek se zaškrtnutím, skeleton při načítání,
// náhled „takhle to bude v šuplíku“. Bez stavu — data dostávají v props.
import { For, Show, type JSX } from "solid-js";
import { t } from "../../i18n";
import { monogram } from "../logic";

/** Logo nebo monogram (ikona z `wiz.icons`; `undefined` = ještě se načítá). */
export function Logo(p: { src: string | null | undefined; name: string; color?: string }) {
  return (
    <span class="ob-logo" style={p.color ? { background: p.color } : undefined}>
      <Show when={p.src} fallback={<span class="ob-mono">{monogram(p.name)}</span>}>
        <img src={p.src!} alt="" draggable={false} />
      </Show>
    </span>
  );
}

/** Řádek seznamu se zaškrtávátkem; `disabled` = nejde vybrat (důvod v `meta`). */
export function CheckRow(p: { on: boolean; disabled?: boolean; onToggle: () => void; logo: JSX.Element; name: string; meta: JSX.Element; tags?: JSX.Element }) {
  return (
    <button type="button" class="ob-row" classList={{ on: p.on, off: !!p.disabled }} disabled={p.disabled} onClick={() => p.onToggle()} role="checkbox" aria-checked={p.on}>
      <span class="ob-check" aria-hidden="true" />
      {p.logo}
      <span class="ob-row-txt">
        <span class="ob-row-name">{p.name}</span>
        <span class="ob-row-meta">{p.meta}</span>
      </span>
      <Show when={p.tags}>
        <span class="ob-row-tags">{p.tags}</span>
      </Show>
    </button>
  );
}

/** Šedé řádky místo prázdna, dokud sken běží. */
export function Skeleton(p: { rows?: number; label: string }) {
  return (
    <div aria-busy="true" aria-label={p.label}>
      <div class="ob-skel-label">{p.label}</div>
      <For each={Array.from({ length: p.rows ?? 6 })}>
        {(_, i) => (
          <div class="ob-skel-row" style={{ "animation-delay": `${i() * 80}ms` }}>
            <span class="ob-skel-logo" />
            <span class="ob-skel-lines">
              <i style={{ width: `${40 + ((i() * 17) % 35)}%` }} />
              <i style={{ width: `${25 + ((i() * 23) % 30)}%` }} />
            </span>
          </div>
        )}
      </For>
    </div>
  );
}

export type PreviewItem = { key: string; name: string; src?: string | null; color?: string };

/** Pravý sloupec: dlaždice toho, co po uložení přibude v šuplíku. */
export function Preview(p: { title: string; items: PreviewItem[]; empty: string; note?: string }) {
  const shown = () => p.items.slice(0, 12);
  return (
    <aside class="ob-preview">
      <h5>{p.title}</h5>
      <Show when={p.items.length} fallback={<div class="ob-preview-empty">{p.empty}</div>}>
        <div class="ob-tiles">
          <For each={shown()}>
            {(it) => (
              <div class="ob-tile">
                <Logo src={it.src} name={it.name} color={it.color} />
                <span>{it.name}</span>
              </div>
            )}
          </For>
        </div>
        <Show when={p.items.length > 12}>
          <div class="ob-more">{t("onboarding.preview.more", "a dalších {n}", { n: p.items.length - 12 })}</div>
        </Show>
      </Show>
      <Show when={p.note}>
        <p class="ob-note">{p.note}</p>
      </Show>
    </aside>
  );
}
