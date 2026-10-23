// Hlavní stránka šuplíku: karty záložky, na které šuplík je (`Snapshot.mode` = id z `layout.pages`).
// Co je na které záložce, v jakém pořadí a jak velké, řídí JEN `layout` (`layout/model.ts`); vykreslí
// to `layout/Deck.tsx` (`.deck` > `.deck-item[data-size][data-card]`). Karty bez dat se schovají samy
// (`shownCards`), v režimu úprav (`layout/edit.ts`, ✎ v hlavičce) jsou vidět jako zástupce. Každá karta
// má v hlavičce ⋯ (`cards/CardMenu.tsx`: velikost, výška, skrýt kartu / sekci, upravit šuplík; u Projektů
// i volby dlaždic `cards/TileOptions.tsx`); prázdná záložka = výzva „Přidej první kartu“ (katalog).
// Při prvním otevření (i po aktualizaci) jednou krátká prohlídka 3 bublin (`layout/Tour.tsx`, `ui.tips`).
// Vlna 8 P: vzhled karty (⋯ → „Vzhled karty…“, panel `layout/CardLook.tsx` pod mřížkou, varianty zobrazení
// karet `viewOf`) a vzhled zástupců (pravý klik → `tilelook/TileMenu.tsx`).
// Dynamické karty (widgety ze skriptů `widget:<soubor>`) kreslí `widgets/WidgetCard.tsx` z `Snapshot.widgets`.
import { createEffect, createMemo, on as onChange, onCleanup, Show, type JSX } from "solid-js";
import type { Snapshot } from "./api";
import type { NotesNav } from "./nav";
import ClaudeSessions, { ClaudeInbox } from "./ClaudeSessions";
import { NotesCard } from "./notes/NotesCard";
import { pinMenuItems } from "./notes/pins";
import type { NotePreset } from "./notes/store";
import Projects from "./cards/Projects";
import Running from "./cards/Running";
import Repos from "./cards/Repos";
import ClockCard from "./cards/ClockCard";
import TimerCard from "./timer/TimerCard";
import WeatherCard from "./cards/WeatherCard";
import CalendarCard from "./calendar/CalendarCard";
import MailCard from "./accounts/MailCard";
import LinksCard from "./cards/LinksCard";
import MediaCard, { PersonalLive } from "./media/MediaCard";
import ActionsCard from "./cards/ActionsCard";
import GithubCard from "./integrations/GithubCard";
import SystemCard from "./integrations/SystemCard";
import ShelfCard, { shelfShown } from "./shelf/ShelfCard";
import BgLaunch from "./sessions/BgLaunch";
import CardMenu, { HiddenNotice } from "./cards/CardMenu";
import TileOptions from "./cards/TileOptions";
import Tour from "./layout/Tour";
import HoldTip from "./layout/HoldTip";
import CardLookPanel, { closeCardLook, lookFor } from "./layout/CardLook";
import { viewOf } from "./layout/look";
import TileMenu from "./tilelook/TileMenu";
import { closeLook } from "./tilelook/state";
import { setActivePage } from "./theme/apply";
import { CLAUDE_PARTS, sectionOn } from "./cards/sections";
import { isLink, visibleGroups } from "./actions/model";
import { lastResult } from "./actions/api";
import Deck from "./layout/Deck";
import Catalog from "./layout/Catalog";
import EditBar from "./layout/EditBar";
import { beginEdit, catalogOpen, draft, editing, editPage, saveLayout, setCatalogOpen } from "./layout/edit";
import { LIMIT_CARDS, limitOf, normalizeLayout, pageHas, sameLayout, pageKind, pageOf, setTiles, shownCards, tilesOf, type CardId, type CardSize, type PartKey } from "./layout/model";
import { sizeOutcome } from "./layout/size";
import { colsNow } from "./layout/cols";
import { t } from "./i18n";
import WidgetCard from "./widgets/WidgetCard";
import { rememberWidgets } from "./widgets/registry";
import SuggestBar from "./suggest/SuggestBar";

