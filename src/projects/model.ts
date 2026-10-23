// Zástupci projektů — čistá logika „co se spustí a odkud“ (vlna 9 Z): stav místa (`Place` z jádra), text
// tooltipu a panelu Upravit…, živé / selhané procesy, výběr složky z víc klonů a zápis `path` / `run` do
// configu. Bez DOM a bez IPC (test `model.test.mjs`); komponenty jsou `ProjectPanel.tsx`, `RunningPill.tsx`.
import type { FolderCandidate, Place, ProcView, ProjectView, SessionView } from "../types.ts";
import { t, tp } from "../i18n.ts";
import { ago } from "../format.ts";
import { samePath } from "../util/path.ts";

export const LIVE_PHASES = ["ready", "running", "starting", "preparing"];
export const isLive = (phase: string | undefined) => !!phase && LIVE_PHASES.includes(phase);
export const liveProcs = (procs: ProcView[]) => procs.filter((x) => isLive(x.phase));

/** Místo projektu; starší jádro bez `place` → podle `pathOk`. */
export function placeOf(pr: Pick<ProjectView, "place" | "pathOk">): Place {
  return pr.place ?? (pr.pathOk ? { kind: "ok" } : { kind: "none" });
}

/** Co dlaždice spustí: příkaz dev serveru, nebo exe / jméno aplikace; nic = null. */
export function whatOf(pr: Pick<ProjectView, "kind" | "run" | "app">): string | null {
  const s = (pr.kind === "app" ? pr.app ?? pr.run : pr.run)?.trim();
  return s ? s : null;
}

/** Odkud: běžící proces ví nejlíp (jeho `cwd`), jinak složka projektu; nejasné = popis proč. */
export function whereOf(pr: Pick<ProjectView, "path" | "place" | "pathOk">, proc?: Pick<ProcView, "cwd"> | null): string {
  if (proc?.cwd) return proc.cwd;
  const pl = placeOf(pr);
  if (pl.kind === "missing") return t("cards.proj.where.missing", "{path} — složka neexistuje", { path: pl.path });
  if (pl.kind === "choose") return tp("cards.proj.where.choose", pl.count, ["{n} složka s tímhle repem", "{n} složky s tímhle repem — vyber jednu", "{n} složek s tímhle repem — vyber jednu"]);
  if (pl.kind === "none" || !pr.path) return t("cards.proj.where.none", "složka nenalezena");
  return pr.path;
}

/** Krátký tooltip dlaždice: co spustí, odkud, port (a stav, když běží / selhal). Řádky bez žargonu. */
export function tileTip(pr: ProjectView, proc?: ProcView | null): string {
  const lines: string[] = [];
  const what = whatOf(pr);
  if (pr.kind === "dev") {
    lines.push(what ? t("cards.proj.tip.run", "Klik spustí: {cmd}", { cmd: what }) : t("cards.proj.tip.noRun", "Chybí příkaz ke spuštění"));
  } else lines.push(what ? t("cards.proj.tip.app", "Klik otevře: {app}", { app: what }) : t("cards.proj.tip.noApp", "Chybí aplikace"));
  lines.push(t("cards.proj.tip.where", "Složka: {dir}", { dir: whereOf(pr, proc) }));
  const port = proc?.port ?? pr.port;
  if (pr.kind === "dev" && port) lines.push(t("cards.proj.tip.port", "Port: {port}", { port }));
  if (proc && isLive(proc.phase)) lines.push(t("cards.proj.tip.live", "Běží — ■ v akcích zástupce zastaví"));
  else if (proc?.phase === "failed") lines.push(t("cards.proj.tip.failed", "Nenaběhl: {msg}", { msg: proc.message ?? "" }));
  return lines.join("\n");
}

