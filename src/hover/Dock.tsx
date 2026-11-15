// Dok po najetí (`drawer.hover: "dock"`; Rust `View::Island`, 640×64): řada slotů [📝 Poznámka] [♪ hudba s hlasitostí]
// [⏱ časovač, když je vidět] [čas] 📌. Najetí = jen náhled; fokus jen klikem do slotu (`QuickNote` → `focusDrawer`)
// nebo zkratkou (událost `dock-note`). Schválení a otázka (čekají na člověka) přebírají celé okno jako v ostrůvku
// (sdílená komponenta `Island`); „hotovo“ je jen slot v řadě [✓ jméno · Otevřít · ✕], potvrzení v `doneAck.ts` (L146). 📌 = `PinButton` (`api.drawerPin`): okno neodjede, blur ho nezavře, odpočet rozbalení stojí.
// Sloty a seek / hlasitost jsou čistá logika v `model.ts`; odpočet `useExpand` nad sloty stojí (`data-noexpand`).
import { createEffect, createSignal, on, onMount, Show } from "solid-js";
import { api, type Snapshot } from "../api";
import type { SessionView } from "../types";
import { fmtClock } from "../format";
import { elapsedMs, fmtTimer, leftMs } from "../timer/model";
import { timerDo } from "../timer/TimerCard";
import QuickNote from "../notes/QuickNote";
import { sayError, say } from "../ui/toast";
import { t } from "../i18n";
import DockMusic from "./DockMusic";
import Island from "./Island";
import { dockSlots, islandEvent } from "./model";
import { dockNoteWanted, dockPinned, wantDockNote } from "./dockmode";
import PinButton from "./PinButton";
import { useExpand } from "./expand";
import { ackDoneSession, doneAcked } from "./doneAck";
import { sessionLabel } from "./parts";
import "../media/media.css";
import "./hover.css";

export default function Dock(p: { s: Snapshot; now: number; expandMs: number }) {
  const ev = () => islandEvent(p.s, p.now, doneAcked());
  // schválení / otázka = celé okno jako v ostrůvku (hotovo je slot v řadě)
  const takeover = () => ["approval", "question"].includes(ev().kind);
  return (
    <Show when={!takeover()} fallback={<Island s={p.s} now={p.now} expandMs={p.expandMs} full />}>
      <DockRow s={p.s} now={p.now} expandMs={p.expandMs} />
    </Show>
  );
}

function DockRow(p: { s: Snapshot; now: number; expandMs: number }) {
  const slots = () => dockSlots(p.s, p.now, doneAcked());
  const [noteOpen, setNoteOpen] = createSignal(false);
  const ex = useExpand(() => p.expandMs, () => noteOpen(), dockPinned);
  let row!: HTMLDivElement;

  function openNote() {
    setNoteOpen(true);
    void api.focusDrawer()?.catch(() => {});
    // okno už může být aktivní (upevněné) → událost `focus` nepřijde, pole zaměřit i tady
    requestAnimationFrame(() => row?.querySelector("textarea")?.focus());
  }
  // zkratka Ctrl+Alt+N: jádro dok otevřelo s Opener::Hotkey (upevnění tím padlo) a vzalo fokus
  const wantNote = () => {
    if (!dockNoteWanted()) return;
    wantDockNote(false); // upevnění případně zrušilo `view` před událostí (dockmode.ts), na upevněném doku zůstává
    openNote();
  };
  onMount(wantNote);
  createEffect(on(dockNoteWanted, wantNote, { defer: true }));

  async function openDone(ss: SessionView) {
    ackDoneSession(ss, p.now);
    if (!ss.canOpen) return say(ss.openHint, true);
    try {
      await api.sessionOpen(ss.id);
    } catch (e) {
      sayError(e);
    }
  }
  const timer = () => {
    const what = slots().timer;
    const tm = p.s.timer;
    if (what === "countdown" && tm?.countdown) {
      const c = tm.countdown;
      return {
        text: c.state === "done" ? t("cards.timer.done", "Čas vypršel") : fmtTimer(leftMs(c, p.now), true),
        act: () => timerDo({ op: c.state === "running" ? "pause" : c.state === "paused" ? "resume" : "cancel" }),
        icon: c.state === "running" ? "⏸" : c.state === "paused" ? "▶" : "✕",
        label: c.state === "running" ? t("cards.timer.pause", "Pauza") : c.state === "paused" ? t("cards.timer.resume", "Pokračovat") : t("cards.timer.cancel", "Zrušit"),
      };
    }
    if (what === "stopwatch" && tm?.stopwatch) return { text: fmtTimer(elapsedMs(tm.stopwatch, p.now)), act: () => timerDo({ op: "swPause" }), icon: "⏸", label: t("cards.timer.pause", "Pauza") };
    return null;
  };

  return (
    <div class="isl isl-dock" ref={row} {...ex}>
      <Show
        when={noteOpen()}
        fallback={
          <button class="isl-slot isl-note-btn" data-noexpand onClick={openNote} title={t("cards.hover.dock.noteHint", "Klikni a piš · Ctrl+Alt+N odkudkoli")}>
            <span aria-hidden="true">📝</span> <span>{t("cards.hover.dock.note", "Poznámka")}</span>
          </button>
        }
      >
        <div class="isl-slot isl-note" data-noexpand>
          <QuickNote
            pinned={dockPinned}
            onFocusRequest={() => void api.focusDrawer()?.catch(() => {})}
            onDone={(saved) => {
              setNoteOpen(false);
              if (saved) say(t("cards.hover.dock.saved", "Poznámka uložena"));
              if (!dockPinned()) void api.close();
            }}
          />
        </div>
      </Show>
      <Show when={!noteOpen()}>
        <Show when={slots().music}>{(m) => <DockMusic m={m().media} vol={m().volume} />}</Show>
        <Show when={timer()}>
          {(tm) => (
            <span class="isl-slot isl-timer" data-noexpand>
              <span aria-hidden="true">⏱</span> <b>{tm().text}</b>
              <button class="mini" title={tm().label} aria-label={tm().label} onClick={() => void tm().act()}>{tm().icon}</button>
            </span>
          )}
        </Show>
        <Show when={slots().done}>
          {(ss) => (
            <span class="isl-slot isl-timer" data-noexpand>
              <span class="isl-ico ok" aria-hidden="true">✓</span> <b>{sessionLabel(ss())}</b>
              <Show when={ss().canOpen}>
                <button class="btn pri" onClick={() => void openDone(ss())}>{t("cards.hover.island.open", "Otevřít")}</button>
              </Show>
              <button class="mini" title={t("cards.timer.dismiss", "Zavřít")} aria-label={t("cards.timer.dismiss", "Zavřít")} onClick={() => ackDoneSession(ss(), p.now)}>✕</button>
            </span>
          )}
        </Show>
        <span class="isl-chip">{fmtClock(new Date(p.now))}</span>
      </Show>
      <PinButton />
    </div>
  );
}
