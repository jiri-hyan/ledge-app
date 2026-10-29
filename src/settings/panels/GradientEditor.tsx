// Editor vlastního gradientu (kolo 25, Studio → Vzhled šuplíku → Pozadí): 2–5 zastávek (barva + poloha) na liště
// s táhnutelnými úchyty, druh (linear / radial / conic / mesh), úhel, název a smazání. Upravuje přímo vybraný
// vlastní gradient v `appearance.backdrop.custom` — každá změna platí hned (živě na šuplíku) a vrací se přes
// historii Studia / Zahodit v liště. Čistá logika (`moveStop`, `addStop`, `updateGradient` …) je v `theme/backdrop.ts`.
import { For, Show, createSignal } from "solid-js";
import { t } from "../../i18n";
import type { Backdrop, CustomGradient, GradKind } from "../../types";
import { MAX_GRAD_NAME, STOPS_MAX, STOPS_MIN, addStop, colorAt, gradientCss, moveStop, posFromPointer, removeGradient, removeStop, sortStops, updateGradient } from "../../theme/backdrop";
import { Seg, Slider, type Ap } from "./apUi";

const KIND_LABEL: Record<GradKind, () => string> = {
  linear: () => t("appearance.bd.kind.linear", "Přímý"),
  radial: () => t("appearance.bd.kind.radial", "Kruhový"),
  conic: () => t("appearance.bd.kind.conic", "Kuželový"),
  mesh: () => t("appearance.bd.kind.mesh", "Mesh"),
};

