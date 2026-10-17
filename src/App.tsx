// Skořápka šuplíku: drží snapshot (`api.on("state")` → store → stránky), stránky a navigaci, morf
// proužek ⇄ šuplík (`prepare` / `play` / `closing` + `motion.ts`, měřeno v `drawer.anim`), hlášení
// výšky obsahu (`report`) a fokus (jen výslovný klik → `focusDrawer`, odchod → `releaseFocus`).
// Vzhled (motiv, animace) aplikuje `theme/apply.ts` (`initAppearance` v onMount).
// Karty jsou v `cards/`, hlavní stránka v `MainPage.tsx`, poznámky v `notes/`.
// anglické slovníky (registerDict) dřív než cokoli, co volá t()
import "./i18n/load";
import { createSignal, Show, onMount, onCleanup, createEffect, on as onChange } from "solid-js";
import { createStore, reconcile } from "solid-js/store";
import { api, on, type Snapshot, type View, type ViewMsg, type NoteView } from "./api";
import { IconShield } from "./icons";
import NoteEditor from "./notes/NoteEditor";
import NotesPage from "./notes/NotesPage";
import { returnTo, setEditPath, setNotePreset, setPageProject, setReturnTo, type NotePreset } from "./notes/store";
import "./notes/notes.css";
import { OVERLAP, spring, collapsedClip, FULL_CLIP, dur, measure, preHide, revealContent, concealContent } from "./motion";
import { initAppearance } from "./theme/apply";
import Settings from "./Settings";
import { holdWide, wantWidth } from "./settings/widen";
import MainPage from "./MainPage";
import Head from "./cards/Head";
import UpdatePanel from "./cards/UpdatePanel";
import Peek from "./cards/Peek";
import { asPage, typing, type Page } from "./nav";
import { EMPTY_SNAPSHOT } from "./snapshot";
import { Toast } from "./ui/toast";
import Palette from "./palette/Palette";
import { gotoSettingsPanel } from "./palette/goto";
import Wizard from "./onboarding/Wizard";
import { checkFirstRun, wizardOpen } from "./onboarding/state";

