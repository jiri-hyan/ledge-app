// Složky zástupců v kartě Projekty (vlna 8 P): přetažení dlaždice na jinou = složka (jako na telefonu),
// na složku = do ní, tah ze složky ven = vyjmout; prázdná složka zmizí. Model = `tiles.folders` karty
// Projekty na záložce (zrcadlo `tilelook::TileFolder`). Čistá logika bez importů za běhu (test `folders.test.mjs`).

export type TileFolder = { id: string; name: string; items: string[] };
/** Volby dlaždic, se kterými složky pracují (podmnožina `TileOpts` z `model.ts`). */
type WithFolders = { folders?: TileFolder[] };

export const MAX_FOLDER_NAME = 32;
const idOk = (id: unknown): id is string => typeof id === "string" && /^[a-z0-9-]{1,24}$/.test(id);

/** Obranné čtení: platné id (každé jednou), název ořezaný na 32 znaků, projekt jen v jedné složce,
 *  prázdné složky pryč. */
export function normalizeFolders(raw: unknown): TileFolder[] {
  const out: TileFolder[] = [];
  const inside = new Set<string>();
  for (const f of Array.isArray(raw) ? raw : []) {
    const r = (f ?? {}) as Record<string, unknown>;
    if (!idOk(r.id) || out.some((x) => x.id === r.id)) continue;
    const items = (Array.isArray(r.items) ? r.items : []).filter((i): i is string => typeof i === "string" && !!i && !inside.has(i));
    const uniq = [...new Set(items)];
    if (!uniq.length) continue;
    uniq.forEach((i) => inside.add(i));
    out.push({ id: r.id, name: typeof r.name === "string" ? [...r.name].slice(0, MAX_FOLDER_NAME).join("") : "", items: uniq });
  }
  return out;
}

const foldersOf = (t: WithFolders) => normalizeFolders(t.folders);
const put = <T extends WithFolders>(t: T, folders: TileFolder[]): T => {
  const out = { ...t };
  const norm = normalizeFolders(folders);
  if (norm.length) out.folders = norm;
  else delete out.folders;
  return out;
};

/** Složka, ve které projekt je (nebo undefined). */
export const folderOf = (t: WithFolders, projectId: string): TileFolder | undefined => foldersOf(t).find((f) => f.items.includes(projectId));

/** Id nové složky: `slozka`, `slozka-2`… */
export function folderIdFor(t: WithFolders): string {
  const taken = new Set(foldersOf(t).map((f) => f.id));
  if (!taken.has("slozka")) return "slozka";
  for (let i = 2; ; i++) if (!taken.has(`slozka-${i}`)) return `slozka-${i}`;
}

/** Vyjme projekt ze složky (prázdná složka zmizí). */
export function takeOut<T extends WithFolders>(t: T, projectId: string): T {
  return put(t, foldersOf(t).map((f) => ({ ...f, items: f.items.filter((i) => i !== projectId) })));
}

/** Projekt do složky (z jiné složky se přesune). */
export function addToFolder<T extends WithFolders>(t: T, folderId: string, projectId: string): T {
  const base = takeOut(t, projectId);
  const fs = foldersOf(base);
  if (!fs.some((f) => f.id === folderId)) return t;
  return put(base, fs.map((f) => (f.id === folderId ? { ...f, items: [...f.items, projectId] } : f)));
}

/** Dlaždice `dragged` puštěná na dlaždici `target`: cíl ve složce = přidat do ní, jinak nová složka
 *  (cíl první, pak tažená). Sama na sebe = beze změny. Vrací volby a id složky. */
export function dropOnTile<T extends WithFolders>(t: T, dragged: string, target: string, name = ""): { tiles: T; folder: string | null } {
  if (dragged === target) return { tiles: t, folder: null };
  const home = folderOf(t, target);
  if (home) return { tiles: addToFolder(t, home.id, dragged), folder: home.id };
  const base = takeOut(takeOut(t, dragged), target);
  const id = folderIdFor(base);
  return { tiles: put(base, [...foldersOf(base), { id, name: [...name].slice(0, MAX_FOLDER_NAME).join(""), items: [target, dragged] }]), folder: id };
}

export function renameFolder<T extends WithFolders>(t: T, folderId: string, name: string): T {
  return put(t, foldersOf(t).map((f) => (f.id === folderId ? { ...f, name: [...name.trim()].slice(0, MAX_FOLDER_NAME).join("") } : f)));
}

/** Zruší složku (projekty zůstanou na kartě volně). */
export function removeFolder<T extends WithFolders>(t: T, folderId: string): T {
  return put(t, foldersOf(t).filter((f) => f.id !== folderId));
}

export type TileEntry<P> = { kind: "project"; p: P } | { kind: "folder"; f: TileFolder; items: P[] };

/** Co karta ukáže v pořadí projektů: volné dlaždice a složky (složka na místě svého prvního projektu;
 *  jen viditelné projekty, neexistující id se ignorují, složka bez viditelných se neukáže). */
export function arrange<P extends { id: string }>(projects: readonly P[], t: WithFolders): TileEntry<P>[] {
  const folders = foldersOf(t);
  const byId = new Map(projects.map((p) => [p.id, p]));
  const out: TileEntry<P>[] = [];
  const shown = new Set<string>();
  for (const p of projects) {
    const f = folders.find((x) => x.items.includes(p.id));
    if (!f) out.push({ kind: "project", p });
    else if (!shown.has(f.id)) {
      shown.add(f.id);
      out.push({ kind: "folder", f, items: f.items.map((i) => byId.get(i)).filter((x): x is P => !!x) });
    }
  }
  return out;
}
