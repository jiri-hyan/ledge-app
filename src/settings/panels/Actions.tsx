// Sekce „Akce a odkazy“ panelu Projekty, akce a odkazy: JEDEN seznam akcí z `cfg.actions` — odkaz je akce,
// která jen otevírá (web, složka, soubor; karta Odkazy je ukazuje, `actions/model.ts` `isLink`) — přidat,
// upravit, řadit přetažením za úchyt nebo Alt+šipkou (`SortList`), spustit, smazat s Vrátit; editor
// `actions/ActionEditor`. Starý `personal.links` převádí na akce Rust (`linkmigrate.rs`).
// Jako všechny panely mění jen draft — zapíše společná lišta (Uložit / Zahodit); smazání nabídne Vrátit
// v liště (`offerUndo`). Souhlas s příkazem přijme jádro až pro uloženou akci → `afterSave`. Spustit jde
// jen akce, která je uložená v tomhle tvaru (jinak by běžela stará verze).
import { createMemo, createSignal, Show } from "solid-js";
import type { PanelProps } from "../types";
import type { ActionCfg, ActionView } from "../../types";
import ActionEditor, { kindLabel } from "../../actions/ActionEditor";
import { actionsApi, runAction } from "../../actions/api";
import { cleanAction, moveItem, slugId } from "../../actions/model";
import { ActionIcon, ConfirmAction, LastOutput, askConfirm } from "../../actions/ui";
import { say } from "../../ui/toast";
import { t } from "../../i18n";
import { afterSave, base, hit, offerUndo } from "../ctx";
import { Section } from "../fields/Row";
import { SortList } from "../fields/SortList";

const NEW: ActionCfg = { id: "", label: "", kind: "url", target: "", showOn: "both" };
const shape = (a: ActionCfg) => JSON.stringify(cleanAction(structuredClone(a)));

export default function ActionsPanel(p: PanelProps) {
  const list = (): ActionCfg[] => p.cfg.actions ?? [];
  const views = () => new Map<string, ActionView>((p.s.actions ?? []).map((v) => [v.id, v]));
  const groups = () => [...new Set(list().map((a) => a.group?.trim()).filter((g): g is string => !!g))];
  /** rozpracovaná akce: index v seznamu, `null` = nová */
  const [editing, setEditing] = createSignal<{ index: number | null; draft: ActionCfg } | null>(null);
  /** uložené akce (id → tvar) — „Spustit“ jen pro akci, která je uložená přesně takhle */
  const savedShapes = createMemo(() => {
    const m = new Map<string, string>();
    try {
      for (const a of (JSON.parse(base() || "{}").actions ?? []) as ActionCfg[]) m.set(a.id, shape(a));
    } catch {
      // bez uložené verze nic nespouštět
    }
    return m;
  });
  const isSaved = (a: ActionCfg) => savedShapes().get(a.id) === shape(a);

  const commit = (next: ActionCfg[]) => p.setCfg("actions", next);

  async function onSave(a: ActionCfg, consent: boolean) {
    const e = editing();
    if (!e) return;
    const others = list().filter((_, i) => i !== e.index);
    const saved = { ...a, id: a.id || slugId(a.label, others.map((x) => x.id)) };
    const next = list().slice();
    if (e.index === null) next.push(saved);
    else next[e.index] = saved;
    commit(next);
    // souhlas jádro přijme jen pro uloženou akci — po Uložit, a jen když se mezitím nezměnila
    if (consent) {
      const want = shape(saved);
      afterSave(() => (list().some((x) => x.id === saved.id && shape(x) === want) ? actionsApi.consent(saved) : undefined));
    }
    setEditing(null);
    say(e.index === null ? t("actions.panel.added", "Akce „{label}“ přidána — ulož ji dole v liště.", { label: saved.label }) : t("actions.panel.saved", "Změněno — ulož dole v liště."));
  }

  function remove(index: number) {
    const a = structuredClone(list()[index]);
    commit(list().filter((_, i) => i !== index));
    offerUndo(t("actions.panel.deleted", "Akce „{label}“ smazána.", { label: a.label }), () => {
      const next = list().slice();
      next.splice(Math.min(index, next.length), 0, a);
      commit(next);
    });
  }

  const lit = (id: string) => hit() === id;
  return (
    <>
      <Section
        id="actions"
        title={t("actions.panel.title", "Akce a odkazy")}
        desc={t("actions.panel.intro", "Weby, složky, soubory, aplikace a příkazy na jedno kliknutí — karta Odkazy a akce, karta Odkazy (weby, složky a soubory) a paleta (Ctrl+K nebo 🔍).")}
        aside={
          <Show when={!editing()}>
            <span id="f-actions.add" class="act-add" classList={{ hit: lit("actions.add") }}>
              <button class="mini" onClick={() => setEditing({ index: null, draft: { ...NEW } })}>+ {t("actions.panel.add", "Přidat akci nebo odkaz")}</button>
            </span>
          </Show>
        }
      >
        <Show when={editing()} fallback={
          <div class="act-list">
            <Show when={list().length} fallback={<div class="empty">{t("actions.panel.empty", "Zatím žádné akce ani odkazy. Začni třeba oblíbeným webem nebo složkou s projekty.")}</div>}>
              <p class="st-note">{t("actions.panel.order", "Pořadí změníš přetažením za úchyt (nebo Alt+šipkou) — v tomhle pořadí je ukazují karty i paleta.")}</p>
              <SortList items={list()} label={t("actions.panel.title", "Akce a odkazy")} onMove={(from, to) => commit(moveItem(list(), from, to))}>
                {(a, i, handle) => {
                  const v = () => views().get(a.id);
                  return (
                    <div class="act-row">
                      {handle()}
                      <Show when={v()} fallback={<span class="act-mono" style={{ width: "26px", height: "26px" }}>?</span>}>{(vv) => <ActionIcon a={vv()} size={26} />}</Show>
                      <div class="grow">
                        <div class="name">{a.label}</div>
                        <div class="sub" classList={{ err: !!v()?.problem }}>{v()?.problem ?? a.target}</div>
                      </div>
                      <span class="pill n">{kindLabel(a.kind)}</span>
                      <Show when={a.group}><span class="pill v">{a.group}</span></Show>
                      <div class="r">
                        <Show when={v()}>
                          {(vv) => (
                            <button class="mini" disabled={!isSaved(a)} title={isSaved(a) ? undefined : t("actions.panel.unsaved", "Nejdřív ulož (lišta dole) — pak poběží v tomhle tvaru")} onClick={() => runAction(vv(), askConfirm("panel"))}>
                              {t("actions.panel.run", "Spustit")}
                            </button>
                          )}
                        </Show>
                        <button class="mini" onClick={() => setEditing({ index: i(), draft: structuredClone(a) })}>{t("actions.panel.edit", "Upravit")}</button>
                        <button class="mini" onClick={() => remove(i())}>{t("actions.panel.delete", "Smazat")}</button>
                      </div>
                    </div>
                  );
                }}
              </SortList>
            </Show>
            <ConfirmAction where="panel" />
            <LastOutput ids={list().map((a) => a.id)} />
          </div>
        }>
          {(e) => (
            <ActionEditor
              initial={e().draft}
              isNew={e().index === null}
              groups={groups()}
              saved={e().index === null ? undefined : views().get(e().draft.id)}
              runnable={isSaved(e().draft)}
              onSave={onSave}
              onCancel={() => setEditing(null)}
            />
          )}
        </Show>
      </Section>
    </>
  );
}
