// Stránka „note“: nová poznámka (koncept) nebo úprava existující (`editPath`). Kategorie, projekt,
// připomínka, text / náhled Markdownu, obrázky ze schránky. Uložení úpravy chrání otisk souboru —
// změnil-li se mezitím na disku (sync), nabídne „Načíst znovu“ / „Uložit jako kopii“.
// Stav přežívající odchod ze stránky je v draft.ts; klávesy (Esc, Ctrl+Enter) obsluhuje tahle stránka.

import { createEffect, createSignal, For, on as onChange, onCleanup, onMount, Show } from "solid-js";
import { api, type Snapshot, type View } from "../api";
import { IconFile, IconShield, IconX } from "../icons";
import { say } from "../ui/toast";
import { t } from "../i18n";
import * as D from "./draft";
import { KindPicker } from "./kinds";
import Markdown, { toggleLocal } from "./Markdown";
import ProjectPicker from "./ProjectPicker";
import RemindPicker from "./RemindPicker";
import { remindDefault } from "./remind";
import { editPath, notePreset, openExternal, projName, setEditPath, setNotePreset, type NotePreset } from "./store";

export default function NoteEditor(p: { s: Snapshot; view: View; onLeave: () => void }) {
  let root!: HTMLDivElement;
  let ta: HTMLTextAreaElement | undefined;
  let remindBtn: HTMLButtonElement | undefined;
  const [confirm, setConfirm] = createSignal(false);
  const [busy, setBusy] = createSignal(false);
  const [preview, setPreview] = createSignal(false);
  const [conflict, setConflict] = createSignal(false);
  const [ready, setReady] = createSignal(false);
  const editing = () => editPath() !== null;

  // pole podle režimu (nová = koncept, úprava = rozepsaná úprava)
  const text = () => (editing() ? D.edText() : D.draft());
  const setText = (v: string) => (editing() ? D.setEdText(v) : D.setDraft(v));
  const proj = () => (editing() ? D.edProj() : D.draftProj());
  const setProj = (v: string) => (editing() ? D.setEdProj(v) : (D.setProjTouched(true), D.setDraftProj(v)));
  const kind = () => (editing() ? D.edKind() : D.draftKind());
  const setKind = (v: string) => (editing() ? D.setEdKind(v) : D.setDraftKind(v));
  const remind = () => (editing() ? D.edRemind() : D.draftRemind());
  const setRemind = (v: string) => (editing() ? D.setEdRemind(v) : D.setDraftRemind(v));
  const imgs = () => (editing() ? D.edImgs() : D.draftImgs());
  const setImgs = (v: string[]) => (editing() ? D.setEdImgs(v) : D.setDraftImgs(v));
  const hasContent = () => text().trim() !== "" || imgs().length > 0;
  const dirty = () => (editing() ? D.edDirty() : hasContent());

  // rozepsané na disk s debounce; obrázky konceptu hned (jsou vzácné)
  createEffect(onChange([D.draft, D.draftProj, D.projTouched, D.draftRemind, D.draftKind], () => ready() && !editing() && D.scheduleDraft(), { defer: true }));
  createEffect(onChange(D.draftImgs, (i) => ready() && !editing() && api.noteDraftImages(D.b64Of(i)).catch((e) => say(String(e), true)), { defer: true }));
  createEffect(onChange([D.edText, D.edProj, D.edKind, D.edRemind], () => ready() && editing() && D.scheduleEdit(), { defer: true }));
  onCleanup(D.flushDraft);

  // otevření: úprava (může přesměrovat na jinou rozepsanou), nebo koncept + předvyplnění
  createEffect(onChange(editPath, async (path) => {
    setReady(false);
    setConflict(false);
    setConfirm(false);
    setPreview(false);
    if (path) {
      const real = await D.openEdit(path);
      if (real === null) return p.onLeave();
      if (real !== path) return setEditPath(real);
    } else {
      await D.restoreDraft();
      const pre = notePreset();
      setNotePreset(null);
      if (pre) applyPreset(pre);
      // předvyplnit projekt podle poslední Claude session — jen dokud ho šéf sám nezměnil
      else if (!D.projTouched() && p.s.mode !== "osobni")
        api.notePrefill().then((id) => !D.projTouched() && D.setDraftProj(id && p.s.projects.some((x) => x.id === id) ? id : ""));
    }
    setReady(true);
  }));

  /** Dlaždice projektu / záložka: rozepsaný koncept k JINÉMU projektu se nepřepisuje, jen otevře. */
  function applyPreset(pre: NotePreset) {
    if (pre.project !== undefined) {
      if (hasContent() && D.draftProj() !== pre.project) {
        say(t("notes.editor.otherDraft", "Máš rozepsanou poznámku ({project}) — dokonči ji, nebo ji Esc zahodí.", { project: projName(p.s, D.draftProj() || null) }));
        return;
      }
      D.setDraftProj(pre.project);
      D.setProjTouched(true);
    }
    if (pre.kind && !hasContent()) D.setDraftKind(pre.kind);
    if (pre.remind) {
      // rozepsanému konceptu nechat jeho termín; prázdný (i se starým termínem) dostane nový
      if (!D.draftRemind() || !hasContent()) D.setDraftRemind(remindDefault());
      queueMicrotask(() => remindBtn?.focus());
    } else if (!hasContent()) D.setDraftRemind("");
  }

  async function save() {
    if (busy() || !hasContent()) return;
    if (editing() && !D.edDirty()) return p.onLeave();
    setBusy(true);
    try {
      if (editing()) await saveEdit(false);
      else await saveNew();
    } catch (e) {
      say(String(e), true);
    } finally {
      setBusy(false);
    }
  }

  async function saveNew() {
    const sent = { ...D.draftMeta(), images: D.draftImgs() };
    await api.noteCreate({ text: sent.text, project: sent.project || null, remind: sent.remind || null, images: D.b64Of(sent.images), kind: sent.kind || null });
    // mezitím dopsané (save je async) nezahodit: reset a zavření jen když se koncept nezměnil
    const same = D.draft() === sent.text && D.draftImgs() === sent.images && D.draftProj() === sent.project && D.draftRemind() === sent.remind;
    if (same) {
      D.resetDraft();
      api.close();
    } else say(t("notes.editor.savedKept", "Uloženo — mezitím dopsaný text zůstal v konceptu."));
  }

  async function saveEdit(copy: boolean) {
    const b = D.edBase();
    if (!b) return;
    const out = await api.noteSave(
      { path: b.path, expectedHash: D.edHash(), body: D.edText(), project: D.edProj() || null, kind: D.edKind(), remind: D.edRemind() || null, images: D.b64Of(D.edImgs()) },
      copy,
    );
    if (out?.outcome === "changed") return setConflict(true);
    D.resetEdit();
    say(copy ? t("notes.editor.savedCopy", "Uloženo jako nová poznámka vedle původní.") : t("notes.editor.saved", "Uloženo"));
    p.onLeave();
  }

  async function reload() {
    const path = D.edBase()?.path;
    D.resetEdit();
    setConflict(false);
    if (!path) return;
    const real = await D.openEdit(path);
    if (real === null) p.onLeave();
  }

  async function toggleTask(line: number, checked: boolean) {
    const b = D.edBase();
    if (!editing() || !b || D.edDirty()) return setText(toggleLocal(text(), line, checked));
    try {
      const out = await api.noteToggleTask(b.path, line, checked, D.edHash());
      if (out?.outcome === "changed") setConflict(true);
      else if (out) D.loadEditFrom(out.note);
    } catch (e) {
      say(String(e), true);
    }
  }

  function discard() {
    setConfirm(false);
    if (editing()) {
      D.resetEdit();
      p.onLeave();
    } else {
      D.resetDraft();
      api.close();
    }
  }

  // na textarea jen když fokus není jinde na stránce (výběr projektu, připomínky) — aktivace okna
  // (`focus_drawer`) přijde asynchronně a nesmí ho přebít
  const focus = () => !root?.contains(document.activeElement) && ta?.focus();
  onMount(() => {
    const key = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        e.preventDefault();
        if (preview()) setPreview(false);
        else if (!dirty() || confirm()) editing() && !dirty() ? p.onLeave() : discard();
        else setConfirm(true);
      } else if (e.key === "Enter" && e.ctrlKey) {
        e.preventDefault();
        save();
      } else if (confirm() && !["Control", "Shift", "Alt"].includes(e.key)) setConfirm(false);
    };
    window.addEventListener("keydown", key);
    window.addEventListener("focus", focus);
    onCleanup(() => {
      window.removeEventListener("keydown", key);
      window.removeEventListener("focus", focus);
    });
  });
  createEffect(() => p.view === "full" && setTimeout(focus, 30));

  return (
    <div class="card note-page" ref={root} data-anim onPaste={(e) => pasteImagesHere(e)}>
      <div class="note-top">
        <KindPicker kinds={p.s.noteKinds} value={kind()} onChange={setKind} />
        <div class="seg">
          <button classList={{ on: !preview() }} onClick={() => setPreview(false)}>{t("notes.editor.edit", "Upravit")}</button>
          <button classList={{ on: preview() }} onClick={() => setPreview(true)}>{t("notes.editor.preview", "Náhled")}</button>
        </div>
        <Show when={editing() && D.edBase()}>
          {(b) => <button class="icon-btn" title={t("notes.editor.openExternal", "Otevřít v editoru\n{path}", { path: b().path })} onClick={() => openExternal(b().path)}><IconFile /></button>}
        </Show>
      </div>
      <Show when={preview()} fallback={
        <textarea
          ref={(el) => (ta = el)}
          class="note-text"
          rows={editing() ? 10 : 6}
          autofocus
          placeholder={t("notes.editor.placeholder", "Co tě napadlo… Markdown: **tučně**, - [ ] úkol. Obrázek: Win+Shift+S a pak Ctrl+V.")}
          value={text()}
          onInput={(e) => setText(e.currentTarget.value)}
        />
      }>
        <div class="note-preview">
          <Show when={text().trim()} fallback={<div class="empty">{t("notes.editor.emptyPreview", "Zatím prázdné.")}</div>}>
            <Markdown body={text()} images={editing() ? D.edBase()?.imageData ?? [] : []} onToggle={toggleTask} />
          </Show>
        </div>
      </Show>
      <Show when={imgs().length > 0}>
        <div class="note-imgs">
          <For each={imgs()}>
            {(u, i) => (
              <div class="note-img">
                <img src={u} alt="" draggable={false} />
                <button title={t("notes.editor.removeImage", "Odebrat obrázek")} onClick={() => setImgs(imgs().filter((_, j) => j !== i()))}><IconX /></button>
              </div>
            )}
          </For>
        </div>
      </Show>
      <Show when={conflict()}>
        <div class="banner warn">
          <IconShield />
          <span>{t("notes.editor.conflict", "Poznámka se mezitím změnila (nejspíš přišla synchronizace z druhého PC). Tvoje úprava zůstala tady.")}</span>
          <button class="mini" onClick={reload}>{t("notes.editor.reload", "Načíst znovu")}</button>
          <button class="mini ok" onClick={() => saveEdit(true).catch((e) => say(String(e), true))}>{t("notes.editor.saveCopy", "Uložit jako kopii")}</button>
        </div>
      </Show>
      <div class="note-bar">
        <ProjectPicker projects={p.s.projects} value={proj()} onChange={setProj} label={t("notes.project", "Projekt")} />
        <RemindPicker value={remind()} onChange={setRemind} label={t("notes.editor.remind", "Připomenout")} ref={(el) => (remindBtn = el)} />
        <Show when={confirm()} fallback={<span class="hint">{editing() ? t("notes.editor.hintEdit", "Ctrl+Enter uloží · Esc zpět") : t("notes.editor.hintNew", "Ctrl+Enter uloží · Esc zavře")}</span>}>
          <span class="hint warn">{editing() ? t("notes.editor.discardEdit", "Zahodit úpravy?") : t("notes.editor.discardNew", "Zahodit rozepsanou poznámku?")} {t("notes.editor.discardHint", "Esc znovu = zahodit")}</span>
        </Show>
        <button class="btn pri" disabled={busy() || !hasContent() || (editing() && !D.edDirty())} onClick={save}>{t("notes.editor.save", "Uložit")}</button>
      </div>
    </div>
  );

  function pasteImagesHere(e: ClipboardEvent) {
    return D.pasteImages(e, imgs, setImgs);
  }
}
