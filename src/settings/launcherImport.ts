// Import zástupců z katalogu Launcheru (`catalog.json`, zadání týden 43 bod 4: Launcher sloučit do Ledge;
// kolo 18). Čistá logika bez DOM: položky `items[]` → projekty Ledge, `family` → složka
// zástupců (název z `families[<id>].label` katalogu, jinak id s velkým písmenem) na záložce s kartou Projekty.
//  - `web.run` (+ `web.port`, `subdir`) → dev server; jinak první `actions[]` s `exec` (+ `run`, `args`) → aplikace;
//    bez příkazu (CLI nástroje, sbírky) se položka přeskočí.
//  - `state: "disabled"` → projekt vypnutý (`enabled: false`), ať se nic nespustí omylem.
//  - Duplicita (stejné id, složka nebo remote jako existující projekt) se přeskočí — import jde pustit znovu.
// Pojistka spouštění: klik na zástupce se nepotvrzuje, proto každý příkaz projde v jádře stejnou kontrolou jako MCP
// (`runguard.rs`, příkaz `launcher_import_check`); co neprojde, se naimportuje vypnuté (`withVerdicts`). Před
// převzetím UI ukáže seznam příkazů k potvrzení (`commandLines`). Výsledek jde jen do návrhu Nastavení: uloží ho až
// společná lišta Uložit (vidíš a můžeš Zahodit). Ledge z katalogu nic nespouští.
import { t } from "../i18n.ts";

export interface ImportResult {
  nextCfg: any;
  added: string[];
  /** id přidaných projektů (pro kontrolu příkazů) */
  addedIds: string[];
  skipped: string[];
}

/** Verdikt pojistky z jádra: `problem` = proč se zástupce importuje vypnutý. */
export type Verdict = { id: string; problem: string | null };

/** Co poslat jádru ke kontrole: jen pole, ze kterých se spouští. */
export function commandsOf(cfg: any, ids: readonly string[]): { id: string; path?: string; run?: string; exe?: string; args?: string[] }[] {
  const pick = (p: any) => ({ id: p.id, ...(p.path ? { path: p.path } : {}), ...(p.run ? { run: p.run } : {}), ...(p.exe ? { exe: p.exe } : {}), ...(p.args?.length ? { args: p.args } : {}) });
  return (cfg?.projects ?? []).filter((p: any) => ids.includes(p?.id)).map(pick);
}

/** Zástupci s problémem (a ti, o kterých jádro nic neřeklo) se vypnou; ostatní beze změny. Nový config. */
export function withVerdicts(cfg: any, ids: readonly string[], verdicts: readonly Verdict[]): any {
  const ok = new Set(verdicts.filter((v) => !v.problem).map((v) => v.id));
  return { ...cfg, projects: (cfg.projects ?? []).map((p: any) => (ids.includes(p?.id) && !ok.has(p.id) ? { ...p, enabled: false } : p)) };
}

/** Řádky potvrzení: co bude který zástupce spouštět, a u vypnutých proč. */
export function commandLines(cfg: any, ids: readonly string[], verdicts: readonly Verdict[]): { name: string; command: string; problem: string | null }[] {
  const why = new Map(verdicts.map((v) => [v.id, v.problem]));
  return (cfg?.projects ?? [])
    .filter((p: any) => ids.includes(p?.id))
    .map((p: any) => ({
      name: String(p.name ?? p.id),
      command: [p.run, p.exe && [p.exe, ...(p.args ?? [])].join(" ")].filter(Boolean).join(" → "),
      problem: why.has(p.id) ? (why.get(p.id) ?? null) : t("settings.proj.importUnchecked", "příkaz se nepodařilo ověřit"),
    }));
}

const HEX = /^#[0-9a-fA-F]{6}$/;
const str = (v: unknown): string => (typeof v === "string" ? v.trim() : "");
const low = (v: unknown) => str(v).toLowerCase();
/** Porovnání cest bez ohledu na lomítka, velikost písmen a koncové lomítko (Windows). */
const pathKey = (v: unknown) => low(v).replace(/\\/g, "/").replace(/\/+$/, "");

/** Id složky z rodiny: `a-z 0-9 -`, nejvýš 24 znaků (`layout/folders.ts`). */
export function folderId(family: string): string {
  const slug = family.toLowerCase().normalize("NFD").replace(/[̀-ͯ]/g, "").replace(/[^a-z0-9-]+/g, "-").replace(/-+/g, "-").replace(/^-|-$/g, "");
  return `f-${slug || "slozka"}`.slice(0, 24).replace(/-$/, "");
}

