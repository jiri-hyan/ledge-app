// Hlavní stránka šuplíku: záložka Práce (dlaždice projektů + mřížka karet) nebo Osobní (hodiny,
// počasí, časovač (`timer/`), odkazy, právě hraje + hlasitost (`media/`), obecné poznámky) + Rychlé akce a Přihrádka na obou. Viditelnost karet řídí
// `Snapshot.sections` (`drawer.sections`, logika `cards/sections.ts` `visibleCards`) / `notesEnabled`,
// pořadí karet Práce `appearance.cards` (`cardOrder`, uložený vzhled z `theme/apply.ts`). Každá karta
// má v hlavičce ⋯ „Skrýt kartu“ (`cards/CardMenu.tsx`); skryté všechno = nápověda, jak karty vrátit.
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
import TimerCard from "./timer/TimerCard";
import WeatherCard from "./cards/WeatherCard";
import LinksCard from "./cards/LinksCard";
import MediaCard, { PersonalLive } from "./media/MediaCard";
import ActionsCard from "./cards/ActionsCard";
import ShelfCard, { shelfShown } from "./shelf/ShelfCard";
import BgLaunch from "./sessions/BgLaunch";
import CardMenu, { HiddenNotice } from "./cards/CardMenu";
import { CLAUDE_PARTS, gridClass, mediaCardKey, sectionOn, visibleCards, type SectionKey } from "./cards/sections";
import { visibleGroups } from "./actions/model";
import { gotoSettingsPanel } from "./palette/goto";
import { t } from "./i18n";

export default function MainPage(p: { s: Snapshot; now: number; onNote: (preset?: NotePreset) => void; nav: NotesNav; onSettings: () => void }) {
  const visible = () =>
    visibleCards({
      mode: p.s.mode === "osobni" ? "osobni" : "prace",
      sec: p.s.sections,
      procs: p.s.procs.length,
      notesEnabled: p.s.notesEnabled,
      actions: visibleGroups([...(p.s.actions ?? []), ...(p.s.scripts ?? [])], p.s.mode).reduce((n, g) => n + g.items.length, 0),
      shelf: shelfShown(),
      media: !!((sectionOn(p.s.sections, "nowPlaying") && p.s.personal.media) || (sectionOn(p.s.sections, "sound") && p.s.personal.volume)),
    });
  const shows = (k: SectionKey) => visible().includes(k);
  const on = (k: SectionKey) => sectionOn(p.s.sections, k);
  const menu = (k: SectionKey) => <CardMenu k={k} cfgError={p.s.cfgError} />;
  const claudeParts = () => CLAUDE_PARTS.filter(on);
  /** baterie v kartě Čas: jen stroj s baterií a zapnutý přepínač */
  const battery = () => (on("battery") ? p.s.personal.battery : null);
  const inbox = () => p.s.pending.length + p.s.questions.length > 0;
  /** nic na stránce (a nic nečeká) → nápověda místo prázdné plochy */
  const empty = () => visible().length === 0 && !inbox();
  const editCards = () => {
    p.onSettings();
    gotoSettingsPanel("suplik");
    requestAnimationFrame(() => requestAnimationFrame(() => document.getElementById("f-drawer.sections")?.scrollIntoView({ block: "start" })));
  };
  const card = (id: CardId): JSX.Element => {
    switch (id) {
      case "running":
        return <Show when={shows("running")}><Running s={p.s} now={p.now} menu={menu("running")} /></Show>;
      case "claude":
        return <Show when={shows("claude")}><ClaudeSessions s={p.s} now={p.now} onSettings={p.onSettings} menu={<CardMenu k="claude" parts={claudeParts()} cfgError={p.s.cfgError} />} /></Show>;
      case "repos":
        return <Show when={shows("repos")}><Repos s={p.s} menu={menu("repos")} /></Show>;
      case "notes":
        return <Show when={shows("notes")}><NotesCard s={p.s} now={p.now} onNew={p.nav.onNew} onOpen={p.nav.onOpen} onAll={() => p.nav.onAll()} menu={menu("notes")} /></Show>;
    }
  };
  return (
    <>
      <Show when={p.s.mode === "prace"} fallback={<>
        <ClaudeInbox s={p.s} />
        <PersonalLive />
        <div class="grid">
          <Show when={shows("clock")}><ClockCard now={p.now} battery={battery()} menu={<CardMenu k="clock" parts={battery() ? ["battery"] : []} cfgError={p.s.cfgError} />} /></Show>
          <Show when={shows("timer")}><TimerCard s={p.s} now={p.now} menu={menu("timer")} /></Show>
          <Show when={shows("weather")}><WeatherCard s={p.s} now={p.now} onSettings={p.onSettings} menu={menu("weather")} /></Show>
          <Show when={shows(mediaCardKey(p.s.sections))}>
            <MediaCard s={p.s} menu={<CardMenu k={mediaCardKey(p.s.sections)} parts={mediaCardKey(p.s.sections) === "nowPlaying" && on("sound") && p.s.personal.volume ? ["sound"] : []} cfgError={p.s.cfgError} />} />
          </Show>
          <Show when={shows("links")}><LinksCard s={p.s} onSettings={p.onSettings} menu={menu("links")} /></Show>
          <Show when={on("actions")}><ActionsCard s={p.s} onSettings={p.onSettings} wide menu={menu("actions")} /></Show>
          <Show when={on("shelf")}><ShelfCard wide menu={menu("shelf")} /></Show>
          <Show when={shows("notes")}><NotesCard s={p.s} now={p.now} onNew={p.nav.onNew} onOpen={p.nav.onOpen} onAll={() => p.nav.onAll(true)} personal menu={menu("notes")} /></Show>
        </div>
      </>}>
        <Show when={!on("claude")}><ClaudeInbox s={p.s} /></Show>
        <BgLaunch s={p.s} />
        <Show when={on("projects")}>
          <Projects s={p.s} onNote={p.onNote} />
        </Show>
        <Show when={on("actions")}><ActionsCard s={p.s} onSettings={p.onSettings} menu={menu("actions")} /></Show>
        <Show when={on("shelf")}><ShelfCard menu={menu("shelf")} /></Show>
        <div class="grid" classList={{ one: gridClass(visible()) === "one", two: gridClass(visible()) === "two" }}>
          <For each={cardOrder(savedAppearance().cards)}>{card}</For>
        </div>
      </Show>
      <Show when={empty()}>
        <div class="empty page-empty" data-anim>
          {t("cards.hide.allHidden", "Na téhle stránce jsou všechny karty skryté.")}
          <button class="mini" onClick={editCards}>{t("cards.hide.restore", "Vrátit karty v Nastavení")}</button>
        </div>
      </Show>
      <HiddenNotice cfgError={p.s.cfgError} />
    </>
  );
}
