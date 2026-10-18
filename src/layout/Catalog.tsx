// Katalog karet v režimu úprav: všechny karty, které na aktuální záložce nejsou — název, jednořádkový
// popis a kde dnes jsou (jiná záložka / skrytá). Klik = přidat na konec aktuální záložky.
import { For, Show } from "solid-js";
import { t } from "../i18n";
import { change, editPage, setCatalogOpen } from "./edit";
import { addCard, catalog, pageOf, type Layout } from "./model";
import { cardDesc, cardName, pageName } from "./text";

export default function Catalog(p: { layout: Layout }) {
  const items = () => catalog(p.layout, editPage());
  const where = (on: string[]) =>
    on.length ? t("cards.layout.catalog.on", "Je na: {tabs}", { tabs: on.map((id) => pageName(pageOf(p.layout, id))).join(", ") }) : t("cards.layout.catalog.hidden", "Skrytá — teď není nikde");
  return (
    <div class="card lay-catalog" role="region" aria-label={t("cards.layout.catalog.title", "Katalog karet")}>
      <h4>
        {t("cards.layout.catalog.title", "Katalog karet")}
        <span class="n">{items().length || ""}</span>
        <button class="mini" onClick={() => setCatalogOpen(false)}>{t("cards.layout.catalog.close", "Zavřít")}</button>
      </h4>
      <For each={items()} fallback={<div class="empty">{t("cards.layout.catalog.empty", "Všechny karty už na téhle záložce jsou.")}</div>}>
        {(x) => (
          <button class="row lay-cat-row" onClick={() => change((l) => addCard(l, editPage(), x.id))} title={t("cards.layout.catalog.addTitle", "Přidat na tuhle záložku")}>
            <div class="grow">
              <div class="name">{cardName(x.id)}</div>
              <div class="sub">{cardDesc(x.id)}</div>
            </div>
            <Show when={x.on.length} fallback={<span class="pill n">{where(x.on)}</span>}>
              <span class="pill b">{where(x.on)}</span>
            </Show>
            <span class="mini">{t("cards.layout.catalog.add", "Přidat")}</span>
          </button>
        )}
      </For>
    </div>
  );
}
