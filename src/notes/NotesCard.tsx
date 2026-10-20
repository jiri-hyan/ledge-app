// Karta Poznámky (záložky podle `kind`), řádek poznámky, záložky a peek připomínky.
// „Vše“ bere řádky ze snapshotu (pořadí a strop dělá Rust `notes::card`); jiná záložka je dotaz
// `note_query` obnovený při změně `notesRev`. Klik na řádek = úprava v Ledge (`onOpen`). Varianta zobrazení
// karty (vzhled karty, vlna 8 P): seznam / dlaždice / kompaktní (`.note-list[data-view]`).

import { createResource, For, Show, type JSX } from "solid-js";
import { api, type NoteView, type Snapshot } from "../api";
import { ago } from "../format";
import { IconBell } from "../icons";
import { KindBadge, kindOf, kindColor, KindIcon } from "./kinds";
import { doneBusy, openExternal, projName, remindLabel, setDone, setPageArchive, setTab, tab } from "./store";
import { t, tp } from "../i18n";
import type { CardView } from "../layout/look";
import { IconPin } from "../pin/icons";
import { isPinned, newPin, togglePin } from "./pins";
import { completeNote, unarchiveNote } from "./lifecycle";
import { IconCheck, IconUnarchive } from "./lifeicons";
import RowMenu from "./RowMenu";

/** Kolik řádků má karta (stejně jako `notes::CARD_ROWS`). */
const CARD_ROWS = 8;

export function NoteRow(p: { s: Snapshot; n: NoteView; now: number; onOpen: (n: NoteView) => void; /** řádek v Archivu: bez zaškrtnutí a lístku, ⋯ nabídne Vrátit z archivu */ archived?: boolean }) {
  const n = () => p.n;
  return (
    <div class="row note-row" classList={{ done: n().done && !p.archived }}>
      <Show when={!p.archived}>
        <button class="check" classList={{ on: n().done }} role="checkbox" aria-checked={n().done} disabled={doneBusy().has(n().path)} title={n().done ? t("notes.row.undo", "Vrátit mezi nehotové") : t("notes.done", "Hotovo")} onClick={() => (n().done ? setDone(n(), false) : void completeNote(n()))} />
      </Show>
      <KindBadge kinds={p.s.noteKinds} id={n().kind} />
      <button class="grow note-open" title={t("notes.row.edit", "Upravit v Ledge")} onClick={() => p.onOpen(n())}>
        <div class="name">{n().title || t("notes.imageOnly", "(jen obrázek)")}</div>
        <div class="sub">{projName(p.s, n().project)} · {ago(n().createdMs, p.now)}{n().images ? tp("notes.row.images", n().images, " · {n}× obrázek") : ""}</div>
      </button>
      <Show when={n().tasksTotal > 0}>
        <span class="pill" classList={{ g: n().tasksDone === n().tasksTotal, n: n().tasksDone < n().tasksTotal }} title={t("notes.row.tasks", "Splněné úkoly")}>{n().tasksDone}/{n().tasksTotal}</span>
      </Show>
      <Show when={remindLabel(n(), p.now)}>{(l) => <span class={`pill ${l()[0]}`}>{l()[1]}</span>}</Show>
      <Show when={!p.archived && !n().done}>
        <button class="note-act" title={t("notes.life.actDone", "Splnit")} aria-label={t("notes.life.actDone", "Splnit")} onClick={() => void completeNote(n())}><IconCheck /></button>
      </Show>
      <Show when={p.archived}>
        <button class="note-act" title={t("notes.life.actUnarchive", "Vrátit z archivu")} aria-label={t("notes.life.actUnarchive", "Vrátit z archivu")} onClick={() => void unarchiveNote(n())}><IconUnarchive /></button>
      </Show>
      <Show when={!p.archived}>
        <button class="note-pin" classList={{ on: isPinned(p.s.pins, n().path) }} aria-pressed={isPinned(p.s.pins, n().path)} title={isPinned(p.s.pins, n().path) ? t("notes.pin.rowUnpin", "Odepnout z plochy") : t("notes.pin.rowPin", "Připnout na plochu — lístek přes ostatní okna")} onClick={() => togglePin(n().path)}><IconPin /></button>
      </Show>
      <RowMenu n={n()} archived={!!p.archived} />
    </div>
  );
}

