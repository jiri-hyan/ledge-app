// Náhled po najetí na dlaždici projektu (vlna 9 Z): jméno, stav (běží / port / startuje / chyba), čím a odkud
// se spouští — jen informace, akce jsou v menu dlaždice (pravý klik, ⋯, Shift+F10). Kreslí se v popoveru
// (`ui/Popover.tsx`, `passive`) nad šuplíkem, u okraje se otočí nahoru / posune dovnitř.
import { onCleanup, onMount, Show } from "solid-js";
import type { ProcView, ProjectView } from "../api";
import { t } from "../i18n";
import { tipState, whatOf, whereOf } from "./model";
import { tipLeave } from "./state";
import "./projpanel.css";

export default function TileTip(p: { pr: ProjectView; proc: ProcView | undefined }) {
  const st = () => tipState(p.pr, p.proc);
  // dlaždici může nový snapshot překreslit (pointerleave starého prvku pak nepřijde) — náhled zmizí, jakmile
  // kurzor není nad dlaždicí tohohle projektu; posluchač jen po dobu náhledu
  onMount(() => {
    const sel = `[data-tl-tile="p:${CSS.escape(p.pr.id)}"]`;
    const move = (e: PointerEvent) => !(e.target as Element | null)?.closest?.(sel) && tipLeave();
    document.addEventListener("pointermove", move, true);
    onCleanup(() => document.removeEventListener("pointermove", move, true));
  });
  return (
    <div class="pz-tip">
      <div class="pz-tip-head">
        <b>{p.pr.name}</b>
        <span class="pill" classList={{ g: st().tone === "g", a: st().tone === "a", r: st().tone === "r", n: st().tone === "n" }}>{st().text}</span>
      </div>
      <div class="pz-tip-row"><span>{p.pr.kind === "dev" ? t("cards.proj.edit.what", "Spustí") : t("cards.proj.edit.app", "Otevře")}</span><code>{whatOf(p.pr) ?? "—"}</code></div>
      <div class="pz-tip-row"><span>{t("cards.proj.edit.where", "Ze složky")}</span><code>{whereOf(p.pr, p.proc)}</code></div>
      <Show when={st().detail}>{(d) => <div class="pz-tip-err">{d()}</div>}</Show>
      <div class="pz-tip-hint">{t("cards.proj.tip.menu", "Pravý klik nebo ⋯ = všechny volby")}</div>
    </div>
  );
}
