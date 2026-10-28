// Ledge Studio (kolo 24) — nové Nastavení. JEDNA skořápka pro samostatné okno (`settings.html`,
// `StudioWindow.tsx`, Rust `studio.rs`) i pro záložní stránku v šuplíku (když okno nejde vytvořit; kompaktní
// rozvržení přes container queries). Titulek: logo · drobečky · hledání (Ctrl+F, `smartSearch`) · stav
// zápisu „Uloženo ✓ · Vrátit“ (Ctrl+Z) · „Změny (n)“ = časová osa relace · Průvodce · scéna · okno.
// Vlevo páteř 8 částí (`parts.ts`) se stanicemi a počtem „upraveno proti výchozímu“, uprostřed domov
// (`Home.tsx`) nebo část (`PartView.tsx`), vpravo scéna (`Scene.tsx`). Platí hned (`autosave.ts`) — žádné
// Uložit/Zahodit. Skoky odjinud: hash URL, událost `studio-goto`, `palette/goto.ts` `settingsTarget`.
import { For, Show, createEffect, createMemo, createSignal, on, onCleanup, onMount } from "solid-js";
import { api, inTauri, on as onEvent, type Snapshot } from "../api";
import { t } from "../i18n";
import { lbl } from "../theme/label";
import { SettingsCtx, cfg, fieldError, isLoaded, loadBase, makeCtx, setHit } from "../settings/ctx";
import { searchItems } from "../settings/catalog";
import { smartSearch, type SmartItem } from "../settings/searchModel";
import { highlight } from "../settings/model";
import { FIELDS, fieldLabel } from "../settings/labels";
import { PANEL_ALIASES } from "../settings/registry";
import { setSettingsTarget, settingsTarget } from "../palette/goto";
import { applyAppearance, savedAppearance } from "../theme/apply";
import { normalizeAppearance } from "../theme/appearance";
import { withThemeDraft } from "../settings/panels/apUi";
import { GUIDE_KEY, blockOf, guideStep, guideSteps, modifiedByPart, parseGuide, partById, partOfBlock, resolveStudioTarget, visibleParts, withDone, type PartId } from "./parts";
import { diffPaths, entryText, nearestLabel } from "./history";
import { namedValue } from "./named";
import { adoptExternal, closeStudio, conflict, history, resolveConflict, saveState, undo, undoAll, undoOne, useAutosave } from "./autosave";
import Home from "./Home";
import PartView from "./PartView";
import Scene, { SCENE_PARTS } from "./Scene";
import { IconCheck, IconClose, IconFind, IconGuide, IconHistory, IconHome, IconMax, IconMin, IconRestore, IconScene, PartIcon } from "./icons";
import "../settings.css";
import "./studio.css";

const fp = (id: string) => FIELDS[id]?.panel ?? null;
const reduced = () => typeof matchMedia === "function" && matchMedia("(prefers-reduced-motion: reduce)").matches;
const store = {
  get: (k: string) => { try { return localStorage.getItem(k); } catch { return null; } },
  set: (k: string, v: string) => { try { localStorage.setItem(k, v); } catch { /* paměť je jen pohodlí */ } },
};
const SCENE_KEY = "ledge.studio.scene";

/** Popisek změny pro časovou osu: nejbližší popsaná volba, hodnoty lidsky (vzhled proužku, po najetí). */
const labelOf = (path: string) => {
  const id = nearestLabel(path, (x) => !!FIELDS[x]);
  return FIELDS[id] ? fieldLabel(id) : path;
};
const named = (path: string, v: string) => {
  const x = namedValue(path, v);
  return x ? t(x[0], x[1]) : null;
};
const words = () => ({
  on: t("settings.studio.v.on", "zapnuto"), off: t("settings.studio.v.off", "vypnuto"), empty: t("settings.studio.v.empty", "nic"),
  items: (n: number) => t("settings.studio.v.items", "{n} položek", { n }), edited: t("settings.studio.v.edited", "upraveno"),
  more: (n: number) => t("settings.studio.v.more", "a {n} další", { n }),
});
const clock = (ms: number) => new Date(ms).toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" });