export default function MainPage(p: { s: Snapshot; now: number; onNote: (preset?: NotePreset) => void; nav: NotesNav; onSettings: () => void }) {
  // stejné rozložení = stejný objekt (nový snapshot karty nepřestaví)
  const saved = createMemo(() => normalizeLayout(p.s.layout), undefined, { equals: sameLayout });
  const layout = () => draft() ?? saved();
  const page = () => pageOf(layout(), editing() ? editPage() : p.s.mode);
  const kind = () => pageKind(page().id);
  const on = (k: Parameters<typeof sectionOn>[1]) => sectionOn(p.s.sections, k);
  const facts = () => ({
    procs: p.s.procs.length,
    notesEnabled: p.s.notesEnabled,
    // odkazy (akce, které jen otevírají) ukazuje na záložce s kartou Odkazy jen ona
    actions: visibleGroups([...(p.s.actions ?? []).filter((a) => !(has("links") && isLink(a))), ...(p.s.scripts ?? [])], page().id).reduce((n, g) => n + g.items.length, 0),
    scriptOut: !!lastResult()?.id.startsWith("script:"),
    shelf: shelfShown(),
    media: !!((on("nowPlaying") && p.s.personal.media) || (on("sound") && p.s.personal.volume) || (on("favorites") && p.s.personal.mediaLook?.favorites?.length)),
  });
  const shown = () => shownCards(page(), facts()).map((c) => c.id);
  const shows = (id: CardId) => shown().includes(id);
  const edit = (catalog = false) => beginEdit(p.s, catalog);
  const cardOf = (id: CardId) => page().cards.find((c) => c.id === id);
  /** co udělá volba velikosti v ⋯ (karty v pořadí, jak je mřížka ukazuje; `size.ts` `sizeOutcome`) */
  const outcome = (id: CardId) => (want: CardSize) => {
    const cards = editing() ? page().cards : page().cards.filter((c) => shows(c.id));
    return sizeOutcome(cards.map((c) => c.size), cards.findIndex((c) => c.id === id), want, colsNow());
  };
  const tiles = () => tilesOf(cardOf("projects"));
  /** varianta zobrazení karty (vzhled karty; i rozpracovaný v panelu) */
  const view = (id: CardId) => viewOf(id, lookFor(page().id, id, cardOf(id)?.look));
  // vzhled záložky (`appearance.pages`, `theme/pages.ts`): platí, dokud je hlavní stránka vidět
  createEffect(() => setActivePage(page().id));
  onCleanup(() => setActivePage(""));
  // jiná záložka / konec úprav: otevřené panely vzhledu se uloží a zavřou
  createEffect(onChange([() => page().id, editing], () => {
    void closeLook(true, editing());
    void closeCardLook(true);
  }, { defer: true }));
  const menu = (card: CardId, parts: PartKey[] = []) => (
    <CardMenu card={card} page={page().id} parts={parts} size={cardOf(card)?.size} h={cardOf(card)?.h} outcome={outcome(card)} limit={LIMIT_CARDS.includes(card) ? limitOf(cardOf(card)) : undefined} look={cardOf(card)?.look} cfgError={p.s.cfgError} onEdit={() => edit()} extra={card === "notes" ? pinMenuItems(p.s) : undefined}>
      <Show when={card === "projects"}>
        <TileOptions page={page().id} tiles={tiles()} projects={p.s.projects} cfgError={p.s.cfgError} />
      </Show>
    </CardMenu>
  );
  /** baterie v kartě Čas: jen stroj s baterií a zapnutá část */
  const battery = () => (on("battery") ? p.s.personal.battery : null);
  /** části karty Právě hraje, které jdou v ⋯ skrýt (hlasitost, oblíbené) — vždy zbyde jiná část */
  const mediaParts = (): PartKey[] => [
    ...(on("nowPlaying") && on("sound") && p.s.personal.volume ? (["sound"] as const) : []),
    ...(on("favorites") && (on("nowPlaying") || on("sound")) ? (["favorites"] as const) : []),
  ];
  const inbox = () => p.s.pending.length + p.s.questions.length > 0;
  const has = (id: CardId) => pageHas(page(), id);
  // názvy umístěných widgetů pro katalog a „karta odebrána“
  createEffect(() => rememberWidgets(p.s.widgets));

  const card = (id: CardId): JSX.Element => {
    switch (id) {
      case "projects":
        return <Projects s={p.s} page={page().id} view={view("projects")} onNote={p.onNote} onSettings={p.onSettings} tiles={tiles()} menu={menu("projects")} onShowAll={() => void saveLayout((l) => setTiles(l, page().id, { hidden: [] }), p.s.cfgError)} />;
      case "running":
        return <Running s={p.s} now={p.now} menu={menu("running")} page={page().id} limit={limitOf(cardOf("running"))} />;
      case "claude":
        return <ClaudeSessions s={p.s} now={p.now} onSettings={p.onSettings} menu={menu("claude", CLAUDE_PARTS.filter(on))} page={page().id} limit={limitOf(cardOf("claude"))} />;
      case "repos":
        return <Repos s={p.s} view={view("repos")} menu={menu("repos")} onSettings={p.onSettings} page={page().id} limit={limitOf(cardOf("repos"))} />;
      case "notes":
        // Osobní = obecné poznámky; Práce a vlastní záložky = po projektech (reaktivně — karta zůstává při přepnutí záložky)
        return <Show when={kind() === "personal"} fallback={<NotesCard s={p.s} now={p.now} view={view("notes")} onNew={p.nav.onNew} onOpen={p.nav.onOpen} onAll={() => p.nav.onAll()} menu={menu("notes")} />}><NotesCard s={p.s} now={p.now} view={view("notes")} onNew={p.nav.onNew} onOpen={p.nav.onOpen} onAll={() => p.nav.onAll(true)} personal menu={menu("notes")} /></Show>;
      case "actions":
        return <ActionsCard s={p.s} view={view("actions")} onSettings={p.onSettings} wide skipLinks={has("links")} menu={menu("actions")} />;
      case "shelf":
        return <ShelfCard wide menu={menu("shelf")} />;
      case "clock":
        return <ClockCard now={p.now} battery={battery()} hideBattery={!on("battery")} menu={menu("clock", battery() ? ["battery"] : [])} />;
      case "timer":
        return <TimerCard s={p.s} now={p.now} menu={menu("timer")} />;
      case "weather":
        return <WeatherCard s={p.s} now={p.now} onSettings={p.onSettings} menu={menu("weather")} />;
      case "calendar":
        return <CalendarCard s={p.s} now={p.now} onSettings={p.onSettings} menu={menu("calendar")} />;
      case "links":
        return <LinksCard s={p.s} mode={page().id} onSettings={p.onSettings} menu={menu("links")} />;
      case "mail":
        return <MailCard s={p.s} now={p.now} onSettings={p.onSettings} menu={menu("mail")} />;
      case "media":
        return <MediaCard s={p.s} menu={menu("media", mediaParts())} />;
      case "github":
        return <GithubCard now={p.now} cfgError={p.s.cfgError} menu={menu("github")} />;
      case "system":
        return <SystemCard menu={menu("system")} />;
      default:
        // widget ze skriptu (`widget:<soubor>`)
        return <WidgetCard s={p.s} id={id} now={p.now} menu={(extra) => <CardMenu card={id} page={page().id} size={cardOf(id)?.size} h={cardOf(id)?.h} outcome={outcome(id)} look={cardOf(id)?.look} cfgError={p.s.cfgError} onEdit={() => edit()} extra={extra} />} />;
    }
  };

  return (
    <>
      <Show when={editing()}><EditBar /></Show>
      <Show when={!has("claude")}><ClaudeInbox s={p.s} /></Show>
      <BgLaunch s={p.s} />
      {/* odběry Právě hraje / hlasitost / baterie startují líně, jen když je karta na otevřené záložce */}
      <Show when={has("media") || has("clock")}><PersonalLive /></Show>
      {/* kontextové návrhy (max 3 chipy) — jen z reálných událostí, `src/suggest/` */}
      <Show when={!editing()}><SuggestBar s={p.s} now={p.now} nav={p.nav} /></Show>
      <Deck page={page()} layout={layout()} editing={editing()} shown={shows} render={card} begin={() => edit()} />
      <CardLookPanel page={page().id} s={p.s} />
      <TileMenu />
      <Show when={editing() && catalogOpen()}><Catalog layout={layout()} /></Show>
      <Show when={!editing() && page().cards.length === 0}>
        <div class="empty page-empty" data-anim>
          {t("cards.layout.emptyTab", "Na téhle záložce zatím nic není.")}
          <button class="mini" onClick={() => edit(true)}>{t("cards.layout.firstCard", "Přidej první kartu")}</button>
        </div>
      </Show>
      <Show when={!editing() && page().cards.length > 0 && shown().length === 0 && !inbox()}>
        <div class="empty page-empty" data-anim>
          {t("cards.layout.nothingNow", "Karty na téhle záložce teď nemají co ukázat.")}
          <button class="mini" onClick={() => edit(true)}>{t("cards.layout.editDrawer", "Upravit šuplík")}</button>
        </div>
      </Show>
      <Show when={editing() && page().cards.length === 0 && !catalogOpen()}>
        <div class="empty page-empty">
          {t("cards.layout.emptyTabEdit2", "Prázdná záložka — přetáhni sem kartu z jiné záložky (pusť ji na ouško této záložky) nebo ji přidej z katalogu.")}
          <button class="mini" onClick={() => setCatalogOpen(true)}>{t("cards.layout.openCatalog", "Otevřít katalog")}</button>
        </div>
      </Show>
      <HiddenNotice cfgError={p.s.cfgError} />
      <Show when={!editing()}><Tour cfgError={p.s.cfgError} /></Show>
      <HoldTip cfgError={p.s.cfgError} />
    </>
  );
}
