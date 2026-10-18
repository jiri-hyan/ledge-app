// Editor rozložení v Nastavení (panel Šuplík a karty): stejný model jako režim úprav v šuplíku —
// záložky (pořadí tažením, název, ikona, smazat), karty na nich (pořadí, velikost, přesun na jinou
// záložku, odebrat, přidat z katalogu), výška karty, volby karty Projekty (zástupci: velikost dlaždic,
// název, git, které projekty na téhle záložce) a skryté části karet. Mění jen draft Nastavení (`c.set("layout")`),
// zapíše společná lišta Uložit. Velké tlačítko „Upravit přímo v šuplíku“ = režim úprav (`askEdit`).
// Pod tím přehled „Upravené karty a zástupci“ (vzhled karet a dlaždic, vlna 8 P — `LooksOverview.tsx`).
import { createMemo, For, onMount, Show } from "solid-js";
import { t } from "../i18n";
import { base, offerUndo, useSettings } from "../settings/ctx";
import { Section } from "../settings/fields/Row";
import { SortList } from "../settings/fields/SortList";
import { askEdit } from "./edit";
import {
  addCard, addPage, catalog, CARD_INFO, HEIGHTS, isDynamic, MAX_PAGES, moveCard, movePage, moveToPage, normalizeLayout, PAGE_ICONS, PART_KEYS, PARTS, partHidden,
  removeCard, removePage, renamePage, restoreCard, restorePage, sameLayout, setHeight, setPartHidden, setSize, setTiles, showProject, SIZES, tilesOf, TILE_SIZES,
  type CardHeight, type CardId, type CardSize, type Layout, type LayoutCard, type PartKey, type TileSize,
} from "./model";
import { heightLabel } from "./Deck";
import { tileSizeName } from "../cards/TileOptions";
import { catalogWidgets } from "../widgets/model";
import { loadWidgets, widgetsListing } from "../widgets/registry";
import { cardDesc, cardName, pageName, partName } from "./text";
import LooksOverview from "./LooksOverview";
import "./layout.css";

const sizeName = (s: CardSize) =>
  ({ s: t("cards.layout.size.s", "Malá"), m: t("cards.layout.size.m", "Střední"), l: t("cards.layout.size.l", "Velká"), xl: t("cards.layout.size.xl", "Přes celou šířku") })[s];

