// Karta „Rychlé akce“ (na kterékoli záložce; Práce / Osobní filtruje `showOn`, vlastní záložka ukáže všechny): mřížka dlaždic s ikonou, volitelně ve
// skupinách; klik = spustit (`runAction`: potvrzení, stav běží, výsledek do hlášky / výstupu).
// Prázdná karta se ukáže mimo Práci (pozvánka), na Práci by jen zabírala místo (`layout/model.ts` `shownCards`). Skripty ze složky
// skriptů jsou tu jen se `scripts.inActions` (`s.scripts`); výstup skriptu spuštěného z palety se ale
// ukáže vždy (karta se kvůli němu objeví i prázdná). Vlna 8 P: vlastní vzhled dlaždice (pravý klik → „Upravit
// vzhled…“, v režimu úprav šuplíku klik; panel pod dlaždicemi), varianta zobrazení dlaždice / seznam / kompaktní.
import { For, Show, type JSX } from "solid-js";
import type { Snapshot } from "../types";
import { isBusy, lastResult } from "../actions/api";
import { runAny } from "../actions/scripts";
import { isLink, visibleGroups } from "../actions/model";
import { ActionIcon, ConfirmAction, LastOutput, askConfirm } from "../actions/ui";
import { gotoSettingsPanel } from "../palette/goto";
import { t } from "../i18n";
import type { ActionView } from "../types";
import type { CardView } from "../layout/look";
import { editing } from "../layout/edit";
import LookEditor from "../tilelook/LookEditor";
import { isPlain, tileLabel } from "../tilelook/model";
import { lookOf, lookTarget, openLook, saveTileLook } from "../tilelook/state";
import { openTileMenu } from "../tilelook/TileMenu";
import { actionIconSrc } from "../actions/api";

/** Vzhled akce jde měnit jen u akcí z configu (skripty ze složky v configu nejsou). */
export function actionLookMenu(e: MouseEvent, a: ActionView, cfgError: string | null) {
  if (a.script) return;
  const items = [{ text: t("cards.tile.edit", "Upravit vzhled…"), run: () => openActionLook(a, cfgError) }];
  if (!isPlain(a.look)) items.push({ text: t("cards.tile.resetLook", "Obnovit vzhled"), run: () => void saveTileLook("action", a.id, undefined) });
  openTileMenu(e, items);
}
export const openActionLook = (a: ActionView, cfgError: string | null) => !a.script && openLook({ kind: "action", id: a.id, name: a.label, initial: a.look }, cfgError);
export const actionAuto = (a: ActionView) => ({ src: actionIconSrc(a), mono: a.monogram, color: a.color, plate: false });

export default function ActionsCard(p: { s: Snapshot; onSettings: () => void; wide?: boolean; skipLinks?: boolean; view?: CardView; menu?: JSX.Element }) {
  /** `skipLinks`: na záložce je i karta Odkazy → weby, složky a soubory jsou jen tam (žádné dvojí dlaždice) */
  const own = () => (p.skipLinks ? (p.s.actions ?? []).filter((a) => !isLink(a)) : (p.s.actions ?? []));
  const groups = () => visibleGroups([...own(), ...(p.s.scripts ?? [])], p.s.mode);
  const ids = () => groups().flatMap((g) => g.items.map((a) => a.id));
  /** výstup skriptu (i z palety, když skripty v kartě nejsou) */
  const scriptOut = () => (lastResult()?.id.startsWith("script:") ? lastResult()!.id : null);
  const edit = () => {
    p.onSettings();
    gotoSettingsPanel("akce");
  };
  return (
    <Show when={ids().length > 0 || p.s.mode !== "prace" || scriptOut()}>
      <div class="card act-card" classList={{ wide: !!p.wide }} data-anim data-view={p.view ?? "tiles"} data-edit={editing() ? "" : undefined}>
        <h4>
          <span class="ic" style="background:var(--teal)" />
          {t("actions.card.title", "Rychlé akce")}
          <span class="n">{ids().length || ""}</span>
          <button class="mini" onClick={edit}>{t("actions.card.edit", "Upravit")}</button>
          {p.menu}
        </h4>
        <For each={groups()} fallback={
          <Show when={!scriptOut()}>
            <div class="empty">
              {t("actions.card.empty", "Weby, složky, aplikace i příkazy na jedno kliknutí.")}
              <button class="mini" onClick={edit}>{t("actions.card.add", "Přidat akci")}</button>
            </div>
          </Show>
        }>
          {(g) => (
            <>
              <Show when={g.group}><div class="act-group">{g.group}</div></Show>
              <div class="act-grid">
                <For each={g.items}>
                  {(a) => {
                    const busy = () => a.running || isBusy(a.id);
                    const look = () => lookOf("action", a.id, a.look);
                    return (
                      <button
                        class="act-tile"
                        data-tl-tile={`a:${a.id}`}
                        classList={{ bad: !!a.problem, busy: busy() }}
                        aria-busy={busy()}
                        aria-label={look()?.hideLabel ? a.label : undefined}
                        title={a.problem ? `${a.target}\n${a.problem}` : a.needsConfirm ? `${a.confirmText}\n${t("actions.card.asks", "Před spuštěním se zeptá.")}` : a.target}
                        onClick={() => (editing() ? openActionLook(a, p.s.cfgError) : runAny(a, askConfirm("card")))}
                        onContextMenu={(e) => actionLookMenu(e, a, p.s.cfgError)}
                      >
                        <span class="act-ico">
                          <ActionIcon a={a} />
                          <Show when={busy()}><span class="act-spin" aria-hidden="true" /></Show>
                        </span>
                        <Show when={!look()?.hideLabel}><span class="act-label">{tileLabel(a.label, look())}</span></Show>
                      </button>
                    );
                  }}
                </For>
              </div>
            </>
          )}
        </For>
        <Show when={lookTarget()?.kind === "action" ? groups().flatMap((g) => g.items).find((a) => a.id === lookTarget()!.id) : null} keyed>
          {(a) => <LookEditor auto={actionAuto(a)} editing={editing()} />}
        </Show>
        <ConfirmAction where="card" />
        <LastOutput ids={scriptOut() ? [...ids(), scriptOut()!] : ids()} />
      </div>
    </Show>
  );
}