/** Stav do náhledu po najetí: pilulka (tón g/a/r/n + text) a případně krátká chyba. */
export function tipState(pr: ProjectView, proc?: ProcView | null): { tone: "g" | "a" | "r" | "n"; text: string; detail: string | null } {
  if (proc?.phase === "ready" || proc?.phase === "running") {
    return { tone: "g", text: proc.port ? t("cards.proj.state.port", "běží :{port}", { port: proc.port }) : t("cards.proj.state.live", "běží"), detail: null };
  }
  if (proc?.phase === "starting" || proc?.phase === "preparing") return { tone: "a", text: t("cards.proj.state.starting", "startuje…"), detail: null };
  const fail = failInfo(proc);
  if (fail) return { tone: "r", text: t("cards.proj.state.failed", "nenaběhl"), detail: fail.message };
  const problem = placeProblem(pr);
  if (problem) return { tone: placeOf(pr).kind === "choose" ? "a" : "r", text: placeOf(pr).kind === "choose" ? t("cards.proj.tile.choose", "vyber složku") : t("cards.proj.tile.missing", "složka chybí"), detail: problem };
  return { tone: "n", text: t("cards.proj.state.idle", "neběží"), detail: null };
}

/** Co klik na dlaždici udělá: spustit, nejdřív vybrat složku, ukázat chybu místa, nebo jen otevřít běžící. */
export type TileClick = "launch" | "choose" | "fix" | "open";
export function clickOf(pr: ProjectView, proc?: ProcView | null): TileClick {
  if (proc && isLive(proc.phase)) return "open";
  const pl = placeOf(pr);
  if (pl.kind === "choose") return "choose";
  // aplikace podle appName složku nepotřebuje
  if (pl.kind === "missing" || (pl.kind === "none" && pr.kind === "dev")) return "fix";
  return "launch";
}

/** Krátká chyba místa pro dlaždici / panel. */
export function placeProblem(pr: ProjectView): string | null {
  const pl = placeOf(pr);
  if (pl.kind === "missing") return t("cards.proj.problem.missing", "Složka {path} neexistuje.", { path: pl.path });
  if (pl.kind === "choose") return tp("cards.proj.problem.choose", pl.count, ["Našel jsem {n} složku s tímto repem.", "Našel jsem {n} složky s tímto repem. Vyber, ze které se má spouštět.", "Našel jsem {n} složek s tímto repem. Vyber, ze které se má spouštět."]);
  if (pl.kind === "none" && pr.kind === "dev") return t("cards.proj.problem.none", "Složka projektu na tomhle PC není.");
  return null;
}

/** Výběr z kandidátů: předvybraný = aktuální `path`, jinak první (jádro řadí nejčerstvější commit první). */
export function preselect(items: FolderCandidate[], current: string | null): string | null {
  const cur = current && items.find((x) => samePath(x.path, current));
  return cur ? cur.path : items[0]?.path ?? null;
}

/** Nejčerstvější = první s datem (pořadí z jádra), jen když je kandidátů víc a datum je známé. */
export function freshest(items: FolderCandidate[]): string | null {
  return items.length > 1 && items[0]?.lastCommit != null ? items[0].path : null;
}

// Porovnání cest je sdílené v `util/path.ts`; odsud se jen re-exportuje.
export { samePath };

/** „poslední commit před 3 h“ / „bez commitu“ (unix s z jádra). */
export function commitText(lastCommit: number | null, now: number): string {
  return lastCommit == null ? t("cards.proj.choose.noCommit", "datum commitu nejde zjistit") : t("cards.proj.choose.commit", "poslední commit {ago}", { ago: ago(lastCommit * 1000, now) });
}

/** Úprava projektu v configu: `path` (prázdné = smazat → dohledá se podle remote), `run` (oříznuté),
 *  `muteDirty` (ztišit upozornění na neuložený git; false = smazat klíč = výchozí chování). */
export type ProjectPatch = { path?: string | null; run?: string | null; muteDirty?: boolean | null };
export function withProjectFields(cfg: any, id: string, patch: ProjectPatch): any {
  const projects = Array.isArray(cfg?.projects) ? cfg.projects : [];
  if (!projects.some((p: any) => p?.id === id)) return cfg;
  return {
    ...cfg,
    projects: projects.map((p: any) => {
      if (p?.id !== id) return p;
      const out = { ...p };
      for (const k of ["path", "run"] as const) {
        if (!(k in patch)) continue;
        const v = patch[k]?.trim();
        if (v) out[k] = v;
        else delete out[k];
      }
      if ("muteDirty" in patch) {
        if (patch.muteDirty) out.muteDirty = true;
        else delete out.muteDirty;
      }
      return out;
    }),
  };
}

