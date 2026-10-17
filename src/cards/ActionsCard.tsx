// Karta „Rychlé akce“ (Práce i Osobní podle `showOn`): mřížka dlaždic s ikonou, volitelně ve
// skupinách; klik = spustit (`runAction`: potvrzení, stav běží, výsledek do hlášky / výstupu).
// Prázdná karta se ukáže jen na Osobní (pozvánka), na Práci by jen zabírala místo. Skripty ze složky
// skriptů jsou tu jen se `scripts.inActions` (`s.scripts`); výstup skriptu spuštěného z palety se ale
// ukáže vždy (karta se kvůli němu objeví i prázdná).
import { For, Show, type JSX } from "solid-js";
import type { Snapshot } from "../types";
import { isBusy, lastResult } from "../actions/api";
import { runAny } from "../actions/scripts";
import { visibleGroups } from "../actions/model";
import { ActionIcon, ConfirmAction, LastOutput, askConfirm } from "../actions/ui";
import { gotoSettingsPanel } from "../palette/goto";
import { t } from "../i18n";

export default function ActionsCard(p: { s: Snapshot; onSettings: () => void; wide?: boolean; menu?: JSX.Element }) {
  const groups = () => visibleGroups([...(p.s.actions ?? []), ...(p.s.scripts ?? [])], p.s.mode);
  const ids = () => groups().flatMap((g) => g.items.map((a) => a.id));
  /** výstup skriptu (i z palety, když skripty v kartě nejsou) */
  const scriptOut = () => (lastResult()?.id.startsWith("script:") ? lastResult()!.id : null);
  const edit = () => {
    p.onSettings();
    gotoSettingsPanel("akce");
  };
  return (
    <Show when={ids().length > 0 || p.s.mode === "osobni" || scriptOut()}>
      <div class="card act-card" classList={{ wide: !!p.wide }} data-anim>
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
                    return (
                      <button
                        class="act-tile"
                        classList={{ bad: !!a.problem, busy: busy() }}
                        aria-busy={busy()}
                        title={a.problem ? `${a.target}\n${a.problem}` : a.needsConfirm ? `${a.confirmText}\n${t("actions.card.asks", "Před spuštěním se zeptá.")}` : a.target}
                        onClick={() => runAny(a, askConfirm("card"))}
                      >
                        <span class="act-ico">
                          <ActionIcon a={a} />
                          <Show when={busy()}><span class="act-spin" aria-hidden="true" /></Show>
                        </span>
                        <span class="act-label">{a.label}</span>
                      </button>
                    );
                  }}
                </For>
              </div>
            </>
          )}
        </For>
        <ConfirmAction where="card" />
        <LastOutput ids={scriptOut() ? [...ids(), scriptOut()!] : ids()} />
      </div>
    </Show>
  );
}
