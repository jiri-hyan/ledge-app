// Hlavní stránka šuplíku: záložka Práce (dlaždice projektů + mřížka karet) nebo Osobní (hodiny,
// počasí, odkazy, obecné poznámky) + Rychlé akce na obou (`showOn`). Viditelnost karet řídí
// `Snapshot.sections` / `notesEnabled`,
// pořadí karet Práce `appearance.cards` (`cardOrder`, uložený vzhled z `theme/apply.ts`).
import { For, Show, type JSX } from "solid-js";
import { cardOrder, type CardId } from "./theme/appearance";
import { savedAppearance } from "./theme/apply";
import type { Snapshot } from "./api";
import type { NotesNav } from "./nav";
import ClaudeSessions, { ClaudeInbox } from "./ClaudeSessions";
import { NotesCard } from "./notes/NotesCard";
import type { NotePreset } from "./notes/store";
import Projects from "./cards/Projects";
import Running from "./cards/Running";
import Repos from "./cards/Repos";
import ClockCard from "./cards/ClockCard";
import WeatherCard from "./cards/WeatherCard";
import LinksCard from "./cards/LinksCard";
import ActionsCard from "./cards/ActionsCard";
import ShelfCard from "./shelf/ShelfCard";
import BgLaunch from "./sessions/BgLaunch";

export default function MainPage(p: { s: Snapshot; now: number; onNote: (preset?: NotePreset) => void; nav: NotesNav; onSettings: () => void }) {
  // prázdná karta Běží jen zabírá sloupec — ukáže se, až Ledge něco spustí (dlaždice stav ukazují samy)
  const running = () => p.s.sections.running && p.s.procs.length > 0;
  const cards = () => [running(), p.s.sections.claude, true].filter(Boolean).length;
  const notes = () => p.s.notesEnabled && p.s.sections.notes;
  const card = (id: CardId): JSX.Element => {
    switch (id) {
      case "running":
        return <Show when={running()}><Running s={p.s} now={p.now} /></Show>;
      case "claude":
        return <Show when={p.s.sections.claude}><ClaudeSessions s={p.s} now={p.now} onSettings={p.onSettings} /></Show>;
      case "repos":
        return <Repos s={p.s} />;
      case "notes":
        return <Show when={notes()}><NotesCard s={p.s} now={p.now} onNew={p.nav.onNew} onOpen={p.nav.onOpen} onAll={() => p.nav.onAll()} /></Show>;
    }
  };
  return (
    <Show when={p.s.mode === "prace"} fallback={<>
      <ClaudeInbox s={p.s} />
      <div class="grid">
        <ClockCard now={p.now} />
        <WeatherCard s={p.s} now={p.now} onSettings={p.onSettings} />
        <LinksCard s={p.s} onSettings={p.onSettings} />
        <ActionsCard s={p.s} onSettings={p.onSettings} wide />
        <ShelfCard wide />
        <Show when={notes()}><NotesCard s={p.s} now={p.now} onNew={p.nav.onNew} onOpen={p.nav.onOpen} onAll={() => p.nav.onAll(true)} personal /></Show>
      </div>
    </>}>
      <Show when={!p.s.sections.claude}><ClaudeInbox s={p.s} /></Show>
      <BgLaunch s={p.s} />
      <Show when={p.s.sections.projects}>
        <Projects s={p.s} onNote={p.onNote} />
      </Show>
      <ActionsCard s={p.s} onSettings={p.onSettings} />
      <ShelfCard />
      <div class={`grid ${cards() === 2 ? "two" : ""}`}>
        <For each={cardOrder(savedAppearance().cards)}>{card}</For>
      </div>
    </Show>
  );
}
