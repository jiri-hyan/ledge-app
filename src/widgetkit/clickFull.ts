// Widgetkit v šuplíku: `action:<id>` přes `runAction` (potvrzení a souhlas příkazů drží `actions.rs`) a
// `drawer:<karta>` = rozbalení šuplíku. Importuje jen `App.tsx` — okno plochy ani Studio ho nenačtou.
import { api } from "../api";
import { askConfirm } from "../actions/ui";
import { runAction } from "../actions/api";
import { t } from "../i18n";
import { say } from "../ui/toast";
import { setKitHandlers } from "./click";

setKitHandlers({
  action: async (id, snap) => {
    const a = snap.actions?.find((x) => x.id === id);
    if (!a) return say(t("cards.widgetkit.act.missing", "Akce „{id}“ tu není (smazaná nebo přejmenovaná).", { id }), true);
    await runAction(a, askConfirm("card"));
  },
  drawer: async () => void (await api.expandDrawer(true)),
});
