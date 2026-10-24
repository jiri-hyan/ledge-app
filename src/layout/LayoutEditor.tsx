// Editor rozložení v Nastavení (panel Šuplík a karty): přehledný přepínač záložek nahoře, živý mini-náhled
// mřížky karet (4 sloupce podle `packSpans`), karty s přímými tlačítky pro velikost (S/M/L/XL), výšku
// (Auto/1–4), zobrazení (Seznam/Dlaždice/Bento/Kompaktní), strop řádků a vizuální katalog karet dole.
// Mění jen draft Nastavení (`c.set("layout")`), zapíše společná lišta Uložit.
import { createMemo, createSignal, For, onMount, Show } from "solid-js";
import { t, tp } from "../i18n";
import { base, offerUndo, useSettings } from "../settings/ctx";
import { Section } from "../settings/fields/Row";
import { SortList } from "../settings/fields/SortList";
import { askEdit } from "./edit";
import {
  addCard, addPage, catalog, CARD_INFO, HEIGHTS, LIMIT_CARDS, limitOf, pageHas, setLimit, isDynamic, MAX_PAGES, moveCard, movePage, moveToPage, nextSize, normalizeLayout, pageOf, PAGE_ICONS, PART_KEYS, PARTS, partHidden,
  removeCard, removePage, renamePage, restoreCard, restorePage, sameLayout, setHeight, setPartHidden, setSize, setTiles, showProject, SIZES, tilesOf, TILE_SIZES,
  type CardHeight, type CardId, type Layout, type LayoutCard, type PartKey,
} from "./model";
import { heightLabel } from "./Deck";
import { packSpans, spanOf } from "./size";
import { setBento } from "./bento";
import { patchCardLook, setCardLook, VIEW_CARDS, VIEW_NAMES, viewOf, viewsOf, type CardView } from "./look";
import { colsNow } from "./cols";
import { tileSizeName } from "../cards/TileOptions";
import { catalogWidgets } from "../widgets/model";
import { loadWidgets, widgetsListing } from "../widgets/registry";
import { cardDesc, cardName, pageName, partName, sizeName, tx } from "./text";
import LooksOverview from "./LooksOverview";
import "./layout.css";

const LIMIT_OPTS = [2, 3, 4, 5, 6, 8, 10, 15, 20, 0];