export default function LayoutEditor() {
  const c = useSettings();
  const L = createMemo(() => normalizeLayout(c.cfg.layout));
  const put = (l: Layout) => c.set("layout", l);
  // widgety ze skriptů do nabídky „+ Přidat kartu“ (sken složky jednou při otevření panelu)
  onMount(() => void loadWidgets().catch(() => null));
  const addable = (pageId: string) => [
    ...catalog(L(), pageId).map((x) => ({ id: x.id as CardId, name: cardName(x.id), hidden: !x.on.length })),
    ...catalogWidgets(widgetsListing()?.widgets ?? [], L().pages, pageId).map((x) => ({ id: x.id as CardId, name: `${x.icon ? `${x.icon} ` : ""}${x.title}`, hidden: !x.on.length })),
  ];
  /** draft rozložení se liší od uloženého → režim úprav v šuplíku by ho neviděl */
  const unsaved = () => {
    const b = base();
    return !!b && !sameLayout(normalizeLayout(JSON.parse(b).layout), L());
  };

  function removeTab(id: string) {
    const r = removePage(L(), id);
    if (!r.undo) return;
    const u = r.undo;
    put(r.layout);
    offerUndo(t("cards.layout.tab.removed", "Záložka „{name}“ smazána — její karty jsou v katalogu", { name: pageName(u.page) }), () => put(restorePage(L(), u)));
  }
  function removeOne(pageId: string, id: CardId) {
    const r = removeCard(L(), pageId, id);
    if (!r.undo) return;
    const u = r.undo;
    put(r.layout);
    offerUndo(t("cards.layout.removed", "Karta „{name}“ odebrána do katalogu", { name: cardName(id) }), () => put(restoreCard(L(), u)));
  }
  const projects = (): { id: string; name: string }[] =>
    (Array.isArray(c.cfg.projects) ? c.cfg.projects : []).filter((x: any) => x && typeof x.id === "string").map((x: any) => ({ id: x.id, name: typeof x.name === "string" && x.name ? x.name : x.id }));
  /** volby karty Projekty na záložce `pageId` (zástupci) */
  const Tiles = (q: { pageId: string; card: LayoutCard }) => {
    const o = () => tilesOf(q.card);
    return (
      <div class="lay-st-tiles">
        <label>
          {t("cards.tiles.size", "Dlaždice")}{" "}
          <select value={o().size} onChange={(e) => put(setTiles(L(), q.pageId, { size: e.currentTarget.value as TileSize }))}>
            <For each={TILE_SIZES}>{(s) => <option value={s}>{tileSizeName(s)}</option>}</For>
          </select>
        </label>
        <label><input type="checkbox" checked={o().label} onChange={(e) => put(setTiles(L(), q.pageId, { label: e.currentTarget.checked }))} /> {t("cards.tiles.label", "Název pod ikonou")}</label>
        <label><input type="checkbox" checked={o().git} onChange={(e) => put(setTiles(L(), q.pageId, { git: e.currentTarget.checked }))} /> {t("cards.tiles.git", "Větev a stav gitu")}</label>
        <Show when={projects().length > 1}>
          <div class="lay-st-which" role="group" aria-label={t("cards.tiles.which", "Projekty na téhle záložce")}>
            <small>{t("cards.tiles.which", "Projekty na téhle záložce")}</small>
            <For each={projects()}>
              {(pr) => (
                <label><input type="checkbox" checked={!o().hidden.includes(pr.id)} onChange={(e) => put(showProject(L(), q.pageId, pr.id, e.currentTarget.checked))} /> {pr.name}</label>
              )}
            </For>
          </div>
        </Show>
      </div>
    );
  };

  function newTab() {
    const r = addPage(L(), t("cards.layout.tab.newName", "Nová záložka"));
    if (r) put(r.layout);
  }

  return (
    <>
      <Section
        id="layout"
        title={t("settings.f.layout", "Záložky a karty")}
        desc={t("settings.layout.desc", "Co šuplík ukazuje, na které záložce a jak velké. Karta, která není na žádné záložce, se skryje a nic nestahuje. Nejrychleji to upravíš přímo v šuplíku.")}
      >
        <div class="lay-st-cta">
          <button class="btn pri" disabled={unsaved()} onClick={askEdit}>✎ {t("settings.layout.inDrawer", "Upravit přímo v šuplíku")}</button>
          <small>{unsaved() ? t("settings.layout.unsaved", "Nejdřív ulož nebo zahoď změny tady — šuplík upravuje uložené rozložení.") : t("settings.layout.inDrawer.hint", "Přetáhni karty, změň velikost, přidej z katalogu, založ vlastní záložku. Esc = zrušit.")}</small>
        </div>
        <SortList items={L().pages} onMove={(f, to) => put(movePage(L(), f, to))} label={t("settings.layout.tabsOrder", "Pořadí záložek")} class="lay-st-pages">
          {(pg, i, handle) => (
            <div class="lay-st-page">
              <div class="st-item lay-st-head">
                {handle()}
                <span class="lay-st-key">{i() + 1}</span>
                <select aria-label={t("cards.layout.tab.icon", "Ikona")} value={pg.icon ?? ""} onChange={(e) => put(renamePage(L(), pg.id, pg.name, e.currentTarget.value))}>
                  <For each={PAGE_ICONS}>{(ic) => <option value={ic}>{ic || t("cards.layout.tab.noIcon", "Bez ikony")}</option>}</For>
                </select>
                <input
                  type="text"
                  class="lay-st-name"
                  value={pg.name}
                  maxLength={32}
                  placeholder={pageName({ ...pg, name: "" })}
                  aria-label={t("cards.layout.tab.name", "Název záložky")}
                  onChange={(e) => {
                    const next = renamePage(L(), pg.id, e.currentTarget.value);
                    if (next === L()) e.currentTarget.value = pg.name;
                    else put(next);
                  }}
                />
                <span class="grow" />
                <button class="mini" disabled={L().pages.length <= 1} onClick={() => removeTab(pg.id)}>{t("settings.layout.removeTab", "Smazat záložku")}</button>
              </div>
              <SortList items={pg.cards} onMove={(f, to) => put(moveCard(L(), pg.id, f, to))} label={t("settings.layout.cardsOrder", "Pořadí karet na záložce {name}", { name: pageName(pg) })} class="lay-st-cards">
                {(cd, _j, h) => (
                  <div class="st-item lay-st-card">
                    {h()}
                    <span class="grow">{cardName(cd.id)}<small>{cardDesc(cd.id)}</small></span>
                    <select aria-label={t("settings.layout.size", "Velikost")} value={cd.size} onChange={(e) => put(setSize(L(), pg.id, cd.id, e.currentTarget.value as CardSize))}>
                      <For each={SIZES}>{(s) => <option value={s}>{sizeName(s)}</option>}</For>
                    </select>
                    <select
                      aria-label={t("cards.layout.heightLabel", "Výška")}
                      value={cd.h === undefined ? "" : String(cd.h)}
                      onChange={(e) => put(setHeight(L(), pg.id, cd.id, e.currentTarget.value ? (Number(e.currentTarget.value) as CardHeight) : undefined))}
                    >
                      <option value="">{t("cards.layout.heightAutoOpt", "Výška podle obsahu")}</option>
                      <For each={HEIGHTS}>{(n) => <option value={String(n)}>{heightLabel(n)}</option>}</For>
                    </select>
                    <Show when={L().pages.length > 1}>
                      <select
                        aria-label={t("settings.layout.moveTo", "Přesunout na záložku")}
                        value=""
                        onChange={(e) => {
                          const to = e.currentTarget.value;
                          e.currentTarget.value = "";
                          if (to) put(moveToPage(L(), pg.id, cd.id, to));
                        }}
                      >
                        <option value="">{t("settings.layout.moveTo", "Přesunout na záložku")}</option>
                        <For each={L().pages.filter((x) => x.id !== pg.id)}>{(x) => <option value={x.id}>{pageName(x)}</option>}</For>
                      </select>
                    </Show>
                    <button class="mini" onClick={() => removeOne(pg.id, cd.id)}>{t("settings.layout.removeCard", "Odebrat")}</button>
                    <Show when={cd.id === "projects"}><Tiles pageId={pg.id} card={cd} /></Show>
                  </div>
                )}
              </SortList>
              <Show when={pg.cards.length === 0}><p class="st-empty">{t("settings.layout.emptyTab", "Na záložce zatím nic není.")}</p></Show>
              <Show when={addable(pg.id).length > 0}>
                <select
                  class="lay-st-add"
                  aria-label={t("cards.layout.add", "+ Přidat kartu")}
                  value=""
                  onChange={(e) => {
                    const id = e.currentTarget.value as CardId;
                    e.currentTarget.value = "";
                    if (id in CARD_INFO || isDynamic(id)) put(addCard(L(), pg.id, id));
                  }}
                >
                  <option value="">{t("cards.layout.add", "+ Přidat kartu")}</option>
                  <For each={addable(pg.id)}>{(x) => <option value={x.id}>{x.name}{x.hidden ? ` (${t("settings.layout.hidden", "skrytá")})` : ""}</option>}</For>
                </select>
              </Show>
            </div>
          )}
        </SortList>
        <Show when={L().pages.length < MAX_PAGES}>
          <button class="mini" onClick={newTab}>{t("cards.layout.tab.addMore", "+ Nová záložka")}</button>
        </Show>
      </Section>

      <Section
        id="layout.looks"
        title={t("settings.f.layout.looks", "Upravené karty a zástupci")}
        desc={t("settings.looks.desc", "Karty a dlaždice s vlastním vzhledem. Měníš ho v šuplíku: ⋯ na kartě → Vzhled karty…, pravý klik na dlaždici → Upravit vzhled… (v režimu úprav stačí klik).")}
      >
        <LooksOverview />
      </Section>

      <Section id="layout.hiddenParts" title={t("settings.layout.parts", "Části karet")} desc={t("settings.layout.parts.desc", "Skrytá část se nevykreslí ani nepočítá (skrytá útrata nečte transcripty). Jde i přímo v šuplíku: ⋯ v hlavičce karty.")}>
        <For each={PART_KEYS}>
          {(k: PartKey) => (
            <div class="st-row" id={`f-layout.part.${k}`}>
              <div class="st-lbl">
                <span class="st-name">{partName(k)}</span>
                <small>{t("settings.layout.partOf", "V kartě {card}", { card: cardName(PARTS[k]) })}</small>
              </div>
              <div class="st-ctl">
                <button class="st-switch" classList={{ on: !partHidden(L(), k) }} role="switch" aria-checked={!partHidden(L(), k)} aria-label={partName(k)} onClick={() => put(setPartHidden(L(), k, !partHidden(L(), k)))} />
              </div>
            </div>
          )}
        </For>
      </Section>
    </>
  );
}

