// Položky palety příkazů ze snapshotu: rychlé akce, projekty (spustit/zastavit/restartovat/vybrat složku/složka/terminál/
// GitHub/poznámka), poznámky (nová + seznam z `note_list`), Claude session, panely Nastavení
// (`settings/registry.ts`) a příkazy appky. Jen sestavení — hledání je v `search.ts`.
import { api, type NoteView, type Snapshot } from "../api";
import type { ActionView } from "../types";
import type { Item } from "./search";
import type { ClipRow } from "../cliphist/model";
import { runAction } from "../actions/api";
import { askConfirm } from "../actions/ui";
import { PANEL_META } from "../settings/registry";
import type { NotePreset } from "../notes/store";
import { say, sayError } from "../ui/toast";
import { t } from "../i18n";
import { canBgLaunch, openBgLaunch } from "../sessions/BgLaunch";
import { timerItems } from "./timer";
import { normalizeLayout, pageOf, placed } from "../layout/model";
import { pasteToShelf } from "../shelf/ShelfCard";
import { pageName } from "../layout/text";
import { placeOf } from "../projects/model";
import { restartProject, startProject, stopAll } from "../projects/state";
import { styleItems } from "./styles";
import { isPinned, newPin, throughAll, throughLabel, togglePin } from "../notes/pins";
import { deskItems } from "../desk/palette";

export type Section = "action" | "project" | "note" | "claude" | "settings" | "app" | "script" | "windows" | "calc" | "convert" | "emoji" | "clip";
export interface PItem extends Item {
  section: Section;
  /** rychlá akce (ikona v řádku) */
  action?: ActionView;
  /** znak místo ikony (emoji, „=“) */
  glyph?: string;
  /** Enter / klik zkopíruje tenhle text do schránky (výpočet, převod, emoji) — `run` se nevolá, do nedávných nejde */
  copy?: string;
  /** řádek historie schránky (`v:`) — Enter / klávesy obsluhuje `Palette.tsx` (`cliphist/model.ts`) */
  clip?: ClipRow;
  /** po spuštění paleta zůstane otevřená (příkaz „Emoji…“ jen přepne dotaz) */
  stay?: boolean;
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
  script: ["palette.section.script", "Skript"],
  windows: ["palette.section.windows", "Windows"],
  calc: ["palette.section.calc", "Výpočet"],
  convert: ["palette.section.convert", "Převod"],
  emoji: ["palette.section.emoji", "Emoji"],
  clip: ["palette.section.clip", "Schránka"],
};

