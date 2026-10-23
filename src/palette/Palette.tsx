// Paleta příkazů (Ctrl+K v šuplíku s fokusem, nebo 🔍 v hlavičce): jedno pole, fuzzy hledání
// přes všechno (`items.ts` + `search.ts`), šipky / Enter / Esc, nedávno použité nahoře (localStorage,
// jen pohodlí) a frecency (četnost × čerstvost použití, `search.ts` `bumpUsage`), zvýraznění shody. Fokus si bere jen po výslovném kliku (`took` → při zavření
// `releaseFocus`); rychlé akce s potvrzením se potvrzují přímo tady. Kompaktní režim
// (`s.paletteCompact`): bez dotazu jen pole, seznam až při psaní / šipkou dolů (`compact.ts`).
// Navíc skripty ze složky skriptů (načtou se při otevření) a s dotazem stránky Nastavení Windows (`more.ts`).
// Rychlé odpovědi z dotazu (`quick.ts` přes `more.ts`): výpočet / převod jednotek jako první řádek,
// `:` → emoji místo seznamu; Enter je zkopíruje do schránky (dokud má paleta fokus) a paletu zavře.
// Historie schránky (`v:`, `cliphist/model.ts`): seznam z jádra při vstupu do režimu a po události
// `cliphist`; Enter = položka zpět do schránky (jádro, vlastní zápis), Ctrl+P připnout, Ctrl+Del smazat.
import { createEffect, createMemo, createSignal, For, Index, on, onCleanup, onMount, Show } from "solid-js";
import { api, on as onEvent, type NoteView, type Snapshot } from "../api";
import { IconFolder, IconLink, IconNote, IconSearch, IconSettings, IconShield, IconTerminal } from "../icons";
import { ActionIcon, ConfirmAction, answerConfirm, confirmPending } from "../actions/ui";
import { buildItems, SECTION_LABEL, type Nav, type PItem, type Section } from "./items";
import { bumpUsage, cleanUsage, pushRecent, rank, segments, type Ranked, type Usage } from "./search";
import { mergeNoteHits, type NoteHit } from "./notehits";
import { arrow, shown, visible } from "./compact";
import { clipCommand, clipItems, emojiCommand, paletteTips, quickRows, scriptItems, windowsItems } from "./more";
import { clampSel, clipQuery, clipRows, type ClipList, type ClipRow } from "../cliphist/model";
import { clipApi } from "../cliphist/api";
import { emojiLoaded, emojiQuery, loadEmoji } from "./emoji";
import { scriptsApi } from "../actions/scripts";
import type { ScriptView } from "../types";
import { lang, t } from "../i18n";
import { say, sayError } from "../ui/toast";
import "./palette.css";

const GLYPH: Record<Section, () => any> = { action: IconLink, project: IconFolder, note: IconNote, claude: IconTerminal, settings: IconSettings, app: IconShield, script: IconTerminal, windows: IconSettings, calc: IconSearch, convert: IconSearch, emoji: IconNote, clip: IconNote };
const RECENT_KEY = "ledge.palette.recent";
const USAGE_KEY = "ledge.palette.usage";
/** Ticho po posledním znaku, než se hledá v těle poznámek (psaní nevolá jádro na každý znak). */
const HIT_DELAY = 180;

function loadRecent(): string[] {
  try {
    const v = JSON.parse(localStorage.getItem(RECENT_KEY) ?? "[]");
    return Array.isArray(v) ? v.filter((x) => typeof x === "string") : [];
  } catch {
    return [];
  }
}
/**
 * Text do schránky z palety. Paleta má vždy fokus (otevírá se klikem nebo Ctrl+K v šuplíku s fokusem),
 * takže stačí `navigator.clipboard`; záloha `execCommand("copy")` přes skryté pole. Bez jádra.
 */
async function copyText(text: string, refocus: HTMLElement): Promise<void> {
  try {
    await navigator.clipboard.writeText(text);
    return;
  } catch {
    /* záloha níž */
  }
  const ta = document.createElement("textarea");
  ta.value = text;
  ta.setAttribute("readonly", "");
  ta.style.cssText = "position:fixed;left:-9999px;opacity:0";
  document.body.appendChild(ta);
  ta.select();
  const ok = document.execCommand("copy");
  ta.remove();
  refocus.focus();
  if (!ok) throw new Error(t("palette.copyFail", "Do schránky to nejde zapsat."));
}

