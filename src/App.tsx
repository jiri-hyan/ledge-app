// Skořápka šuplíku: drží snapshot (`api.on("state")` → store → stránky), stránky a navigaci, morf
// proužek ⇄ šuplík (`prepare` / `play` / `closing` + `motion.ts`, měřeno v `drawer.anim`), hlášení
// výšky obsahu (`report`) a fokus (jen výslovný klik → `focusDrawer`, odchod → `releaseFocus`). Malé
// pohledy (peek, náhled po najetí `compact` / `island`) kreslí `hover/SmallView.tsx` stejným morfem.
// Vzhled (motiv, animace) aplikuje `theme/apply.ts` (`initAppearance` v onMount).
// Karty jsou v `cards/`, hlavní stránka v `MainPage.tsx`, poznámky v `notes/`. Nastavení, průvodce, paleta, Poznámky a
// panel „Nová verze“ jsou líné chunky (`lazyViews.tsx`), přednačtené v klidu po prvním otevření šuplíku.
// loader anglických slovníků (líný chunk, jen pro jazyk ≠ cs) dřív než cokoli, co volá setLang()
import "./i18n/load";
import { createSignal, Show, onMount, onCleanup, createEffect, on as onChange } from "solid-js";
import { createStore, reconcile } from "solid-js/store";
import { api, on, type Snapshot, type View, type ViewMsg, type NoteView } from "./api";
import { IconShield } from "./icons";
import { returnTo, setEditPath, setNotePreset, setPageProject, setReturnTo, type NotePreset } from "./notes/store";
import "./notes/notes.css";
import { OVERLAP, spring, collapsedClip, bandClip, FULL_CLIP, measure, motionNow, preHide, revealContent, concealContent } from "./motion";
import { closePlan, openPlan, type Geo, type Plan } from "./theme/openAnim";
import { initAppearance } from "./theme/apply";
import { holdWide, wantWidth } from "./settings/widen";
import MainPage from "./MainPage";
import Head from "./cards/Head";
import SmallView from "./hover/SmallView";
import { isHoverView, smallWidth } from "./hover/model";
import { asPage, typing, type Page } from "./nav";
import { EMPTY_SNAPSHOT } from "./snapshot";
import { Toast, say } from "./ui/toast";
import UndoBar from "./notes/UndoBar";
import { gotoSettingsPanel } from "./palette/goto";
import { checkFirstRun, wizardOpen } from "./onboarding/state";
import { beginEdit, cancelEdit, catalogOpen, draft, editAsk, editing, escEdit, resumeEdit, setCatalogOpen, setEditPage } from "./layout/edit";
import { normalizeLayout } from "./layout/model";
import { NoteEditor, NotesPage, Palette, Settings, UpdatePanel, Wizard, preloadViews } from "./lazyViews";
import { installQuickKeys } from "./quick/quick";
import "./quick/quick.css";
import { clearDrop, offerDrop } from "./actions/dropstate";

