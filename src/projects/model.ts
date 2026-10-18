// Zástupci projektů — čistá logika „co se spustí a odkud“ (vlna 9 Z): stav místa (`Place` z jádra), text
// tooltipu a panelu Upravit…, živé / selhané procesy, výběr složky z víc klonů a zápis `path` / `run` do
// configu. Bez DOM a bez IPC (test `model.test.mjs`); komponenty jsou `ProjectPanel.tsx`, `RunningPill.tsx`.
import type { FolderCandidate, Place, ProcView, ProjectView } from "../types.ts";
import { t, tp } from "../i18n.ts";
import { ago } from "../format.ts";

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
  if (proc && isLive(proc.phase)) lines.push(t("cards.proj.tip.live", "Běží — ■ v akcích dlaždice zastaví"));
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

export const samePath = (a: string, b: string) => norm(a) === norm(b);
const norm = (p: string) => p.replace(/[\\/]+$/, "").replace(/\//g, "\\").toLowerCase();

/** „poslední commit před 3 h“ / „bez commitu“ (unix s z jádra). */
export function commitText(lastCommit: number | null, now: number): string {
  return lastCommit == null ? t("cards.proj.choose.noCommit", "datum commitu nejde zjistit") : t("cards.proj.choose.commit", "poslední commit {ago}", { ago: ago(lastCommit * 1000, now) });
}

/** Úprava projektu v configu: `path` (prázdné = smazat → dohledá se podle remote), `run` (oříznuté). */
export type ProjectPatch = { path?: string | null; run?: string | null };
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
