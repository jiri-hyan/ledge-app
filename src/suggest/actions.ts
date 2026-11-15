// Akce chipů pruhu návrhů (s vedlejším účinkem — čistá logika je v `model.ts`): uložit adresu jako odkaz
// (akce typu web v configu), adresu do poznámky a poznámku ze schůzky. Vše na výslovný klik.
import { createSignal } from "solid-js";
import { api, type Snapshot } from "../api";
import type { NotePreset } from "../notes/store";
import { BRIEF_KIND } from "../notes/kinds";
import { uptime } from "../format";
import { devBriefBody } from "./devbrief";
import { slugId } from "../actions/model";
import { t } from "../i18n";
import { say, sayError } from "../ui/toast";
import { dismiss, hostOf, meetingNoteText, type Dismissed, type Meeting, type Suggestion } from "./model";

/** Zavřené návrhy (klíč → do kdy), jen v paměti — po restartu Ledge se zeptá znovu. */
const [dismissed, setDismissed] = createSignal<Dismissed>(new Map());
export { dismissed };

export const closeSuggestion = (key: string) => setDismissed((d) => dismiss(d, key, Date.now()));

/** Nejvíc akcí v configu (Rust `actions::MAX_ACTIONS`). */
const MAX_ACTIONS = 64;

/** Uloží adresu jako odkaz (akce `url` pro obě záložky). Rozbitý config neukládá — přepsal by uživatelův. */
export async function saveLink(url: string, cfgError: string | null): Promise<boolean> {
  if (cfgError) {
    say(t("cards.suggest.cfgError", "Nastavení obsahuje chybu — odkaz teď přidáš jen v Nastavení."), true);
    return false;
  }
  try {
    const cfg = await api.getConfig();
    if (!cfg) throw new Error(t("cards.suggest.noConfig", "Nastavení se nepodařilo načíst."));
    const list: { id: string }[] = cfg.actions ?? [];
    if (list.length >= MAX_ACTIONS) throw new Error(t("cards.suggest.tooMany", "Víc než 64 akcí a odkazů nejde — nějaký nejdřív smaž."));
    const label = (hostOf(url) || url).slice(0, 40);
    const id = slugId(label, list.map((a) => a.id));
    await api.saveConfig({ ...cfg, actions: [...list, { id, label, kind: "url", target: url, showOn: "both" }] });
    say(t("cards.suggest.linkSaved", "Odkaz „{label}“ uložen.", { label }), false);
    return true;
  } catch (e) {
    sayError(e);
    return false;
  }
}

/** Uloží adresu jako novou obecnou poznámku. */
export async function urlToNote(url: string): Promise<boolean> {
  try {
    await api.noteCreate({ text: url, project: null, remind: null, images: [], kind: null });
    say(t("cards.suggest.noteSaved", "Uloženo do poznámky."), false);
    return true;
  } catch (e) {
    sayError(e);
    return false;
  }
}

/** Nová poznámka ze schůzky (titulek „{název} — {datum}“, kategorie výchozí = Poznámka); vrací cestu, aby ji pruh otevřel v editoru. */
export async function meetingToNote(m: Pick<Meeting, "title" | "start">): Promise<string | false> {
  try {
    return (await api.noteCreate({ text: meetingNoteText(m), project: null, remind: null, images: [], kind: null })) ?? false;
  } catch (e) {
    sayError(e);
    return false;
  }
}

/** Dev chyba → předvyplnění nového zadání (druh `zadani`, projekt, tělo z balíčku kontextu). Jen z toho, co Ledge ví
 *  (proces, projekt + git); nic se neposílá — „Uložit a poslat“ je výslovný klik v editoru. null = proces už neběží. */
export function devErrorPreset(s: Snapshot, c: Pick<Suggestion, "proc" | "url" | "detail">, now = Date.now()): NotePreset | null {
  const proc = s.procs.find((x) => x.id === c.proc);
  if (!proc) return null;
  const proj = s.projects.find((x) => x.id === proc.id);
  const body = devBriefBody(
    {
      project: proj?.name ?? proc.id,
      url: c.url ?? null,
      branch: proj?.git?.branch ?? null,
      dirty: proj?.git ? proj.git.dirty : null,
      cmd: proc.cmd ?? null,
      uptime: uptime(proc.readyAt ?? proc.startedAt, now),
      errors: proc.errors,
      lines: proc.errorLines ?? [],
    },
    {
      title: t("cards.suggest.devBrief.title", "Chyba dev serveru"),
      project: t("cards.suggest.devBrief.project", "Projekt"),
      url: t("cards.suggest.devBrief.url", "URL"),
      branch: t("cards.suggest.devBrief.branch", "Větev"),
      dirty: (n) => t("cards.suggest.devBrief.dirty", "{n} neuložených změn", { n }),
      cmd: t("cards.suggest.devBrief.cmd", "Příkaz"),
      uptime: t("cards.suggest.devBrief.uptime", "Běží"),
      errors: t("cards.suggest.devBrief.errors", "Chyb ve výstupu"),
      output: t("cards.suggest.devBrief.output", "Výstup"),
      steps: t("cards.suggest.devBrief.steps", "Kroky"),
      expected: t("cards.suggest.devBrief.expected", "Čekal jsem"),
      happened: t("cards.suggest.devBrief.happened", "Stalo se"),
      task: t("cards.suggest.devBrief.task", "Najít příčinu chyby a opravit ji"),
    },
  );
  return { project: proj?.id ?? "", kind: BRIEF_KIND, body };
}