export default function Studio(p: { s: Snapshot; windowed: boolean; initial?: string | null }) {
  const ctx = makeCtx(p.s, async () => {});
  const [view, setView] = createSignal<PartId | "home">("home");
  const [visited, setVisited] = createSignal<PartId[]>([]);
  const [guide, setGuide] = createSignal(false);
  const [done, setDone] = createSignal<PartId[]>(parseGuide(store.get(GUIDE_KEY)));
  const [showAll, setShowAll] = createSignal(false);
  const [sceneOn, setSceneOn] = createSignal(store.get(SCENE_KEY) !== "0");
  // mimo Tauri (jen ukázka `settings.html?demo=1&q=…` pro snímky) lze hledání předvyplnit
  const [query, setQuery] = createSignal(!inTauri ? new URLSearchParams(location.search).get("q") ?? "" : "");
  const [sel, setSel] = createSignal(0);
  const [changesOpen, setChangesOpen] = createSignal(false);
  const [maxed, setMaxed] = createSignal(false);
  const [defaults, setDefaults] = createSignal<any>(null);
  /** úzké okno / šuplík: scéna jde pod hlavičku části, páteř jen s ikonami (CSS) */
  const [narrow, setNarrow] = createSignal(false);
  let root!: HTMLDivElement;
  let main!: HTMLDivElement;
  let searchEl!: HTMLInputElement;

  useAutosave(() => root);

  // rozpracovaný vzhled hned na celé Studio (tokeny uživatelova motivu); po odchodu (šuplík) zpět uložený
  createEffect(() => isLoaded() && applyAppearance(withThemeDraft(normalizeAppearance(cfg.appearance))));
  onCleanup(() => applyAppearance(savedAppearance()));

  /** viditelné části a kroky průvodce (predikáty `visible` v `parts.ts`) */
  const parts = createMemo(() => visibleParts(cfg));
  const steps = createMemo(() => guideSteps(cfg));
  const modified = createMemo(() => modifiedByPart(defaults() && isLoaded() ? diffPaths(defaults(), cfg).map((c) => c.path) : [], fp));

  createEffect(on(view, (v) => {
    if (v !== "home" && !visited().includes(v)) setVisited((l) => [...l, v]);
    setShowAll(false);
    setChangesOpen(false);
    if (main) main.scrollTop = 0;
  }));

  /** Skok na volbu: zvýraznit (rozbalí Doladit), posunout do středu. */
  function reveal(anchor: string) {
    setHit(anchor);
    window.setTimeout(() => setHit((h) => (h === anchor ? null : h)), 1800);
    requestAnimationFrame(() => requestAnimationFrame(() => {
      const shown = main?.querySelector<HTMLElement>(".sp-wrap:not([hidden])") ?? main;
      shown?.querySelector<HTMLElement>(`[id="f-${anchor}"]`)?.scrollIntoView({ block: "center", behavior: reduced() ? "auto" : "smooth" });
    }));
  }
  function go(target: string | null | undefined) {
    const to = resolveStudioTarget(target, PANEL_ALIASES, fp);
    if (!to) return;
    setGuide(to.guide);
    setView(to.part);
    if (to.anchor) reveal(to.anchor);
  }

  // chyba ověření: pole v Doladit rozbalit a ukázat (když je v otevřené části)
  createEffect(on(() => fieldError()?.field, (f) => {
    if (!f) return;
    const part = partOfBlock(blockOf(f, fp));
    if (part && part === view()) reveal(f);
  }, { defer: true }));

  onMount(() => {
    api.getConfig().then((c) => c && loadBase(c), () => {});
    api.configDefaults().then((d) => d && setDefaults(d), () => {});
    go(p.initial ?? (location.hash || null));
    const offs = [onEvent("config-changed", () => void adoptExternal())];
    if (p.windowed) offs.push(onEvent<string>("studio-goto", (x) => go(x)));
    onCleanup(() => offs.forEach((o) => void o.then((f) => f())));
    const keys = (e: KeyboardEvent) => {
      const k = e.key.toLowerCase();
      const inText = /^(INPUT|TEXTAREA|SELECT)$/.test((e.target as HTMLElement)?.tagName ?? "");
      if (e.ctrlKey && !e.altKey && k === "f") {
        e.preventDefault();
        searchEl?.focus();
        searchEl?.select();
      } else if (e.ctrlKey && !e.altKey && k === "z" && !inText) {
        e.preventDefault();
        undo();
      } else if (e.key === "Escape" && (changesOpen() || query())) {
        e.stopPropagation();
        setChangesOpen(false);
        setQuery("");
      }
    };
    const ro = new ResizeObserver(() => setNarrow(root.clientWidth < 1100));
    ro.observe(root);
    onCleanup(() => ro.disconnect());
    root.addEventListener("keydown", keys);
    onCleanup(() => root.removeEventListener("keydown", keys));
  });
  // skok z šuplíku, když Studio běží jako záložní stránka (`palette/goto.ts`)
  createEffect(() => {
    const id = settingsTarget();
    if (!id) return;
    setSettingsTarget(null);
    go(id);
  });

  // ---------- hledání ----------
  const items = createMemo<SmartItem[]>(() => searchItems());
  const results = createMemo(() => smartSearch(items(), query()).slice(0, 12));
  createEffect(on(query, () => setSel(0)));
  const partOfItem = (it: SmartItem): PartId | null => {
    const to = resolveStudioTarget(it.anchor ? `f-${it.anchor}` : it.panel, PANEL_ALIASES, fp);
    return to && to.part !== "home" ? to.part : null;
  };
  function pick(it: SmartItem | undefined) {
    if (!it) return;
    setQuery("");
    setGuide(false);
    const part = partOfItem(it);
    if (part) setView(part);
    if (it.anchor) reveal(it.anchor);
  }
  const searchKey = (e: KeyboardEvent) => {
    const n = results().length;
    if ((e.key === "ArrowDown" || e.key === "ArrowUp") && n) {
      e.preventDefault();
      setSel((sel() + (e.key === "ArrowDown" ? 1 : n - 1)) % n);
    } else if (e.key === "Enter") {
      e.preventDefault();
      pick(results()[sel()]);
    }
  };

  // ---------- průvodce ----------
  const startGuide = () => {
    setGuide(true);
    setView((steps().find((x) => !done().includes(x as PartId)) ?? steps()[0] ?? "home") as PartId);
  };
  const markDone = (id: PartId) => {
    const next = withDone(done(), id);
    setDone(next);
    store.set(GUIDE_KEY, JSON.stringify(next));
  };
  const nextPart = (finish: boolean) => {
    const cur = view();
    if (cur === "home") return;
    if (finish) markDone(cur);
    const n = guideStep(steps(), cur, 1);
    if (n) setView(n as PartId);
    else (setGuide(false), setView("home"));
  };

  const prevPart = () => {
    const n = guideStep(steps(), view() as PartId, -1);
    if (n) setView(n as PartId);
  };

  // ---------- okno ----------
  const win = (op: string) => void api.studioWin(op).then((m) => m !== undefined && setMaxed(m), () => {});
  const drag = (e: MouseEvent) => {
    if (!p.windowed || e.button !== 0 || (e.target as HTMLElement).closest("button, input, a, [data-nodrag]")) return;
    if (e.detail === 2) win("toggle_max");
    else win("drag");
  };
  const toggleScene = () => {
    setSceneOn(!sceneOn());
    store.set(SCENE_KEY, sceneOn() ? "1" : "0");
  };

  const cur = () => (view() === "home" ? null : partById(view()));
  const status = () => {
    if (conflict()) return "conflict";
    if (fieldError()) return "invalid";
    return saveState();
  };
  const hasScene = () => view() !== "home" && SCENE_PARTS.includes(view() as PartId);

  return (
    <SettingsCtx.Provider value={ctx}>
      <div class="studio st" ref={root} classList={{ windowed: p.windowed, narrow: narrow(), guiding: guide() && view() !== "home", "no-scene": narrow() || !sceneOn() || !hasScene() }}>
        <header class="stu-title" onMouseDown={drag}>
          <button class="stu-logo" onClick={() => (setGuide(false), setView("home"))} title={t("settings.studio.home", "Tvůj Ledge")}>
            <i class="stu-mark" aria-hidden="true" />
            <b>Ledge Studio</b>
          </button>
          <nav class="stu-crumbs" aria-label={t("settings.studio.crumbs", "Kde jsi")}>
            <button onClick={() => (setGuide(false), setView("home"))}>{t("settings.studio.home", "Tvůj Ledge")}</button>
            <Show when={cur()}>{(c) => <><span aria-hidden="true">›</span><b>{lbl(c().name)}</b></>}</Show>
          </nav>
          <div class="stu-find" data-nogate data-nodrag>
            <IconFind />
            <input ref={searchEl} type="search" data-nogate value={query()} spellcheck={false}
              placeholder={t("settings.studio.search", "Hledat volbu…  Ctrl+F")} aria-label={t("settings.search.aria", "Hledat v nastavení")}
              onInput={(e) => setQuery(e.currentTarget.value)} onKeyDown={searchKey} />
            <Show when={query().trim()}>
              <div class="stu-results" role="listbox" aria-label={t("settings.search.results", "Výsledky hledání")}>
                <Show when={results().length} fallback={<p class="stu-none">{t("settings.search.none", "Nic nenalezeno. Zkus jiné slovo — hledá se i v popisech.")}</p>}>
                  <For each={results()}>
                    {(r, i) => {
                      const part = () => partById(partOfItem(r));
                      return (
                        <button role="option" aria-selected={sel() === i()} classList={{ on: sel() === i() }} onMouseEnter={() => setSel(i())} onMouseDown={(e) => e.preventDefault()} onClick={() => pick(r)}>
                          <span class="stu-r-ico"><PartIcon icon={part()?.icon ?? "app"} /></span>
                          <span class="stu-r-txt">
                            <b><For each={highlight(r.label, query())}>{(seg) => (seg.hit ? <mark>{seg.t}</mark> : seg.t)}</For></b>
                            <small>
                              {part() ? lbl(part()!.name) : ""}
                              <Show when={r.adv}> · {t("settings.studio.tune", "Doladit")}</Show>
                              <Show when={r.keyOnly || (r.key && query().includes("."))}> · <code>{r.key}</code></Show>
                            </small>
                          </span>
                        </button>
                      );
                    }}
                  </For>
                </Show>
              </div>
            </Show>
          </div>
          <div class="stu-state" role="status" data-state={status()}>
            <Show when={status() === "saving"}><span>{t("settings.studio.saving", "Ukládám…")}</span></Show>
            <Show when={status() === "saved" || status() === "idle"}>
              <span class="stu-ok"><IconCheck />{t("settings.studio.saved", "Uloženo")}</span>
            </Show>
            <Show when={status() === "invalid"}>
              <span class="stu-bad">{t("settings.studio.invalid", "Neuloženo: {msg}", { msg: fieldError()?.message ?? "" })}</span>
              <Show when={fieldError()?.field}><button class="stu-link" onClick={() => go(`f-${fieldError()!.field}`)}>{t("settings.bar.show", "Ukázat")}</button></Show>
            </Show>
            <Show when={status() === "conflict"}>
              <span class="stu-bad">{t("settings.studio.conflict", "Nastavení se mezitím změnilo jinde")}</span>
              <button class="stu-link" onClick={() => void resolveConflict(false)}>{t("settings.studio.conflict.reload", "Vzít nové")}</button>
              <button class="stu-link" onClick={() => void resolveConflict(true)}>{t("settings.studio.conflict.mine", "Nechat moje")}</button>
            </Show>
          </div>
          <div class="stu-tools" data-nodrag>
            <button class="stu-tool" classList={{ on: changesOpen() }} disabled={!history().entries.length} aria-expanded={changesOpen()} onClick={() => setChangesOpen(!changesOpen())}>
              <IconHistory />{t("settings.studio.changes", "Historie ({n})", { n: history().entries.length })}
            </button>
            <button class="stu-tool" classList={{ on: guide() }} onClick={() => (guide() ? setGuide(false) : startGuide())}><IconGuide />{t("settings.studio.guide", "Průvodce")}</button>
            <Show when={hasScene()}>
              <button class="stu-tool icon" classList={{ on: sceneOn() }} onClick={toggleScene} title={t("settings.studio.scene.toggle", "Ukázka vpravo")} aria-pressed={sceneOn()}><IconScene /></button>
            </Show>
          </div>
          <Show when={p.windowed}>
            <div class="stu-win" data-nodrag>
              <button onClick={() => win("minimize")} aria-label={t("settings.studio.win.min", "Minimalizovat")}><IconMin /></button>
              <button onClick={() => win("toggle_max")} aria-label={t("settings.studio.win.max", "Maximalizovat")}>{maxed() ? <IconRestore /> : <IconMax />}</button>
              <button class="close" onClick={() => void closeStudio()} aria-label={t("settings.studio.win.close", "Zavřít")}><IconClose /></button>
            </div>
          </Show>
          <Show when={changesOpen()}>
            <div class="stu-changes" data-nodrag role="dialog" aria-label={t("settings.studio.changes.title", "Změny v této relaci")}>
              <header><b>{t("settings.studio.changes.title", "Změny v této relaci")}</b><small>{t("settings.studio.changes.hint", "Všechno už platí. Vrátit jde každou zvlášť.")}</small></header>
              <ol>
                <For each={[...history().entries].reverse()}>
                  {(e) => (
                    <li>
                      <time>{clock(e.at)}</time>
                      <span>{entryText(e, labelOf, words(), named)}</span>
                      <button class="stu-link" onClick={() => undoOne(e.id)}>{t("settings.studio.undo", "Vrátit")}</button>
                    </li>
                  )}
                </For>
              </ol>
              <footer><button class="btn" onClick={() => (undoAll(), setChangesOpen(false))}>{t("settings.studio.undoAll", "Vrátit vše do stavu při otevření")}</button></footer>
            </div>
          </Show>
        </header>

        <Show when={guide()}>
          <ol class="stu-guide" aria-label={t("settings.studio.guide.progress", "Postup průvodce")}>
            <For each={parts()}>
              {(pt) => (
                <li classList={{ on: view() === pt.id, done: done().includes(pt.id) }}>
                  <button onClick={() => setView(pt.id)}><i>{done().includes(pt.id) ? "✓" : pt.n}</i><span>{lbl(pt.name)}</span></button>
                </li>
              )}
            </For>
          </ol>
        </Show>

        <nav class="stu-spine" aria-label={t("settings.studio.parts", "Části")}>
          <button class="stu-station home" classList={{ on: view() === "home" }} onClick={() => (setGuide(false), setView("home"))}>
            <span class="stu-dot"><IconHome /></span><span class="stu-name">{t("settings.studio.home", "Tvůj Ledge")}</span>
          </button>
          <For each={parts()}>
            {(pt) => (
              <button class="stu-station" classList={{ on: view() === pt.id, bad: partOfBlock(blockOf(fieldError()?.field, fp)) === pt.id }} aria-current={view() === pt.id ? "page" : undefined}
                onClick={() => setView(pt.id)} title={lbl(pt.name)}>
                <span class="stu-dot"><PartIcon icon={pt.icon} /></span>
                <span class="stu-name"><small>{pt.n}</small>{lbl(pt.name)}</span>
                <Show when={modified()[pt.id]}><em class="stu-count" title={t("settings.studio.modified.hint", "Volby, které se liší od výchozích")}>{modified()[pt.id]}</em></Show>
              </button>
            )}
          </For>
        </nav>

        <main class="stu-main" ref={main}>
          <Show when={isLoaded()} fallback={<div class="stu-loading" aria-busy="true"><i /><i /><i /></div>}>
            <Show when={view() === "home"}>
              <Home s={p.s} go={go} modified={modified()} onGuide={startGuide} guideDone={done().length} />
            </Show>
            <For each={visited()}>
              {(id) => (
                <div class="sp-wrap" hidden={view() !== id}>
                  <PartView part={id} s={p.s} guide={guide()} showAll={showAll()} onShowAll={() => setShowAll(true)} modified={modified()[id]}
                    scene={<Show when={narrow() && sceneOn() && SCENE_PARTS.includes(id)}><div class="stu-scene-inline"><Scene part={id} onHide={toggleScene} /></div></Show>} />
                </div>
              )}
            </For>
          </Show>
        </main>

        <Show when={guide() && view() !== "home"}>
          <footer class="stu-guide-foot">
            <button class="btn" disabled={!guideStep(steps(), view() as PartId, -1)} onClick={prevPart}>{t("settings.studio.guide.back", "← Zpět")}</button>
            <span class="grow">{t("settings.studio.guide.pos", "Krok {i} z {n}", { i: Math.max(1, steps().indexOf(view() as PartId) + 1), n: steps().length })}</span>
            <button class="btn" onClick={() => nextPart(false)}>{t("settings.studio.guide.skip", "Přeskočit")}</button>
            <Show when={guideStep(steps(), view() as PartId, 1)} fallback={<button class="btn pri" onClick={() => nextPart(true)}>{t("settings.studio.guide.finish", "Hotovo")}</button>}>
              <button class="btn pri" onClick={() => nextPart(true)}>{t("settings.studio.guide.next", "Další část →")}</button>
            </Show>
          </footer>
        </Show>
        <Show when={!narrow() && hasScene() && sceneOn() && isLoaded()}>
          <div class="stu-scene"><Scene part={view() as PartId} onHide={toggleScene} /></div>
        </Show>
        <Show when={!inTauri && p.windowed}><i class="stu-demo">{t("settings.studio.demo", "Ukázka mimo Ledge — nic se neukládá")}</i></Show>
      </div>
    </SettingsCtx.Provider>
  );
}