export type Tab = { id: string; label: string; count: number; color?: string; icon?: string };

/** Záložky: Vše + kategorie z configu s aspoň jednou nehotovou + neznámé kategorie (jako text). */
export function tabsOf(s: Snapshot): Tab[] {
  const counts = s.notesKindCounts ?? {};
  const all = Object.values(counts).reduce((a, b) => a + b, 0);
  const known = s.noteKinds.filter((k) => counts[k.id] > 0).map((k) => ({ id: k.id, label: k.label, count: counts[k.id], color: kindColor(k), icon: k.icon }));
  const unknown = Object.keys(counts)
    .filter((id) => counts[id] > 0 && !s.noteKinds.some((k) => k.id === id))
    .map((id) => ({ id, label: kindOf(s.noteKinds, id).label, count: counts[id] }));
  return [{ id: "all", label: t("notes.tabs.all", "Vše"), count: all }, ...known, ...unknown];
}

export function Tabs(p: { tabs: Tab[]; value: string; onChange: (id: string) => void }) {
  return (
    <div class="note-tabs" role="tablist">
      <For each={p.tabs}>
        {(tb) => (
          <button class="note-tab" classList={{ on: tb.id === p.value }} role="tab" aria-selected={tb.id === p.value} style={tb.color ? { "--k": tb.color } : undefined} onClick={() => p.onChange(tb.id)}>
            <Show when={tb.icon !== undefined}><KindIcon icon={tb.icon!} /></Show>
            {tb.label}
            <span class="cnt">{tb.count}</span>
          </button>
        )}
      </For>
    </div>
  );
}

/** Štítek synchronizace poznámek — jen to, co Rust opravdu ví (žádný odhad stavu). */
function syncLabel(s: Snapshot, now: number): [string, string, string] | null {
  const y = s.notesSync;
  if (!y || y.state === "off") return null;
  const last = y.lastOkMs != null ? ago(y.lastOkMs, now) : null;
  const title = [y.root, y.files.join(", "), y.message].filter(Boolean).join("\n");
  switch (y.state) {
    case "syncing": return ["n", t("notes.sync.syncing", "synchronizuji…"), title];
    case "ok": return ["n", t("notes.sync.ok", "synchronizováno {when}", { when: last ?? "" }).trim(), title];
    case "offline": return ["a", last ? t("notes.sync.offlineLast", "offline — poslední sync {when}", { when: last }) : t("notes.sync.offlineNever", "offline — zatím nesynchronizováno"), title];
    case "neprihlaseno": return ["a", t("notes.sync.signedOut", "sync: nepřihlášeno (gh auth setup-git)"), title];
    case "konflikt": return ["r", t("notes.sync.conflict", "konflikt: {files}", { files: y.files.join(", ") || t("notes.sync.rebase", "rozpracovaný rebase") }), title];
    default: return ["r", t("notes.sync.error", "sync: chyba"), title];
  }
}