export default function App() {
  const [s, setS] = createStore<Snapshot>(structuredClone(EMPTY_SNAPSHOT));
  const [view, setView] = createSignal<View>("hidden");
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
  function editNote(n: NoteView) {
    setReturnTo(page() === "notes" ? "notes" : "main");
    setEditPath(n.path);
    setPage("note");
    api.focusDrawer();
  }
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
  };

  // ---------- morf: proužek ⇄ šuplík ----------
  let pendingFrom: ViewMsg | null = null;

  function prepare(msg: ViewMsg) {
    setView(msg.view);
    if (msg.view === "hidden") {
      // skrytí bez morfu (bez `closing`) — příště začít na hlavní stránce
      setPage("main");
      return;
    }
    panel.getAnimations().forEach((a) => a.cancel());
    if (msg.morph) {
      // složit do výchozího tvaru (proužek / předchozí pohled) — okno je zatím schované
      panel.style.clipPath = collapsedClip(msg.fromW, msg.fromH);
      preHide(content);
      void panel.offsetHeight; // vynutit styl dřív, než Rust okno ukáže
      pendingFrom = msg;
      api.animAck("prepared");
    } else {
      panel.style.clipPath = "";
      revealContent(content, 40);
    }
  }

  function play() {
    const msg = pendingFrom;
    pendingFrom = null;
    if (!msg) return;
    const sp = spring(240, 25);
    panel
      .animate([{ clipPath: collapsedClip(msg.fromW, msg.fromH) }, { clipPath: FULL_CLIP }], {
        duration: sp.duration,
        easing: sp.easing,
      })
      .finished.then(() => (panel.style.clipPath = ""), () => {});
    // celá plocha se zároveň „nadechne“ z proužku
    const breath = spring(300, 22);
    content.animate([{ transform: "scale(0.97)", transformOrigin: "50% 0" }, { transform: "scale(1)", transformOrigin: "50% 0" }], {
      duration: breath.duration,
      easing: breath.easing,
    });
    revealContent(content, 60);
    measure("open", sp.duration + 120);
  }

  async function closing(msg: ViewMsg) {
    panel.getAnimations().forEach((a) => a.cancel());
    measure("close", 250);
    const hide = concealContent(content, 110);
    const shrink = panel.animate([{ clipPath: FULL_CLIP }, { clipPath: collapsedClip(msg.toW, msg.toH) }], {
      duration: dur(240),
      delay: dur(40),
      easing: "cubic-bezier(.55,0,.75,.05)",
      fill: "forwards",
    });
    await Promise.all([hide, shrink.finished.catch(() => {})]);
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
      initAppearance(),
    );

    const tick = window.setInterval(() => view() !== "hidden" && setNow(Date.now()), 1000);
    onCleanup(() => clearInterval(tick));

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
      if (e.key === "Escape") {
        if (page() === "settings" || page() === "notes") setPage("main");
        else api.close();
      } else if (!e.ctrlKey && !e.altKey && !["INPUT", "TEXTAREA", "SELECT"].includes((e.target as HTMLElement).tagName)) {
        if (e.key === "1" && s.personalEnabled) api.setMode("prace");
        if (e.key === "2" && s.personalEnabled) api.setMode("osobni");
      }
    };
    window.addEventListener("keydown", key);
    onCleanup(() => window.removeEventListener("keydown", key));

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
    api.reportSize(v, v === "peek" ? 680 : wantWidth(page()), Math.ceil(content.getBoundingClientRect().height) - OVERLAP);
  }
  createEffect(onChange([view, page, holdWide], () => queueMicrotask(report)));
  createEffect(onChange(view, (v) => v === "hidden" && setPalette(null), { defer: true }));
  // průvodce prvním spuštěním: config se čte při prvním otevření; dokud běží, šuplík otevírá na něm
  createEffect(() => view() === "full" && void checkFirstRun());
  createEffect(() => view() === "full" && wizardOpen() && setPage("welcome"));
  // z poznámky zpět na šuplík (Zpět, uložení, Esc) → opener jako před `focusDrawer`
  createEffect(onChange(page, (pg, prev) => prev && typing(prev) && !typing(pg) && view() !== "hidden" && api.releaseFocus(), { defer: true }));

  return (
    <div class="shell" ref={panel}>
      <div ref={content} style={{ "padding-top": `${OVERLAP}px` }}>
        <Show when={view() === "peek"} fallback={
          <div class="content">
            <Show when={page() !== "welcome"}>
              <Head s={s} now={now()} page={page()} onNote={() => startNote()} onSettings={() => setPage(page() === "settings" ? "main" : "settings")} onBack={() => setPage(page() === "note" ? returnTo() : "main")} onUpdate={() => setUpdOpen(!updOpen())} onPalette={() => (palette() ? closePalette() : openPalette(true))} />
            </Show>
            <Show when={palette()}>{(pl) => <Palette s={s} took={pl().took} nav={palNav} onClose={() => setPalette(null)} />}</Show>
            <Show when={s.cfgError}>
              <div class="banner" data-anim><IconShield /> <span>{s.cfgError}</span></div>
            </Show>
            <Show when={updOpen() && page() === "main" && s.update.state === "dostupna" ? s.update : null}>
              {(u) => <UpdatePanel u={u()} onClose={() => setUpdOpen(false)} />}
            </Show>
            <Show when={page() === "main"}><MainPage s={s} now={now()} onNote={startNote} nav={notesNav} onSettings={() => setPage("settings")} /></Show>
            <Show when={page() === "settings"}><div data-anim><Settings update={s.update} s={s} /></div></Show>
            <Show when={page() === "note"}><NoteEditor s={s} view={view()} onLeave={() => setPage(returnTo())} /></Show>
            <Show when={page() === "welcome"}><Wizard onExit={() => setPage("main")} /></Show>
            <Show when={page() === "notes"}><NotesPage s={s} now={now()} onOpen={editNote} onNew={notesNav.onNew} /></Show>
          </div>
        }>
          <Peek s={s} />
        </Show>
      </div>
      <Toast />
    </div>
  );
}