export type Nav = {
  onNote: (preset?: NotePreset) => void;
  onOpenNote: (n: NoteView) => void;
  onAllNotes: () => void;
  onSettings: (panelId?: string) => void;
  /** režim úprav šuplíku (hlavní stránka + `layout/edit.ts` `beginEdit`) */
  onEditDrawer: () => void;
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
    if (live) {
      add({ id: `stop:${pr.id}`, section: "project", order: 2, title: t("palette.project.stop", "Zastavit {name}", { name: pr.name }), keywords: `${kw} stop vypnout server localhost`, run: () => report(api.stop(pr.id) as Promise<unknown>) });
      add({ id: `restart:${pr.id}`, section: "project", order: 2, title: t("palette.project.restart", "Restartovat {name}", { name: pr.name }), keywords: `${kw} restart znovu server`, run: () => restartProject(pr.id, pr.name) });
    } else if (placeOf(pr).kind === "choose") add({ id: `choose:${pr.id}`, section: "project", order: 2, title: t("palette.project.choose", "Vybrat složku pro {name}…", { name: pr.name }), keywords: `${kw} spustit slozka`, run: () => startProject(pr) });
    else if (pr.pathOk && (pr.run || pr.kind === "app")) add({ id: `launch:${pr.id}`, section: "project", order: 2, title: t("palette.project.launch", "Spustit {name}", { name: pr.name }), keywords: kw, run: () => report(api.launch(pr.id) as Promise<unknown>) });
    if (pr.pathOk) {
      add({ id: `folder:${pr.id}`, section: "project", order: 3, title: t("palette.project.folder", "Otevřít složku {name}", { name: pr.name }), sub: pr.path ?? "", keywords: kw, run: () => report(api.openIn(pr.id, "folder") as Promise<unknown>) });
      add({ id: `term:${pr.id}`, section: "project", order: 3, title: t("palette.project.terminal", "Terminál v {name}", { name: pr.name }), sub: pr.path ?? "", keywords: `${kw} konzole cmd`, run: () => report(api.openIn(pr.id, "terminal") as Promise<unknown>) });
    }
    if (pr.github) {
      const url = pr.github;
      add({ id: `gh:${pr.id}`, section: "project", order: 3, title: t("palette.project.github", "GitHub: {name}", { name: pr.name }), sub: url, keywords: kw, run: () => report(api.openUrl(url) as Promise<unknown>) });
    }
    if (canBgLaunch(s, pr) && s.mode === "prace") add({ id: `bgclaude:${pr.id}`, section: "claude", order: 3, title: t("palette.project.bgClaude", "Claude na pozadí v {name}…", { name: pr.name }), keywords: `${kw} claude bg background agent pozadi`, run: () => openBgLaunch(pr.id) });
    if (s.notesEnabled) add({ id: `pnote:${pr.id}`, section: "note", order: 4, title: t("palette.project.note", "Poznámka k {name}", { name: pr.name }), keywords: kw, run: () => nav.onNote({ project: pr.id }) });
  }

  if (s.procs.some((x) => LIVE.includes(x.phase))) {
    add({ id: "stop:all", section: "project", order: 2, title: t("palette.project.stopAll", "Zastavit všechny servery"), keywords: "stop vypnout servery localhost vse", run: () => stopAll() });
  }

  if (s.notesEnabled) {
    add({ id: "note:new", section: "note", order: 0, title: t("palette.note.new", "Nová poznámka"), keywords: "poznamka zapsat napsat", run: () => nav.onNote() });
    add({ id: "note:all", section: "note", order: 4, title: t("palette.note.all", "Všechny poznámky"), keywords: "seznam", run: () => nav.onAllNotes() });
    // lístky na plochu (`notes/pins.ts`): nový, připnout / odepnout poznámku, průchozí režim
    add({ id: "pin:new", section: "note", order: 0, title: t("palette.pin.new", "Nový lístek na plochu"), sub: t("palette.pin.newSub", "Poznámka přes ostatní okna — checklist k QA, Ctrl+V vloží snímek"), keywords: "listek plocha pripnout sticky overlay qa checklist navrchu", run: () => newPin() });
    if (s.pins?.open.length) add({ id: "pin:through", section: "note", order: 1, title: throughLabel(s), keywords: "listky pruchozi mys click through klikat skrz", run: () => throughAll() });
    for (const n of notes) {
      const proj = n.project ? (s.projects.find((x) => x.id === n.project)?.name ?? n.project) : "";
      add({ id: `note:${n.path}`, section: "note", order: 5, title: n.title || t("palette.note.untitled", "(bez názvu)"), sub: [proj, n.kind, n.done ? t("palette.note.done", "hotovo") : ""].filter(Boolean).join(" · "), run: () => nav.onOpenNote(n) });
      const name = n.title || t("palette.note.untitled", "(bez názvu)");
      const pinned = isPinned(s.pins, n.path);
      add({ id: `pin:${n.path}`, section: "note", order: 6, title: pinned ? t("palette.pin.unpin", "Odepnout z plochy: {name}", { name }) : t("palette.pin.pin", "Připnout na plochu: {name}", { name }), keywords: "listek plocha pripnout odepnout sticky", run: () => togglePin(n.path) });
    }
  }

  if (s.claudeEnabled) {
    for (const x of s.sessions) {
      add({ id: `session:${x.id}`, section: "claude", order: 4, title: x.title ?? x.name, sub: [x.project ?? x.cwd, x.status].join(" · "), keywords: `claude session ${x.name}`, run: () => report(api.sessionOpen(x.id) as Promise<unknown>) });
    }
  }

  add({ id: "settings:", section: "settings", order: 5, title: t("palette.settings.open", "Otevřít Nastavení"), keywords: "nastaveni config", run: () => nav.onSettings() });
  for (const pn of PANEL_META) {
    add({ id: `settings:${pn.id}`, section: "settings", order: 5, title: t("palette.settings.panel", "Nastavení: {name}", { name: t(pn.key, pn.cs) }), keywords: `${t(pn.kw[0], pn.kw[1])} ${pn.kw[1]}`, run: () => nav.onSettings(pn.id) });
  }

  if (s.notesEnabled && s.notesSync.state !== "off")
    add({ id: "app:sync", section: "app", order: 6, title: t("palette.app.sync", "Synchronizovat poznámky"), keywords: "sync git", run: () => report(api.notesSyncNow() as Promise<unknown>, (r) => r ?? t("palette.app.synced", "Synchronizováno.")) });
  add({ id: "app:update", section: "app", order: 6, title: t("palette.app.update", "Zkontrolovat aktualizaci Ledge"), keywords: "verze update nova", run: () => report(api.updateCheckNow() as Promise<unknown>, () => t("palette.app.checking", "Kontroluji novou verzi…")) });
  if (s.claudeEnabled && s.hooks.state !== "full")
    add({ id: "app:hooks", section: "app", order: 6, title: t("palette.app.hooks", "Zapojit hooky Claude Code"), keywords: "claude hooky schvalovani", run: () => report(api.hooksInstall() as Promise<unknown>, () => t("palette.app.hooked", "Hooky zapojené.")) });
  // záložky z `layout`: přejít na jinou (klávesy 1–9 jsou v App) a režim úprav šuplíku
  const lay = normalizeLayout(s.layout);
  const cur = pageOf(lay, s.mode).id;
  lay.pages.forEach((pg, i) => {
    if (pg.id !== cur) add({ id: `tab:${pg.id}`, section: "app", order: 6, title: t("palette.app.tab", "Přejít na záložku {name}", { name: pageName(pg) }), keywords: `zalozka tab prepnout ${i + 1} ${pg.id}`, run: () => api.setMode(pg.id) });
  });
  if (placed(lay, "shelf"))
    add({ id: "app:shelfPaste", section: "app", order: 6, title: t("palette.app.shelfPaste", "Vložit schránku do přihrádky"), keywords: "prihradka shelf vlozit paste schranka obrazek soubor text", run: () => void pasteToShelf() });
  add({ id: "app:editDrawer", section: "app", order: 6, title: t("palette.app.editDrawer", "Upravit šuplík (karty a záložky)"), keywords: "upravit rozlozeni karty zalozky presunout pridat odebrat velikost katalog edit layout", run: () => nav.onEditDrawer() });
  for (const x of timerItems(s)) add(x);
  for (const x of styleItems(s)) add(x);
  // widgety na ploše (`desk/palette.ts`): připnout / odepnout, schovat vše, profily, průchozí
  for (const x of deskItems(s)) add(x);
  add({ id: "app:keys", section: "app", order: 6, title: t("ui.keys.palette.cmd", "? Klávesy a gesta"), sub: t("ui.keys.palette.sub", "Všechny zkratky a gesta na jednom místě (Nastavení → Aplikace → Jazyk a průvodce)"), keywords: "? klavesy zkratky gesta napoveda help keys shortcuts gestures", run: () => nav.onSettings("f-onboarding.keys") });
  add({ id: "app:gamemode", section: "app", order: 7, title: t("palette.app.gameMode", "Zapnout herní režim (Ledge zmizí)"), keywords: "hra game fullscreen schovat skryt nerusit herni rezim", run: () => api.setGameMode(true) });
  add({ id: "app:close", section: "app", order: 7, title: t("palette.app.close", "Zavřít šuplík"), run: () => api.close() });
  return out;
}
