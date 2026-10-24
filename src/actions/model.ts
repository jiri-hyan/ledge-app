// Čistá logika rychlých akcí pro UI (bez DOM a Solid): id z popisku, přesun v seznamu, odkazy
// (akce, které jen otevírají), viditelné skupiny podle záložky, úklid před uložením a rychlá kontrola pole.
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

/** Odkaz = akce, která jen něco otevře (web, okno s webem, složka, soubor) — ukazuje je karta Odkazy.
 *  Starý `personal.links` převádí na akce Rust (`linkmigrate.rs`) při načtení configu. */
export const LINK_KINDS: readonly ActionKind[] = ["url", "web", "folder", "file"];
export const isLink = (a: { kind: ActionKind; script?: boolean }) => !a.script && LINK_KINDS.includes(a.kind);

/** Patří akce na záložku `mode`? Práce = pro práci, Osobní = pro osobní, vlastní záložka = všechny. */
export function onPage(a: { showOn: ShowOn }, mode: string): boolean {
  const want: ShowOn | null = mode === "osobni" ? "personal" : mode === "prace" ? "work" : null;
  return !want || a.showOn === "both" || a.showOn === want;
}

/** Odkazy (akce `isLink`) viditelné na záložce `mode`, v pořadí seznamu akcí (karta Odkazy). */
export function linkActions<T extends { kind: ActionKind; showOn: ShowOn; script?: boolean }>(list: readonly T[], mode: string): T[] {
  return list.filter((a) => isLink(a) && onPage(a, mode));
}

/** Akce viditelné na záložce `mode` (id z `layout`) ve skupinách; bez skupiny první, jinak pořadí výskytu.
 *  Práce = akce pro práci, Osobní = pro osobní, vlastní záložka = všechny. */
export function visibleGroups<T extends { showOn: ShowOn; group: string | null }>(list: readonly T[], mode: string): { group: string | null; items: T[] }[] {
  const groups = new Map<string | null, T[]>([[null, []]]);
  for (const a of list) {
    if (!onPage(a, mode)) continue;
    const g = a.group?.trim() || null;
    if (!groups.has(g)) groups.set(g, []);
    groups.get(g)!.push(a);
  }
  return [...groups].filter(([, items]) => items.length).map(([group, items]) => ({ group, items }));
}

/** Popisek z cíle, když ho člověk nevyplnil: host webu, jméno souboru / složky. */
export function labelFrom(target: string): string {
  const s = target.trim();
  const host = /^https?:\/\/(?:www\.)?([^/:?#]+)/i.exec(s)?.[1];
  if (host) return host;
  const name = s.replace(/[\\/]+$/, "").split(/[\\/]/).pop() ?? "";
  return name.replace(/\.exe$/i, "");
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
  if (a.kind === "web") {
    const w = Math.round(Number(a.width)), h = Math.round(Number(a.height));
    if (w && w !== WEB_SIZE.w) out.width = w;
    if (h && h !== WEB_SIZE.h) out.height = h;
  }
  // vzhled dlaždice (šuplík → Upravit vzhled…) editor akce nemění, jen ho přenese
  if (a.look && Object.keys(a.look).length) out.look = a.look;
  if (a.kind === "command") {
    if (a.shell === "powershell") out.shell = "powershell";
    if (a.terminal) out.terminal = true;
    if (a.timeoutS && a.timeoutS !== 60) out.timeoutS = a.timeoutS;
  }
  return out;
}

/** Okno s webem (`webpanel.rs`): výchozí velikost a meze (logické px). */
export const WEB_SIZE = { w: 480, h: 720, minW: 320, maxW: 3840, minH: 240, maxH: 2160 };

export type DraftError = { field: "label" | "target" | "args" | "cwd" | "icon" | "size"; key: string; cs: string };

const absPath = (p: string) => /^[a-zA-Z]:[\\/]/.test(p) && !p.slice(2).includes(":");

/** Rychlá kontrola rozepsané akce (tvar jako Rust `validate_one`); `null` = v pořádku. */
export function draftError(a: ActionCfg): DraftError | null {
  const t = a.target.trim();
  if (!t) return { field: "target", key: "actions.err.target", cs: "Vyplň cíl." };
  if ((a.kind === "url" || a.kind === "web") && !/^https?:\/\/[^\s/]+\S*$/i.test(t)) return { field: "target", key: "actions.err.url", cs: "Web musí začínat http:// nebo https://." };
  if (a.kind === "web") {
    const host = /^https?:\/\/([^/?#]*)/i.exec(t)?.[1] ?? "";
    if (host.includes("@")) return { field: "target", key: "actions.err.webCreds", cs: "Adresa nesmí obsahovat jméno ani heslo." };
    if (/\.localhost\.?(:\d+)?$/i.test(host)) return { field: "target", key: "actions.err.webLocal", cs: "Adresy *.localhost patří Ledge, okno s webem je neotevře." };
    const bad = (v: number | undefined, lo: number, hi: number) => v != null && !(Number.isInteger(v) && v >= lo && v <= hi);
    if (bad(a.width, WEB_SIZE.minW, WEB_SIZE.maxW) || bad(a.height, WEB_SIZE.minH, WEB_SIZE.maxH))
      return { field: "size", key: "actions.err.size", cs: "Velikost okna: šířka 320–3840, výška 240–2160 px." };
  }
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
