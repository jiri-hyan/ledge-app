// Propojení poznámek se složkou (Nastavení → Poznámky): čistá logika popisků a stavu. Rust: `notes/folder_sync*`.
import { t } from "../i18n.ts";
import type { NotesFolderView } from "../types.ts";

export type LinkDirection = "import" | "export" | "both";
export interface FolderLinkCfg { enabled: boolean; path: string; direction: LinkDirection; propagateDeletes: boolean }

/** Výchozí = vypnuto, oba směry, mazání se nepřenáší (jako Rust `FolderLinkConfig::default`). */
export const DEFAULT_LINK: FolderLinkCfg = { enabled: false, path: "", direction: "both", propagateDeletes: false };

/** Config `notes.folderLink` s doplněnými výchozími hodnotami (starý config ho nemá). */
export function linkCfg(notes: { folderLink?: Partial<FolderLinkCfg> } | undefined): FolderLinkCfg {
  const l = notes?.folderLink ?? {};
  const dir = l.direction === "import" || l.direction === "export" ? l.direction : "both";
  return { enabled: !!l.enabled, path: typeof l.path === "string" ? l.path : "", direction: dir, propagateDeletes: !!l.propagateDeletes };
}

/** Pořadí voleb směru (konstanta kvůli kontrole počtu voleb přepínače). */
export const DIRECTION_IDS: LinkDirection[] = ["both", "import", "export"];

export function directionLabel(id: LinkDirection): string {
  switch (id) {
    case "import": return t("notes.folder.dir.import", "Jen načítat");
    case "export": return t("notes.folder.dir.export", "Jen ukládat");
    default: return t("notes.folder.dir.both", "Oběma směry");
  }
}

export function directionHint(id: LinkDirection): string {
  switch (id) {
    case "import": return t("notes.folder.dir.import.hint", "Ze složky do Ledge, složka se nemění");
    case "export": return t("notes.folder.dir.export.hint", "Z Ledge do složky, nic se nenačítá zpět");
    default: return t("notes.folder.dir.both.hint", "Poznámky se kopírují sem i tam");
  }
}

export function directionOptions(): { v: LinkDirection; label: string; hint: string }[] {
  return DIRECTION_IDS.map((id) => ({ v: id, label: directionLabel(id), hint: directionHint(id) }));
}

/** Jasná věta, co se kam zapisuje (podle zvoleného směru). */
export function writeSentence(dir: LinkDirection): string {
  switch (dir) {
    case "import":
      return t("notes.folder.write.import", "Ledge jen čte složku: každý .md soubor se zkopíruje do tvých poznámek (do „Obecné“). Do složky se nezapíše nic.");
    case "export":
      return t("notes.folder.write.export", "Ledge jen zapisuje: každá tvoje poznámka se uloží do složky jako .md soubor (a obrázky vedle). Úpravy ve složce se do Ledge nenačtou. Při prvním propojení se zkopírují všechny poznámky.");
    default:
      return t("notes.folder.write.both", "Každá poznámka je ve složce i v Ledge jako stejný .md soubor, úpravy se přenášejí oběma směry. Při prvním propojení se všechny poznámky zkopírují do složky a všechny .md ze složky do Ledge.");
  }
}

export function deleteSentence(on: boolean): string {
  return on
    ? t("notes.folder.del.on", "Smazání se přenáší: soubor smazaný ve složce jde do Archivu Ledge, poznámka smazaná nebo archivovaná v Ledge do podsložky .trash ve složce. Nic se nemaže natrvalo.")
    : t("notes.folder.del.off", "Smazání se nepřenáší: smažeš-li poznámku na jedné straně, na druhé zůstane a už se nevrátí.");
}

export const conflictSentence = () =>
  t("notes.folder.conflict.rule", "Změníš-li stejnou poznámku na obou stranách, nic se nepřepíše potichu: druhá verze se uloží jako „název (konflikt Ledge 2026-10-24).md“ a objeví se tady.");

export type Tone = "off" | "ok" | "warn" | "bad";

/** Stav pro tečku. */
export function stateTone(v: NotesFolderView | undefined): Tone {
  switch (v?.state) {
    case "idle": return "ok";
    case "syncing": return "warn";
    case "error": return "bad";
    default: return "off";
  }
}

export function stateText(v: NotesFolderView | undefined): string {
  switch (v?.state) {
    case "idle": return t("notes.folder.st.idle", "V pořádku");
    case "syncing": return t("notes.folder.st.syncing", "Synchronizuji…");
    case "error": return t("notes.folder.st.error", "Nejde synchronizovat");
    default: return t("notes.folder.st.off", "Zatím nic nesynchronizováno");
  }
}

/** Kopie konfliktu, kterou jde otevřít v Ledge (cesta v poznámkách). */
export function openablePath(c: { ledge: string | null }): string | null {
  return c.ledge && c.ledge.toLowerCase().endsWith(".md") ? c.ledge : null;
}

/** Rozpracovaná změna: synchronizuje se podle ULOŽENÉHO nastavení (`base` = uložený config jako JSON). */
export function dirtyLink(base: string, cur: FolderLinkCfg): boolean {
  try {
    return JSON.stringify(linkCfg(JSON.parse(base).notes)) !== JSON.stringify(linkCfg({ folderLink: cur }));
  } catch {
    return false;
  }
}
