// Popisky a ikony akcí nad poznámkou (Splnit · Archivovat · Smazat) — společné pro ⋯ řádku, editor a lístek.
import { t } from "../i18n";
import type { LifeAction } from "./lifeactions";
import { IconArchive, IconCheck, IconTrash, IconUnarchive } from "./lifeicons";

/** Popisek akce (společný pro ⋯ řádku, editor i lístek). */
export function actionText(a: LifeAction): string {
  switch (a) {
    case "done": return t("notes.life.actDone", "Splnit");
    case "undone": return t("notes.life.actUndone", "Vrátit mezi nehotové");
    case "archive": return t("notes.life.actArchive", "Archivovat");
    case "unarchive": return t("notes.life.actUnarchive", "Vrátit z archivu");
    case "delete": return t("notes.life.actDelete", "Smazat");
  }
}

export function actionIcon(a: LifeAction) {
  switch (a) {
    case "done":
    case "undone": return <IconCheck />;
    case "archive": return <IconArchive />;
    case "unarchive": return <IconUnarchive />;
    case "delete": return <IconTrash />;
  }
}
