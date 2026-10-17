// Položky palety příkazů ze snapshotu: rychlé akce, projekty (spustit/zastavit/složka/terminál/
// GitHub/poznámka), poznámky (nová + seznam z `note_list`), Claude session, panely Nastavení
// (`settings/registry.ts`) a příkazy appky. Jen sestavení — hledání je v `search.ts`.
import { api, type NoteView, type Snapshot } from "../api";
import type { ActionView } from "../types";
import type { Item } from "./search";
import { runAction } from "../actions/api";
import { askConfirm } from "../actions/ui";
import { EXTERNAL_PANELS } from "../settings/registry";
import type { NotePreset } from "../notes/store";
import { say, sayError } from "../ui/toast";
import { t } from "../i18n";

export type Section = "action" | "project" | "note" | "claude" | "settings" | "app";
export interface PItem extends Item {
  section: Section;
  /** rychlá akce (ikona v řádku) */
  action?: ActionView;
  /** spustí položku (paleta počká, než doběhne případné potvrzení uvnitř) */
  run: () => unknown;
}

export const SECTION_LABEL: Record<Section, [string, string]> = {
  action: ["palette.section.action", "Akce"],
  project: ["palette.section.project", "Projekt"],
  note: ["palette.section.note", "Poznámka"],
  claude: ["palette.section.claude", "Claude"],
  settings: ["palette.section.settings", "Nastavení"],
  app: ["palette.section.app", "Ledge"],
};

export type Nav = {
  onNote: (preset?: NotePreset) => void;
  onOpenNote: (n: NoteView) => void;
  onAllNotes: () => void;
  onSettings: (panelId?: string) => void;
};

const LIVE = ["ready", "running", "starting", "preparing"];

/** Výsledek příkazu jádra do hlášky (chyby česky). */
const report = (p: Promise<unknown>, ok?: (v: any) => string) =>
  p.then((v) => ok && say(ok(v)), sayError);

