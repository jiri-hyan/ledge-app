// Okno lístku na plochu (`pin.html`, label `note-<klíč>`): jedna existující poznámka Ledge přes ostatní
// okna. Obsah jen přes příkazy poznámek s otiskem (`note_read` / `note_save` / `note_toggle_task`),
// obrázek ze schránky stejnou cestou jako editor (`pasteImages` → `images` v `note_save` → PNG vedle
// poznámky). Změnu zvenku hlásí Rust událostí `pin-note` (otisk souboru), volby `pin-prefs`.
// Lišta: tah = posun (`pin_drag`), dvojklik = sbalit; ⋯ = volby (navrchu, zámek, průchozí,
// průhlednosti); ✕ = odepnout (poznámka zůstane). Průchozí lístek myš nevidí — vypíná ho zkratka
// (`hotkeys.notePassthrough`) nebo Ctrl+K v Ledge; nápověda se ukáže při prvním zapnutí.
import { createSignal, onCleanup, onMount, Show } from "solid-js";
import { api, on, type NoteDetail, type PinInit, type PinPrefs } from "../api";
import Markdown from "../notes/Markdown";
import { b64Of, pasteImages } from "../notes/draft";
import { fromMs } from "../notes/remind";
import { setLang, t } from "../i18n";
import { errorText, say, sayError, Toast } from "../ui/toast";
import { IconX } from "../icons";
import { IconEdit, IconLock, IconMore, IconPin, IconPlus, IconThrough } from "./icons";
import { appendLine, pinColors, taskLine, TEXT_MIN } from "./model";

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
  /** nápověda před prvním průchozím režimem (s čerstvým stavem zkratky) */
  const [hint, setHint] = createSignal<PinInit | null>(null);
  /** posuvníky průhledností: náhled během tahu, uloží se až po puštění */
  const [look, setLook] = createSignal<{ bg: number; text: number } | null>(null);
  let ta: HTMLTextAreaElement | undefined;
  let addInput: HTMLInputElement | undefined;
  /** běžící zápisy — událost `pin-note` z vlastního zápisu přijde dřív než odpověď */
  let saving = 0;
  let pendingHash: string | null = null;
  let editTimer: number | undefined;

  const p = () => prefs();
  const through = () => !!p()?.through;
  const dirty = () => editing() && text().trimEnd() !== (note()?.body ?? "").trimEnd();
  const colors = () => {
    const l = look();
    return pinColors(l?.bg ?? p()?.bg ?? 88, l?.text ?? p()?.text ?? 100);
  };

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
      setNote(n);
      setGone(null);
      if (editing() && !dirty()) setText(n.body);
    } catch (e) {
      setGone(errorText(e));
    }
  }

  /** Otisk souboru po přenačtení poznámek: jiný → přečíst znovu (rozepsaná úprava → banner). */
  function external(hash: string) {
    if (saving) {
      pendingHash = hash;
      return;
    }
    if (hash && hash === note()?.hash) return;
    if (!hash) return void setGone(t("notes.pin.gone", "Poznámka zmizela — je smazaná nebo přesunutá."));
    if (dirty()) return void setConflict(true);
    void load();
  }

  async function busy<T>(f: () => Promise<T>): Promise<T> {
    saving++;
    try {
      return await f();
    } finally {
      saving--;
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
      if (editing() && (body !== before || images.length) && text() === before) setText(r.note.body);
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
        if (r === "saved") return true;
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
        if (r?.outcome === "saved") return setNote(r.note);
        await load();
        say(t("notes.pin.reloaded", "Poznámka se mezitím změnila — načetl jsem novou verzi."), true);
      });
    } catch (e) {
      sayError(e);
    }
  }

  // ---------- úprava textu ----------
  function startEdit() {
    const n = note();
    if (!n || through()) return;
    setMenu(false);
    setText(n.body);
    setConflict(false);
    setEditing(true);
    queueMicrotask(() => ta?.focus());
  }
  async function flush(): Promise<boolean> {
    clearTimeout(editTimer);
    editTimer = undefined;
    if (!dirty()) return true;
    try {
      if ((await write(text())) === "saved") return true;
      setConflict(true);
    } catch (e) {
      sayError(e);
    }
    return false;
  }
  const schedule = () => {
    clearTimeout(editTimer);
    editTimer = window.setTimeout(() => void flush(), 900);
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
    const line = taskLine(addText());
    if (!line) return setAdding(false);
    if (await add(line)) setAddText("");
    addInput?.focus();
  }

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
  const unpin = () => api.pinUnpin().catch(sayError);

  onMount(() => {
    const offs = [on<string>("pin-note", external), on<PinPrefs>("pin-prefs", setPrefs)];
    onCleanup(() => offs.forEach((o) => o.then((f) => f())));
    const key = (e: KeyboardEvent) => {
      if (e.key !== "Escape") return;
      if (hint()) setHint(null);
      else if (menu()) setMenu(false);
      else if (adding()) setAdding(false);
      else if (editing()) void finishEdit();
    };
    window.addEventListener("keydown", key);
    document.addEventListener("paste", paste);
    onCleanup(() => {
      window.removeEventListener("keydown", key);
      document.removeEventListener("paste", paste);
      clearTimeout(editTimer);
    });
    void api.pinInit().then(
      async (i) => {
        if (!i) return;
        setLang(i.lang);
        setInit(i);
        setPrefs(i.prefs);
        await load();
        if (i.fresh) openAdd();
      },
      (e) => setGone(errorText(e)),
    );
  });

  const keyText = () => init()?.key || "";
  const title = () => note()?.title || (note() ? t("notes.imageOnly", "(jen obrázek)") : "…");

  return (
    <div
      class="pin"
      classList={{ through: through(), collapsed: !!p()?.collapsed, locked: !!p()?.locked }}
      style={{ "--pin-bg": colors().background, "--pin-bar": colors().bar, "--pin-fg": colors().color, "--pin-shadow": colors().shadow }}
    >
      <div class="pin-bar" onMouseDown={barDown} title={p()?.locked ? t("notes.pin.lockedTitle", "Zamčeno na místě") : t("notes.pin.barTitle", "Táhni = posunout · dvojklik = sbalit / rozbalit")}>
        <span class="pin-title">{title()}</span>
        <Show when={p()?.locked}><span class="pin-state" title={t("notes.pin.lockedTitle", "Zamčeno na místě")}><IconLock /></span></Show>
        <Show when={!through()}>
          <button class="pin-ic" classList={{ on: !!p()?.onTop }} aria-pressed={!!p()?.onTop} title={p()?.onTop ? t("notes.pin.onTopOn", "Vždy navrchu — klik: normální okno") : t("notes.pin.onTopOff", "Normální okno — klik: vždy navrchu")} onClick={() => patch({ onTop: !p()?.onTop })}><IconPin /></button>
          <button class="pin-ic" aria-haspopup="menu" aria-expanded={menu()} title={t("notes.pin.more", "Volby lístku")} onClick={() => setMenu(!menu())}><IconMore /></button>
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
                <div class="pin-md" onDblClick={(e) => !(e.target as HTMLElement).closest(".md-check, .md-link") && startEdit()}>
                  <Markdown body={n().body} images={n().imageData} onToggle={toggle} />
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
                  }}
                  onKeyDown={(e) => {
                    if (e.key === "Enter" && e.ctrlKey) {
                      e.preventDefault();
                      void finishEdit();
                    }
                  }}
                />
              </Show>
            )}
          </Show>
          <Show when={conflict()}>
            <div class="pin-banner">
              <span>{t("notes.pin.conflict", "Poznámka se mezitím změnila jinde. Tvoje úprava zůstala tady.")}</span>
              <button class="pin-btn" onClick={reloadDiscard}>{t("notes.pin.reload", "Načíst znovu")}</button>
              <button class="pin-btn pri" onClick={saveCopy}>{t("notes.pin.saveCopy", "Uložit jako kopii")}</button>
            </div>
          </Show>
        </div>

        <Show when={!through() && note() && !gone()}>
          <Show when={adding()} fallback={
            <div class="pin-foot">
              <button class="pin-btn" onClick={openAdd} title={t("notes.pin.addTitle", "Přidat bod checklistu (- [ ])")}><IconPlus /> {t("notes.pin.add", "bod")}</button>
              <Show when={editing()} fallback={<button class="pin-btn ghost" onClick={startEdit} title={t("notes.pin.editTitle", "Upravit text (nebo dvojklik do lístku)")}><IconEdit /></button>}>
                <button class="pin-btn pri" onClick={finishEdit}>{t("notes.pin.done", "Hotovo")}</button>
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
          <div class="pin-menu" role="menu">
            <button role="menuitemcheckbox" aria-checked={pr().onTop} classList={{ on: pr().onTop }} onClick={() => patch({ onTop: !pr().onTop })}>
              <span class="box" />{t("notes.pin.onTop", "Vždy navrchu")}
            </button>
            <button role="menuitemcheckbox" aria-checked={pr().locked} classList={{ on: pr().locked }} onClick={() => patch({ locked: !pr().locked })}>
              <span class="box" />{t("notes.pin.lock", "Zamknout na místě")}
            </button>
            <button role="menuitemcheckbox" aria-checked={false} onClick={wantThrough}>
              <span class="box" />{t("notes.pin.through", "Průchozí — myš projde na stránku pod lístkem")}
            </button>
            <label class="pin-range">
              <span>{t("notes.pin.bg", "Pozadí")} <b>{look()?.bg ?? pr().bg} %</b></span>
              <input type="range" min="0" max="100" step="5" value={pr().bg} onInput={(e) => setLook({ bg: +e.currentTarget.value, text: look()?.text ?? pr().text })} onChange={(e) => patch({ bg: +e.currentTarget.value }).then(() => setLook(null))} />
            </label>
            <label class="pin-range">
              <span>{t("notes.pin.text", "Text")} <b>{look()?.text ?? pr().text} %</b></span>
              <input type="range" min={TEXT_MIN} max="100" step="5" value={pr().text} onInput={(e) => setLook({ bg: look()?.bg ?? pr().bg, text: +e.currentTarget.value })} onChange={(e) => patch({ text: +e.currentTarget.value }).then(() => setLook(null))} />
            </label>
            <button role="menuitem" onClick={startEdit}>{t("notes.pin.editText", "Upravit text")}</button>
            <button role="menuitem" onClick={() => { setMenu(false); void patch({ collapsed: true }); }}>{t("notes.pin.collapse", "Sbalit na lištu (dvojklik na lištu)")}</button>
            <button role="menuitem" onClick={() => { setMenu(false); const n = note(); if (n) api.noteOpen(n.path).catch(sayError); }}>{t("notes.pin.openExternal", "Otevřít v editoru")}</button>
            <button role="menuitem" onClick={unpin}>{t("notes.pin.unpinLong", "Odepnout z plochy (poznámka zůstane)")}</button>
          </div>
        )}
      </Show>

      <Show when={!through() && hint()}>
        {(h) => (
          <div class="pin-menu pin-hint" role="dialog">
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
      <Toast />
    </div>
  );
}
