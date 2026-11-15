// Widgetkit — klik na řádek / tlačítko (`WidgetItem.kit`): jen existující cesty UI → jádro, žádná nová oprávnění.
// url = `open_url` (jádro dovolí jen http/https), copy = schránka. `action:<id>` a `drawer:<karta>` se liší podle okna
// (šuplík: `runAction` + `expand_drawer`, plocha: `desk_open_drawer`), proto je dodává okno přes `setKitHandlers`
// (`clickFull.ts` v šuplíku, `desk/views.tsx` na ploše) — okno plochy tak nezískává cizí příkazy (test ipc-window-acl).
// Cesta (`open`) nemá cestu přes UI: bez příkazu jádra by to byl nový povrch (`docs/codemap/widgets-detail.md`).
import { api, type Snapshot } from "../api";
import { t } from "../i18n";
import { say, sayError } from "../ui/toast";
import type { KitAct } from "../types";

export interface KitHandlers {
  action: (id: string, snap: Snapshot) => Promise<void>;
  drawer: () => Promise<void>;
}
let handlers: Partial<KitHandlers> = {};
export const setKitHandlers = (h: Partial<KitHandlers>): void => {
  handlers = { ...handlers, ...h };
};

const unavailable = () => say(t("cards.widgetkit.act.unavailable", "Tohle okno tuhle akci neumí — otevři šuplík."), true);

export async function kitClick(act: KitAct, snap: Snapshot): Promise<void> {
  try {
    switch (act.type) {
      case "url":
        await api.openUrl(act.url);
        return;
      case "copy":
        await navigator.clipboard.writeText(act.text);
        say(t("cards.widgetkit.act.copied", "Zkopírováno do schránky."));
        return;
      case "action":
        return handlers.action ? await handlers.action(act.id, snap) : unavailable();
      case "drawer":
        return handlers.drawer ? await handlers.drawer() : unavailable();
      case "open":
        if (/^https?:\/\//i.test(act.path)) await api.openUrl(act.path);
        else say(t("cards.widgetkit.act.open", "Otevření cesty zatím widgetkit neumí — použij tlačítko s akcí."), true);
        return;
    }
  } catch (e) {
    sayError(e);
  }
}