function loadUsage(): Usage {
  try {
    return cleanUsage(JSON.parse(localStorage.getItem(USAGE_KEY) ?? "{}"));
  } catch {
    return {};
  }
}

function saveUsage(u: Usage) {
  try {
    localStorage.setItem(USAGE_KEY, JSON.stringify(u));
  } catch {
    /* bez úložiště jen nebude frecency */
  }
}

function saveRecent(ids: string[]) {
  try {
    localStorage.setItem(RECENT_KEY, JSON.stringify(ids));
  } catch {
    /* bez úložiště jen nebudou nedávné */
  }
}

export default function Palette(p: { s: Snapshot; took: boolean; nav: Nav; onClose: () => void }) {
  const [q, setQ] = createSignal("");
  const [sel, setSel] = createSignal(0);
  const [notes, setNotes] = createSignal<NoteView[]>([]);
  const [scripts, setScripts] = createSignal<ScriptView[]>([]);
  const [recent, setRecent] = createSignal(loadRecent());
  const [usage, setUsage] = createSignal(loadUsage());
  /** poznámky nalezené podle obsahu (`note_search`) k dotazu `q` */
  const [hits, setHits] = createSignal<{ q: string; list: NoteHit[] }>({ q: "", list: [] });
  const [waiting, setWaiting] = createSignal(false);
  /** kompaktní paleta s prázdným polem rozbalená šipkou dolů */
  const [browsing, setBrowsing] = createSignal(false);
  /** historie schránky (`v:`); `undefined` = ještě nenačtená */
  const [clip, setClip] = createSignal<ClipList>();
  let input!: HTMLInputElement;
  let list!: HTMLDivElement;

  onMount(() => {
    requestAnimationFrame(() => input.focus());
    if (p.s.notesEnabled) api.noteList(200).then((n) => n && setNotes(n), () => {});
    // levný sken složky skriptů (stat, jen změněné soubory); vypnuté / bez složky = prázdné
    scriptsApi.list().then((l) => l && setScripts(l.scripts), () => {});
    // změna historie (nový text, připnutí, zamčení) — jen s otevřeným režimem `v:`
    const un = onEvent("cliphist", () => clipText() !== null && loadClip());
    onCleanup(() => void un.then((f) => f(), () => {}));
  });

  const clipText = createMemo(() => clipQuery(q()));
  const loadClip = () => clipApi.list().then((l) => setClip(l ?? { enabled: false, items: [] }), () => setClip({ enabled: false, items: [] }));
  createEffect(on(() => clipText() !== null, (inClip) => inClip && void loadClip()));

  const base = createMemo(() => [...buildItems(p.s, notes(), p.nav), ...scriptItems(scripts()), emojiCommand(setQ), clipCommand(setQ)]);
  // stránky Nastavení Windows jen s dotazem — přepočet při psaní nesahá na `buildItems`
  const items = createMemo(() => [...mergeNoteHits(base(), hits().list, hits().q, p.nav, t("palette.note.untitled", "(bez názvu)")), ...windowsItems(p.s, q())]);
  // Hledání v těle poznámek: po 180 ms ticha, od 2 znaků, ne v režimu schránky; starší odpověď se zahodí
  let hitTimer: ReturnType<typeof setTimeout> | undefined;
  let hitSeq = 0;
  createEffect(on(q, (text) => {
    clearTimeout(hitTimer);
    const id = ++hitSeq;
    const query = text.trim();
    if (!p.s.notesEnabled || clipText() !== null || query.length < 2) {
      setHits((h) => (h.list.length ? { q: "", list: [] } : h));
      return;
    }
    hitTimer = setTimeout(() => {
      api.noteSearch(query, 8).then(
        (list) => id === hitSeq && setHits({ q: query, list: list ?? [] }),
        () => id === hitSeq && setHits({ q: "", list: [] }),
      );
    }, HIT_DELAY);
  }));
  onCleanup(() => clearTimeout(hitTimer));
  const recentSet = createMemo(() => new Set(recent()));
  const fold = () => ({ compact: p.s.paletteCompact, query: q(), browsing: browsing() });
  // nedávné, které v položkách pořád jsou (smazaná akce / poznámka se nepočítá)
  const what = createMemo(() => shown(fold(), items().filter((i) => recentSet().has(i.id)).length));
  // emoji tabulka je líný chunk (`emoji.ts` `loadEmoji`): načte se až při prvním `:` dotazu, pak přepočet
  const [emojiOk, setEmojiOk] = createSignal(emojiLoaded());
  createEffect(() => {
    if (!emojiOk() && emojiQuery(q()) !== null) loadEmoji().then(() => setEmojiOk(true), sayError);
  });
  // výpočet / převod / emoji jen z dotazu — přepočet při psaní, nic na pozadí
  const quick = createMemo(() => {
    emojiOk();
    return quickRows(q(), lang());
  });
  const results = createMemo(() => {
    const ct = clipText();
    if (ct !== null) return clipItems(clipRows(clip(), ct, Date.now()));
    const qr = quick();
    if (qr.only) return qr.rows;
    const list = visible(rank(items(), q(), recent(), 60, usage()), what(), recentSet());
    return qr.rows.length ? [...qr.rows, ...list] : list;
  });
  // jiný dotaz = výběr nahoru; smazaný dotaz kompaktní paletu zase sbalí
  createEffect(on(q, () => (setSel(0), setBrowsing(false)), { defer: true }));
  // smazaná položka historie: výběr zůstane na místě (nejvýš poslední řádek)
  createEffect(on(() => results().length, (n) => setSel((i) => clampSel(i, n)), { defer: true }));
  createEffect(on(sel, (i) => (list?.children[i] as HTMLElement | undefined)?.scrollIntoView({ block: "nearest" })));

  function close() {
    if (p.took) api.releaseFocus();
    p.onClose();
  }

  /** Řádek historie schránky: položka zpět do schránky / zapnout / vymazat / Nastavení. */
  async function chooseClip(row: ClipRow) {
    if (row.kind === "entry" && row.entry) {
      setWaiting(true);
      try {
        await clipApi.paste(row.entry.id);
      } catch (e) {
        setWaiting(false);
        sayError(e);
        return;
      }
      close();
      say(t("palette.clip.pasted", "Ve schránce — vlož ho Ctrl+V"));
      return;
    }
    if (row.kind === "settings") {
      close();
      p.nav.onSettings("f-clipboard.enabled");
      return;
    }
    if (row.kind === "off") {
      try {
        await clipApi.enable();
        say(t("palette.clip.enabled", "Historie schránky je zapnutá — od teď si pamatuje zkopírovaný text"));
      } catch (e) {
        sayError(e);
      }
      await loadClip();
    } else if (row.kind === "clear") {
      await clipApi.clear().catch(sayError);
      await loadClip();
    }
    input.focus();
  }

  /** Ctrl+P připnout / odepnout, Ctrl+Delete smazat vybranou položku historie. */
  function clipKey(e: KeyboardEvent): boolean {
    const row = results()[sel()]?.item.clip;
    if (row?.kind !== "entry" || !row.entry || !e.ctrlKey) return false;
    const id = row.entry.id;
    if (e.key === "p" || e.key === "P") {
      e.preventDefault();
      // připnutá jde nahoru — výběr jde s ní
      clipApi.pin(id, !row.entry.pinned).then(() => loadClip().then(() => setSel(Math.max(0, results().findIndex((r) => r.item.clip?.entry?.id === id)))), sayError);
      return true;
    }
    if (e.key === "Delete") {
      e.preventDefault();
      clipApi.remove(id).then(loadClip, sayError);
      return true;
    }
    return false;
  }

  async function choose(r: Ranked<PItem> | undefined) {
    if (!r || waiting()) return;
    if (r.item.clip) return chooseClip(r.item.clip);
    const copy = r.item.copy;
    if (copy !== undefined) {
      // kopírovat dřív, než paleta pustí fokus; do nedávných nepatří (vznikají z dotazu)
      setWaiting(true);
      try {
        await copyText(copy, input);
      } catch (e) {
        setWaiting(false);
        sayError(e);
        return;
      }
      close();
      say(t("palette.copied", "Zkopírováno: {value}", { value: copy }));
      return;
    }
    if (r.item.stay) {
      r.item.run();
      input.focus();
      return;
    }
    const next = pushRecent(recent(), r.item.id);
    setRecent(next);
    saveRecent(next);
    const used = bumpUsage(usage(), r.item.id, Date.now());
    setUsage(used);
    saveUsage(used);
    if (r.item.action?.needsConfirm) {
      // potvrzení se ukáže v paletě; zavře se až po odpovědi
      setWaiting(true);
      await r.item.run();
      setWaiting(false);
      close();
      return;
    }
    close();
    r.item.run();
  }

  function key(e: KeyboardEvent) {
    const n = results().length;
    if (clipKey(e)) return;
    if (e.key === "Escape") {
      e.preventDefault();
      e.stopPropagation();
      if (confirmPending("palette")) answerConfirm(false);
      else close();
    } else if (e.key === "Enter") {
      e.preventDefault();
      if (confirmPending("palette")) answerConfirm(true);
      else choose(results()[sel()]);
    } else if (e.key === "ArrowDown" || e.key === "ArrowUp") {
      e.preventDefault();
      const next = arrow(e.key, fold(), sel(), n);
      setBrowsing(next.browsing);
      setSel(next.sel);
    }
  }

  return (
    <div class="card pal" classList={{ "pal-bare": what() === "none" }} data-anim onKeyDown={key}>
      <div class="pal-input">
        <IconSearch />
        <input
          ref={input}
          type="text"
          value={q()}
          spellcheck={false}
          autocomplete="off"
          placeholder={t("palette.placeholder", "Hledej akce, projekty, poznámky, session, nastavení…")}
          aria-label={t("palette.aria", "Paleta příkazů")}
          aria-controls="pal-list"
          onInput={(e) => setQ(e.currentTarget.value)}
        />
        <kbd>Esc</kbd>
      </div>
      <Show when={what() !== "none"}>
        <div class="pal-list" id="pal-list" ref={list} role="listbox">
          <Index each={results()} fallback={<div class="empty">{t("palette.none", "Nic nenalezeno.")}</div>}>
            {(r, i) => (
              <div class="pal-row" role="option" aria-selected={i === sel()} classList={{ on: i === sel() }} onMouseMove={() => i !== sel() && setSel(i)} onClick={() => choose(r())}>
                <span class="pal-ico">
                  <Show when={r().item.action} fallback={<Show when={r().item.glyph} fallback={GLYPH[r().item.section]()}>{(g) => <span class="pal-glyph">{g()}</span>}</Show>}>
                    {(a) => <ActionIcon a={a()} size={20} />}
                  </Show>
                </span>
                <div class="grow">
                  <div class="name">
                    <For each={segments(r().item.title, r().ranges)}>{(sg) => (sg.hit ? <mark>{sg.text}</mark> : sg.text)}</For>
                  </div>
                  <Show when={r().item.sub}><div class="sub">{r().item.sub}</div></Show>
                </div>
                <Show when={!q() && recentSet().has(r().item.id)}><span class="pal-recent">{t("palette.recent", "nedávno")}</span></Show>
                <span class="pill n">{t(...SECTION_LABEL[r().item.section])}</span>
              </div>
            )}
          </Index>
        </div>
      </Show>
      <ConfirmAction where="palette" />
      <div class="pal-foot">
        <span>
          {what() === "none"
            ? t("palette.keysBare", "Piš pro hledání · ↓ procházet · Esc zavřít")
            : results()[sel()]?.item.clip?.kind === "entry"
              ? t("palette.clip.keys", "↑↓ výběr · Enter do schránky · Ctrl+P připnout · Ctrl+Del smazat · Esc zavřít")
              : results()[sel()]?.item.copy !== undefined
              ? t("palette.keysCopy", "↑↓ výběr · Enter zkopírovat · Esc zavřít")
              : t("palette.keys", "↑↓ výběr · Enter spustit · Esc zavřít")}
        </span>
        <Show when={!q()}><span class="pal-tip">{paletteTips()}</span></Show>
      </div>
    </div>
  );
}
