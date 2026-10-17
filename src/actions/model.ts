// Čistá logika rychlých akcí pro UI (bez DOM a Solid): id z popisku, přesun v seznamu, převod
// odkazů Osobní, viditelné skupiny podle záložky, úklid před uložením a rychlá kontrola pole.
// Autoritou je Rust (`actions.rs` validate, příkaz `action_check`); tohle je okamžitá zpětná vazba.
// Testy: `src/palette/search.test.mjs`. Texty chyb vrací jako {key, cs} — přeloží je volající `t()`.
import type { ActionCfg, ActionKind, ShowOn } from "../types";
import { fold } from "../palette/search.ts";

/** Id z popisku: bez diakritiky, a–z0–9 a pomlčky, jednoznačné vůči `taken`. */
export function slugId(label: string, taken: string[]): string {
  const base =
    fold(label)
      .replace(/[^a-z0-9]+/g, "-")
      .replace(/^-+|-+$/g, "")
      .slice(0, 40)
      .replace(/-+$/, "") || "akce";
  if (!taken.includes(base)) return base;
  for (let i = 2; ; i++) if (!taken.includes(`${base}-${i}`)) return `${base}-${i}`;
}

/** Kopie seznamu s položkou `from` přesunutou na index `to` (mimo rozsah = beze změny). */
export function moveItem<T>(list: readonly T[], from: number, to: number): T[] {
  const out = list.slice();
  if (from === to || from < 0 || from >= list.length || to < 0 || to >= list.length) return out;
  const [x] = out.splice(from, 1);
  out.splice(to, 0, x);
  return out;
}

/** Přípony, které odkaz/akce typu soubor otevře (zrcadlo `personal::FILE_ALLOW`). */
const FILE_EXT = ["txt", "md", "pdf", "doc", "docx", "xls", "xlsx", "csv", "ppt", "pptx", "odt", "ods", "png", "jpg", "jpeg", "gif", "webp", "svg", "mp3", "mp4", "mkv", "zip"];
const ICON_EXT = ["png", "ico", "jpg", "jpeg", "gif", "webp", "bmp", "svg"];
const ext = (p: string) => (/\.([a-z0-9]+)$/i.exec(p.trim())?.[1] ?? "").toLowerCase();

/** Typ cíle odhadem z textu (bez disku): web, .exe, známá přípona souboru, jinak složka. */
export function guessKind(target: string): ActionKind {
  const t = target.trim();
  if (/^https?:\/\//i.test(t)) return "url";
  const e = ext(t);
  if (e === "exe") return "app";
  return FILE_EXT.includes(e) ? "file" : "folder";
}

/** Odkazy z `personal.links` jako akce na záložce Osobní. Nic nemaže; cíl, který už akce má, přeskočí. */
export function linksToActions(links: { name: string; target: string }[], existing: ActionCfg[]): ActionCfg[] {
  const targets = new Set(existing.map((a) => a.target.trim().toLowerCase()));
  const ids = existing.map((a) => a.id);
  const out: ActionCfg[] = [];
  for (const l of links) {
    const target = l.target.trim();
    if (!target || targets.has(target.toLowerCase())) continue;
    targets.add(target.toLowerCase());
    const label = l.name.trim() || target;
    const id = slugId(label, ids);
    ids.push(id);
    out.push({ id, label, kind: guessKind(target), target, showOn: "personal" });
  }
  return out;
}

/** Akce viditelné na záložce (`prace`/`osobni`) ve skupinách; bez skupiny první, jinak pořadí výskytu. */
export function visibleGroups<T extends { showOn: ShowOn; group: string | null }>(list: readonly T[], mode: string): { group: string | null; items: T[] }[] {
  const want: ShowOn = mode === "osobni" ? "personal" : "work";
  const groups = new Map<string | null, T[]>([[null, []]]);
  for (const a of list) {
    if (a.showOn !== "both" && a.showOn !== want) continue;
    const g = a.group?.trim() || null;
    if (!groups.has(g)) groups.set(g, []);
    groups.get(g)!.push(a);
  }
  return [...groups].filter(([, items]) => items.length).map(([group, items]) => ({ group, items }));
}

/** Akce k uložení: oříznuté texty, bez prázdných a nepatřičných polí (výchozí hodnoty se nepíšou). */
export function cleanAction(a: ActionCfg): ActionCfg {
  const runs = a.kind === "app" || a.kind === "command";
  const out: ActionCfg = { id: a.id, label: a.label.trim(), kind: a.kind, target: a.target.trim() };
  const args = (a.args ?? []).map((x) => x.trim()).filter(Boolean);
  if (runs && args.length) out.args = args;
  if (runs && a.cwd?.trim()) out.cwd = a.cwd.trim();
  const icon = a.icon?.trim();
  if (icon && icon !== "auto") out.icon = icon;
  if (a.group?.trim()) out.group = a.group.trim();
  if (a.confirm) out.confirm = true;
  if (a.showOn && a.showOn !== "both") out.showOn = a.showOn;
  if (a.kind === "command") {
    if (a.shell === "powershell") out.shell = "powershell";
    if (a.terminal) out.terminal = true;
    if (a.timeoutS && a.timeoutS !== 60) out.timeoutS = a.timeoutS;
  }
  return out;
}

export type DraftError = { field: "label" | "target" | "args" | "cwd" | "icon"; key: string; cs: string };

const absPath = (p: string) => /^[a-zA-Z]:[\\/]/.test(p) && !p.slice(2).includes(":");

/** Rychlá kontrola rozepsané akce (tvar jako Rust `validate_one`); `null` = v pořádku. */
export function draftError(a: ActionCfg): DraftError | null {
  const t = a.target.trim();
  if (!t) return { field: "target", key: "actions.err.target", cs: "Vyplň cíl." };
  if (a.kind === "url" && !/^https?:\/\/[^\s/]+\S*$/i.test(t)) return { field: "target", key: "actions.err.url", cs: "Web musí začínat http:// nebo https://." };
  if ((a.kind === "file" || a.kind === "folder") && !absPath(t)) return { field: "target", key: "actions.err.abs", cs: "Zadej celou cestu (C:\\…)." };
  if (a.kind === "app" && !(/^[^\\/:"]+\.exe$/i.test(t) || (absPath(t) && ext(t) === "exe")))
    return { field: "target", key: "actions.err.exe", cs: "Aplikace musí být .exe (skript spusť jako příkaz)." };
  if (!a.label.trim() || a.label.length > 80) return { field: "label", key: "actions.err.label", cs: "Vyplň popisek (nejvýš 80 znaků)." };
  if ((a.args ?? []).some((x) => /["\r\n]/.test(x))) return { field: "args", key: "actions.err.args", cs: "Argumenty bez uvozovek (každý na vlastní řádek)." };
  if (a.cwd?.trim() && !absPath(a.cwd.trim())) return { field: "cwd", key: "actions.err.cwd", cs: "Pracovní složka musí být celá cesta (C:\\…)." };
  const icon = a.icon?.trim();
  if (icon && icon !== "auto" && icon !== "monogram" && !(absPath(icon) && ICON_EXT.includes(ext(icon))))
    return { field: "icon", key: "actions.err.icon", cs: "Ikona musí být obrázek (png, ico, jpg, svg…) s celou cestou." };
  return null;
}