export default function GradientEditor(q: { ap: Ap; bd: Backdrop; g: CustomGradient }) {
  const [sel, setSel] = createSignal(0);
  let bar!: HTMLDivElement;
  const put = (patch: Parameters<typeof updateGradient>[2]) => q.ap.set("backdrop", { ...q.bd, custom: updateGradient(q.bd.custom, q.g.id, patch) });
  const stops = () => sortStops(q.g.stops);
  const cur = () => stops()[Math.min(sel(), stops().length - 1)];
  const i = () => Math.min(sel(), stops().length - 1);

  const drag = (idx: number, e: PointerEvent) => {
    e.preventDefault();
    setSel(idx);
    const el = e.currentTarget as HTMLElement;
    el.setPointerCapture(e.pointerId);
    let at = idx;
    const move = (ev: PointerEvent) => {
      const r = bar.getBoundingClientRect();
      const m = moveStop(stops(), at, posFromPointer(ev.clientX, r.left, r.width));
      at = m.index;
      setSel(m.index);
      put({ stops: m.stops });
    };
    const up = () => {
      el.removeEventListener("pointermove", move);
      el.removeEventListener("pointerup", up);
      el.removeEventListener("pointercancel", up);
    };
    el.addEventListener("pointermove", move);
    el.addEventListener("pointerup", up);
    el.addEventListener("pointercancel", up);
  };
  /** Klik do prázdného místa lišty = nová zastávka tam (barva podle gradientu v tom místě). */
  const barClick = (e: MouseEvent) => {
    if ((e.target as HTMLElement).closest(".gd-h") || stops().length >= STOPS_MAX) return;
    const r = bar.getBoundingClientRect();
    const p = Math.round(posFromPointer(e.clientX, r.left, r.width));
    const s = sortStops(stops());
    const withNew = sortStops([...s, { c: colorAt(s, p), p }]);
    setSel(withNew.findIndex((x) => x.p === p));
    put({ stops: withNew });
  };
  const key = (idx: number, e: KeyboardEvent) => {
    const d = e.key === "ArrowLeft" ? -1 : e.key === "ArrowRight" ? 1 : 0;
    if (!d) return;
    e.preventDefault();
    const m = moveStop(stops(), idx, stops()[idx].p + d * (e.shiftKey ? 10 : 1));
    setSel(m.index);
    put({ stops: m.stops });
  };

  return (
    <div class="gd" id="f-appearance.backdrop.editor">
      <div class="opt">
        <span>{t("appearance.bd.ed.name", "Název")}</span>
        <input class="st-text" type="text" maxLength={MAX_GRAD_NAME} value={q.g.name} aria-label={t("appearance.bd.ed.name", "Název")} onChange={(e) => put({ name: e.currentTarget.value })} />
      </div>
      <div class="opt">
        <span>{t("appearance.bd.ed.kind", "Druh")}<small>{t("appearance.bd.ed.kindHint", "Mesh = víc měkkých barevných skvrn, velikost skvrny je poloha zastávky.")}</small></span>
        <Seg<GradKind> label={t("appearance.bd.ed.kind", "Druh")} value={q.g.kind} set={(v) => put({ kind: v })} options={(["linear", "radial", "conic", "mesh"] as GradKind[]).map((k) => [k, KIND_LABEL[k]()])} />
      </div>
      <Slider label={q.g.kind === "radial" ? t("appearance.bd.ed.origin", "Směr světla") : q.g.kind === "mesh" ? t("appearance.bd.ed.rotate", "Natočení skvrn") : t("appearance.bd.ed.angle", "Úhel")} min={0} max={360} step={5} value={q.g.angle} fmt={(v) => `${v}°`} set={(v) => put({ angle: v })} />

      <div class="gd-preview" style={{ "background-image": gradientCss(q.g) }} aria-hidden="true" />
      <div class="gd-bar" ref={bar} onClick={barClick} title={t("appearance.bd.ed.barHint", "Klikni do lišty pro novou zastávku, úchyt můžeš táhnout.")}>
        <div class="gd-track" style={{ "background-image": gradientCss({ kind: "linear", angle: 90, stops: q.g.stops }) }} />
        <For each={stops()}>
          {(s, idx) => (
            <button
              class="gd-h" classList={{ on: idx() === i() }}
              style={{ left: `${s.p}%`, background: s.c }}
              aria-label={t("appearance.bd.ed.stop", "Zastávka {n}, poloha {p} %", { n: idx() + 1, p: Math.round(s.p) })}
              onPointerDown={(e) => drag(idx(), e)}
              onKeyDown={(e) => key(idx(), e)}
              onFocus={() => setSel(idx())}
            />
          )}
        </For>
      </div>

      <Show when={cur()}>
        <div class="opt">
          <span>{t("appearance.bd.ed.stopSel", "Vybraná zastávka")}<small>{t("appearance.bd.ed.stopHint", "Barva a poloha; šipky na úchytu ji posouvají (Shift = po 10).")}</small></span>
          <span class="gd-stop">
            <input type="color" value={cur().c} aria-label={t("appearance.bd.ed.color", "Barva zastávky")} onInput={(e) => put({ stops: stops().map((s, k) => (k === i() ? { ...s, c: e.currentTarget.value } : s)) })} />
            <input class="gd-num" type="number" min={0} max={100} step={1} value={Math.round(cur().p)} aria-label={t("appearance.bd.ed.pos", "Poloha zastávky v %")}
              onChange={(e) => { const m = moveStop(stops(), i(), +e.currentTarget.value || 0); setSel(m.index); put({ stops: m.stops }); }} />
            <button class="btn" disabled={stops().length >= STOPS_MAX} onClick={() => { const n = addStop(stops()); put({ stops: n }); setSel(n.findIndex((x) => !stops().some((o) => o.p === x.p && o.c === x.c))); }}>{t("appearance.bd.ed.add", "Přidat")}</button>
            <button class="btn" disabled={stops().length <= STOPS_MIN} onClick={() => { put({ stops: removeStop(stops(), i()) }); setSel(0); }}>{t("appearance.bd.ed.remove", "Odebrat")}</button>
          </span>
        </div>
      </Show>
      <div class="opt">
        <span>{t("appearance.bd.ed.delete", "Smazat tento gradient")}<small>{t("appearance.bd.ed.deleteHint", "Vrátí se přes Zahodit změny v liště, dokud neuložíš.")}</small></span>
        <button class="btn" onClick={() => q.ap.set("backdrop", removeGradient(q.bd, q.g.id))}>{t("appearance.bd.ed.deleteBtn", "Smazat")}</button>
      </div>
    </div>
  );
}
