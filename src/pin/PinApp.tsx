// Okno lístku na plochu (`pin.html`, label `note-<klíč>`): jedna existující poznámka Ledge přes ostatní
// okna. Obsah jen přes příkazy poznámek s otiskem (`note_read` / `note_save` / `note_toggle_task`),
// obrázek ze schránky stejnou cestou jako editor (`pasteImages` → `images` v `note_save` → PNG vedle
// poznámky). Změnu zvenku hlásí Rust událostí `pin-note` (otisk souboru), volby `pin-prefs`.
// Lišta: tah = posun (`pin_drag`), dvojklik = sbalit; ⋯ = volby (`PinMenu`: vzhled a styl, barva, štítky,
// okno, Schovat do…, export); ✕ = odepnout (poznámka zůstane, poloha a vzhled se pamatují). Průchozí lístek
// myš nevidí — vypíná ho zkratka (`hotkeys.notePassthrough`) nebo Ctrl+K v Ledge; nápověda při prvním zapnutí.
// Barva a štítky jdou do frontmatteru poznámky (`note_set_meta`), vazby `[[…]]` otevřou další lístek.
import { createSignal, For, onCleanup, onMount, Show } from "solid-js";
import { api, on, type NoteDetail, type PinInit, type PinPrefs } from "../api";
import Markdown from "../notes/Markdown";
import { b64Of, pasteImages } from "../notes/draft";
import { trimBody } from "../notes/editdraft";
import { listKey } from "../notes/listkeys";
import { fromMs } from "../notes/remind";
import { setLang, t, type Lang } from "../i18n";
import { errorText, say, sayError, Toast } from "../ui/toast";
import UndoBar from "../notes/UndoBar";
import { archiveNote, completeNote, deleteNote, reopenNote } from "../notes/lifecycle";
import type { LifeAction } from "../notes/lifeactions";
import { IconX } from "../icons";
import { debounce } from "../util/time";
import { IconEdit, IconLock, IconMore, IconPin, IconPlus, IconThrough } from "./icons";
import { appendLine, caretFromText, pinSuggestVisible, pointLine, remindAskText, SAVE_DELAY, type SaveState } from "./model";
import { pinLook } from "./styles";
import PinMenu from "./PinMenu";
import { colorHex, noteColor, withoutTag } from "../notes/meta";
import { headline } from "../notes/headline";
import { pinMeta } from "./pinmeta";
import { notesApi } from "../notes/api";
import { useWikiFollow, useWikiSuggest, WikiList, WikiMissing } from "../notes/WikiLinks";
import { SUGGEST_DELAY, toLocalValue, type Suggest } from "../notes/suggest";

/** Štítek zápisu v patičce — jen to, co se opravdu děje (žádné „uloženo“ dřív, než se uložilo). */
function saveLabel(st: SaveState): string {
  switch (st) {
    case "pending": return t("notes.pin.savePending", "Rozepsáno…");
    case "saving": return t("notes.pin.saving", "Ukládám…");
    case "saved": return t("notes.pin.saved", "Uloženo ✓");
    default: return "";
  }
}