export default function LayoutEditor() {
  const c = useSettings();
  const L = createMemo(() => normalizeLayout(c.cfg.layout));
  const [activeId, setActiveId] = createSignal<string>("");
  const pg = createMemo(() => pageOf(L(), activeId() || L().pages[0]?.id || "prace"));
  const pgIdx = createMemo(() => L().pages.findIndex((x) => x.id === pg().id));
  const put = (l: Layout) => c.set("layout", l);

  // widgety ze skriptů do nabídky „+ Přidat kartu“ (sken složky jednou při otevření panelu)
  onMount(() => void loadWidgets().catch(() => null));

  const addable = (pageId: string) => [
    ...catalog(L(), pageId).map((x) => ({ id: x.id as CardId, name: cardName(x.id), desc: cardDesc(x.id), hidden: !x.on.length })),
    ...catalogWidgets(widgetsListing()?.widgets ?? [], L().pages, pageId).map((x) => ({
      id: x.id as CardId,
      name: `${x.icon ? `${x.icon} ` : ""}${x.title}`,
      desc: cardDesc(x.id as CardId),
      hidden: !x.on.length,
    })),
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
    setActiveId(r.layout.pages[0]?.id ?? "");
    offerUndo(t("cards.layout.tab.removed", "Záložka „{name}“ smazána — její karty jsou v katalogu", { name: pageName(u.page) }), () => {
      put(restorePage(L(), u));
      setActiveId(u.page.id);
    });
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
        <span class="lay-st-grp">
          <span class="lay-st-grp-lbl">{t("cards.tiles.size", "Dlaždice")}</span>
          <span class="lay-st-seg" role="group" aria-label={t("cards.tiles.size", "Dlaždice")}>
            <For each={TILE_SIZES}>
              {(s) => (
                <button classList={{ on: o().size === s }} onClick={() => put(setTiles(L(), q.pageId, { size: s }))}>
                  {tileSizeName(s)}
                </button>
              )}
            </For>
          </span>
        </span>
        <button class="lay-st-pchip" classList={{ on: o().label }} onClick={() => put(setTiles(L(), q.pageId, { label: !o().label }))}>
          {t("cards.tiles.label", "Název pod ikonou")}
        </button>
        <button class="lay-st-pchip" classList={{ on: o().git }} onClick={() => put(setTiles(L(), q.pageId, { git: !o().git }))}>
          {t("cards.tiles.git", "Větev a stav gitu")}
        </button>
        <Show when={projects().length > 1}>
          <div class="lay-st-which" role="group" aria-label={t("cards.tiles.which", "Projekty na téhle záložce")}>
            <small>{t("cards.tiles.which", "Projekty na téhle záložce")}:</small>
            <Show when={o().hidden.length > 0}>
              <button class="mini" onClick={() => put(setTiles(L(), q.pageId, { hidden: [] }))}>{t("settings.layout.showAllProjects", "Všechny")}</button>
            </Show>
            <For each={projects()}>
              {(pr) => {
                const shown = () => !o().hidden.includes(pr.id);
                return (
                  <button class="lay-st-pchip" classList={{ on: shown() }} aria-pressed={shown()} onClick={() => put(showProject(L(), q.pageId, pr.id, !shown()))}>
                    {pr.name}
                  </button>
                );
              }}
            </For>
          </div>
        </Show>
      </div>
    );
  };

  function newTab() {
    const r = addPage(L(), t("cards.layout.tab.newName", "Nová záložka"));
    if (r) {
      put(r.layout);
      setActiveId(r.id);
    }
  }

  // náhled se stejným počtem sloupců, jaký má mřížka šuplíku teď (`--deck-cols` stylu)
  const cols = () => colsNow();
  const spans = createMemo(() => (pg().bento ? pg().cards.map((k) => spanOf(k.size, cols())) : packSpans(pg().cards.map((k) => k.size), cols())));

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

        {/* Lišta záložek — klik přepne záložku, + přidá novou */}
        <div class="lay-st-tabbar" role="tablist" aria-label={t("settings.layout.tabsOrder", "Pořadí záložek")}>
          <For each={L().pages}>
            {(tab, i) => (
              <button
                role="tab"
                aria-selected={tab.id === pg().id}
                class="lay-st-tabbtn"
                classList={{ on: tab.id === pg().id }}
                onClick={() => setActiveId(tab.id)}
              >
                <span class="lay-st-key">{i() + 1}</span>
                <Show when={tab.icon}><span>{tab.icon}</span></Show>
                <span>{pageName(tab)}</span>
                <span class="lay-st-cnt">{tab.cards.length}</span>
              </button>
            )}
          </For>
          <Show when={L().pages.length < MAX_PAGES}>
            <button class="mini" onClick={newTab}>{t("cards.layout.tab.addMore", "+ Nová záložka")}</button>
          </Show>
        </div>

        {/* Aktivní záložka */}
        <div class="lay-st-page">
          <div class="lay-st-head">
            <div class="lay-st-icons" role="group" aria-label={t("cards.layout.tab.icon", "Ikona")}>
              <For each={PAGE_ICONS}>
                {(ic) => (
                  <button
                    class="lay-st-icbtn"
                    classList={{ on: (pg().icon ?? "") === ic }}
                    title={ic || t("cards.layout.tab.noIcon", "Bez ikony")}
                    aria-label={ic || t("cards.layout.tab.noIcon", "Bez ikony")}
                    onClick={() => put(renamePage(L(), pg().id, pg().name, ic))}
                  >
                    {ic || "∅"}
                  </button>
                )}
              </For>
            </div>
            <input
              type="text"
              class="lay-st-name"
              value={pg().name}
              maxLength={32}
              placeholder={pageName({ ...pg(), name: "" })}
              aria-label={t("cards.layout.tab.name", "Název záložky")}
              onChange={(e) => {
                const next = renamePage(L(), pg().id, e.currentTarget.value);
                if (next === L()) e.currentTarget.value = pg().name;
                else put(next);
              }}
            />
            <label class="lay-st-bento" title={t("settings.layout.bentoHint", "Karty jako dlaždice 1 × 1, 2 × 1, 2 × 2 v pevných řádcích, mezery se zaplní samy. Velikost dlaždice: ⋯ na kartě → Dlaždice.")}>
              <input type="checkbox" checked={!!pg().bento} onChange={(e) => put(setBento(L(), pg().id, e.currentTarget.checked))} />
              {t("settings.layout.bento", "Bento mřížka")}
            </label>
            <span class="grow" />
            <Show when={L().pages.length > 1}>
              <button
                class="mini"
                disabled={pgIdx() <= 0}
                title={t("settings.layout.tabLeft", "Posunout záložku doleva")}
                aria-label={t("settings.layout.tabLeft", "Posunout záložku doleva")}
                onClick={() => put(movePage(L(), pgIdx(), pgIdx() - 1))}
              >←</button>
              <button
                class="mini"
                disabled={pgIdx() >= L().pages.length - 1}
                title={t("settings.layout.tabRight", "Posunout záložku doprava")}
                aria-label={t("settings.layout.tabRight", "Posunout záložku doprava")}
                onClick={() => put(movePage(L(), pgIdx(), pgIdx() + 1))}
              >→</button>
            </Show>
            <button class="mini" disabled={L().pages.length <= 1} onClick={() => removeTab(pg().id)}>
              {t("settings.layout.removeTab", "Smazat záložku")}
            </button>
          </div>

          {/* Živý mini-náhled mřížky záložky */}
          <Show when={pg().cards.length > 0}>
            <div class="lay-st-pv">
              <div class="lay-st-pv-lbl">
                <span>{t("settings.layout.preview", "Náhled mřížky záložky (klik na kartu změní šířku)")}</span>
              </div>
              <div class="lay-st-pv-grid" data-bento={pg().bento ? "" : undefined}>
                <For each={pg().cards}>
                  {(cd, idx) => (
                    <button
                      class="lay-st-pv-cell"
                      style={{ "--span": String(spans()[idx()] ?? 2), "--h": pg().bento ? String(cd.h ?? 2) : undefined }}
                      title={t("cards.layout.sizeTitle", "Velikost: {size} — klik změní", { size: sizeName(cd.size) })}
                      onClick={() => put(setSize(L(), pg().id, cd.id, nextSize(cd.size)))}
                    >
                      <span>{cardName(cd.id)}</span>
                      <small>{cd.size.toUpperCase()}{cd.h ? `×${cd.h}` : ""}</small>
                    </button>
                  )}
                </For>
              </div>
            </div>
          </Show>

          {/* Seznam karet na aktivní záložce */}
          <SortList items={pg().cards} onMove={(f, to) => put(moveCard(L(), pg().id, f, to))} label={t("settings.layout.cardsOrder", "Pořadí karet na záložce {name}", { name: pageName(pg()) })} class="lay-st-cards">
            {(cd, _j, h) => {
              const hasView = () => cd.id in VIEW_CARDS;
              const curView = () => viewOf(cd.id, cd.look);
              const setView = (v: CardView) => put(setCardLook(L(), pg().id, cd.id, patchCardLook(cd.look, cd.id, { view: v === VIEW_CARDS[cd.id] ? undefined : v })));
              const hasLimit = () => LIMIT_CARDS.includes(cd.id);
              const otherTabs = () => L().pages.filter((x) => x.id !== pg().id && !pageHas(x, cd.id));
              return (
                <div class="lay-st-card">
                  <div class="lay-st-card-top">
                    {h()}
                    <div class="lay-st-card-info">
                      <span class="lay-st-card-title">{cardName(cd.id)}</span>
                      <small>{cardDesc(cd.id)}</small>
                    </div>
                    <span class="lay-st-grp">
                      <span class="lay-st-grp-lbl">{t("settings.layout.size", "Velikost")}</span>
                      <span class="lay-st-seg" role="group" aria-label={t("settings.layout.size", "Velikost")}>
                        <For each={SIZES}>
                          {(s) => (
                            <button classList={{ on: cd.size === s }} title={sizeName(s)} onClick={() => put(setSize(L(), pg().id, cd.id, s))}>
                              {s.toUpperCase()}
                            </button>
                          )}
                        </For>
                      </span>
                    </span>
                    <label class="lay-st-grp">
                      <span class="lay-st-grp-lbl">{t("cards.layout.heightLabel", "Výška")}</span>
                      <select
                        value={cd.h === undefined ? "" : String(cd.h)}
                        onChange={(e) => put(setHeight(L(), pg().id, cd.id, e.currentTarget.value ? (Number(e.currentTarget.value) as CardHeight) : undefined))}
                      >
                        <option value="">{t("cards.layout.heightAutoOpt", "Výška podle obsahu")}</option>
                        <For each={HEIGHTS}>{(n) => <option value={String(n)}>{heightLabel(n)}</option>}</For>
                      </select>
                    </label>
                    <button class="mini" onClick={() => removeOne(pg().id, cd.id)}>{t("settings.layout.removeCard", "Odebrat")}</button>
                  </div>

                  <Show when={hasView() || hasLimit() || otherTabs().length > 0}>
                    <div class="lay-st-card-sub">
                      <Show when={hasView()}>
                        <span class="lay-st-grp">
                          <span class="lay-st-grp-lbl">{t("cards.cardlook.view", "Zobrazení")}</span>
                          <span class="lay-st-seg" role="group" aria-label={t("cards.cardlook.view", "Zobrazení")}>
                            <For each={viewsOf(cd.id)}>
                              {(v) => (
                                <button classList={{ on: curView() === v }} onClick={() => setView(v)}>
                                  {tx(VIEW_NAMES[v])}
                                </button>
                              )}
                            </For>
                          </span>
                        </span>
                      </Show>
                      <Show when={hasLimit()}>
                        <span class="lay-st-grp" title={t("settings.layout.limitTitle", "Víc řádků schová karta pod „Zobrazit další“. Session, které čekají na tebe, se neschovají nikdy.")}>
                          <span class="lay-st-grp-lbl">{t("settings.layout.limit", "Kolik řádků ukázat")}</span>
                          <select aria-label={t("settings.layout.limit", "Kolik řádků ukázat")} value={String(limitOf(cd))} onChange={(e) => put(setLimit(L(), pg().id, cd.id, Number(e.currentTarget.value)))}>
                            <For each={LIMIT_OPTS}>
                              {(n) => <option value={String(n)}>{n ? tp("settings.layout.limitN", n, ["Nejvýš {n} řádek", "Nejvýš {n} řádky", "Nejvýš {n} řádků"]) : t("settings.layout.limitAll", "Všechny řádky")}</option>}
                            </For>
                          </select>
                        </span>
                      </Show>
                      <Show when={otherTabs().length > 0}>
                        <span class="lay-st-grp">
                          <span class="lay-st-grp-lbl">{t("settings.layout.moveTo", "Přesunout na záložku")}:</span>
                          <For each={otherTabs()}>
                            {(target) => (
                              <button class="mini" onClick={() => put(moveToPage(L(), pg().id, cd.id, target.id))}>
                                → {pageName(target)}
                              </button>
                            )}
                          </For>
                        </span>
                      </Show>
                    </div>
                  </Show>

                  <Show when={cd.id === "projects"}><Tiles pageId={pg().id} card={cd} /></Show>
                </div>
              );
            }}
          </SortList>

          <Show when={pg().cards.length === 0}>
            <p class="st-empty">{t("settings.layout.emptyTab", "Na záložce zatím nic není.")}</p>
          </Show>

          {/* Katalog karet pro rychlé přidání */}
          <Show when={addable(pg().id).length > 0}>
            <div class="lay-st-cat">
              <div class="lay-st-cat-head">{t("cards.layout.add", "+ Přidat kartu")}</div>
              <div class="lay-st-cat-grid">
                <For each={addable(pg().id)}>
                  {(x) => (
                    <button
                      class="lay-st-cat-item"
                      onClick={() => {
                        if (x.id in CARD_INFO || isDynamic(x.id)) put(addCard(L(), pg().id, x.id));
                      }}
                    >
                      <div class="lay-st-cat-top">
                        <span>{x.name}{x.hidden ? ` (${t("settings.layout.hidden", "skrytá")})` : ""}</span>
                        <span class="lay-st-cat-plus">+</span>
                      </div>
                      <small>{x.desc}</small>
                    </button>
                  )}
                </For>
              </div>
            </div>
          </Show>
        </div>
      </Section>

      <Section
        id="layout.looks"
        title={t("settings.f.layout.looks", "Upravené karty a zástupci")}
        desc={t("settings.looks.desc", "Karty a zástupci s vlastním vzhledem. Měníš ho v šuplíku: ⋯ na kartě → Vzhled karty…, pravý klik na zástupce → Upravit vzhled… (v režimu úprav stačí klik).")}
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
