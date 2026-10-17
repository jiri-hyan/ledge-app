// Panel Nastavení „Rychlé akce“: seznam akcí z `cfg.actions` (přidat, upravit, řadit přetažením,
// spustit, smazat s Vrátit), převod odkazů z Osobní (nic nemaže) a editor (`actions/ActionEditor`).
// Ukládá přes `save()` skořápky; souhlas s příkazem až po uložení (`action_consent`).
import { createSignal, For, onCleanup, Show } from "solid-js";
import type { PanelProps } from "../types";
import type { ActionCfg, ActionView } from "../../types";
import ActionEditor, { kindLabel } from "../../actions/ActionEditor";
import { actionsApi, runAction } from "../../actions/api";
import { linksToActions, moveItem, slugId } from "../../actions/model";
import { ActionIcon, ConfirmAction, LastOutput, askConfirm } from "../../actions/ui";
import ScriptsSettings from "../../actions/ScriptsSettings";
import { say, sayError } from "../../ui/toast";
import { t, tp } from "../../i18n";

const NEW: ActionCfg = { id: "", label: "", kind: "url", target: "", showOn: "both" };
const UNDO_MS = 8000;

export default function ActionsPanel(p: PanelProps) {
  const list = (): ActionCfg[] => p.cfg.actions ?? [];
  const views = () => new Map<string, ActionView>((p.s.actions ?? []).map((v) => [v.id, v]));
  const groups = () => [...new Set(list().map((a) => a.group?.trim()).filter((g): g is string => !!g))];
  /** rozpracovaná akce: index v seznamu, `null` = nová */
  const [editing, setEditing] = createSignal<{ index: number | null; draft: ActionCfg } | null>(null);
  const [undo, setUndo] = createSignal<{ a: ActionCfg; index: number } | null>(null);
  const [drag, setDrag] = createSignal<number | null>(null);
  const [over, setOver] = createSignal<number | null>(null);
  let undoTimer: number | undefined;
  onCleanup(() => clearTimeout(undoTimer));

  async function commit(next: ActionCfg[]) {
    p.setCfg("actions", next);
    await p.save();
  }

  async function onSave(a: ActionCfg, consent: boolean) {
    const e = editing();
    if (!e) return;
    const others = list().filter((_, i) => i !== e.index);
    const saved = { ...a, id: a.id || slugId(a.label, others.map((x) => x.id)) };
    const next = list().slice();
    if (e.index === null) next.push(saved);
    else next[e.index] = saved;
    await commit(next);
    if (consent) await actionsApi.consent(saved);
    setEditing(null);
    say(e.index === null ? t("actions.panel.added", "Akce „{label}“ přidána.", { label: saved.label }) : t("actions.panel.saved", "Uloženo."));
  }

  async function remove(index: number) {
    const a = list()[index];
    try {
      await commit(list().filter((_, i) => i !== index));
      setUndo({ a, index });
      clearTimeout(undoTimer);
      undoTimer = window.setTimeout(() => setUndo(null), UNDO_MS);
    } catch (e) {
      sayError(e);
    }
  }

  async function restore() {
    const u = undo();
    if (!u) return;
    setUndo(null);
    const next = list().slice();
    next.splice(Math.min(u.index, next.length), 0, u.a);
    await commit(next).catch(sayError);
  }

  async function drop(to: number) {
    const from = drag();
    setDrag(null);
    setOver(null);
    if (from === null || from === to) return;
    await commit(moveItem(list(), from, to)).catch(sayError);
  }

  const links = () => p.cfg.personal?.links ?? [];
  const convertible = () => linksToActions(links(), list());
  async function convert() {
    const add = convertible();
    await commit([...list(), ...add]).catch(sayError);
    say(t("actions.panel.converted", "Převedeno {n} odkazů. V kartě Odkazy zůstaly — smazat je můžeš tam.", { n: add.length }));
  }

  return (
    <>
    <div class="card wide" data-anim>
      <h4>
        <span class="ic" style="background:var(--teal)" />
        {t("actions.panel.title", "Rychlé akce")}
        <span class="n">{list().length || ""}</span>
        <Show when={!editing()}>
          <button class="mini" onClick={() => setEditing({ index: null, draft: { ...NEW } })}>+ {t("actions.panel.add", "Přidat akci")}</button>
        </Show>
      </h4>
      <div class="empty" style="padding-top:0">
        {t("actions.panel.intro", "Weby, složky, soubory, aplikace a příkazy na jedno kliknutí — karta Rychlé akce na záložce Práce / Osobní a paleta (Ctrl+K nebo 🔍).")}
      </div>
      <Show when={convertible().length > 0 && !editing()}>
        <div class="act-note">
          <span class="grow">{tp("actions.panel.links", convertible().length, ["Na záložce Osobní máš {n} odkaz, který tu ještě není.", "Na záložce Osobní máš {n} odkazy, které tu ještě nejsou.", "Na záložce Osobní máš {n} odkazů, které tu ještě nejsou."])}</span>
          <button class="mini" onClick={convert}>{t("actions.panel.convert", "Převést na akce")}</button>
        </div>
      </Show>
      <Show when={undo()}>
        {(u) => (
          <div class="act-note">
            <span class="grow">{t("actions.panel.deleted", "Akce „{label}“ smazána.", { label: u().a.label })}</span>
            <button class="mini ok" onClick={restore}>{t("actions.panel.undo", "Vrátit")}</button>
          </div>
        )}
      </Show>
      <Show when={editing()} fallback={
        <div class="act-list">
          <For each={list()} fallback={<div class="empty">{t("actions.panel.empty", "Zatím žádné akce. Začni třeba oblíbeným webem nebo složkou s projekty.")}</div>}>
            {(a, i) => {
              const v = () => views().get(a.id);
              return (
                <div
                  class="act-row"
                  classList={{ dragging: drag() === i(), "drag-over": over() === i() && drag() !== i() }}
                  draggable={true}
                  onDragStart={(e) => {
                    setDrag(i());
                    e.dataTransfer?.setData("text/plain", a.id);
                  }}
                  onDragOver={(e) => {
                    e.preventDefault();
                    setOver(i());
                  }}
                  onDragEnd={() => {
                    setDrag(null);
                    setOver(null);
                  }}
                  onDrop={(e) => {
                    e.preventDefault();
                    drop(i());
                  }}
                >
                  <span class="act-handle" title={t("actions.panel.drag", "Přetažením změníš pořadí")}>⋮⋮</span>
                  <Show when={v()} fallback={<span class="act-mono" style={{ width: "26px", height: "26px" }}>?</span>}>{(vv) => <ActionIcon a={vv()} size={26} />}</Show>
                  <div class="grow">
                    <div class="name">{a.label}</div>
                    <div class="sub" classList={{ err: !!v()?.problem }}>{v()?.problem ?? a.target}</div>
                  </div>
                  <span class="pill n">{kindLabel(a.kind)}</span>
                  <Show when={a.group}><span class="pill v">{a.group}</span></Show>
                  <div class="r">
                    <Show when={v()}>{(vv) => <button class="mini" onClick={() => runAction(vv(), askConfirm("panel"))}>{t("actions.panel.run", "Spustit")}</button>}</Show>
                    <button class="mini" onClick={() => setEditing({ index: i(), draft: structuredClone(a) })}>{t("actions.panel.edit", "Upravit")}</button>
                    <button class="mini" onClick={() => remove(i())}>{t("actions.panel.delete", "Smazat")}</button>
                  </div>
                </div>
              );
            }}
          </For>
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
            onSave={onSave}
            onCancel={() => setEditing(null)}
          />
        )}
      </Show>
    </div>
    <ScriptsSettings />
    </>
  );
}