/** Název složky: popisek rodiny z katalogu, jinak id s velkým písmenem; nejvýš 32 znaků. */
export function folderName(family: string, families: unknown): string {
  const label = str((families as Record<string, any> | undefined)?.[family]?.label);
  const name = label || (family ? family.charAt(0).toUpperCase() + family.slice(1) : t("settings.proj.importFolder", "Z Launcheru"));
  return [...name].slice(0, 32).join("");
}

/** Jedna položka katalogu → projekt Ledge, nebo `null` (bez příkazu / bez id). */
export function projectOf(item: any): Record<string, unknown> | null {
  const id = str(item?.id);
  if (!id) return null;
  const run = str(item?.web?.run);
  const action = Array.isArray(item?.actions) ? item.actions.find((a: any) => str(a?.exec)) : null;
  if (!run && !action) return null;
  const p: Record<string, unknown> = { id, name: str(item.name) || id, kind: run ? "dev" : "app" };
  if (item.state === "disabled") p.enabled = false;
  if (str(item.path)) p.path = str(item.path);
  if (str(item.remote)) p.remote = str(item.remote);
  const subdir = str(item.subdir) || str(item.web?.subdir);
  if (subdir) p.subdir = subdir;
  if (run) {
    p.run = run;
    if (Number.isInteger(item.web?.port) && item.web.port > 0 && item.web.port < 65536) p.port = item.web.port;
  } else {
    // aplikace nemá port ani prohlížeč (jako u MCP: pro `app` vždy vypnuté)
    Object.assign(p, { passPort: false, openBrowser: false });
    p.exe = str(action.exec);
    if (str(action.run)) p.run = str(action.run);
    if (Array.isArray(action.args) && action.args.length) p.args = action.args.map(String);
  }
  if (HEX.test(str(item.accent))) p.accent = str(item.accent).toLowerCase();
  return p;
}

export function importLauncherCatalog(raw: any, cfg: any): ImportResult {
  const next = structuredClone(cfg ?? {});
  next.projects = Array.isArray(next.projects) ? next.projects : [];
  const ids = new Set(next.projects.map((p: any) => low(p?.id)));
  const paths = new Set(next.projects.map((p: any) => pathKey(p?.path)).filter(Boolean));
  const remotes = new Set(next.projects.map((p: any) => low(p?.remote)).filter(Boolean));
  const added: string[] = [];
  const addedIds: string[] = [];
  const skipped: string[] = [];
  /** rodina → id přidaných projektů (pořadí z katalogu) */
  const byFamily = new Map<string, string[]>();
  for (const item of Array.isArray(raw?.items) ? raw.items : []) {
    if (!item || typeof item !== "object") continue;
    const label = str(item.name) || str(item.id) || "?";
    const p = projectOf(item);
    const dup = p && (ids.has(low(p.id)) || (p.path && paths.has(pathKey(p.path))) || (p.remote && remotes.has(low(p.remote))));
    if (!p || dup) {
      skipped.push(label);
      continue;
    }
    next.projects.push(p);
    ids.add(low(p.id));
    if (p.path) paths.add(pathKey(p.path));
    if (p.remote) remotes.add(low(p.remote));
    added.push(String(p.name));
    addedIds.push(String(p.id));
    const fam = str(item.family);
    if (fam) byFamily.set(fam, [...(byFamily.get(fam) ?? []), String(p.id)]);
  }
  if (byFamily.size) placeFolders(next, byFamily, raw?.families);
  return { nextCfg: next, added, addedIds, skipped };
}

/** Složky na záložku s kartou Projekty (jinak první záložka + karta Projekty); existující složka se doplní. */
function placeFolders(cfg: any, byFamily: Map<string, string[]>, families: unknown) {
  if (!cfg.layout || typeof cfg.layout !== "object" || !Array.isArray(cfg.layout.pages) || !cfg.layout.pages.length) return;
  const pages = cfg.layout.pages;
  let page = pages.find((pg: any) => Array.isArray(pg?.cards) && pg.cards.some((c: any) => c?.id === "projects"));
  if (!page) {
    page = pages[0];
    page.cards = [{ id: "projects", size: "xl" }, ...(Array.isArray(page.cards) ? page.cards : [])];
  }
  const card = page.cards.find((c: any) => c?.id === "projects");
  card.tiles = card.tiles && typeof card.tiles === "object" ? card.tiles : { size: "tile", label: true, git: true, hidden: [] };
  const folders: any[] = Array.isArray(card.tiles.folders) ? card.tiles.folders : [];
  for (const [fam, items] of byFamily) {
    const id = folderId(fam);
    const name = folderName(fam, families);
    const f = folders.find((x) => x?.id === id || low(x?.name) === low(name));
    if (f) f.items = [...new Set([...(Array.isArray(f.items) ? f.items : []), ...items])];
    else folders.push({ id, name, items });
  }
  card.tiles.folders = folders;
}
