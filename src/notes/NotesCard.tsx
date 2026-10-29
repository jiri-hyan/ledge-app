// Karta Poznámky (záložky podle `kind`), řádek poznámky, záložky a peek připomínky.
// „Vše“ bere řádky ze snapshotu (pořadí a strop dělá Rust `notes::card`); jiná záložka je dotaz
// `note_query` obnovený při změně `notesRev`. Klik na řádek = úprava v Ledge (`onOpen`). Varianta zobrazení
// karty (vzhled karty, vlna 8 P): seznam / dlaždice / kompaktní (`.note-list[data-view]`). Lístky 2.0: řádek nese
// barvu poznámky (proužek vlevo) a štítky, pod záložkami filtr podle štítku (`note_tags`, jen když nějaké jsou).

import { createMemo, createResource, For, Match, Show, Switch, type JSX } from "solid-js";
import { api, type NoteView, type Snapshot } from "../api";
import { ago } from "../format";
import { IconBell } from "../icons";
import { KindBadge, kindOf, kindColor, KindIcon } from "./kinds";
import { doneBusy, openExternal, projectAccent, projName, remindLabel, setDone, setPageArchive, setTab, setTagFilter, tab, tagFilter } from "./store";
import { stripeColor } from "./meta";
import { notesApi } from "./api";
import { lang, t, tp } from "../i18n";
import { groupByOf, groupNotes, groupStyleOf, type Group } from "./group";
import { headlineParts, shortDate } from "./headline";
import { kanbanSplit, notesDesign } from "./noteDesigns";
import { IconPin } from "../pin/icons";
import { isPinned, newPin, togglePin } from "./pins";
import { completeNote, unarchiveNote } from "./lifecycle";
import { IconCheck, IconUnarchive } from "./lifeicons";
import RowMenu from "./RowMenu";
import { say, sayError } from "../ui/toast";
import { bentoLines, heroOf, TEMPLATES } from "./bento";
import { tx } from "../layout/text";
import { exportNote } from "./exportActions";

function IconCopy() {
  return (
    <svg viewBox="0 0 16 16" fill="currentColor" aria-hidden="true">
      <path d="M4 2a2 2 0 0 1 2-2h6a2 2 0 0 1 2 2v8a2 2 0 0 1-2 2H6a2 2 0 0 1-2-2V2zm2-1a1 1 0 0 0-1 1v8a1 1 0 0 0 1 1h6a1 1 0 0 0 1-1V2a1 1 0 0 0-1-1H6zM2 5a1 1 0 0 0-1 1v8a2 2 0 0 0 2 2h6a1 1 0 0 0 1-1v-1H3a2 2 0 0 1-2-2V6a1 1 0 0 0-1-1H2z"/>
    </svg>
  );
}

/** Text poznámky do schránky — přes jádro (šuplík nemá fokus, `navigator.clipboard` by selhal). */
async function copyNote(path: string) {
  try {
    await api.noteCopyText(path);
    say(t("notes.row.copied", "Zkopírováno do schránky"));
  } catch (e) {
    sayError(e);
  }
}

/** Kolik řádků má karta (stejně jako `notes::CARD_ROWS`). */
const CARD_ROWS = 8;