export default function App() {
  const [s, setS] = createStore<Snapshot>(structuredClone(EMPTY_SNAPSHOT));
  const [view, setView] = createSignal<View>("hidden");
  /** náhled po najetí (compact / island): za kolik ms se sám rozbalí — z `view` (`drawer.hoverExpandMs`) */
  const [expandMs, setExpandMs] = createSignal(0);
  const [page, setPage] = createSignal<Page>("main");
  const [now, setNow] = createSignal(Date.now());
  /** panel „Nová verze“ (poznámky k vydání + Instalovat) — otevírá chip v hlavičce */
  const [updOpen, setUpdOpen] = createSignal(false);
  let panel!: HTMLDivElement;
  let content!: HTMLDivElement;

  const apply = (snap: Snapshot) => setS(reconcile(snap, { key: "id", merge: true }));

  /** Vstup do poznámky VÝSLOVNÝM klikem (dlaždice, hlavička, „+ Nová“) — jen odtud se bere fokus:
   *  šuplík otevřený najetím je NOACTIVATE a do textarea by jinak nešlo psát. */
  function startNote(preset: NotePreset | null = null) {
    setNotePreset(preset);
    setEditPath(null);
    setReturnTo("main");
    setPage("note");
    api.focusDrawer();
  }
  /** Úprava poznámky v Ledge (klik na řádek) — po uložení / Esc zpět, odkud se přišlo. */
  function editNotePath(path: string) {
    setReturnTo(page() === "notes" ? "notes" : "main");
    setEditPath(path);
    setPage("note");
    api.focusDrawer();
  }
  const editNote = (n: NoteView) => editNotePath(n.path);
  /** Celá stránka Poznámky („+N dalších“); `general` = jen bez projektu (záložka Osobní). */
  function allNotes(general = false) {
    setPageProject(general ? "" : "*");
    setPage("notes");
    api.focusDrawer();
  }
  const notesNav = { onNew: (kind?: string) => startNote(kind ? { kind } : null), onOpen: editNote, onAll: allNotes };

  // ---------- paleta příkazů (Ctrl+K s fokusem, 🔍 klikem — jen klik si bere fokus) ----------
  const [palette, setPalette] = createSignal<{ took: boolean } | null>(null);
  const openPalette = (click: boolean) => {
    if (click) api.focusDrawer();
    setPalette({ took: click });
  };
  const closePalette = () => {
    if (palette()?.took) api.releaseFocus();
    setPalette(null);
  };
  const palNav = {
    onNote: (preset?: NotePreset) => startNote(preset ?? null),
    onOpenNote: editNote,
    onAllNotes: () => allNotes(),
    onSettings: (id?: string) => {
      setPage("settings");
      if (id) gotoSettingsPanel(id);
    },
    onEditDrawer: () => {
      setPage("main");
      beginEdit(s);
    },
  };

  // ---------- morf: proužek ⇄ šuplík (co se animuje, určuje styl pohybu — `theme/openAnim.ts`) ----------
  let pending: Plan | null = null;
  const geo = (w: number, h: number): Geo => ({ collapsed: collapsedClip(w, h), band: bandClip(h), full: FULL_CLIP });
  /** po animaci zpět na „žádný inline styl“ (clip, posun i průhlednost řídí jen plán) */
  const settle = () => {
    panel.style.clipPath = "";
    panel.style.transform = "";
    panel.style.opacity = "";
  };

  function prepare(msg: ViewMsg) {
    const fromVisible = view() !== "hidden";
    setExpandMs(msg.expandMs ?? 0);
    setView(msg.view);
    if (msg.view === "hidden") {
      // skrytí bez morfu (bez `closing`) — příště začít na hlavní stránce
      setPage("main");
      return;
    }
    panel.getAnimations().forEach((a) => a.cancel());
    settle();
    const m = motionNow();
    const plan = msg.morph ? openPlan(m.style, m.speed, geo(msg.fromW, msg.fromH), fromVisible) : null;
    if (plan) {
      // výchozí stav plánu (složený tvar / nad hranou / průhledný) — okno je zatím schované
      Object.assign(panel.style, plan.start);
      preHide(content);
      void panel.offsetHeight; // vynutit styl dřív, než Rust okno ukáže
      pending = plan;
      api.animAck("prepared");
    } else {
      revealContent(content, 40);
      if (msg.morph) api.animAck("prepared");
    }
  }

  function play() {
    const plan = pending;
    pending = null;
    if (!plan) return;
    panel.animate(plan.frames, { duration: plan.duration, easing: plan.easing }).finished.then(settle, () => {});
    // přehrávání začalo z prvního snímku — inline výchozí stav už nepotřebuje (po zrušení by zůstal)
    settle();
    if (plan.breath) {
      // celá plocha se zároveň „nadechne“ z proužku
      const breath = spring(300, 22);
      content.animate([{ transform: "scale(0.97)", transformOrigin: "50% 0" }, { transform: "scale(1)", transformOrigin: "50% 0" }], {
        duration: breath.duration,
        easing: breath.easing,
      });
    }
    revealContent(content, 60, plan.reveal);
    measure("open", plan.duration + 120);
  }

  async function closing(msg: ViewMsg) {
    clearDrop(); // nepotvrzený návrh zástupce z přetaženého se zavřením šuplíku zahodí
    panel.getAnimations().forEach((a) => a.cancel());
    measure("close", 250);
    const hide = concealContent(content, 110);
    const m = motionNow();
    const plan = closePlan(m.style, m.speed, geo(msg.toW, msg.toH));
    const shrink = plan
      ? panel.animate(plan.frames, { duration: plan.duration, delay: plan.delay, easing: plan.easing, fill: "forwards" }).finished.catch(() => {})
      : Promise.resolve();
    await Promise.all([hide, shrink]);
    api.animAck("closed");
    setView("hidden");
    setPage("main");
  }

  const offs: Promise<() => void>[] = [];
  onCleanup(() => offs.forEach((p) => p.then((f) => f())));
  onMount(() => {
    offs.push(
      on<Snapshot>("state", apply),
      on<ViewMsg>("view", prepare),
      on<View>("play", play),
      on<ViewMsg>("closing", closing),
      on<boolean>("glass", (g) => (document.documentElement.dataset.glass = g ? "on" : "off")),
      on<string>("page", (p) => setPage(asPage(p))),
      // lístek na ploše → „Upravit v Ledge“ (Rust pošle cestu poznámky, šuplík se otevře na jejím editoru)
      on<string>("edit-note", editNotePath),
      // přetažení odkazu / souboru na otevřený šuplík → návrh zástupce (`actions/DropNew.tsx`), vždy na hlavní stránce
      on<unknown>("drop-shortcut", (x) => offerDrop(x) && setPage("main")),
      // puštěný soubor, který by akce stejně odmítla (.lnk, .url, .bat, .cmd…): jedna srozumitelná věta z jádra
      on<string>("drop-refused", (why) => typeof why === "string" && why && say(why)),
      initAppearance(),
    );

    // velikost obsahu → Rust nastaví okno (bez přesahu nad okrajem)
    const ro = new ResizeObserver(() => report());
    ro.observe(content);
    onCleanup(() => ro.disconnect());

    const key = (e: KeyboardEvent) => {
      if (page() === "note") return; // klávesy poznámky obsluhuje NoteEditor
      if ((e.ctrlKey || e.metaKey) && !e.altKey && e.key.toLowerCase() === "k") {
        e.preventDefault();
        if (!palette()) openPalette(false);
        return;
      }
      if (e.key === "Escape" && palette()) return closePalette();
      // režim úprav šuplíku: Esc = zavřít katalog, jinak Zrušit (pole přejmenování záložky si Esc obslouží samo)
      if (e.key === "Escape" && editing() && page() === "main") return void (catalogOpen() ? setCatalogOpen(false) : escEdit());
      if (e.key === "Escape") {
        if (page() === "settings" || page() === "notes") setPage("main");
        else api.close();
      } else if (!e.ctrlKey && !e.altKey && !["INPUT", "TEXTAREA", "SELECT"].includes((e.target as HTMLElement).tagName)) {
        // 1–9 = záložka v pořadí z `layout` (v úpravách jen rozpracovaná)
        const n = Number(e.key);
        if (Number.isInteger(n) && n >= 1 && n <= 9) {
          const pg = (draft() ?? normalizeLayout(s.layout)).pages[n - 1];
          if (pg && editing()) setEditPage(pg.id);
          else if (pg) void api.setMode(pg.id);
        }
      }
    };
    window.addEventListener("keydown", key);
    onCleanup(() => window.removeEventListener("keydown", key));
    // Alt+1…9 = N-tý zástupce, šipky mezi zástupci (`quick/`, volba `smart.quickKeys`)
    onCleanup(installQuickKeys(() => ({ enabled: s.smart?.quickKeys ?? false, page: page(), editing: editing(), paletteOpen: !!palette() })));

    // reflektor pod kurzorem pro karty a dlaždice (jen CSS proměnné, žádný re-render)
    const spot = (e: PointerEvent) => {
      const el = (e.target as HTMLElement).closest<HTMLElement>(".card, .proj");
      if (!el) return;
      const r = el.getBoundingClientRect();
      el.style.setProperty("--mx", `${e.clientX - r.left}px`);
      el.style.setProperty("--my", `${e.clientY - r.top}px`);
    };
    window.addEventListener("pointermove", spot, { passive: true });
    onCleanup(() => window.removeEventListener("pointermove", spot));

    // studený WebView: událost `page` (zkratka poznámky, je-li zapnutá) mohla přijít dřív, než existoval → vyzvednout
    Promise.all([api.getState(), api.takePage()]).then(([first, pg]) => {
      if (pg) setPage(asPage(pg));
      if (!first) return;
      apply(first);
      api.uiReady();
    });
  });

  function report() {
    if (!content || view() === "hidden") return;
    const v = view();
    api.reportSize(v, smallWidth(v) ?? wantWidth(page()), Math.ceil(content.getBoundingClientRect().height) - OVERLAP);
  }
  // hodiny v hlavičce: sekundový časovač existuje jen, dokud je šuplík vidět (zavřený = žádné buzení WebView)
  createEffect(() => {
    if (view() === "hidden") return;
    setNow(Date.now());
    const tick = window.setInterval(() => setNow(Date.now()), 1000);
    onCleanup(() => clearInterval(tick));
  });
  createEffect(onChange([view, page, holdWide], () => queueMicrotask(report)));
  createEffect(onChange(view, (v) => v === "hidden" && setPalette(null), { defer: true }));
  // zavřený šuplík rozpracované úpravy rozložení NEzahodí (draft i Zpět žijí v `edit.ts`); po znovuotevření
  // fokus zpět + připomínka. Výslovný odchod na jinou stránku je zahodí.
  createEffect(onChange(page, (pg) => pg !== "main" && cancelEdit(false), { defer: true }));
  let hiddenEditing = false;
  createEffect(onChange(view, (v) => {
    if (v === "hidden") hiddenEditing = editing();
    else if (v === "full" && hiddenEditing) { hiddenEditing = false; if (page() === "main") resumeEdit(); }
  }, { defer: true }));
  // „Upravit přímo v šuplíku“ z Nastavení
  createEffect(onChange(editAsk, () => {
    setPage("main");
    queueMicrotask(() => beginEdit(s));
  }, { defer: true }));
  // průvodce prvním spuštěním: config se čte při prvním otevření; dokud běží, šuplík otevírá na něm
  createEffect(() => view() === "full" && void checkFirstRun());
  // líné pohledy (Nastavení, paleta…) se přednačtou až po prvním zobrazení šuplíku, v klidu
  createEffect(() => view() === "full" && preloadViews());
  createEffect(() => view() === "full" && wizardOpen() && setPage("welcome"));
  // z poznámky zpět na šuplík (Zpět, uložení, Esc) → opener jako před `focusDrawer`
  createEffect(onChange(page, (pg, prev) => prev && typing(prev) && !typing(pg) && view() !== "hidden" && api.releaseFocus(), { defer: true }));

  return (
    <div class="shell" ref={panel}>
      <div ref={content} style={{ "padding-top": `${OVERLAP}px` }}>
        <Show when={view() === "peek" || isHoverView(view())} fallback={
          <div class="content">
            <Show when={page() !== "welcome"}>
              <Head s={s} now={now()} page={page()} onNote={() => startNote()} onSettings={() => setPage(page() === "settings" ? "main" : "settings")} onBack={() => setPage(page() === "note" ? returnTo() : "main")} onUpdate={() => setUpdOpen(!updOpen())} onPalette={() => (palette() ? closePalette() : openPalette(true))} />
            </Show>
            <Show when={palette()}>{(pl) => <Palette s={s} took={pl().took} nav={palNav} onClose={() => setPalette(null)} />}</Show>
            <Show when={s.cfgError}>
              <div class="banner" data-anim><IconShield /> <span>{s.cfgError}</span></div>
            </Show>
            <Show when={updOpen() && page() === "main" && (s.update.state === "dostupna" || s.update.whatsNew?.unread) ? s.update : null}>
              {(u) => <UpdatePanel u={u()} onClose={() => setUpdOpen(false)} />}
            </Show>
            <Show when={page() === "main"}><MainPage s={s} now={now()} onNote={startNote} nav={notesNav} onSettings={() => setPage("settings")} /></Show>
            <Show when={page() === "settings"}><div data-anim><Settings update={s.update} s={s} /></div></Show>
            <Show when={page() === "note"}><NoteEditor s={s} view={view()} onLeave={() => setPage(returnTo())} /></Show>
            <Show when={page() === "welcome"}><Wizard onExit={() => setPage("main")} /></Show>
            <Show when={page() === "notes"}><NotesPage s={s} now={now()} onOpen={editNote} onNew={notesNav.onNew} /></Show>
          </div>
        }>
          <SmallView view={view()} s={s} now={now()} expandMs={expandMs()} />
        </Show>
      </div>
      <UndoBar />
      <Toast />
    </div>
  );
}
