// Nastavení = skořápka (kolo 21): vlevo hledání, přepínač Základní / Pokročilé a málo hlavních sekcí podle
// úkolů uživatele (`settings/structure.ts`: Doporučené · Vzhled · Šuplík a karty · Claude · Projekty a akce ·
// Poznámky a lístky · Na ploše · Aplikace; sekce s víc stránkami je rozbalí pod sebou). Uprostřed stránka
// s vlastním posuvem, u Vzhledu dole náhled toho, co vyjede (`settings/PreviewDock.tsx`), a dole lišta
// Uložit / Zahodit / Vrátit. **Navštívené stránky zůstávají naživu** (přepnutí je jen skryje — žádné nové
// načítání, stav i posuv zůstanou); výška je pevná (`settings.css` `.st`), takže přepnutí nerozhodí okno.
// Rozpracovaný config drží `settings/ctx.ts` (přežije zavření stránky), ověřuje ho Rust (`check_config`)
// 250 ms po změně a chyba se ukáže u pole. Ukládá se ručně (Ctrl+S): config sdílí Claude i druhé PC.
import { createEffect, createMemo, createSignal, For, on, onCleanup, onMount, Show } from "solid-js";
import { Dynamic } from "solid-js/web";
import { api, on as onEvent, type FieldError, type Snapshot, type UpdateView } from "./api";
import { t } from "./i18n";
import { say, sayError } from "./ui/toast";
import { PANELS, panelLabel, searchItems } from "./settings/catalog";
import {
  SettingsCtx, active, adoptBase, base, dirty, discard, fieldError, isLoaded, loadBase, makeCtx, rev, runAfterSave, runUndo,
  setActive, setCfg, setDirty, setFieldError, setHit, snapshotDraft, undoMsg,
} from "./settings/ctx";
import { IconSearch, PanelIcon } from "./settings/icons";
import { canon, cleanDraft, highlight, panelForField, resolveTarget } from "./settings/model";
import { smartSearch, type SmartItem } from "./settings/searchModel";
import { PANEL_ALIASES } from "./settings/registry";
import { SECTIONS, pageForSection, sectionOf, withVisited } from "./settings/structure";
import { mode, setMode } from "./settings/mode";
import PreviewDock from "./settings/PreviewDock";
import { narrowOut, widenIn } from "./settings/widen";
import { FIELDS, sectionLabel } from "./settings/labels";
import { setSettingsTarget, settingsTarget } from "./palette/goto";
import { external } from "./mcp/merge";
import "./settings.css";

const reduced = () => window.matchMedia("(prefers-reduced-motion: reduce)").matches;

/** Naposledy otevřená stránka v každé sekci (klik na sekci se k ní vrátí) — přežije zavření stránky. */
const [lastInSection, setLastInSection] = createSignal<Record<string, string>>({});

/** Skok na řádek: posun do středu, krátké zvýraznění. Zvýraznění (`hit`) jde HNED — stránka s kroky
 *  (Vzhled) podle něj přepne krok a rozbalí „Pokročilé“; řádek se hledá až po dvou snímcích, kdy už existuje. */
function reveal(anchor: string | null, main: HTMLElement) {
  if (anchor) {
    setHit(anchor);
    window.setTimeout(() => setHit((h) => (h === anchor ? null : h)), 1800);
  }
  requestAnimationFrame(() =>
    requestAnimationFrame(() => {
      const shown = main.querySelector<HTMLElement>(".st-panel:not([hidden])") ?? main;
      const el = (anchor && shown.querySelector<HTMLElement>(`[id="f-${anchor}"]`)) || shown.querySelector<HTMLElement>(".st-err");
      el?.scrollIntoView({ block: "center", behavior: reduced() ? "auto" : "smooth" });
    }),
  );
}

