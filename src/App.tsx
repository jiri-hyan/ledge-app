// Skořápka šuplíku: drží snapshot (`api.on("state")` → store → stránky), stránky a navigaci, morf
// proužek ⇄ šuplík (`prepare` / `play` / `closing` + `motion.ts`, měřeno v `drawer.anim`), hlášení
// výšky obsahu (`report`) a fokus (jen výslovný klik → `focusDrawer`, odchod → `releaseFocus`). Malé
// pohledy (peek, náhled po najetí `island` / `dock`) kreslí `hover/SmallView.tsx` stejným morfem.
// Vzhled (motiv, animace) aplikuje `theme/apply.ts` (`initAppearance` v onMount).
// Karty jsou v `cards/`, hlavní stránka v `MainPage.tsx`, poznámky v `notes/`. Nastavení, průvodce, paleta, Poznámky a
// panel „Nová verze“ jsou líné chunky (`lazyViews.tsx`); v klidu po prvním otevření se přednačte jen paleta a Poznámky, zbytek až při potřebě.
// loader anglických slovníků (líný chunk, jen pro jazyk ≠ cs) dřív než cokoli, co volá setLang()
import "./i18n/load";
import { now, setNow } from "./clock";
import Backdrop from "./theme/BackdropLayer";
import { createSignal, createMemo, Show, onMount, onCleanup, createEffect, on as onChange } from "solid-js";
import { createStore, reconcile } from "solid-js/store";
import { api, on, type Snapshot, type View, type ViewMsg, type NoteView } from "./api";
import { IconShield } from "./icons";
import { returnTo, setEditPath, setNotePreset, setPageProject, setReturnTo, type NotePreset } from "./notes/store";
import "./notes/notes.css";
import { OVERLAP, spring, collapsedClip, bandClip, FULL_CLIP, measure, motionNow, preHide, revealContent, concealContent } from "./motion";
import { closePlan, openPlan, type Geo, type Plan } from "./theme/openAnim";
import { initAppearance } from "./theme/apply";
import { wantWidth } from "./settings/widen";
import MainPage from "./MainPage";
import Head from "./cards/Head";
import SmallView from "./hover/SmallView";
import Mid from "./hover/Mid";
import { dockMode } from "./hover/dockmode";
import { isHoverView, smallWidth } from "./hover/model";
import { asPage, typing, type Page } from "./nav";
import { DORMANT_AFTER_MS, isDormant } from "./dormant";
import { dropLogos } from "./logos";
import { dropCovers } from "./media/covers";
import { EMPTY_SNAPSHOT } from "./snapshot";
import { Toast, say } from "./ui/toast";
import { ConfigNotice } from "./ConfigNotice";
import UndoBar from "./notes/UndoBar";
import { isTextTarget, typeToPalette } from "./palette/typeto";
import { gotoSettingsPanel, openSettings, setStudioFallback } from "./palette/goto";
import { checkFirstRun, startWizard, wizardOpen } from "./onboarding/state";
import { askEdit, beginEdit, cancelEdit, catalogOpen, draft, editAsk, editing, escEdit, resumeEdit, setCatalogOpen, setEditPage } from "./layout/edit";
import { normalizeLayout } from "./layout/model";
import { NoteEditor, NotesPage, Palette, Settings, UpdatePanel, Wizard, lazyCardIds, preloadCards, preloadEditViews, preloadUpdatePanel, preloadViews } from "./lazyViews";
import { installQuickKeys } from "./quick/quick";
import "./quick/quick.css";
import { clearDrop, offerDrop } from "./actions/dropstate";
import { mainHeight } from "./layout/drawerSize";
import { contentHeight } from "./theme/size";
import Boundary from "./ui/Boundary";
import "./widgetkit/clickFull";

