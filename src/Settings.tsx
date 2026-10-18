// Nastavení = skořápka: postranní navigace panelů (`settings/catalog.ts`), hledání napříč všemi volbami,
// obsah panelu s vlastním posuvem a lišta Uložit / Zahodit. Rozpracovaný config drží `settings/ctx.ts`
// (přežije zavření stránky), ověřuje ho Rust (`check_config`) 250 ms po změně a chyba se ukáže u pole.
// Ukládá se ručně (Ctrl+S): config sdílí Claude i druhé PC a projekty bývají chvíli napůl vyplněné.
import { createEffect, createMemo, createSignal, For, on, onCleanup, onMount, Show } from "solid-js";
import { Dynamic } from "solid-js/web";
import { api, type FieldError, type Snapshot, type UpdateView } from "./api";
import { t } from "./i18n";
import { say, sayError } from "./ui/toast";
import { PANELS, panelLabel, searchItems } from "./settings/catalog";
import {
  SettingsCtx, active, base, dirty, discard, fieldError, isLoaded, loadBase, makeCtx, rev, runAfterSave, runUndo,
  setActive, setCfg, setDirty, setFieldError, setHit, snapshotDraft, undoMsg,
} from "./settings/ctx";
import { IconSearch, PanelIcon } from "./settings/icons";
import { canon, cleanDraft, highlight, panelForField, search, type SearchItem } from "./settings/model";
import { narrowOut, widenIn } from "./settings/widen";
import "./settings.css";

const reduced = () => window.matchMedia("(prefers-reduced-motion: reduce)").matches;

/** Skok na řádek: panel, posun do středu, krátké zvýraznění. */
function reveal(anchor: string | null, main: HTMLElement) {
  requestAnimationFrame(() => {
    const el = (anchor && document.getElementById(`f-${anchor}`)) || main.querySelector<HTMLElement>(".st-err");
    if (!el) return;
    el.scrollIntoView({ block: "center", behavior: reduced() ? "auto" : "smooth" });
    if (anchor) {
      setHit(anchor);
      window.setTimeout(() => setHit(null), 1800);
    }
  });
}

export default function Settings(p: { update: UpdateView; s: Snapshot }) {
  const [saving, setSaving] = createSignal(false);
  const [conflict, setConflict] = createSignal(false);
  const [query, setQuery] = createSignal("");
  const [sel, setSel] = createSignal(0);
  let main!: HTMLDivElement;
  let searchEl!: HTMLInputElement;
  let nav!: HTMLDivElement;

  const ctx = makeCtx(p.s, () => save());
  const baseCanon = createMemo(() => (base() ? canon(JSON.parse(base())) : ""));
  const panel = () => PANELS.find((x) => x.id === active()) ?? PANELS[0];

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
    api.getConfig().then((c) => c && (!isLoaded() || !dirty()) && loadBase(c), sayError);
  });
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
    reveal(null, main);
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
  const items = createMemo<SearchItem[]>(() => searchItems());
  const results = createMemo(() => search(items(), query()).slice(0, 30));
  createEffect(on(query, () => setSel(0)));
  function go(it: SearchItem | undefined) {
    if (!it) return;
    setActive(it.panel);
    setQuery("");
    reveal(it.anchor || null, main);
  }
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
  const panelName = (id: string) => {
    const x = PANELS.find((q) => q.id === id);
    return x ? panelLabel(x) : id;
  };

  // ---------- navigace ----------
  const navKey = (e: KeyboardEvent) => {
    const i = PANELS.findIndex((x) => x.id === active());
    const to = e.key === "ArrowDown" ? i + 1 : e.key === "ArrowUp" ? i - 1 : e.key === "Home" ? 0 : e.key === "End" ? PANELS.length - 1 : -2;
    if (to === -2) return;
    e.preventDefault();
    const next = PANELS[(to + PANELS.length) % PANELS.length];
    setActive(next.id);
    (nav.querySelector(`[data-id="${next.id}"]`) as HTMLElement | null)?.focus();
  };
  createEffect(on(active, () => main && (main.scrollTop = 0), { defer: true }));

  const status = () => {
    const e = fieldError();
    if (e) return { tone: "bad", text: e.message, err: e };
    if (dirty()) return { tone: "warn", text: t("settings.bar.dirty", "Neuložené změny"), err: null };
    return { tone: "ok", text: t("settings.bar.saved", "Uloženo · změny se projeví hned"), err: null };
  };

  return (
    <SettingsCtx.Provider value={ctx}>
      <div class="st" onKeyDown={(e) => e.key === "Escape" && query() && (e.stopPropagation(), setQuery(""))}>
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
          <Show
            when={query().trim()}
            fallback={
              <div class="st-tabs" role="tablist" aria-orientation="vertical" ref={nav} onKeyDown={navKey}>
                <For each={PANELS}>
                  {(x) => (
                    <button
                      role="tab"
                      data-id={x.id}
                      aria-selected={active() === x.id}
                      tabIndex={active() === x.id ? 0 : -1}
                      classList={{ on: active() === x.id, bad: panelForField(fieldError()?.field) === x.id }}
                      onClick={() => setActive(x.id)}
                    >
                      <PanelIcon id={x.id} />
                      <span>{panelLabel(x)}</span>
                    </button>
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
                      <PanelIcon id={r.panel} />
                      <span>
                        <For each={highlight(r.label, query())}>{(seg) => (seg.hit ? <mark>{seg.t}</mark> : seg.t)}</For>
                        <small>{r.anchor ? panelName(r.panel) : t("settings.search.panel", "panel")}</small>
                      </span>
                    </button>
                  )}
                </For>
              </Show>
            </div>
          </Show>
        </aside>

        <div class="st-main" ref={main} role="tabpanel" aria-label={panelLabel(panel())}>
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
            <For each={[panel()]}>
              {(x) => (
                <div class="st-panel" data-panel={x.id}>
                  <Dynamic component={x.component} cfg={ctx.cfg} setCfg={setCfg} s={p.s} />
                </div>
              )}
            </For>
          </Show>
        </div>

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
