// Celá stránka Poznámky („+N dalších“): záložky kategorií, filtr projektu, přepínač „i hotové“.
// Data přes `note_query` (z paměti Rustu), obnova při změně `notesRev`.

import { createResource, createSignal, For, Show } from "solid-js";
import { api, type NoteView, type Snapshot } from "../api";
import ProjectPicker from "./ProjectPicker";
import { NoteRow, Tabs, tabsOf } from "./NotesCard";
import { pageProject, setPageProject, setTab, tab } from "./store";
import { t } from "../i18n";

const PAGE_LIMIT = 300;

export default function NotesPage(p: { s: Snapshot; now: number; onOpen: (n: NoteView) => void; onNew: (kind?: string) => void }) {
  const [withDone, setWithDone] = createSignal(false);
  const tabs = () => tabsOf(p.s);
  // na stránce jde vybrat i záložku bez nehotových (s „i hotové“ může mít obsah)
  const cur = () => (tab() === "all" || p.s.noteKinds.some((k) => k.id === tab()) || tabs().some((tb) => tb.id === tab()) ? tab() : "all");
  const allTabs = () => {
    const tb = tabs();
    const extra = p.s.noteKinds.filter((k) => !tb.some((x) => x.id === k.id) && (withDone() || k.id === cur())).map((k) => ({ id: k.id, label: k.label, count: 0, color: k.color, icon: k.icon }));
    return [...tb, ...extra];
  };
  const project = () => (pageProject() === "*" ? null : pageProject() === "" ? "_obecne" : pageProject());
  const [res] = createResource(
    () => ({ kind: cur() === "all" ? null : cur(), project: project(), includeDone: withDone(), rev: p.s.notesRev }),
    (q) => api.noteQuery({ kind: q.kind, project: q.project, includeDone: q.includeDone, limit: PAGE_LIMIT }),
  );
  const list = () => res.latest?.notes ?? [];
  return (
    <div class="card notes-page" data-anim>
      <div class="note-bar">
        <Tabs tabs={allTabs()} value={cur()} onChange={setTab} />
      </div>
      <div class="note-bar">
        <ProjectPicker projects={p.s.projects} value={pageProject()} onChange={setPageProject} all label={t("notes.project", "Projekt")} />
        <button class="opt-inline" onClick={() => setWithDone(!withDone())} role="switch" aria-checked={withDone()}>
          <span class="switch" classList={{ on: withDone() }} /> {t("notes.page.withDone", "i hotové")}
        </button>
        <button class="mini push" onClick={() => p.onNew(cur() === "all" ? undefined : cur())}>{t("notes.card.new", "+ Nová")}</button>
      </div>
      <div class="notes-list">
        <For each={list()} fallback={<div class="empty">{res.loading ? t("notes.page.loading", "Načítám…") : t("notes.page.empty", "Nic tu není.")}</div>}>
          {(n) => <NoteRow s={p.s} n={n} now={p.now} onOpen={p.onOpen} />}
        </For>
      </div>
      <Show when={(res.latest?.total ?? 0) > list().length}>
        <div class="empty">{t("notes.page.limited", "Zobrazeno {shown} z {total} — zuž výběr záložkou nebo projektem.", { shown: list().length, total: res.latest!.total })}</div>
      </Show>
    </div>
  );
}
