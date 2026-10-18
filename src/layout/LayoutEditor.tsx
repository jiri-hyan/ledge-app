// Editor rozložení v Nastavení (panel Šuplík a karty): stejný model jako režim úprav v šuplíku —
// záložky (pořadí tažením, název, ikona, smazat), karty na nich (pořadí, velikost, přesun na jinou
// záložku, odebrat, přidat z katalogu) a skryté části karet. Mění jen draft Nastavení (`c.set("layout")`),
// zapíše společná lišta Uložit. Velké tlačítko „Upravit přímo v šuplíku“ = režim úprav (`askEdit`).
import { createMemo, For, Show } from "solid-js";
import { t } from "../i18n";
import { base, offerUndo, useSettings } from "../settings/ctx";
import { Section } from "../settings/fields/Row";
import { SortList } from "../settings/fields/SortList";
import { askEdit } from "./edit";
import {
  addCard, addPage, catalog, CARD_INFO, MAX_PAGES, moveCard, movePage, moveToPage, normalizeLayout, PAGE_ICONS, PART_KEYS, PARTS, partHidden,
  removeCard, removePage, renamePage, restoreCard, restorePage, sameLayout, setPartHidden, setSize, SIZES, type CardId, type CardSize, type Layout, type PartKey,
} from "./model";
import { cardDesc, cardName, pageName, partName } from "./text";
import "./layout.css";

const sizeName = (s: CardSize) =>
  ({ s: t("cards.layout.size.s", "Malá"), m: t("cards.layout.size.m", "Střední"), l: t("cards.layout.size.l", "Velká"), xl: t("cards.layout.size.xl", "Přes celou šířku") })[s];

export default function LayoutEditor() {
  const c = useSettings();
  const L = createMemo(() => normalizeLayout(c.cfg.layout));
  const put = (l: Layout) => c.set("layout", l);
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
                  <div class="st-item">
                    {h()}
                    <span class="grow">{cardName(cd.id)}<small>{cardDesc(cd.id)}</small></span>
                    <select aria-label={t("settings.layout.size", "Velikost")} value={cd.size} onChange={(e) => put(setSize(L(), pg.id, cd.id, e.currentTarget.value as CardSize))}>
                      <For each={SIZES}>{(s) => <option value={s}>{sizeName(s)}</option>}</For>
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
                  </div>
                )}
              </SortList>
              <Show when={pg.cards.length === 0}><p class="st-empty">{t("settings.layout.emptyTab", "Na záložce zatím nic není.")}</p></Show>
              <Show when={catalog(L(), pg.id).length > 0}>
                <select
                  class="lay-st-add"
                  aria-label={t("cards.layout.add", "+ Přidat kartu")}
                  value=""
                  onChange={(e) => {
                    const id = e.currentTarget.value as CardId;
                    e.currentTarget.value = "";
                    if (id in CARD_INFO) put(addCard(L(), pg.id, id));
                  }}
                >
                  <option value="">{t("cards.layout.add", "+ Přidat kartu")}</option>
                  <For each={catalog(L(), pg.id)}>{(x) => <option value={x.id}>{cardName(x.id)}{x.on.length ? "" : ` (${t("settings.layout.hidden", "skrytá")})`}</option>}</For>
                </select>
              </Show>
            </div>
          )}
        </SortList>
        <Show when={L().pages.length < MAX_PAGES}>
          <button class="mini" onClick={newTab}>{t("cards.layout.tab.addMore", "+ Nová záložka")}</button>
        </Show>
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

