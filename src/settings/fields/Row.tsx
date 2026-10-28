// Stavební bloky panelu: hlavička panelu, sekce (karta) a řádek volby — název a nápověda z
// `labels.ts` vlevo, ovládání vpravo, chyba validace pod ním. `id` řádku = kotva hledání (`#f-<id>`)
// a cesta pole pro chybu z Rustu.
import { Show, type JSX } from "solid-js";
import { hit, useSettings } from "../ctx";
import { fieldHint, fieldLabel } from "../labels";

/** `id` = kotva hledání (`#f-<id>`) pro volbu, která žije v hlavičce (hlavní vypínač panelu). */
export function PanelHead(p: { title: string; desc?: string; id?: string; children?: JSX.Element }) {
  return (
    <header class="st-phead" id={p.id ? `f-${p.id}` : undefined} classList={{ hit: !!p.id && hit() === p.id }}>
      <div>
        <h2>{p.title}</h2>
        <Show when={p.desc}><p>{p.desc}</p></Show>
      </div>
      <Show when={p.children}><div class="st-phead-act">{p.children}</div></Show>
    </header>
  );
}

/** `id` = kotva hledání pro volbu, která je celou sekcí (víc řádků pod jedním názvem). */
export function Section(p: { title?: string; desc?: string; id?: string; children: JSX.Element; aside?: JSX.Element }) {
  return (
    <section class="st-sec" id={p.id ? `f-${p.id}` : undefined} classList={{ hit: !!p.id && hit() === p.id }}>
      <Show when={p.title || p.aside}>
        <div class="st-sec-head">
          <Show when={p.title}><h3>{p.title}</h3></Show>
          <Show when={p.aside}><span class="st-sec-aside">{p.aside}</span></Show>
        </div>
      </Show>
      <Show when={p.desc}><p class="st-sec-desc">{p.desc}</p></Show>
      {p.children}
    </section>
  );
}

/** Štítek „Platí hned“ vedle ovládání volby, která se NEukládá lištou Uložit (zapnutí MCP, zapojení hooků,
 *  přihlášení účtu…) — jedna komponenta pro všechny; `Row live` ji dává před ovládací prvek. */
/** Dřív štítek „Platí hned“ u voleb mimo Uložit. Od kola 24 (Ledge Studio) platí hned VŠECHNO → nic nekreslí;
 *  volání v panelech zůstávají kvůli vlně obsahu (smaže je spolu s `Row live`). */
export function LiveBadge() {
  return null;
}

export function Row(p: { id: string; label?: string; hint?: string; stack?: boolean; nested?: boolean; dim?: boolean; live?: boolean; children?: JSX.Element }) {
  const c = useSettings();
  const err = () => c.error(p.id, p.nested);
  const hint = () => p.hint ?? fieldHint(p.id);
  return (
    <div class="st-row" id={`f-${p.id}`} classList={{ stack: p.stack, hit: hit() === p.id, bad: !!err(), dim: p.dim }}>
      <div class="st-lbl">
        <span class="st-name">{p.label ?? fieldLabel(p.id)}</span>
        <Show when={hint()}><small>{hint()}</small></Show>
      </div>
      <Show when={p.children}><div class="st-ctl"><Show when={p.live}><LiveBadge /></Show>{p.children}</div></Show>
      <Show when={err()}><div class="st-err" role="alert">{err()}</div></Show>
    </div>
  );
}

/** Chyba u konkrétního pole uvnitř složitějšího editoru (projekt, kategorie). */
export function FieldErr(p: { path: string }) {
  const c = useSettings();
  return <Show when={c.error(p.path)}>{(m) => <div class="st-err" role="alert">{m()}</div>}</Show>;
}
