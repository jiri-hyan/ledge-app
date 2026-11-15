// Rychlá poznámka v malém okně (dock): jedno pole + chip času. Sdílí koncept `D.draft` s plným editorem
// (žádná divergence, rozepsané přežije pád). Psaní začíná výslovným klikem (`onFocusRequest` → hostitel zavolá
// `focus_drawer`) — najetí fokus nebere. Esc / blur / odchod = uložit, když je text (prázdné jen zavřít);
// Ctrl+Enter = uložit (při `pinned()` pole vyprázdnit a zůstat, jinak zavřít). Hostitel (`onDone`) okno zavře.

import { createEffect, createSignal, on as onChange, onCleanup, onMount, Show } from "solid-js";
import { IconX } from "../icons";
import { api } from "../api";
import { sayError } from "../ui/toast";
import { t } from "../i18n";
import * as D from "./draft";
import { listKey } from "./listkeys";
import { blurPlan, DICTATE_BLUR_GRACE_MS, quickKeyPlan } from "./quick";
import { draftHasContent, saveDraft } from "./quicksave";
import RemindSuggest from "./RemindSuggest";
import { quickChip, quickTaskLine, worthParsing } from "./quicktask";
import type { QuickTask } from "../types/notes";

/** Prodleva před dotazem parseru při psaní (ms) — jen jednořádkový text, nic se nezapisuje. */
const PARSE_DELAY = 250;