/** Selhání na dlaždici: krátká zpráva + poslední řádky (z jádra `tail`, jinak poslední řádek). */
export function failInfo(proc: ProcView | undefined | null): { message: string; tail: string[] } | null {
  if (!proc || (proc.phase !== "failed" && !(proc.phase === "exited" && proc.exitCode !== 0 && proc.exitCode != null))) return null;
  const tail = proc.tail?.length ? proc.tail : proc.lastLine ? [proc.lastLine] : [];
  return { message: proc.message ?? t("cards.proj.fail.unknown", "Server se nespustil."), tail };
}

/** Zdraví projektu na zástupci: neuložené změny, nepushnuté commity a „Claude tu pracuje“ (jen ze snapshotu). */
export interface TileHealth {
  dirty: number;
  ahead: number;
  claude: boolean;
}

/** Odznaky zástupce z `projects[].git` a `sessions[]`; vše v pořádku (nic k ukázání) = null. Session s `projectId`
 *  projektu a stavem `working` (ukončená se nepočítá). */
export function tileHealth(pr: Pick<ProjectView, "id" | "git">, sessions: readonly Pick<SessionView, "projectId" | "status" | "ended">[]): TileHealth | null {
  const dirty = Math.max(0, pr.git?.dirty ?? 0);
  const ahead = Math.max(0, pr.git?.ahead ?? 0);
  const claude = sessions.some((x) => x.projectId === pr.id && x.status === "working" && !x.ended);
  return dirty > 0 || ahead > 0 || claude ? { dirty, ahead, claude } : null;
}

/** Odstín jediné tečky (kompaktní vzhled, malé dlaždice): Claude pracuje > neuložené změny > nepushnuté. */
export const healthTone = (h: TileHealth): "claude" | "dirty" | "ahead" => (h.claude ? "claude" : h.dirty > 0 ? "dirty" : "ahead");

/** Věta do náhledu po najetí / popisu zástupce: „Claude tu pracuje · 3 neuložené změny · 2 nepushnuté commity“. */
export function healthText(h: TileHealth): string {
  const parts: string[] = [];
  if (h.claude) parts.push(t("cards.proj.health.claude", "Claude tu pracuje"));
  if (h.dirty > 0) parts.push(tp("cards.proj.health.dirty", h.dirty, ["{n} neuložená změna", "{n} neuložené změny", "{n} neuložených změn"]));
  if (h.ahead > 0) parts.push(tp("cards.proj.health.ahead", h.ahead, ["{n} nepushnutý commit", "{n} nepushnuté commity", "{n} nepushnutých commitů"]));
  return parts.join(" · ");
}

/** Kolik poznámek projektu panel ukáže (zbytek jako „+N“). */
export const PROJECT_NOTES = 3;

/** Dotaz `note_query` pro panel projektu: nehotové poznámky tohoto projektu, nejvýš `PROJECT_NOTES` (jedno volání při otevření). */
export const projectNotesQuery = (id: string) => ({ project: id, limit: PROJECT_NOTES, includeDone: false });

/** Kolik poznámek se do panelu nevešlo (`total` z `note_query` bez stropu); záporné / chybějící = 0. */
export const notesHidden = (total: number | null | undefined, shown: number): number => Math.max(0, (total ?? 0) - shown);

/** Varianta Bento karty Projekty: běžící zástupci dopředu (velké dlaždice jsou první dvě), jinak pořadí beze
 *  změny; složky zůstanou, kde jsou. Stabilní — v režimu úprav se nepoužije (tah by skákal). */
export function liveFirst<E extends { kind: string; p?: { id: string } }>(entries: readonly E[], live: (id: string) => boolean): E[] {
  const on = (e: E) => e.kind === "project" && !!e.p && live(e.p.id);
  return [...entries.filter(on), ...entries.filter((e) => !on(e))];
}

/** Stisk v liště akcí dlaždice (bod 2 zadání týden 43): na tlačítku akce patří tlačítku (dlaždice nezačne tah ani
 *  spuštění), jinde v liště je to obyčejný stisk dlaždice (tah za prázdné místo funguje). Zavírání otevřených menu
 *  (`PopMenu`, posluchač ve fázi capture na dokumentu) to nijak neovlivní. */
export function actsPress(target: { closest(sel: string): unknown } | null): "button" | "tile" {
  return target?.closest("button") ? "button" : "tile";
}
