// Nastavení → Šuplík a karty → „Upravené karty a zástupci“ (vlna 8 P): přehled všeho, co má vlastní vzhled —
// karty na záložkách (`layout…cards[].look`) a zástupci (projekty a akce, `look`) — s „Obnovit“ u každého a
// „Obnovit vše“. Mění jen draft Nastavení (`c.set`), zapíše společná lišta Uložit; „Vrátit“ jako jinde.
import { For, Show } from "solid-js";
import { t } from "../i18n";
import { offerUndo, useSettings } from "../settings/ctx";
import { customizedCards, resetCardLooks, setCardLook } from "./look";
import { normalizeLayout, pageOf, type CardId } from "./model";
import { cardName, pageName } from "./text";
import { customizedTiles, resetTileLooks, withTileLook, type Customized } from "../tilelook/model";

export default function LooksOverview() {
  const c = useSettings();
  const L = () => normalizeLayout(c.cfg.layout);
  const cards = () => customizedCards(L());
  const tiles = () => customizedTiles(c.cfg);
  const putTiles = (cfg: any) => {
    c.set("projects", cfg.projects ?? []);
    c.set("actions", cfg.actions ?? []);
  };
  const resetTile = (x: Customized) => putTiles(withTileLook(c.cfg, x.kind, x.id, undefined));
  function resetAll() {
    const before = { layout: structuredClone(c.cfg.layout), projects: structuredClone(c.cfg.projects ?? []), actions: structuredClone(c.cfg.actions ?? []) };
    c.set("layout", resetCardLooks(L()));
    putTiles(resetTileLooks(c.cfg));
    offerUndo(t("settings.looks.resetDone", "Vzhled karet a zástupců obnoven"), () => {
      c.set("layout", before.layout);
      c.set("projects", before.projects);
      c.set("actions", before.actions);
    });
  }
  return (
    <Show when={cards().length + tiles().length} fallback={<p class="st-empty">{t("settings.looks.none", "Nic upraveného — vzhled změníš v šuplíku: ⋯ na kartě → Vzhled karty…, pravý klik na zástupce → Upravit vzhled…")}</p>}>
      <For each={cards()}>
        {(x) => (
          <div class="st-row">
            <div class="st-lbl">
              <span class="st-name">{cardName(x.cardId as CardId)}</span>
              <small>{t("settings.looks.cardOn", "Karta na záložce {tab}", { tab: pageName(pageOf(L(), x.pageId)) })}</small>
            </div>
            <div class="st-ctl"><button class="mini" onClick={() => c.set("layout", setCardLook(L(), x.pageId, x.cardId, undefined))}>{t("settings.looks.reset", "Obnovit")}</button></div>
          </div>
        )}
      </For>
      <For each={tiles()}>
        {(x) => (
          <div class="st-row">
            <div class="st-lbl">
              <span class="st-name">{x.name}</span>
              <small>{x.kind === "project" ? t("settings.looks.project", "Zástupce projektu") : t("settings.looks.action", "Zástupce akce / odkazu")}</small>
            </div>
            <div class="st-ctl"><button class="mini" onClick={() => resetTile(x)}>{t("settings.looks.reset", "Obnovit")}</button></div>
          </div>
        )}
      </For>
      <div class="st-row">
        <div class="st-lbl"><small>{t("settings.looks.allHint", "Karty zase podle design stylu, zástupci s automatickou ikonou. Složky zůstanou.")}</small></div>
        <div class="st-ctl"><button class="mini" onClick={resetAll}>{t("settings.looks.resetAll", "Obnovit vše")}</button></div>
      </div>
    </Show>
  );
}
