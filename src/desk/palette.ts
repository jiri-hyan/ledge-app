// Ctrl+K: widgety na ploše — připnout / odepnout vestavěné druhy, schovat / ukázat všechny, obnovit profil
// rozmístění, vypnout průchozí u všech (průchozí widget myš nevidí, odjinud než z Ledge se vypnout nedá).
import type { Snapshot } from "../types";
import type { PItem } from "../palette/items";
import { t } from "../i18n";
import { say, sayError } from "../ui/toast";
import { deskApi } from "./api";
import { BUILTIN_KINDS, widgetOf } from "./model";
import { deskKindName, pinKind, unpinWidget } from "./pin";

const KW = "plocha widget widgety desktop monitor pripnout na plochu";

export function deskItems(s: Snapshot): PItem[] {
  const v = s.desk;
  const out: PItem[] = [];
  for (const k of BUILTIN_KINDS) {
    if ((k === "limits" || k === "waiting") && !s.claudeEnabled) continue;
    const w = widgetOf(v, k);
    const name = deskKindName(k);
    out.push(
      w
        ? { id: `desk:unpin:${k}`, section: "app", order: 6, title: t("cards.desk.pal.unpin", "Odepnout z plochy: {name}", { name }), keywords: KW, run: () => unpinWidget(w.id, k) }
        : { id: `desk:pin:${k}`, section: "app", order: 6, title: t("cards.desk.pal.pin", "Na plochu: {name}", { name }), keywords: KW, run: () => pinKind(k) },
    );
  }
  if (v?.widgets.length) {
    out.push({
      id: "desk:hideAll",
      section: "app",
      order: 6,
      title: v.hiddenAll ? t("cards.desk.pal.showAll", "Ukázat widgety na ploše") : t("cards.desk.pal.hideAll", "Schovat widgety na ploše"),
      keywords: `${KW} schovat ukazat skryt`,
      run: () => deskApi.hideAll(!v.hiddenAll).catch(sayError),
    });
    const through = v.widgets.filter((w) => w.through);
    if (through.length) {
      out.push({
        id: "desk:through",
        section: "app",
        order: 6,
        title: t("cards.desk.pal.throughOff", "Widgety na ploše: vypnout průchozí"),
        keywords: `${KW} pruchozi klik skrz`,
        run: () => Promise.all(through.map((w) => deskApi.set({ through: false }, w.id))).then(() => say(t("cards.desk.throughOffDone", "Widgety jdou zase chytit myší.")), sayError),
      });
    }
  }
  for (const name of v?.profiles ?? []) {
    out.push({ id: `desk:profile:${name}`, section: "app", order: 6, title: t("cards.desk.pal.profile", "Rozmístění plochy: {name}", { name }), keywords: `${KW} profil rozlozeni`, run: () => deskApi.profileApply(name).catch(sayError) });
  }
  return out;
}