export function buildItems(s: Snapshot, notes: NoteView[], nav: Nav): PItem[] {
  const out: PItem[] = [];
  const add = (x: PItem) => out.push(x);

  for (const a of s.actions ?? []) {
    add({ id: `action:${a.id}`, section: "action", order: 1, title: a.label, sub: a.target, keywords: `${a.group ?? ""} ${a.kind}`, action: a, run: () => runAction(a, askConfirm("palette")) });
  }

  for (const pr of s.projects) {
    const live = s.procs.some((x) => x.id === pr.id && LIVE.includes(x.phase));
    const kw = `projekt ${pr.id}`;
    if (live) add({ id: `stop:${pr.id}`, section: "project", order: 2, title: t("palette.project.stop", "Zastavit {name}", { name: pr.name }), keywords: kw, run: () => report(api.stop(pr.id) as Promise<unknown>) });
    else if (pr.pathOk && (pr.run || pr.kind === "app")) add({ id: `launch:${pr.id}`, section: "project", order: 2, title: t("palette.project.launch", "Spustit {name}", { name: pr.name }), keywords: kw, run: () => report(api.launch(pr.id) as Promise<unknown>) });
    if (pr.pathOk) {
      add({ id: `folder:${pr.id}`, section: "project", order: 3, title: t("palette.project.folder", "Otevřít složku {name}", { name: pr.name }), sub: pr.path ?? "", keywords: kw, run: () => report(api.openIn(pr.id, "folder") as Promise<unknown>) });
      add({ id: `term:${pr.id}`, section: "project", order: 3, title: t("palette.project.terminal", "Terminál v {name}", { name: pr.name }), sub: pr.path ?? "", keywords: `${kw} konzole cmd`, run: () => report(api.openIn(pr.id, "terminal") as Promise<unknown>) });
    }
    if (pr.github) {
      const url = pr.github;
      add({ id: `gh:${pr.id}`, section: "project", order: 3, title: t("palette.project.github", "GitHub: {name}", { name: pr.name }), sub: url, keywords: kw, run: () => report(api.openUrl(url) as Promise<unknown>) });
    }
    if (s.notesEnabled) add({ id: `pnote:${pr.id}`, section: "note", order: 4, title: t("palette.project.note", "Poznámka k {name}", { name: pr.name }), keywords: kw, run: () => nav.onNote({ project: pr.id }) });
  }

  if (s.notesEnabled) {
    add({ id: "note:new", section: "note", order: 0, title: t("palette.note.new", "Nová poznámka"), keywords: "poznamka zapsat napsat", run: () => nav.onNote() });
    add({ id: "note:all", section: "note", order: 4, title: t("palette.note.all", "Všechny poznámky"), keywords: "seznam", run: () => nav.onAllNotes() });
    for (const n of notes) {
      const proj = n.project ? (s.projects.find((x) => x.id === n.project)?.name ?? n.project) : "";
      add({ id: `note:${n.path}`, section: "note", order: 5, title: n.title || t("palette.note.untitled", "(bez názvu)"), sub: [proj, n.kind, n.done ? t("palette.note.done", "hotovo") : ""].filter(Boolean).join(" · "), run: () => nav.onOpenNote(n) });
    }
  }

  if (s.claudeEnabled) {
    for (const x of s.sessions) {
      add({ id: `session:${x.id}`, section: "claude", order: 4, title: x.title ?? x.name, sub: [x.project ?? x.cwd, x.status].join(" · "), keywords: `claude session ${x.name}`, run: () => report(api.sessionOpen(x.id) as Promise<unknown>) });
    }
  }

  add({ id: "settings:", section: "settings", order: 5, title: t("palette.settings.open", "Otevřít Nastavení"), keywords: "nastaveni config", run: () => nav.onSettings() });
  for (const pn of EXTERNAL_PANELS) {
    add({ id: `settings:${pn.id}`, section: "settings", order: 5, title: t("palette.settings.panel", "Nastavení: {name}", { name: t(pn.key, pn.cs) }), keywords: `${t(pn.kw[0], pn.kw[1])} ${pn.kw[1]}`, run: () => nav.onSettings(pn.id) });
  }

  if (s.notesEnabled && s.notesSync.state !== "off")
    add({ id: "app:sync", section: "app", order: 6, title: t("palette.app.sync", "Synchronizovat poznámky"), keywords: "sync git", run: () => report(api.notesSyncNow() as Promise<unknown>, (r) => r ?? t("palette.app.synced", "Synchronizováno.")) });
  add({ id: "app:update", section: "app", order: 6, title: t("palette.app.update", "Zkontrolovat aktualizaci Ledge"), keywords: "verze update nova", run: () => report(api.updateCheckNow() as Promise<unknown>, () => t("palette.app.checking", "Kontroluji novou verzi…")) });
  if (s.claudeEnabled && s.hooks.state !== "full")
    add({ id: "app:hooks", section: "app", order: 6, title: t("palette.app.hooks", "Zapojit hooky Claude Code"), keywords: "claude hooky schvalovani", run: () => report(api.hooksInstall() as Promise<unknown>, () => t("palette.app.hooked", "Hooky zapojené.")) });
  if (s.personalEnabled)
    add(s.mode === "prace"
      ? { id: "app:osobni", section: "app", order: 6, title: t("palette.app.personal", "Přepnout na Osobní"), run: () => api.setMode("osobni") }
      : { id: "app:prace", section: "app", order: 6, title: t("palette.app.work", "Přepnout na Práce"), run: () => api.setMode("prace") });
  add({ id: "app:gamemode", section: "app", order: 7, title: t("palette.app.gameMode", "Zapnout herní režim (Ledge zmizí)"), keywords: "hra game fullscreen schovat skryt nerusit herni rezim", run: () => api.setGameMode(true) });
  add({ id: "app:close", section: "app", order: 7, title: t("palette.app.close", "Zavřít šuplík"), run: () => api.close() });
  return out;
}
