// Paleta příkazů (Ctrl+K v šuplíku s fokusem, nebo 🔍 v hlavičce): jedno pole, fuzzy hledání
// přes všechno (`items.ts` + `search.ts`), šipky / Enter / Esc, nedávno použité nahoře (localStorage,
// jen pohodlí), zvýraznění shody. Fokus si bere jen po výslovném kliku (`took` → při zavření
// `releaseFocus`); rychlé akce s potvrzením se potvrzují přímo tady. Kompaktní režim
// (`s.paletteCompact`): bez dotazu jen pole, seznam až při psaní / šipkou dolů (`compact.ts`).
// Navíc skripty ze složky skriptů (načtou se při otevření) a s dotazem stránky Nastavení Windows (`more.ts`).
// Rychlé odpovědi z dotazu (`quick.ts` přes `more.ts`): výpočet / převod jednotek jako první řádek,
// `:` → emoji místo seznamu; Enter je zkopíruje do schránky (dokud má paleta fokus) a paletu zavře.
import { createEffect, createMemo, createSignal, For, Index, on, onMount, Show } from "solid-js";
import { api, type NoteView, type Snapshot } from "../api";
import { IconFolder, IconLink, IconNote, IconSettings, IconShield, IconTerminal } from "../icons";
import { ActionIcon, ConfirmAction, answerConfirm, confirmPending } from "../actions/ui";
import { buildItems, SECTION_LABEL, type Nav, type PItem, type Section } from "./items";
import { pushRecent, rank, segments, type Ranked } from "./search";
import { arrow, shown, visible } from "./compact";
import { emojiCommand, paletteTips, quickRows, scriptItems, windowsItems } from "./more";
import { scriptsApi } from "../actions/scripts";
import type { ScriptView } from "../types";
import { lang, t } from "../i18n";
import { say, sayError } from "../ui/toast";
import "./palette.css";

export const IconSearch = () => (
  <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
    <circle cx="11" cy="11" r="7" />
    <path d="m20 20-3.5-3.5" />
  </svg>
);

const GLYPH: Record<Section, () => any> = { action: IconLink, project: IconFolder, note: IconNote, claude: IconTerminal, settings: IconSettings, app: IconShield, script: IconTerminal, windows: IconSettings, calc: IconSearch, convert: IconSearch, emoji: IconNote };
const RECENT_KEY = "ledge.palette.recent";

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
  const [waiting, setWaiting] = createSignal(false);
  /** kompaktní paleta s prázdným polem rozbalená šipkou dolů */
  const [browsing, setBrowsing] = createSignal(false);
  let input!: HTMLInputElement;
  let list!: HTMLDivElement;

  onMount(() => {
    requestAnimationFrame(() => input.focus());
    if (p.s.notesEnabled) api.noteList(200).then((n) => n && setNotes(n), () => {});
    // levný sken složky skriptů (stat, jen změněné soubory); vypnuté / bez složky = prázdné
    scriptsApi.list().then((l) => l && setScripts(l.scripts), () => {});
  });

  const base = createMemo(() => [...buildItems(p.s, notes(), p.nav), ...scriptItems(scripts()), emojiCommand(setQ)]);
  // stránky Nastavení Windows jen s dotazem — přepočet při psaní nesahá na `buildItems`
  const items = createMemo(() => [...base(), ...windowsItems(p.s, q())]);
  const recentSet = createMemo(() => new Set(recent()));
  const fold = () => ({ compact: p.s.paletteCompact, query: q(), browsing: browsing() });
  // nedávné, které v položkách pořád jsou (smazaná akce / poznámka se nepočítá)
  const what = createMemo(() => shown(fold(), items().filter((i) => recentSet().has(i.id)).length));
  // výpočet / převod / emoji jen z dotazu — přepočet při psaní, nic na pozadí
  const quick = createMemo(() => quickRows(q(), lang()));
  const results = createMemo(() => {
    const qr = quick();
    if (qr.only) return qr.rows;
    const list = visible(rank(items(), q(), recent(), 60), what(), recentSet());
    return qr.rows.length ? [...qr.rows, ...list] : list;
  });
  // jiný dotaz = výběr nahoru; smazaný dotaz kompaktní paletu zase sbalí
  createEffect(on(q, () => (setSel(0), setBrowsing(false)), { defer: true }));
  createEffect(on(sel, (i) => (list?.children[i] as HTMLElement | undefined)?.scrollIntoView({ block: "nearest" })));

  function close() {
    if (p.took) api.releaseFocus();
    p.onClose();
  }

  async function choose(r: Ranked<PItem> | undefined) {
    if (!r || waiting()) return;
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
            : results()[sel()]?.item.copy !== undefined
              ? t("palette.keysCopy", "↑↓ výběr · Enter zkopírovat · Esc zavřít")
              : t("palette.keys", "↑↓ výběr · Enter spustit · Esc zavřít")}
        </span>
        <Show when={!q()}><span class="pal-tip">{paletteTips()}</span></Show>
      </div>
    </div>
  );
}