/** Karta Poznámky. `personal` = záložka Osobní: jen obecné poznámky ze snapshotu, bez záložek. */
export function NotesCard(p: {
  s: Snapshot;
  now: number;
  onNew: (kind?: string) => void;
  onOpen: (n: NoteView) => void;
  onAll: () => void;
  personal?: boolean;
  /** ⋯ „Skrýt kartu“ z hlavní stránky (`cards/CardMenu.tsx`) */
  menu?: JSX.Element;
  view?: CardView;
}) {
  const tabs = () => (p.personal ? [] : tabsOf(p.s));
  // uložená záložka bez nehotových poznámek → Vše (volba zůstane zapamatovaná)
  const cur = () => (tabs().length > 2 && tabs().some((tb) => tb.id === tab()) ? tab() : "all");
  const [picked] = createResource(
    () => (cur() === "all" ? null : { kind: cur(), rev: p.s.notesRev }),
    (k) => api.noteQuery({ kind: k.kind, limit: CARD_ROWS }),
  );
  const items = () => (p.personal ? p.s.personal.notes : cur() === "all" ? p.s.notes : picked.latest?.notes ?? []);
  const moreCount = () => (p.personal ? p.s.personal.notesMore : cur() === "all" ? p.s.notesMore : Math.max(0, (picked.latest?.total ?? 0) - items().length));
  const open = () => (p.personal ? items().filter((n) => !n.done).length : tabs().find((tb) => tb.id === cur())?.count ?? 0);
  const newKind = () => (cur() === "all" ? undefined : cur());
  return (
    <div class="card wide" data-anim>
      <h4>
        <span class="ic" style="background:var(--blue)" />{t("notes.card.title", "Poznámky")} <span class="n">{open() || ""}</span>
        <button class="mini note-pin-new" onClick={() => newPin()} title={t("notes.pin.newTitle", "Nový lístek na plochu — poznámka přes ostatní okna (QA, checklist)")}><IconPin /></button>
        <button class="mini" onClick={() => p.onNew(newKind())} title={newKind() ? t("notes.card.newKind", "Nová: {kind}", { kind: kindOf(p.s.noteKinds, newKind()!).label }) : t("notes.card.newNote", "Nová poznámka")}>{t("notes.card.new", "+ Nová")}</button>
        {p.menu}
      </h4>
      <Show when={tabs().length > 2}>
        <Tabs tabs={tabs()} value={cur()} onChange={setTab} />
      </Show>
      <Show when={!items().length}>
        <div class="empty">
          {t("notes.card.emptyShort", "Zatím žádné poznámky — i s obrázkem ze schránky.")}
          <button class="mini ok" onClick={() => p.onNew(newKind())}>{t("notes.card.add", "Přidat poznámku")}</button>
          <button class="mini" onClick={() => newPin()} title={t("notes.pin.newTitle", "Nový lístek na plochu — poznámka přes ostatní okna (QA, checklist)")}>{t("notes.card.addPin", "Nový lístek na plochu")}</button>
        </div>
      </Show>
      <div class="note-list" data-view={p.view ?? "list"}>
        <For each={items()}>{(n) => <NoteRow s={p.s} n={n} now={p.now} onOpen={p.onOpen} />}</For>
      </div>
      <Show when={moreCount() > 0 || (!p.personal && items().length > 0)}>
        <button class="mini note-more-btn" onClick={() => { setPageArchive(false); p.onAll(); }}>{moreCount() > 0 ? tp("notes.card.more", moreCount(), "+{n} dalších") : t("notes.card.all", "Všechny poznámky")}</button>
      </Show>
      <Show when={!p.personal && p.s.notesArchived > 0}>
        <button class="mini note-more-btn" title={t("notes.card.archiveTitle", "Archivované poznámky — jdou vrátit nebo smazat")} onClick={() => { setPageArchive(true); p.onAll(); }}>{t("notes.card.archive", "Archiv ({n})", { n: p.s.notesArchived })}</button>
      </Show>
      <Show when={syncLabel(p.s, p.now)}>
        {(l) => <div style={{ "margin-top": "6px" }}><span class={`pill ${l()[0]}`} title={l()[2]}>{l()[1]}</span></div>}
      </Show>
    </div>
  );
}

/** Peek pro připomínku po termínu — vyjede bez fokusu, zmizí po „Hotovo“. */
export function ReminderPeek(p: { s: Snapshot }) {
  const first = () => p.s.reminders[0];
  return (
    <Show when={first()}>
      {(n) => (
        <div class="peek" data-anim>
          <div class="ico"><IconBell /></div>
          <div class="tx">
            <b>{t("notes.peek.title", "Připomínka · {project}", { project: projName(p.s, n().project) })}{p.s.remindersCount > 1 ? ` (+${p.s.remindersCount - 1})` : ""}</b>
            <div class="code" title={n().title}>{n().title || t("notes.imageOnly", "(jen obrázek)")}</div>
          </div>
          <button class="btn" onClick={() => openExternal(n().path)}>{t("notes.peek.open", "Otevřít")}</button>
          <button class="btn pri" onClick={() => setDone(n(), true)}>{t("notes.done", "Hotovo")}</button>
        </div>
      )}
    </Show>
  );
}