export default function PinApp() {
  const [init, setInit] = createSignal<PinInit | null>(null);
  const [prefs, setPrefs] = createSignal<PinPrefs | null>(null);
  const [note, setNote] = createSignal<NoteDetail | null>(null);
  const [gone, setGone] = createSignal<string | null>(null);
  const [editing, setEditing] = createSignal(false);
  const [text, setText] = createSignal("");
  const [conflict, setConflict] = createSignal(false);
  const [adding, setAdding] = createSignal(false);
  const [addText, setAddText] = createSignal("");
  const [menu, setMenu] = createSignal(false);
  /** návrh připomínky z právě přidaného bodu („zítra v 10 zavolat X“) — jen po přidání, nic se neptá v klidu */
  const [sug, setSug] = createSignal<Suggest | null>(null);
  let sugSeq = 0;
  let menuBtn: HTMLButtonElement | undefined;
  /** po Splnit / Archivovat / Smazat: lišta „Zpět“, lístek se po ní sám zavře */
  const [after, setAfter] = createSignal(false);
  /** nápověda před prvním průchozím režimem (s čerstvým stavem zkratky) */
  const [hint, setHint] = createSignal<PinInit | null>(null);
  /** posuvníky průhledností: náhled během tahu, uloží se až po puštění */
  const [look, setLook] = createSignal<{ bg: number; text: number } | null>(null);
  let ta: HTMLTextAreaElement | undefined;
  let addInput: HTMLInputElement | undefined;
  /** běžící zápisy — událost `pin-note` z vlastního zápisu přijde dřív než odpověď */
  let saving = 0;
  let pendingHash: string | null = null;
  /** stav zápisu pro štítek v patičce: čeká na uložení / ukládá se / uloženo */
  const [saveState, setSaveState] = createSignal<SaveState>("idle");
  let savedTimer: number | undefined;

  const p = () => prefs();
  const through = () => !!p()?.through;
  const dirty = () => editing() && text().trimEnd() !== trimBody(note()?.body ?? "").trimEnd();
  /** barva podle štítku (`notes.tagColors`), když poznámka nemá vlastní */
  const tagColor = () => noteColor({ tags: note()?.tags, project: note()?.project }, init()?.tagColors, init()?.projectColors);
  const colors = () => {
    const l = look();
    return pinLook(p()?.style, l?.bg ?? p()?.bg ?? 88, l?.text ?? p()?.text ?? 100, noteColor(note(), init()?.tagColors, init()?.projectColors));
  };
  // vazby [[…]]: klik otevře poznámku jako další lístek, neexistující nabídne vytvořit
  const wiki = useWikiFollow((path) => void notesApi.pinOpen(path).catch(sayError));
  const wikiS = useWikiSuggest((v) => {
    setText(v);
    schedule();
  });

  async function patch(x: Partial<PinPrefs>) {
    try {
      const r = await api.pinSet(x);
      if (r) setPrefs(r);
    } catch (e) {
      sayError(e);
    }
  }

  async function load() {
    const i = init();
    if (!i) return;
    try {
      const n = await api.noteRead(i.path);
      if (!n) return;
      // čistý text (nic nepřipsáno) vs. dosud známá verze — rozhoduje se PŘED setNote
      const clean = editing() && !dirty();
      setNote(n);
      setGone(null);
      // text se přepíše jen při skutečné změně zvenku (editor, sync); shodný až na koncové mezery / zalomení
      // zůstane, jak ho šéf píše — jinak by mu load() žral mezeru a skákal kurzor
      if (clean && text().trimEnd() !== trimBody(n.body).trimEnd()) {
        const sel = ta ? { a: ta.selectionStart, b: ta.selectionEnd } : null;
        setText(trimBody(n.body));
        if (ta && sel) ta.setSelectionRange(Math.min(sel.a, ta.value.length), Math.min(sel.b, ta.value.length));
      }
    } catch (e) {
      setGone(errorText(e));
    }
  }

  /** Otisk souboru po přenačtení poznámek: jiný → přečíst znovu (rozepsaná úprava → banner). */
  function external(hash: string) {
    if (after()) return; // poznámka se právě splnila / archivovala / smazala — lišta „Zpět“ rozhoduje
    if (saving) {
      pendingHash = hash;
      return;
    }
    if (hash && hash === note()?.hash) return;
    if (!hash) return void setGone(t("notes.pin.gone", "Poznámka zmizela — je smazaná nebo přesunutá."));
    if (dirty()) return void setConflict(true);
    void load();
  }

  /** „Uloženo ✓“ na pár sekund — jen po zápisu, který opravdu doběhl. */
  function markSaved() {
    if (dirty()) return setSaveState("pending"); // mezitím se psalo dál
    setSaveState("saved");
    clearTimeout(savedTimer);
    savedTimer = window.setTimeout(() => setSaveState("idle"), 2500);
  }

  async function busy<T>(f: () => Promise<T>): Promise<T> {
    saving++;
    setSaveState("saving");
    try {
      return await f();
    } finally {
      saving--;
      if (!saving && saveState() === "saving") setSaveState("idle");
      if (!saving && pendingHash !== null) {
        const h = pendingHash;
        pendingHash = null;
        external(h);
      }
    }
  }

  /** Zápis celého těla (+ nové PNG) s otiskem; „changed“ = soubor se mezitím změnil jinde. */
  function write(body: string, images: string[] = []): Promise<"saved" | "changed"> {
    const n = note();
    if (!n) return Promise.resolve("changed");
    const before = text();
    return busy(async () => {
      const r = await api.noteSave({ path: n.path, expectedHash: n.hash, body, project: n.project ?? "", kind: n.kind, remind: fromMs(n.remindMs) || null, images });
      if (r?.outcome !== "saved") return "changed";
      setNote(r.note);
      // přidání během úpravy (bod, obrázek): pole převezme uložené tělo — jen když se mezitím nepsalo
      const nb = trimBody(r.note.body);
      if (editing() && (body !== before || images.length) && text() === before && text().trimEnd() !== nb.trimEnd()) setText(nb);
      return "saved";
    });
  }

  /** Přidání na konec (bod, vložený text, obrázek): při změně jinde načíst a zkusit ještě jednou. */
  async function add(line: string, images: string[] = []): Promise<boolean> {
    try {
      for (let i = 0; i < 2; i++) {
        const n = note();
        if (!n) return false;
        const r = await write(appendLine(editing() ? text() : n.body, line), images);
        if (r === "saved") {
          markSaved();
          return true;
        }
        if (editing()) {
          setConflict(true);
          return false;
        }
        await load();
      }
      say(t("notes.pin.retry", "Poznámka se právě měnila jinde — zkus to znovu."), true);
    } catch (e) {
      sayError(e);
    }
    return false;
  }

  async function toggle(line: number, checked: boolean) {
    const n = note();
    if (!n || through()) return;
    try {
      await busy(async () => {
        const r = await api.noteToggleTask(n.path, line, checked, n.hash);
        if (r?.outcome === "saved") {
          setNote(r.note);
          markSaved();
          return;
        }
        await load();
        say(t("notes.pin.reloaded", "Poznámka se mezitím změnila — načetl jsem novou verzi."), true);
      });
    } catch (e) {
      sayError(e);
    }
  }

  // ---------- úprava textu ----------
  /** `caret` = pozice v těle (dvojklik do textu), jinak konec; kurzor nikdy za koncové zalomení. */
  function startEdit(caret: number | null = null) {
    const n = note();
    if (!n || through()) return;
    setMenu(false);
    const body = trimBody(n.body);
    setText(body);
    setConflict(false);
    setEditing(true);
    queueMicrotask(() => {
      if (!ta) return;
      ta.focus();
      const at = Math.min(caret ?? body.length, body.length);
      ta.setSelectionRange(at, at);
    });
  }
  /** Dvojklik do vykresleného textu → kam v těle dát kurzor (text uzlu pod myší najde řádek těla). */
  function caretAt(e: MouseEvent): number | null {
    const n = note();
    const r = n ? document.caretRangeFromPoint?.(e.clientX, e.clientY) : null;
    const node = r?.startContainer;
    if (!n || !r || !node || node.nodeType !== Node.TEXT_NODE) return null;
    return caretFromText(trimBody(n.body), node.textContent ?? "", r.startOffset);
  }
  async function flush(): Promise<boolean> {
    autosave.cancel();
    if (!dirty()) return true;
    try {
      if ((await write(text())) === "saved") {
        markSaved();
        return true;
      }
      setConflict(true);
    } catch (e) {
      sayError(e);
    }
    setSaveState("pending");
    return false;
  }
  /** Automatické uložení: po chvíli ticha při psaní (a vždy při odepnutí, Hotovo, ztrátě fokusu). */
  const autosave = debounce(() => void flush(), SAVE_DELAY);
  const schedule = () => {
    setSaveState("pending");
    autosave();
  };
  async function finishEdit() {
    if (await flush()) setEditing(false);
  }
  async function reloadDiscard() {
    setConflict(false);
    setEditing(false);
    await load();
  }
  async function saveCopy() {
    const n = note();
    if (!n) return;
    try {
      await api.noteSave({ path: n.path, expectedHash: n.hash, body: text(), project: n.project ?? "", kind: n.kind, remind: fromMs(n.remindMs) || null, images: [] }, true);
      say(t("notes.pin.copied", "Uloženo jako nová poznámka — najdeš ji v kartě Poznámky."));
      await reloadDiscard();
    } catch (e) {
      sayError(e);
    }
  }

  // ---------- + bod ----------
  function openAdd() {
    if (through()) return;
    setMenu(false);
    setAdding(true);
    queueMicrotask(() => addInput?.focus());
  }
  async function submitAdd() {
    const line = pointLine(editing() ? text() : note()?.body ?? "", addText());
    if (!line) return setAdding(false);
    const typed = addText();
    if (await add(line)) {
      setAddText("");
      askRemind(typed);
    }
    addInput?.focus();
  }

  /** Po přidání bodu s časem: jedno zeptání jádra (po `SUGGEST_DELAY` ticha, další bod předchozí zeptání zahodí). */
  const askRemind = debounce((typed: string) => {
    const ask = remindAskText(typed, note()?.remindMs);
    const id = ++sugSeq;
    if (!ask) return void setSug(null);
    api.noteSuggestRemind(ask).then(
      (r) => id === sugSeq && setSug(r ?? null),
      () => id === sugSeq && setSug(null),
    );
  }, SUGGEST_DELAY);

  /** Klik na chip: připomínka poznámky přes stejný zápis jako editor (`note_save` s `remind`). */
  async function acceptRemind(s: Suggest) {
    const v = toLocalValue(s.iso);
    setSug(null);
    if (!v || !(await flush())) return;
    try {
      for (let i = 0; i < 2; i++) {
        const n = note();
        if (!n) return;
        const r = await busy(() => api.noteSave({ path: n.path, expectedHash: n.hash, body: n.body, project: n.project ?? "", kind: n.kind, remind: v, images: [] }));
        if (r?.outcome === "saved") {
          setNote(r.note);
          markSaved();
          say(t("notes.pin.remindSet", "Připomínka nastavena: {when}", { when: s.label }));
          return;
        }
        await load();
      }
      say(t("notes.pin.retry", "Poznámka se právě měnila jinde — zkus to znovu."), true);
    } catch (e) {
      sayError(e);
    }
  }

  // ---------- barva a štítky (frontmatter poznámky, `pinmeta.ts`) ----------
  const { meta, front, addTag } = pinMeta({ note, setNote, flush, busy, markSaved, load });

  // ---------- schránka: obrázek (Win+Shift+S → Ctrl+V) i text ----------
  async function paste(e: ClipboardEvent) {
    if (through() || !note()) return;
    const items = [...(e.clipboardData?.items ?? [])];
    if (items.some((i) => i.kind === "file" && i.type.startsWith("image/"))) {
      let got: string[] = [];
      await pasteImages(e, () => got, (v) => (got = v));
      if (got.length && (await add("", b64Of(got)))) say(t("notes.pin.imageSaved", "Obrázek uložený do poznámky."));
      return;
    }
    const tag = (e.target as HTMLElement | null)?.tagName ?? "";
    if (tag === "TEXTAREA" || tag === "INPUT") return; // text do pole normálně
    const txt = e.clipboardData?.getData("text/plain") ?? "";
    if (!txt.trim()) return;
    e.preventDefault();
    await add(txt);
  }

  // ---------- lišta ----------
  function barDown(e: MouseEvent) {
    if (e.button !== 0 || (e.target as HTMLElement).closest("button")) return;
    e.preventDefault();
    if (e.detail >= 2) return void patch({ collapsed: !p()?.collapsed });
    if (p()?.locked) return say(t("notes.pin.lockedHint", "Lístek je zamčený na místě — odemkneš ho v ⋯."));
    void api.pinDrag();
  }

  /** Průchozí režim: poprvé nejdřív nápověda, jak se z něj dostat. */
  async function wantThrough() {
    setMenu(false);
    const fresh = await api.pinInit().catch(() => null);
    if (fresh && !fresh.hintSeen) return void setHint(fresh);
    await patch({ through: true });
  }
  async function confirmThrough() {
    setHint(null);
    await api.pinHintSeen().catch(() => {});
    await patch({ through: true });
  }
  /** Rozepsané (text v úpravě, napsaný bod) se uloží do poznámky dřív, než okno zmizí. */
  async function settle(): Promise<boolean> {
    if (!(await flush())) return false;
    const line = adding() ? pointLine(editing() ? text() : note()?.body ?? "", addText()) : "";
    if (!line) return true;
    if (!(await add(line))) return false;
    setAddText("");
    return true;
  }
  /** Splnit · Archivovat · Smazat z lístku: lístek zůstane na pár sekund s lištou „Zpět“, pak se zavře. */
  async function life(a: LifeAction) {
    setMenu(false);
    const n = note();
    if (!n || a === "unarchive") return;
    if (a === "delete") {
      autosave.cancel();
    } else if (!(await settle())) {
      return say(t("notes.pin.unpinBlocked", "Odepnutí počká — rozepsaný text se nepodařilo uložit (viz upozornění v lístku)."), true);
    }
    if (a === "undone") {
      if (await reopenNote(n)) await load();
      return;
    }
    setAfter(true);
    const opts = {
      keepPin: true,
      onExpire: () => void api.pinUnpin().catch(sayError),
      onUndone: () => {
        setAfter(false);
        void load();
      },
    };
    const ok = a === "done" ? await completeNote(n, opts) : a === "archive" ? await archiveNote(n, opts) : await deleteNote(n, opts);
    if (!ok) setAfter(false);
  }

  async function unpin() {
    if (!(await settle())) return say(t("notes.pin.unpinBlocked", "Odepnutí počká — rozepsaný text se nepodařilo uložit (viz upozornění v lístku)."), true);
    await api.pinUnpin().catch(sayError);
  }

  /** „Schovat do…“: rozepsané se uloží, lístek zmizí a v ten čas se vrátí. */
  async function hide(until: string) {
    setMenu(false);
    if (!(await settle())) return say(t("notes.pin.unpinBlocked", "Odepnutí počká — rozepsaný text se nepodařilo uložit (viz upozornění v lístku)."), true);
    await notesApi.pinHide(until).catch(sayError);
  }

  async function lookAll() {
    try {
      const n = await notesApi.pinLookAll();
      say(t("notes.pin.lookAllDone", "Vzhled platí pro všechny lístky ({n}) i pro nové.", { n: n ?? 0 }));
    } catch (e) {
      sayError(e);
    }
  }

  onMount(() => {
    const offs = [on<string>("pin-note", external), on<PinPrefs>("pin-prefs", setPrefs), on<Lang>("pin-lang", (l) => void setLang(l))];
    onCleanup(() => offs.forEach((o) => o.then((f) => f())));
    const key = (e: KeyboardEvent) => {
      if (e.key !== "Escape") return;
      if (hint()) setHint(null);
      else if (adding()) setAdding(false);
      else if (editing()) void finishEdit();
    };
    // klik jinam = rozepsané hned do poznámky (nečekat na pauzu v psaní)
    const blur = () => void flush();
    window.addEventListener("keydown", key);
    window.addEventListener("blur", blur);
    document.addEventListener("paste", paste);
    onCleanup(() => {
      window.removeEventListener("keydown", key);
      window.removeEventListener("blur", blur);
      document.removeEventListener("paste", paste);
      autosave.cancel();
      clearTimeout(savedTimer);
    });
    void api.pinInit().then(
      async (i) => {
        if (!i) return;
        setLang(i.lang);
        setInit(i);
        setPrefs(i.prefs);
        await load();
        // nový lístek: rovnou se píše (předvyplněný titulek je vybraný a přepíše se prvním písmenem)
        if (i.fresh) {
          startEdit();
          queueMicrotask(() => ta?.select());
        }
      },
      (e) => setGone(errorText(e)),
    );
  });

  const keyText = () => init()?.key || "";
  const title = () => {
    const n = note();
    if (!n) return "…";
    // vlastní nadpis → „Projekt · Název · 26. 10.“; bez něj dnešní první řádek
    if (n.heading) return headline(n, n.project ? init()?.projects?.[n.project] ?? n.project : null, init()?.lang);
    return n.title || t("notes.imageOnly", "(jen obrázek)");
  };

  return (
    <div
      class="pin"
      classList={{ through: through(), collapsed: !!p()?.collapsed, locked: !!p()?.locked, after: after(), tinted: !!colors()["--pin-tint"] }}
      data-style={p()?.style ?? "classic"}
      style={colors()}
    >
      <div class="pin-bar" onMouseDown={barDown} title={p()?.locked ? t("notes.pin.lockedTitle", "Zamčeno na místě") : t("notes.pin.barTitle", "Táhni = posunout · dvojklik = sbalit / rozbalit")}>
        <span class="pin-title">{title()}</span>
        <Show when={p()?.locked}><span class="pin-state" title={t("notes.pin.lockedTitle", "Zamčeno na místě")}><IconLock /></span></Show>
        <Show when={!through()}>
          <button class="pin-ic" classList={{ on: !!p()?.onTop }} aria-pressed={!!p()?.onTop} title={p()?.onTop ? t("notes.pin.onTopOn", "Vždy navrchu — klik: normální okno") : t("notes.pin.onTopOff", "Normální okno — klik: vždy navrchu")} onClick={() => patch({ onTop: !p()?.onTop })}><IconPin /></button>
          <button class="pin-ic" ref={menuBtn} aria-haspopup="menu" aria-expanded={menu()} title={t("notes.pin.more", "Volby lístku")} onClick={() => setMenu(!menu())}><IconMore /></button>
          <button class="pin-ic" title={t("notes.pin.unpin", "Odepnout z plochy (poznámka zůstane v Ledge)")} onClick={unpin}><IconX /></button>
        </Show>
      </div>

      <Show when={!p()?.collapsed}>
        <div class="pin-body">
          <Show when={gone()}>
            {(g) => (
              <div class="pin-gone">
                <span>{g()}</span>
                <button class="pin-btn" onClick={unpin}>{t("notes.pin.unpinShort", "Odepnout")}</button>
              </div>
            )}
          </Show>
          <Show when={!gone() && note()}>
            {(n) => (
              <Show when={editing()} fallback={
                <div class="pin-md" onDblClick={(e) => !(e.target as HTMLElement).closest(".md-check, .md-link, .md-wiki") && startEdit(caretAt(e))}>
                  <Markdown body={n().body} images={n().imageData} onToggle={toggle} onWiki={(name) => !through() && void wiki.follow(name)} />
                </div>
              }>
                <textarea
                  ref={(el) => (ta = el)}
                  class="pin-text"
                  value={text()}
                  placeholder={t("notes.pin.placeholder", "Piš… - [ ] = bod checklistu. Obrázek: Win+Shift+S a Ctrl+V.")}
                  onInput={(e) => {
                    setText(e.currentTarget.value);
                    schedule();
                    wikiS.onInput(e.currentTarget);
                  }}
                  onBlur={() => wikiS.close()}
                  onKeyDown={(e) => {
                    if (wikiS.onKey(e)) return;
                    if (e.key === "Enter" && e.ctrlKey) {
                      e.preventDefault();
                      void finishEdit();
                    } else {
                      listKey(e, e.currentTarget, (v) => {
                        setText(v);
                        schedule();
                      });
                    }
                  }}
                />
              </Show>
            )}
          </Show>
          <Show when={editing()}>
            <WikiList s={wikiS} pin />
          </Show>
          <WikiMissing w={wiki} pin />
          <Show when={!editing() && note()?.tags?.length}>
            <div class="pin-tags" aria-label={t("notes.pin.tags", "Štítky")}>
              <For each={note()?.tags ?? []}>{(tg) => <span class="pin-tag">#{tg}</span>}</For>
            </div>
          </Show>
          <Show when={conflict()}>
            <div class="pin-banner">
              <span>{t("notes.pin.conflict", "Poznámka se mezitím změnila jinde. Tvoje úprava zůstala tady.")}</span>
              <button class="pin-btn" onClick={reloadDiscard}>{t("notes.pin.reload", "Načíst znovu")}</button>
              <button class="pin-btn pri" onClick={saveCopy}>{t("notes.pin.saveCopy", "Uložit jako kopii")}</button>
            </div>
          </Show>
        </div>

        <Show when={!through() && !gone() ? pinSuggestVisible(sug(), note()?.remindMs) : null}>
          {(g) => (
            <div class="pin-sug">
              <button class="pin-sug-go" title={t("notes.suggest.title", "Nastaví připomínku podle času v textu")} onClick={() => void acceptRemind(g())}>
                ⏰ {t("notes.suggest.ask", "Připomenout {when}?", { when: g().label })}
              </button>
              <button class="pin-sug-x" aria-label={t("notes.suggest.dismiss", "Nenabízet")} title={t("notes.suggest.dismiss", "Nenabízet")} onClick={() => setSug(null)}><IconX /></button>
            </div>
          )}
        </Show>
        <Show when={!through() && note() && !gone()}>
          <Show when={adding()} fallback={
            <div class="pin-foot">
              <button class="pin-btn" onClick={openAdd} title={t("notes.pin.addTitle", "Přidat bod checklistu (- [ ])")}><IconPlus /> {t("notes.pin.add", "bod")}</button>
              <Show when={editing()} fallback={<button class="pin-btn ghost" onClick={() => startEdit()} title={t("notes.pin.editTitle", "Upravit text (nebo dvojklik do lístku)")}><IconEdit /></button>}>
                <button class="pin-btn pri" onClick={finishEdit}>{t("notes.pin.done", "Hotovo")}</button>
              </Show>
              <Show when={saveState() !== "idle"}>
                <span class="pin-save" classList={{ ok: saveState() === "saved" }} role="status">{saveLabel(saveState())}</span>
              </Show>
            </div>
          }>
            <div class="pin-foot">
              <input
                ref={(el) => (addInput = el)}
                class="pin-add"
                value={addText()}
                placeholder={t("notes.pin.addPlaceholder", "Nový bod… Enter přidá, Esc zavře")}
                onInput={(e) => setAddText(e.currentTarget.value)}
                onKeyDown={(e) => {
                  if (e.key === "Enter") {
                    e.preventDefault();
                    void submitAdd();
                  }
                }}
              />
            </div>
          </Show>
        </Show>
      </Show>

      <Show when={menu() && !through() && p()}>
        {(pr) => (
          <PinMenu
            prefs={pr()}
            note={note()}
            trigger={menuBtn}
            look={look()}
            setLook={setLook}
            color={note()?.color ?? null}
            tagColor={tagColor()}
            tags={note()?.tags ?? []}
            frontTags={note() ? front(note()!) : []}
            body={() => (editing() ? text() : note()?.body ?? "")}
            onClose={() => setMenu(false)}
            patch={patch}
            setColor={(c) => void meta((n) => ((colorHex(n.color) ?? "") === (colorHex(c) ?? "") ? null : { color: c }))}
            addTag={addTag}
            removeTag={(tg) => void meta((n) => ({ tags: withoutTag(front(n), tg) }))}
            lookAll={() => void lookAll()}
            hide={(v) => void hide(v)}
            wantThrough={() => void wantThrough()}
            edit={() => startEdit()}
            editInLedge={() => { const n = note(); if (n) void settle().then((ok) => ok && api.pinEditInLedge(n.path).catch(sayError)); }}
            openExternal={() => { const n = note(); if (n) api.noteOpen(n.path).catch(sayError); }}
            unpin={() => void unpin()}
            life={(a) => void life(a)}
            settle={settle}
          />
        )}
      </Show>

      <Show when={!through() && hint()}>
        {(h) => (
          <div class="pop pop-fill pin-hint" role="dialog">
            <b>{t("notes.pin.hintTitle", "Průchozí režim")}</b>
            <p>{t("notes.pin.hintText", "Myš půjde lístkem na stránku pod ním — lístek zůstane vidět, jen ho nepůjde ovládat myší.")}</p>
            <Show when={h().key && h().keyOk} fallback={<p class="warn">{t("notes.pin.hintNoKey", "Zkratka pro vypnutí teď nefunguje (vypnutá nebo obsazená jinou aplikací). Vypneš ho v Ledge: Ctrl+K → „Lístky: vypnout průchozí režim“.")}</p>}>
              <p>{t("notes.pin.hintKey", "Vypneš ho zkratkou {key} (platí pro všechny lístky), nebo v Ledge: Ctrl+K → „Lístky: vypnout průchozí režim“.", { key: h().key })}</p>
            </Show>
            <div class="pin-row">
              <button class="pin-btn pri" onClick={confirmThrough}>{t("notes.pin.hintOk", "Rozumím, zapnout")}</button>
              <button class="pin-btn" onClick={() => setHint(null)}>{t("notes.pin.hintCancel", "Zrušit")}</button>
            </div>
          </div>
        )}
      </Show>

      <Show when={through()}>
        <span class="pin-through">
          <IconThrough />
          {keyText() ? t("notes.pin.throughBadge", "průchozí · {key} vypne", { key: keyText() }) : t("notes.pin.throughBadgeNoKey", "průchozí · vypneš v Ledge (Ctrl+K)")}
        </span>
      </Show>
      <UndoBar pin />
      <Toast />
    </div>
  );
}