export function NoteRow(p: { s: Snapshot; n: NoteView; now: number; onOpen: (n: NoteView) => void; /** řádek v Archivu: bez zaškrtnutí a lístku, ⋯ nabídne Vrátit z archivu */ archived?: boolean }) {
  const n = () => p.n;
  const stripe = () => stripeColor(n(), projectAccent(p.s, n().project));
  // vlastní nadpis → hlavička „Projekt · Název · 26. 10.“ (projekt jen když je); bez něj dnešní první řádek
  const head = () => (n().heading ? headlineParts(n(), n().project ? projName(p.s, n().project) : null, lang()).join(" · ") : n().title);
  return (
    <div class="row note-row" classList={{ done: n().done && !p.archived }} style={stripe() ? { "box-shadow": `inset 3px 0 0 ${stripe()}`, "--note-c": stripe()! } : undefined}>
      <Show when={!p.archived}>
        <button class="check" classList={{ on: n().done }} role="checkbox" aria-checked={n().done} disabled={doneBusy().has(n().path)} title={n().done ? t("notes.row.undo", "Vrátit mezi nehotové") : t("notes.done", "Hotovo")} onClick={() => (n().done ? setDone(n(), false) : void completeNote(n()))} />
      </Show>
      <KindBadge kinds={p.s.noteKinds} id={n().kind} />
      <button class="grow note-open" title={t("notes.row.edit", "Upravit v Ledge")} onClick={() => p.onOpen(n())}>
        <div class="name">{head() || t("notes.imageOnly", "(jen obrázek)")}</div>
        <div class="sub">{n().heading ? "" : `${projName(p.s, n().project)} · `}{ago(n().createdMs, p.now)}{n().images ? tp("notes.row.images", n().images, " · {n}× obrázek") : ""}{n().tags?.length ? ` · ${n().tags!.slice(0, 4).map((x) => `#${x}`).join(" ")}` : ""}</div>
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
        <button class="note-act note-act-copy" title={t("notes.row.copy", "Kopírovat text")} aria-label={t("notes.row.copy", "Kopírovat text")} onClick={() => void copyNote(n().path)}><IconCopy /></button>
      </Show>
      <Show when={!p.archived}>
        <button class="note-pin" classList={{ on: isPinned(p.s.pins, n().path) }} aria-pressed={isPinned(p.s.pins, n().path)} title={isPinned(p.s.pins, n().path) ? t("notes.pin.rowUnpin", "Odepnout z plochy") : t("notes.pin.rowPin", "Připnout na plochu — lístek přes ostatní okna")} onClick={() => togglePin(n().path)}><IconPin /></button>
      </Show>
      <RowMenu n={n()} archived={!!p.archived} inherited={projectAccent(p.s, n().project)} />
    </div>
  );
}

/** Položka časové osy: datum vytvoření vlevo, tečka v barvě poznámky na ose, název a podtitul vpravo. */
function TimelineItem(p: { s: Snapshot; n: NoteView; now: number; onOpen: (n: NoteView) => void }) {
  const c = () => stripeColor(p.n, projectAccent(p.s, p.n.project));
  return (
    <div class="note-tl-item" classList={{ done: p.n.done }} style={c() ? { "--note-c": c()! } : undefined}>
      <span class="note-tl-date">{shortDate(p.n.createdMs, lang())}</span>
      <span class="note-tl-dot" aria-hidden="true" />
      <button class="note-tl-body" title={t("notes.row.edit", "Upravit v Ledge")} onClick={() => p.onOpen(p.n)}>
        <span class="note-tl-title">{p.n.title || t("notes.imageOnly", "(jen obrázek)")}</span>
        <span class="note-tl-sub">{p.n.project ? `${projName(p.s, p.n.project)} · ` : ""}{ago(p.n.createdMs, p.now)}</span>
      </button>
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
  /** vzhled karty (`noteDesigns.ts`); chybí / neznámé = seznam */
  design?: string;
  /** seskupení a styl oddělení (volby vzhledu karty, `group.ts`) */
  groupBy?: string;
  groupStyle?: string;
}) {
  const design = () => notesDesign(p.design);
  const tabs = () => (p.personal ? [] : tabsOf(p.s));
  // uložená záložka bez nehotových poznámek → Vše (volba zůstane zapamatovaná)
  const cur = () => (tabs().length > 2 && tabs().some((tb) => tb.id === tab()) ? tab() : "all");
  // štítky pro filtr: jen z paměti jádra, znovu jen při změně poznámek (notesRev)
  const [tags] = createResource(() => (p.personal ? null : { rev: p.s.notesRev }), () => notesApi.tags().catch(() => []));
  const tagList = () => (tags.latest ?? []).slice(0, 8);
  // vybraný štítek, který už žádná nehotová poznámka nemá → bez filtru
  const tag = () => (tagFilter() && (tags.latest ?? []).some((x) => x.tag === tagFilter()) ? tagFilter() : null);
  const filtered = () => cur() !== "all" || !!tag();
  const [picked] = createResource(
    () => (filtered() ? { kind: cur() === "all" ? null : cur(), tag: tag(), rev: p.s.notesRev } : null),
    (k) => api.noteQuery({ kind: k.kind, tag: k.tag, limit: CARD_ROWS }),
  );
  const items = () => (p.personal ? p.s.personal.notes : !filtered() ? p.s.notes : picked.latest?.notes ?? []);
  const moreCount = () => (p.personal ? p.s.personal.notesMore : !filtered() ? p.s.notesMore : Math.max(0, (picked.latest?.total ?? 0) - items().length));
  const open = () => (p.personal ? items().filter((n) => !n.done).length : tabs().find((tb) => tb.id === cur())?.count ?? 0);
  const newKind = () => (cur() === "all" ? undefined : cur());

  const by = () => groupByOf(p.groupBy);
  const gstyle = () => groupStyleOf(p.groupStyle);
  /** `.note-list[data-view]` podle vzhledu (CSS už zná list / compact / tiles / bento) */
  const listView = () => ({ list: "list", compact: "compact", cards: "tiles", bento: "bento", kanban: "list", timeline: "list" })[design()];
  const groups = createMemo<Group[]>(() =>
    groupNotes(items(), by(), {
      now: p.now,
      project: (id) => projName(p.s, id),
      kind: (id) => kindOf(p.s.noteKinds, id).label,
      pinned: (n) => isPinned(p.s.pins, n.path),
      text: { today: t("notes.group.today", "Dnes"), week: t("notes.group.week", "Tento týden"), older: t("notes.group.older", "Starší"), pinned: t("notes.group.pinned", "Připnuté"), rest: t("notes.group.rest", "Ostatní") },
    }),
  );
  // Kanban: hotové poznámky karta jinak nedrží, proto vlastní dotaz s `includeDone` (jen ve vzhledu Kanban)
  // dva dotazy, každý s vlastním stropem: mnoho otevřených nesmí vytlačit hotové
  const [kanRes] = createResource(
    () => (design() === "kanban" && !p.personal ? { kind: cur() === "all" ? null : cur(), tag: tag(), rev: p.s.notesRev } : null),
    async (k) => {
      const [open, done] = await Promise.all([
        api.noteQuery({ kind: k.kind, tag: k.tag, limit: CARD_ROWS }),
        api.noteQuery({ kind: k.kind, tag: k.tag, onlyDone: true, limit: CARD_ROWS }),
      ]);
      return { open: open?.notes ?? [], done: done?.notes ?? [] };
    },
  );
  const kan = () => {
    if (p.personal || !kanRes.latest) return kanbanSplit(items());
    return kanRes.latest;
  };

  // varianta Bento (`bento.ts`): velká připnutá / nejnovější poznámka + dlaždice akcí
  const bento = () => design() === "bento" && items().length > 0;
  const hero = () => heroOf(items(), p.s.pins);
  const [heroDetail] = createResource(
    () => (bento() && hero() ? { path: hero()!.path, rev: p.s.notesRev } : null),
    (k) => api.noteRead(k.path).catch(() => null),
  );
  async function fromTemplate(kind: string, text: string) {
    try {
      const path = await api.noteCreate({ text, kind, project: null, remind: null, images: [] });
      const detail = path ? await api.noteRead(path).catch(() => null) : null;
      if (detail) p.onOpen(detail);
    } catch (e) {
      sayError(e);
    }
  }

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
      <Show when={tagList().length}>
        <div class="note-tagbar" role="group" aria-label={t("notes.card.tagFilter", "Filtr podle štítku")}>
          <For each={tagList()}>
            {(x) => (
              <button class="note-tagchip" classList={{ on: tag() === x.tag }} aria-pressed={tag() === x.tag} title={tag() === x.tag ? t("notes.card.tagOff", "Zrušit filtr štítku") : t("notes.card.tagOn", "Jen poznámky se štítkem #{tag}", { tag: x.tag })} onClick={() => setTagFilter(tag() === x.tag ? null : x.tag)}>
                #{x.tag}<span class="cnt">{x.count}</span>
              </button>
            )}
          </For>
        </div>
      </Show>
      <Show when={!items().length}>
        <div class="empty">
          {t("notes.card.emptyShort", "Zatím žádné poznámky — i s obrázkem ze schránky.")}
          <button class="mini ok" onClick={() => p.onNew(newKind())}>{t("notes.card.add", "Přidat poznámku")}</button>
          <button class="mini" onClick={() => newPin()} title={t("notes.pin.newTitle", "Nový lístek na plochu — poznámka přes ostatní okna (QA, checklist)")}>{t("notes.card.addPin", "Nový lístek na plochu")}</button>
        </div>
      </Show>
      <Show when={bento() ? hero() : null}>
        {(h) => (
          <div class="note-bento">
            <button class="note-bento-hero" onClick={() => p.onOpen(h())}>
              <span class="note-bento-head">
                <span class="note-bento-tag">{isPinned(p.s.pins, h().path) ? t("notes.bento.pinned", "Připnutá na ploše") : t("notes.bento.latest", "Nejnovější")}</span>
                <span class="note-bento-title">{h().title || t("notes.imageOnly", "(jen obrázek)")}</span>
                <span class="note-bento-sub">{projName(p.s, h().project)} · {ago(h().createdMs, p.now)}</span>
              </span>
              <span class="note-bento-preview">
                <Show when={bentoLines(heroDetail()?.body).length} fallback={<span class="note-bento-muted">{t("notes.bento.emptyBody", "(bez textu)")}</span>}>
                  <For each={bentoLines(heroDetail()?.body)}>{(l) => <span class="note-bento-line" classList={{ "note-bento-task": l.task }}>{l.text}</span>}</For>
                </Show>
              </span>
            </button>
            <div class="note-bento-acts">
              <button class="mini ok note-bento-btn" onClick={() => p.onNew(newKind())}>+ {t("notes.bento.newMd", "Nová .md")}</button>
              <div class="note-bento-templates" role="group" aria-label={t("notes.bento.templates", "Nová ze šablony")}>
                <For each={TEMPLATES}>
                  {(x) => <button class="mini note-bento-tmpl" title={t("notes.bento.templates", "Nová ze šablony")} onClick={() => void fromTemplate(x.kind, tx(x.body))}>{tx(x.name)}</button>}
                </For>
              </div>
              <div class="note-bento-tools">
                <button class="mini note-bento-tool" onClick={() => void exportNote(h().path, "html")} title={t("notes.bento.htmlTitle2", "Uložit jako samostatný HTML soubor s obrázky")}>{t("notes.bento.html", "Uložit HTML…")}</button>
                <button class="mini note-bento-tool" onClick={() => void exportNote(h().path, "pdf")} title={t("notes.bento.pdfTitle", "Uložit jako PDF (A4) — bez prohlížeče a tiskárny")}>{t("notes.bento.pdf", "Uložit PDF…")}</button>
                <button class="mini note-bento-tool" onClick={() => void openExternal(h().path)} title={t("notes.bento.openTitle", "Otevřít v aplikaci pro .md")}>{t("notes.bento.open", "Otevřít")}</button>
              </div>
            </div>
          </div>
        )}
      </Show>
      <div class="note-wrap">
        <Switch>
          <Match when={design() === "kanban" && items().length > 0}>
            <div class="note-kanban">
              <For each={[[t("notes.kanban.open", "Otevřené"), kan().open], [t("notes.kanban.done", "Hotové"), kan().done]] as [string, NoteView[]][]}>
                {([label, rows]) => (
                  <section class="note-kcol">
                    <div class="note-group-head"><span class="note-group-label">{label}</span><span class="note-group-n">{rows.length}</span></div>
                    <div class="note-list" data-view="list">
                      <For each={rows} fallback={<div class="note-kempty">—</div>}>{(n) => <NoteRow s={p.s} n={n} now={p.now} onOpen={p.onOpen} />}</For>
                    </div>
                  </section>
                )}
              </For>
            </div>
          </Match>
          <Match when={design() === "timeline" && items().length > 0}>
            <div class="note-tl">
              <For each={[...items()].sort((a, b) => b.createdMs - a.createdMs)}>{(n) => <TimelineItem s={p.s} n={n} now={p.now} onOpen={p.onOpen} />}</For>
            </div>
          </Match>
          <Match when={by() !== "none" && items().length > 0}>
            <div class="note-groups" data-gs={gstyle()}>
              <For each={groups()}>
                {(g) => (
                  <section class="note-group">
                    <div class="note-group-head"><span class="note-group-label">{g.label}</span><span class="note-group-n">{g.items.length}</span></div>
                    <div class="note-list" data-view={listView()}>
                      <For each={g.items}>{(n) => <NoteRow s={p.s} n={n} now={p.now} onOpen={p.onOpen} />}</For>
                    </div>
                  </section>
                )}
              </For>
            </div>
          </Match>
          <Match when={true}>
            <div class="note-list" data-view={listView()}>
              <For each={items()}>{(n) => <NoteRow s={p.s} n={n} now={p.now} onOpen={p.onOpen} />}</For>
            </div>
          </Match>
        </Switch>
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