export default function QuickNote(p: { pinned: () => boolean; onDone: (saved: boolean) => void; onFocusRequest: () => void }) {
  let root!: HTMLDivElement;
  let ta: HTMLTextAreaElement | undefined;
  const [ready, setReady] = createSignal(false);
  let busy = false;
  let finished = false;
  /** do kdy (ms) diktát Win+H omlouvá blur okna (`blurPlan`) */
  let dictateUntil = 0;

  onMount(async () => {
    await D.restoreDraft();
    setReady(true);
  });
  createEffect(onChange([D.draft, D.draftRemind], () => ready() && D.scheduleDraft(), { defer: true }));

  // úkol z věty („v 9:00 doktor !“): náhled chipu pod polem a při uložení řádek `- [ ] … ⏰ … !` místo prostého textu
  const [parsed, setParsed] = createSignal<{ text: string; q: QuickTask } | null>(null);
  let parseTimer: number | undefined;
  createEffect(onChange(D.draft, (text) => {
    clearTimeout(parseTimer);
    if (!worthParsing(text)) return setParsed(null);
    parseTimer = window.setTimeout(async () => {
      const q = await api.noteQuickParse(text).catch(() => undefined);
      if (D.draft() === text) setParsed(q ? { text, q } : null);
    }, PARSE_DELAY);
  }, { defer: true }));
  onCleanup(() => clearTimeout(parseTimer));
  const chip = () => (parsed() && parsed()!.text === D.draft() ? quickChip(D.draft(), parsed()!.q) : null);
  /** Před uložením: vypadá-li text jako úkol, nahradí koncept řádkem úkolu (termín `⏰` nahrazuje připomínku poznámky). */
  async function toTask() {
    const text = D.draft();
    if (!worthParsing(text)) return;
    const q = parsed()?.text === text ? parsed()!.q : await api.noteQuickParse(text).catch(() => undefined);
    const line = quickTaskLine(text, q);
    if (!line || D.draft() !== text) return;
    D.setDraft(line);
    D.setDraftRemind("");
  }

  /** Uloží (je-li co) a podle `close` zavře, nebo vyprázdní pole a nechá psát dál. Chyba = nic se nezavře, koncept zůstane. */
  async function commit(close: boolean) {
    if (busy || finished || !ready()) return;
    busy = true;
    let saved = false;
    try {
      await toTask();
      saved = !!(await saveDraft());
    } catch (e) {
      sayError(e);
      return;
    } finally {
      busy = false;
    }
    if (close) {
      finished = true;
      p.onDone(saved);
    } else queueMicrotask(() => ta?.focus());
  }

  const focus = () => !root?.contains(document.activeElement) && ta?.focus();
  const onBlur = () => {
    const plan = blurPlan({ finished, busy, pinned: p.pinned(), dictateUntil, now: Date.now() });
    if (plan === "flush") D.flushDraft(); // připnuté zůstává; koncept je jen jistota na disku
    else if (plan === "commit") void commit(true);
  };
  onMount(() => {
    window.addEventListener("blur", onBlur);
    window.addEventListener("focus", focus);
  });
  onCleanup(() => {
    window.removeEventListener("blur", onBlur);
    window.removeEventListener("focus", focus);
    D.flushDraft();
    // odchod bez Esc (okno zmizelo): s textem uložit; koncept na disku zůstává pojistkou, kdyby zápis nestihl doběhnout
    if (!finished && !busy && !p.pinned() && draftHasContent()) void saveDraft().catch(() => {});
  });

  /** 🎤: výslovný klik → fokus do pole a Win+H (klávesy pošle Rust jen do vlastního okna s fokusem v poli). */
  async function dictate() {
    ta?.focus();
    const go = async () => {
      const ok = await api.dictateStart(document.activeElement === ta);
      if (ok) dictateUntil = Date.now() + DICTATE_BLUR_GRACE_MS; // panel Windows může okno deaktivovat — blur diktát neukončí
      return ok;
    };
    try {
      // fokus okna se po kliku přepíná asynchronně (`onFocusRequest`) — když Rust řekne „nic“, jednou zkusit znovu
      if (!(await go())) {
        await new Promise((r) => setTimeout(r, 150));
        ta?.focus();
        await go();
      }
    } catch (e) {
      sayError(e);
    }
  }

  function key(e: KeyboardEvent & { currentTarget: HTMLTextAreaElement }) {
    const plan = quickKeyPlan({ key: e.key, ctrl: e.ctrlKey, hasText: draftHasContent() });
    if (plan === "none") return void listKey(e, e.currentTarget, D.setDraft);
    e.preventDefault();
    void commit(plan === "save-close" || !p.pinned());
  }

  return (
    <div class="note-quick" ref={root} onMouseDown={() => p.onFocusRequest()}>
      <textarea
        ref={(el) => (ta = el)}
        class="note-text note-quick-ta"
        rows={2}
        placeholder={t("notes.quick.placeholder", "Klikni a piš…")}
        value={D.draft()}
        onInput={(e) => D.setDraft(e.currentTarget.value)}
        onKeyDown={key}
      />
      <div class="note-quick-bar">
        <Show when={chip()}>
          {(c) => (
            <div class="note-sug">
              <span class="note-sug-go" title={t("notes.quick.taskTitle", "Uloží se jako úkol (- [ ]) — uvidíš ho v záložce Úkoly")}>
                {t("notes.quick.task", "Úkol")}{c().when ? ` · ⏰ ${c().when}` : ""}{c().prio ? ` · ${"!".repeat(c().prio)}` : ""}
              </span>
            </div>
          )}
        </Show>
        <Show when={!chip()}>
          <RemindSuggest text={D.draft} remind={D.draftRemind} onPick={D.setDraftRemind} />
        </Show>
        <Show when={D.draftRemind()}>
          <div class="note-sug">
            <span class="note-sug-go">⏰ {D.draftRemind().replace("T", " ")}</span>
            <button class="note-sug-x" aria-label={t("notes.quick.remindClear", "Zrušit připomínku")} title={t("notes.quick.remindClear", "Zrušit připomínku")} onClick={() => D.setDraftRemind("")}>
              <IconX />
            </button>
          </div>
        </Show>
        <button
          class="mini note-quick-mic"
          aria-label={t("notes.quick.dictate", "Hlasové zadávání Windows (Win+H)")}
          title={t("notes.quick.dictate", "Hlasové zadávání Windows (Win+H)")}
          onMouseDown={(e) => e.preventDefault()}
          onClick={dictate}
        >
          🎤
        </button>
        <span class="note-quick-hint">{p.pinned() ? t("notes.quick.hintPinned", "Ctrl+Enter uloží a pokračuj · Esc uloží a zavře") : t("notes.quick.hint", "Esc uloží a zavře")}</span>
      </div>
    </div>
  );
}
