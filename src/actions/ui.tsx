// Sdílené kousky UI rychlých akcí: ikona (obrázek z jádra nebo monogram v barvě), potvrzení
// spuštění uvnitř šuplíku (ne `window.confirm` — modální okno by bralo fokus a šuplík zavřelo)
// a výsledek posledního skrytého příkazu. Používá karta, panel Nastavení i paleta. Akce s vlastním
// vzhledem (`ActionView.look`, vlna 8 P) se kreslí přes `tilelook/TileIcon.tsx` — i v paletě.
import { createSignal, Show } from "solid-js";
import type { ActionView } from "../types";
import { actionIconSrc, lastResult, setLastResult } from "./api";
import { t } from "../i18n";
import { useTone } from "../ui/iconTone";
import TileIcon from "../tilelook/TileIcon";
import { isPlain } from "../tilelook/model";
import { lookOf } from "../tilelook/state";
import "./actions.css";

export function ActionIcon(p: { a: Pick<ActionView, "id" | "iconKey" | "monogram" | "color" | "look">; size?: number }) {
  const src = () => actionIconSrc(p.a);
  const px = () => `${p.size ?? 30}px`;
  // tmavá / světlá kresba (favicon GitHubu) dostane podklad, když by splynula s motivem (`ui/iconTone.ts`)
  const tone = useTone(src);
  const look = () => lookOf("action", p.a.id, p.a.look);
  return (
    <Show when={!isPlain(look()) ? look() : null} fallback={
      <Show when={src()} fallback={<span class="act-mono" style={{ "--c": p.a.color, width: px(), height: px() }}>{p.a.monogram}</span>}>
        {(s) => <img class="act-img" classList={{ "tone-dark": tone() === "dark", "tone-light": tone() === "light" }} src={s()} alt="" style={{ width: px(), height: px() }} draggable={false} />}
      </Show>
    }>
      {(l) => <TileIcon look={l()} auto={{ src: src(), mono: p.a.monogram, color: p.a.color, plate: false }} size={px()} />}
    </Show>
  );
}

type Pending = { a: ActionView; where: string; resolve: (ok: boolean) => void };
const [pending, setPending] = createSignal<Pending | null>(null);

/** `ask` pro `runAction`: potvrzení se ukáže v komponentě `<ConfirmAction where={…} />`. */
export const askConfirm = (where: string) => (a: ActionView) =>
  new Promise<boolean>((resolve) => {
    pending()?.resolve(false);
    setPending({ a, where, resolve });
  });

/** Čeká potvrzení na tomhle místě? (paleta podle toho mapuje Enter/Esc) */
export const confirmPending = (where: string) => pending()?.where === where;

export function answerConfirm(ok: boolean) {
  const p = pending();
  setPending(null);
  p?.resolve(ok);
}

export function ConfirmAction(p: { where: string }) {
  return (
    <Show when={pending()?.where === p.where ? pending() : null}>
      {(x) => (
        <div class="act-confirm" role="alertdialog" aria-label={t("actions.confirm.aria", "Potvrzení akce")}>
          <div class="grow">
            <div class="name">{t("actions.confirm.title", "Spustit „{label}“?", { label: x().a.label })}</div>
            <div class="code" title={x().a.confirmText}>{x().a.confirmText}</div>
            <Show when={x().a.confirmNote}>{(n) => <div class="act-confirm-note">{n()}</div>}</Show>
          </div>
          <button class="mini" onClick={() => answerConfirm(false)}>{t("actions.confirm.cancel", "Zrušit")}</button>
          <button class="mini ok" onClick={() => answerConfirm(true)}>{t("actions.confirm.run", "Spustit")}</button>
        </div>
      )}
    </Show>
  );
}

/** Výstup posledního skrytého příkazu (jen když patří k akci z `ids`). */
export function LastOutput(p: { ids: string[] }) {
  return (
    <Show when={lastResult() && p.ids.includes(lastResult()!.id) ? lastResult() : null}>
      {(r) => (
        <div class="act-out" classList={{ err: !r().ok }}>
          <div class="act-out-head">
            <span>{r().title}</span>
            <button class="mini" onClick={() => setLastResult(null)}>{t("actions.output.close", "Skrýt")}</button>
          </div>
          <pre class="log">{r().output}</pre>
        </div>
      )}
    </Show>
  );
}