export default function Settings(p: { update: UpdateView; s: Snapshot }) {
  const [saving, setSaving] = createSignal(false);
  const [conflict, setConflict] = createSignal(false);
  const [query, setQuery] = createSignal("");
  const [sel, setSel] = createSignal(0);
  /** stránky, které už byly otevřené — drží se naživu (skryté), aby přepnutí nic nezničilo */
  const [visited, setVisited] = createSignal<string[]>([]);
  let main!: HTMLDivElement;
  let searchEl!: HTMLInputElement;
  let nav!: HTMLDivElement;

  const ctx = makeCtx(p.s, () => save());
  const baseCanon = createMemo(() => (base() ? canon(JSON.parse(base())) : ""));
  const panelOf = (id: string) => PANELS.find((x) => x.id === id);
  const page = () => panelOf(active()) ?? PANELS[0];
  const section = () => sectionOf(page().id) ?? SECTIONS[0];

  createEffect(on(active, (id) => {
    setVisited((v) => withVisited(v, id));
    const sec = sectionOf(id);
    if (sec) setLastInSection((m) => (m[sec.id] === id ? m : { ...m, [sec.id]: id }));
  }));

  onMount(() => {
    // Nastavení se otevírá jen výslovným klikem → klávesnice pro pole (odchod vrací `releaseFocus`)
    api.focusDrawer();
    widenIn();
    const keys = (e: KeyboardEvent) => {
      if (e.ctrlKey && !e.altKey && e.key.toLowerCase() === "f") {
        e.preventDefault();
        searchEl?.focus();
        searchEl?.select();
      } else if (e.ctrlKey && !e.altKey && e.key.toLowerCase() === "s") {
        e.preventDefault();
        if (dirty()) save();
      }
    };
    window.addEventListener("keydown", keys);
    onCleanup(() => window.removeEventListener("keydown", keys));
    // rozpracované změny z minula zůstanou; bez nich vzít čerstvý stav (mohl ho změnit Claude)
    // (rozpracované + mezitím změna zvenku → stejná politika jako za běhu, `adoptExternal`)
    api.getConfig().then((c) => {
      if (!c) return;
      if (!isLoaded() || !dirty()) loadBase(c);
      else void adoptExternal();
    }, sayError);
    // config změněný mimo Nastavení (Claude přes MCP, přepínač MCP): načíst / sloučit / ukázat konflikt
    const off = onEvent("config-changed", () => void adoptExternal());
    onCleanup(() => void off.then((f) => f()));
  });

  async function adoptExternal() {
    if (!isLoaded() || saving()) return;
    const fresh = await api.getConfig().catch(() => null);
    if (!fresh) return;
    const r = external(JSON.parse(base()), snapshotDraft(), fresh, canon);
    if (r.action === "load") loadBase(fresh);
    else if (r.action === "merge") adoptBase(fresh, r.draft);
    else if (r.action === "conflict") setConflict(true);
  }
  onCleanup(() => narrowOut());

  // ověření v Rustu + „neuloženo“ 250 ms po poslední změně
  let timer: number | undefined;
  let checkSeq = 0;
  createEffect(on(rev, () => {
    clearTimeout(timer);
    timer = window.setTimeout(check, 250);
  }));
  onCleanup(() => clearTimeout(timer));
  async function check() {
    if (!isLoaded()) return;
    const draft = snapshotDraft();
    setDirty(canon(draft) !== baseCanon());
    const seq = ++checkSeq;
    try {
      const e = await api.checkConfig(cleanDraft(draft));
      if (seq === checkSeq) setFieldError(e ?? null);
    } catch {
      // ověření je jen nápověda — rozhodne až uložení
    }
  }

  function jump(e: FieldError | null) {
    const target = panelForField(e?.field);
    if (target) setActive(target);
    // pole s chybou může být v „Pokročilé“ — `hit` ho rozbalí; bez vlastního řádku zůstane první .st-err
    reveal(e?.field ?? null, main);
  }

  async function save(force = false) {
    if (saving() || !isLoaded()) return;
    setSaving(true);
    try {
      const out = cleanDraft(snapshotDraft());
      if (!force) {
        // někdo config mezitím změnil (Claude, druhé okno, ruční úprava) → nepřepsat potichu
        const live = await api.getConfig();
        if (live && canon(live) !== baseCanon()) {
          setConflict(true);
          return;
        }
      }
      const err = await api.checkConfig(out);
      if (err) {
        setFieldError(err);
        jump(err);
        say(err.message, true);
        return;
      }
      await api.saveConfig(out);
      // co panely odložily na „až bude uloženo“ (souhlas s příkazem akce)
      for (const e of await runAfterSave()) sayError(e);
      const fresh = await api.getConfig();
      if (fresh) loadBase(fresh);
      setConflict(false);
      setFieldError(null);
      setDirty(false);
      say(t("settings.saved", "Uloženo — změny platí hned"));
    } catch (e) {
      sayError(e);
    } finally {
      setSaving(false);
    }
  }

  async function reloadLive() {
    const live = await api.getConfig();
    if (live) loadBase(live);
    setConflict(false);
    say(t("settings.reloaded", "Načteno aktuální nastavení"));
  }

  // ---------- hledání ----------
  const items = createMemo<SmartItem[]>(() => searchItems());
  const results = createMemo(() => smartSearch(items(), query()).slice(0, 30));
  createEffect(on(query, () => setSel(0)));
  function go(it: SmartItem | undefined) {
    if (!it) return;
    setActive(it.panel);
    setQuery("");
    reveal(it.anchor || null, main);
  }
  // skok odjinud (karta, paleta, hlavička, Doporučené — `palette/goto.ts` `gotoSettingsPanel`): id stránky,
  // nebo kotva `f-<pole>` → stránka toho pole (`labels.ts`, jinak podle cesty) + posun a zvýraznění
  // (staré id panelů vedou přes `PANEL_ALIASES` na nové)
  createEffect(() => {
    const id = settingsTarget();
    if (!id) return;
    setSettingsTarget(null);
    const to = resolveTarget(id, PANELS.map((x) => x.id), PANEL_ALIASES, (f) => FIELDS[f]?.panel ?? panelForField(f));
    if (to.panel) setActive(to.panel);
    if (to.anchor) reveal(to.anchor, main);
  });
  const searchKey = (e: KeyboardEvent) => {
    if (e.key === "ArrowDown" || e.key === "ArrowUp") {
      e.preventDefault();
      const n = results().length;
      if (n) setSel((sel() + (e.key === "ArrowDown" ? 1 : n - 1)) % n);
    } else if (e.key === "Enter") {
      e.preventDefault();
      go(results()[sel()]);
    } else if (e.key === "Escape" && query()) {
      e.stopPropagation();
      setQuery("");
    }
  };
  /** „Sekce › stránka“ (u sekce s jedinou stránkou jen sekce) */
  const placeName = (id: string) => {
    const x = panelOf(id);
    const sec = sectionOf(id);
    if (!x) return id;
    return sec && sec.pages.length > 1 ? `${sectionLabel(sec)} › ${panelLabel(x)}` : sec ? sectionLabel(sec) : panelLabel(x);
  };

  // ---------- navigace ----------
  const goSection = (id: string) => {
    const sec = SECTIONS.find((s) => s.id === id);
    if (sec) setActive(pageForSection(sec, lastInSection()));
  };
  /** šipky / Home / End mezi viditelnými položkami navigace (sekce i rozbalené stránky) */
  const navKey = (e: KeyboardEvent) => {
    const btns = [...nav.querySelectorAll<HTMLButtonElement>("button[data-nav]")];
    const i = btns.findIndex((b) => b === document.activeElement);
    const to = e.key === "ArrowDown" ? i + 1 : e.key === "ArrowUp" ? i - 1 : e.key === "Home" ? 0 : e.key === "End" ? btns.length - 1 : -2;
    if (to === -2 || !btns.length) return;
    e.preventDefault();
    const b = btns[(to + btns.length) % btns.length];
    b.click();
    queueMicrotask(() => nav.querySelector<HTMLButtonElement>(`button[data-nav="${b.dataset.nav}"]`)?.focus());
  };
  // posuv každé stránky zvlášť: návrat na stránku vrátí, kde byla (skok z hledání si pak posune sám)
  const scrollOf: Record<string, number> = {};
  createEffect(on(active, (id, prev) => {
    if (!main) return;
    if (prev) scrollOf[prev] = main.scrollTop;
    queueMicrotask(() => (main.scrollTop = scrollOf[id] ?? 0));
  }, { defer: true }));

  const status = () => {
    const e = fieldError();
    if (e) return { tone: "bad", text: e.message, err: e };
    if (dirty()) return { tone: "warn", text: t("settings.bar.dirty", "Neuložené změny"), err: null };
    return { tone: "ok", text: t("settings.bar.saved", "Uloženo · změny se projeví hned"), err: null };
  };
  const badPage = () => panelForField(fieldError()?.field);

  return (
    <SettingsCtx.Provider value={ctx}>
      <div class="st" data-mode={mode()} onKeyDown={(e) => e.key === "Escape" && query() && (e.stopPropagation(), setQuery(""))}>
        <aside class="st-nav">
          <label class="st-search">
            <IconSearch />
            <input
              ref={searchEl}
              type="search"
              placeholder={t("settings.search", "Hledat v nastavení  (Ctrl+F)")}
              aria-label={t("settings.search.aria", "Hledat v nastavení")}
              value={query()}
              spellcheck={false}
              onInput={(e) => setQuery(e.currentTarget.value)}
              onKeyDown={searchKey}
            />
          </label>
          <div class="seg st-mode" role="radiogroup" aria-label={t("settings.mode.aria", "Kolik voleb ukázat")}>
            <button role="radio" aria-checked={mode() === "basic"} classList={{ on: mode() === "basic" }} title={t("settings.mode.basicHint", "Pokročilé volby jsou schované")} onClick={() => setMode("basic")}>
              {t("settings.mode.basic", "Základní")}
            </button>
            <button role="radio" aria-checked={mode() === "advanced"} classList={{ on: mode() === "advanced" }} title={t("settings.mode.advancedHint", "Ukáže všechno včetně technických voleb")} onClick={() => setMode("advanced")}>
              {t("settings.mode.advanced", "Pokročilé")}
            </button>
          </div>
          <Show
            when={query().trim()}
            fallback={
              <div class="st-tabs" role="tablist" aria-orientation="vertical" ref={nav} onKeyDown={navKey}>
                <For each={SECTIONS}>
                  {(sec) => (
                    <>
                      <button
                        role="tab"
                        data-nav={sec.id}
                        class="st-sec-btn"
                        aria-selected={section().id === sec.id}
                        tabIndex={section().id === sec.id && sec.pages.length === 1 ? 0 : -1}
                        classList={{ on: section().id === sec.id, bad: !!badPage() && sec.pages.includes(badPage()!), group: sec.pages.length > 1 }}
                        onClick={() => goSection(sec.id)}
                      >
                        <PanelIcon id={sec.id} />
                        <span>{sectionLabel(sec)}</span>
                      </button>
                      <Show when={sec.pages.length > 1 && section().id === sec.id}>
                        <For each={sec.pages}>
                          {(id) => (
                            <button
                              role="tab"
                              data-nav={id}
                              class="st-sub-btn"
                              aria-selected={active() === id}
                              tabIndex={active() === id ? 0 : -1}
                              classList={{ on: active() === id, bad: badPage() === id }}
                              onClick={() => setActive(id)}
                            >
                              <span>{panelLabel(panelOf(id)!)}</span>
                            </button>
                          )}
                        </For>
                      </Show>
                    </>
                  )}
                </For>
              </div>
            }
          >
            <div class="st-results" role="listbox" aria-label={t("settings.search.results", "Výsledky hledání")}>
              <Show when={results().length} fallback={<p class="st-empty">{t("settings.search.none", "Nic nenalezeno. Zkus jiné slovo — hledá se i v popisech.")}</p>}>
                <For each={results()}>
                  {(r, i) => (
                    <button role="option" aria-selected={sel() === i()} classList={{ on: sel() === i() }} onMouseEnter={() => setSel(i())} onClick={() => go(r)}>
                      <PanelIcon id={sectionOf(r.panel)?.id ?? r.panel} />
                      <span>
                        <For each={highlight(r.label, query())}>{(seg) => (seg.hit ? <mark>{seg.t}</mark> : seg.t)}</For>
                        <Show when={r.adv}><em class="st-tag">{t("settings.search.adv", "Pokročilé")}</em></Show>
                        <small>
                          {r.anchor || r.keyOnly ? placeName(r.panel) : t("settings.search.page", "stránka")}
                          <Show when={r.keyOnly || (r.key && query().includes("."))}> · <code>{r.key}</code></Show>
                        </small>
                      </span>
                    </button>
                  )}
                </For>
              </Show>
            </div>
          </Show>
        </aside>

        <div class="st-main" ref={main} role="tabpanel" aria-label={panelLabel(page())}>
          <Show when={conflict()}>
            <div class="st-conflict" role="alert">
              <span>{t("settings.conflict", "Nastavení se mezitím změnilo jinde (třeba ho upravil Claude nebo druhé okno).")}</span>
              <button class="mini" onClick={reloadLive}>{t("settings.conflict.reload", "Načíst nové (moje změny zahodit)")}</button>
              <button class="mini ok" onClick={() => save(true)}>{t("settings.conflict.overwrite", "Přesto uložit moje")}</button>
            </div>
          </Show>
          <Show
            when={isLoaded() && p.s}
            fallback={
              <div class="st-loading" aria-busy="true">
                <div class="st-skel h" />
                <For each={[1, 2, 3, 4, 5]}>{() => <div class="st-skel" />}</For>
              </div>
            }
          >
            {/* navštívené stránky žijí dál skryté: přepnutí nic neničí ani znovu nenačítá */}
            <For each={visited()}>
              {(id) => (
                <div class="st-panel" data-panel={id} hidden={active() !== id}>
                  <Dynamic component={panelOf(id)?.component} cfg={ctx.cfg} setCfg={setCfg} s={p.s} />
                  <p class="st-advnote">
                    <span>{t("settings.mode.hidden", "Některé volby na téhle stránce jsou pokročilé (porty, repa, soubory).")}</span>
                    <button class="mini" onClick={() => setMode("advanced")}>{t("settings.mode.show", "Ukázat pokročilé")}</button>
                  </p>
                </div>
              )}
            </For>
          </Show>
        </div>

        <Show when={page().id === "vzhled" && isLoaded()}>
          <PreviewDock />
        </Show>

        <footer class={`st-bar ${status().tone}`}>
          <Show
            when={undoMsg()}
            fallback={
              <span class="st-bar-msg" role="status">
                <span class="st-dot" classList={{ ok: status().tone === "ok", warn: status().tone === "warn", bad: status().tone === "bad" }} />
                <span class="txt">{status().text}</span>
                <Show when={status().err}>{(e) => <button class="link" onClick={() => jump(e())}>{t("settings.bar.show", "Ukázat")}</button>}</Show>
              </span>
            }
          >
            {(u) => (
              <span class="st-bar-msg" role="status">
                <span class="txt">{u().text}</span>
                <button class="link" onClick={runUndo}>{t("settings.bar.undo", "Vrátit")}</button>
              </span>
            )}
          </Show>
          <button class="btn" disabled={!dirty() || saving()} onClick={() => (discard(), setFieldError(null), setConflict(false))}>{t("settings.bar.discard", "Zahodit")}</button>
          <button class="btn pri" disabled={!dirty() || saving()} onClick={() => save()} title="Ctrl+S">
            {saving() ? t("settings.bar.saving", "Ukládám…") : t("settings.bar.save", "Uložit")}
          </button>
        </footer>
      </div>
    </SettingsCtx.Provider>
  );
}