export default function App() {
  const [s, setS] = createStore<Snapshot>(structuredClone(EMPTY_SNAPSHOT));
  const [view, setView] = createSignal<View>("hidden");
  /** náhled po najetí (island / dok): za kolik ms se sám rozbalí — z `view` (`drawer.hoverExpandMs`) */
  const [expandMs, setExpandMs] = createSignal(0);
  /** karty středního panelu z docku (`drawer.dockCards`, ověřené v Rustu) — z `view` */
  const [dockCards, setDockCards] = createSignal<string[]>([]);
  const [page, setPage] = createSignal<Page>("main");
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
  const notesNav = { onNew: (kind?: string | NotePreset) => startNote(typeof kind === "string" ? (kind ? { kind } : null) : (kind ?? null)), onOpen: editNote, onAll: allNotes };
  // ve středním panelu (L147) stránky poznámek / nastavení nejsou: karta přepne stránku a panel se rozbalí do celého šuplíku
  const midNav = { onNew: (kind?: string | NotePreset) => { notesNav.onNew(kind); void api.expandDrawer(); }, onOpen: (n: NoteView) => { notesNav.onOpen(n); void api.expandDrawer(); }, onAll: (personal?: boolean) => { notesNav.onAll(personal); void api.expandDrawer(); } };

  // ---------- paleta příkazů (Ctrl+K s fokusem, 🔍 klikem — jen klik si bere fokus) ----------
  const [palette, setPalette] = createSignal<{ took: boolean; q?: string } | null>(null);
  const openPalette = (click: boolean, q?: string) => {
    if (click) api.focusDrawer();
    setPalette({ took: click, q });
  };
  const closePalette = () => {
    if (palette()?.took) api.releaseFocus();
    setPalette(null);
  };
  const palNav = {
    onNote: (preset?: NotePreset) => startNote(preset ?? null),
    onOpenNote: editNote,
    onAllNotes: () => allNotes(),
    onSettings: (id?: string) => (id ? gotoSettingsPanel(id) : openSettings()),
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
    setDockCards(msg.dockCards ?? []);
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
      // bez plánu morfu: `closing` nechal na `[data-anim]` animace `fill: "forwards"` (opacity 0) — bez zrušení by po doběhu
      // naskočení vrátily obsah zpět na průhledný (animace „none“ / otevření bez morfu)
      preHide(content);
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
  // Nastavení = okno Ledge Studio; když nevznikne, stránka `settings` v šuplíku se stejným Studiem
  setStudioFallback(() => setPage("settings"));
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
      // okno Ledge Studio chce průvodce nebo úpravy přímo v šuplíku (Rust `studio_win`)
      on<string>("studio-ask", (x) => (x === "wizard" ? startWizard() : x === "edit" ? askEdit() : undefined)),
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
      } else if (!e.ctrlKey && !e.altKey && !isTextTarget(e.target as HTMLElement)) {
        // 1–9 = záložka v pořadí z `layout` (v úpravách jen rozpracovaná)
        const n = Number(e.key);
        if (Number.isInteger(n) && n >= 1 && n <= 9) {
          const pg = (draft() ?? normalizeLayout(s.layout)).pages[n - 1];
          if (pg && editing()) setEditPage(pg.id);
          else if (pg) void api.setMode(pg.id);
        } else if (!palette() && !editing() && page() === "main" && view() === "full") {
          // psaní v šuplíku s fokusem na těle = rovnou do palety (bez Ctrl+K)
          const ch = typeToPalette(e, e.target as HTMLElement);
          if (ch) {
            e.preventDefault();
            openPalette(false, ch);
          }
        }
      }
    };
    window.addEventListener("keydown", key);
    onCleanup(() => window.removeEventListener("keydown", key));
    // Alt+1…9 = N-tý zástupce, šipky mezi zástupci (`quick/`, vždy zapnuto od kola 33)
    onCleanup(installQuickKeys(() => ({ enabled: true, page: page(), editing: editing(), paletteOpen: !!palette() })));

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
    const h = contentHeight(content.getBoundingClientRect().height, content.offsetHeight) - OVERLAP;
    api.reportSize(v, smallWidth(v, dockMode() === true) ?? wantWidth(page()), h, v === "full" && h < 300 && h !== lowLogged ? lowDiag() : undefined);
    if (v === "full" && h < 300) lowLogged = h;
  }
  // diagnostika „uříznutého šuplíku“ (0.11.x): co přesně určilo nízkou výšku → řádek v ledge.log
  let lowLogged = -1;
  function lowDiag() {
    const c = document.querySelector<HTMLElement>(".content");
    const cs = c ? getComputedStyle(c) : null;
    const deck = document.querySelector<HTMLElement>(".content > .deck");
    const kids = c ? [...c.children].map((e) => `${(e as HTMLElement).className.toString().split(" ")[0] || e.tagName}:${Math.round(e.getBoundingClientRect().height)}`).join(",") : "";
    return `page=${page()} style=${c?.style.height || "-"} css=${cs?.height}/${cs?.maxHeight} var=${getComputedStyle(document.documentElement).getPropertyValue("--drawer-max-h") || "-"} scroll=${c?.scrollHeight} deck=${deck ? `${deck.clientHeight}/${deck.scrollHeight}` : "-"} win=${innerHeight} avail=${screen.availHeight} dpr=${devicePixelRatio} kids=${kids}`;
  }
  // hodiny v hlavičce: sekundový časovač existuje jen, dokud je šuplík vidět (zavřený = žádné buzení WebView)
  createEffect(() => {
    if (view() === "hidden") return;
    setNow(Date.now());
    const tick = window.setInterval(() => setNow(Date.now()), 1000);
    onCleanup(() => clearInterval(tick));
  });
  createEffect(onChange([view, page], () => queueMicrotask(report)));
  // skrytý déle než `DORMANT_AFTER_MS` → strom v `.content` se odmontuje (`dormant.ts`); jednorázový časovač při přechodu
  // do `hidden`, zrušený návratem (žádné periodické buzení). Návrat = `hiddenLong` false ještě v téže synchronní dávce jako `setView`.
  const [hiddenLong, setHiddenLong] = createSignal(false);
  createEffect(() => {
    if (view() !== "hidden") return setHiddenLong(false);
    const t = window.setTimeout(() => setHiddenLong(true), DORMANT_AFTER_MS);
    onCleanup(() => clearTimeout(t));
  });
  const dormant = () => isDormant({ view: view(), page: page(), editing: editing(), hiddenLong: hiddenLong() });
  // odmontovaný strom už loga nepotřebuje — uvolnit data URL (příští zobrazení je vyžádá znovu)
  createEffect(() => dormant() && (dropLogos(), dropCovers()));
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
  // chunky lazy karet, které rozložení opravdu obsahuje (ostatní se nenačtou vůbec); klíč = seznam id, takže se to spustí jen při změně skladby
  const cardKey = createMemo(() => lazyCardIds(normalizeLayout(s.layout).pages.flatMap((pg) => pg.cards.map((c) => c.id))).join(","), "", { equals: (a, b) => a === b });
  createEffect(() => preloadCards(cardKey().split(",").filter(Boolean)));
  // katalog a vzhled karty (úpravy) a panel „Nová verze“: chunk až při potřebě, ne spolu s ostatními v klidu
  createEffect(() => editing() && preloadEditViews());
  createEffect(() => (s.update.state === "dostupna" || s.update.whatsNew?.unread) && preloadUpdatePanel());
  // z poznámky zpět na šuplík (Zpět, uložení, Esc) → opener jako před `focusDrawer`
  createEffect(onChange(page, (pg, prev) => prev && typing(prev) && !typing(pg) && view() !== "hidden" && api.releaseFocus(), { defer: true }));

  return (
    <div class="shell" ref={panel}>
      <Backdrop open={() => view() === "full"} />
      <div ref={content} style={{ "padding-top": `${OVERLAP}px` }}>
        <Show when={view() === "peek" || isHoverView(view())} fallback={
          <Show when={view() === "mid"} fallback={
          <div class="content" style={{ height: page() === "main" && !dormant() ? mainHeight() : undefined }}>
            <Show when={!dormant()}>
            <Show when={page() !== "welcome"}>
              <Head s={s} now={now()} page={page()} onNote={() => startNote()} onSettings={() => (page() === "settings" ? setPage("main") : openSettings())} onBack={() => setPage(page() === "note" ? returnTo() : "main")} onUpdate={() => setUpdOpen(!updOpen())} onPalette={() => (palette() ? closePalette() : openPalette(true))} />
            </Show>
            <Show when={palette()}>{(pl) => <Palette s={s} took={pl().took} initial={pl().q} nav={palNav} onClose={() => setPalette(null)} />}</Show>
            <Show when={s.cfgError}>
              <div class="banner" data-anim><IconShield /> <span>{s.cfgError}</span></div>
            </Show>
            <Show when={s.configNotice}>{(n) => <ConfigNotice n={n()} />}</Show>
            <Show when={updOpen() && page() === "main" && (s.update.state === "dostupna" || s.update.whatsNew?.unread) ? s.update : null}>
              {(u) => <UpdatePanel u={u()} onClose={() => setUpdOpen(false)} />}
            </Show>
            <Boundary resetOn={page}>
            <Show when={page() === "main"}><MainPage s={s} now={now()} onNote={startNote} nav={notesNav} onSettings={() => openSettings()} /></Show>
            <Show when={page() === "settings"}><div data-anim><Settings s={s} windowed={false} /></div></Show>
            <Show when={page() === "note"}><NoteEditor s={s} view={view()} onLeave={() => setPage(returnTo())} /></Show>
            <Show when={page() === "welcome"}><Wizard onExit={() => setPage("main")} /></Show>
            <Show when={page() === "notes"}><NotesPage s={s} now={now()} onOpen={editNote} onNew={notesNav.onNew} /></Show>
            </Boundary>
            </Show>
          </div>
          }>
            <Mid s={s} now={now()} cards={dockCards()} nav={midNav} onNote={(pr) => { startNote(pr ?? null); void api.expandDrawer(); }} onSettings={() => { openSettings(); void api.expandDrawer(); }} />
          </Show>
        }>
          <SmallView view={view()} s={s} now={now()} expandMs={expandMs()} />
        </Show>
      </div>
      <UndoBar />
      <Toast />
    </div>
  );
}
