// Panel „Nový zástupce z přetaženého“ pod hlavičkou hlavní stránky (kolo 20): editor akce (`ActionEditor`)
// předvyplněný z puštěného odkazu / souboru / složky / .exe (`dropshortcut.ts`). Nic se nespustí ani neuloží, dokud
// člověk nepotvrdí „Přidat“; pak se akce zapíše do configu (`get_config` → `save_config`, validace v Rustu jako
// všude) a spouští se dál obvyklou cestou akcí (kontrola cíle, potvrzení, souhlasy). Když na záložce není karta,
// která akci ukáže, nabídne se přidat ji (zaškrtnuto, jedním klikem se vrátí v ⋯ Skrýt kartu).
import { createSignal, Show } from "solid-js";
import { api } from "../api";
import type { ActionCfg, Snapshot } from "../types";
import { addCard, pageOf } from "../layout/model";
import { saveLayout } from "../layout/edit";
import { say, sayError } from "../ui/toast";
import { t, tp } from "../i18n";
import ActionEditor from "./ActionEditor";
import { actionsApi } from "./api";
import { draftFrom, visibility } from "./dropshortcut";
import { clearDrop, dropped } from "./dropstate";
import { cardName } from "../layout/text";
import { slugId } from "./model";

export default function DropNew(p: { s: Snapshot; page: string }) {
  const [addHost, setAddHost] = createSignal(true);
  const taken = () => (p.s.actions ?? []).map((a) => a.id);
  const groups = () => [...new Set((p.s.actions ?? []).map((a) => a.group?.trim()).filter((g): g is string => !!g))];

  async function save(a: ActionCfg, consent: boolean) {
    const cfg = await api.getConfig();
    if (!cfg) return;
    const list: ActionCfg[] = cfg.actions ?? [];
    const saved: ActionCfg = { ...a, id: slugId(a.label, list.map((x) => x.id)) };
    await api.saveConfig({ ...cfg, actions: [...list, saved] });
    if (consent) await actionsApi.consent(saved).catch(sayError);
    const v = visibility(pageOf(p.s.layout, p.page).cards.map((c) => c.id), saved.kind);
    if (!v.visible && addHost()) await saveLayout((l) => addCard(l, p.page, v.add), p.s.cfgError);
    clearDrop();
    say(t("actions.drop.added", "Zástupce „{label}“ přidán.", { label: saved.label }));
  }

  return (
    <Show when={dropped()} keyed>
      {(d) => {
        const draft = draftFrom(d, taken());
        const hostToAdd = () => {
          const v = visibility(pageOf(p.s.layout, p.page).cards.map((c) => c.id), draft.kind);
          return v.visible ? null : v.add;
        };
        return (
          <div class="card act-drop" role="dialog" aria-label={t("actions.drop.title", "Nový zástupce z přetaženého")}>
            <h4>
              <span class="ic" style="background:var(--teal)" />
              {t("actions.drop.title", "Nový zástupce z přetaženého")}
            </h4>
            <p class="act-dropnote">{t("actions.drop.note", "Zkontroluj a potvrď. Dokud nedáš Přidat, nic se neuloží ani nespustí.")}</p>
            <Show when={d.skipped > 0}>
              <p class="act-dropnote">{tp("actions.drop.skipped", d.skipped, "Další přetažené položky ({n}) jsem přeskočil — navrhuju jen první.")}</p>
            </Show>
            <ActionEditor initial={draft} isNew groups={groups()} onSave={save} onCancel={clearDrop} />
            <Show when={hostToAdd()}>
              {(card) => (
                <label class="act-check act-drophost">
                  <input type="checkbox" checked={addHost()} onChange={(e) => setAddHost(e.currentTarget.checked)} />
                  <span>{t("actions.drop.host", "Na téhle záložce není karta, která by zástupce ukázala — přidat kartu „{name}“.", { name: cardName(card()) })}</span>
                </label>
              )}
            </Show>
          </div>
        );
      }}
    </Show>
  );
}
