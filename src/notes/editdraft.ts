// Rozhodnutí, co s rozepsanou úpravou z disku (`<root>\.draft\edit.json`), když se otevírá poznámka.
// Čistá funkce bez DOM a Tauri (test `editdraft.test.mjs`); volá ji `draft.ts` `openEdit`.

import { samePath } from "../pin/model.ts";

/** Tělo poznámky bez koncových zalomení: Rust ho dřív vracel s <LF> na konci, teď bez — editor i lístek
 *  pracují vždy s oříznutým (jinak by kurzor skákal na prázdný řádek a text by se zdál změněný). */
export const trimBody = (body: string) => body.replace(/[\r\n]+$/, "");

/** Rozepsaná úprava na disku (Rust `notes::edit::EditDraft`). */
export type EditDraftLike = { path: string; hash: string; text: string; project: string; kind: string; remind: string; /** vlastní nadpis; chybí/null = nadpis z disku (starší `edit.json`) */ title?: string | null };
/** Poznámka tak, jak je teď na disku (z `note_read`), v tvaru polí editoru. */
export type DiskNote = { path: string; hash: string; text: string; project: string; kind: string; remind: string; title?: string };

export type DraftPlan =
  /** rozepsaná úprava je k jiné poznámce a stojí za dokončení → otevřít požadovanou a tu nabídnout v liště
   *  („Otevřít ji · Zahodit“); nikdy ji tiše nepřepnout ani neztratit */
  | { do: "redirect"; path: string }
  /** obnovit text rozepsané úpravy; `stale` = soubor se od ní změnil (lístek, sync) → ukázat konflikt */
  | { do: "restore"; stale: boolean }
  /** nic nevracet, úpravu na disku smazat (prázdná, shodná s diskem nebo k zmizelé poznámce) */
  | { do: "drop" }
  /** nic nevracet a nic nemazat */
  | { do: "ignore" };

const same = (a: string, b: string) => a.trimEnd() === b.trimEnd();

/** Prázdný text nejde uložit (Rust `check_content`) — taková úprava není co zachraňovat. */
export const draftIsJunk = (d: EditDraftLike) => !d.text.trim();

/** Úprava jiné poznámky, než která se otevírá: přednost má jen ta, v níž něco opravdu je. */
export function planOther(d: EditDraftLike | null, requested: string): DraftPlan {
  if (!d || samePath(d.path, requested)) return { do: "ignore" };
  return draftIsJunk(d) ? { do: "drop" } : { do: "redirect", path: d.path };
}

/** Úprava téže poznámky proti jejímu aktuálnímu stavu na disku. `imgs` = úprava (z paměti) nese nové obrázky,
 *  takže i bez změny textu stojí za obnovu; změněný jen nadpis se pozná z `d.title`. */
export function planSame(d: EditDraftLike | null, n: DiskNote, extra: { imgs?: boolean } = {}): DraftPlan {
  if (!d || !samePath(d.path, n.path)) return { do: "ignore" };
  if (draftIsJunk(d)) return { do: "drop" };
  const titleSame = d.title == null || d.title === (n.title ?? "");
  const noChange = same(d.text, n.text) && d.project === n.project && d.kind === n.kind && d.remind === n.remind && titleSame && !extra.imgs;
  if (noChange) return { do: "drop" };
  return { do: "restore", stale: d.hash !== n.hash };
}

/** Pole konceptu NOVÉ poznámky (`draft.ts` `draftMeta`) — co se píše na disk. */
export type DraftMetaLike = { text: string; projectTouched: boolean; remind: string; kind: string; title: string };

/** Autosave konceptu: prázdný koncept (nic napsáno, nic vybráno) se z disku maže, jinak se zapíše. */
export const planDraftPersist = (m: DraftMetaLike, imgCount: number): "clear" | "save" =>
  !m.text.trim() && !imgCount && !m.remind && !m.projectTouched && !m.kind && !m.title.trim() ? "clear" : "save";

/** Koncept z disku (Rust `DraftLoad`) → pole editoru; chybějící `kind`/`title` dostanou výchozí hodnoty. */
export type DiskDraftLike = { text: string; project: string; projectTouched: boolean; remind: string; kind?: string; title?: string; images: string[] };
export const draftFields = (d: DiskDraftLike, defaultKind: string) => ({
  text: d.text,
  project: d.project,
  projectTouched: d.projectTouched,
  remind: d.remind,
  kind: d.kind || defaultKind,
  title: d.title ?? "",
  imgs: d.images.map((b) => `data:image/png;base64,${b}`),
});

/** Obnova z disku jen do prázdného konceptu: co už se mezitím napsalo, se nepřepíše. */
export const canRestoreDraft = (memText: string, memImgCount: number, d: unknown) => !!d && !memText && !memImgCount;

/** Rozepsaná úprava (otevřená nebo odložená) v tvaru pro disk (`edit.json`); null = na disku nemá být nic. */
export type EditSnap = { base: { path: string }; hash: string; text: string; project: string; kind: string; remind: string; title?: string };
export const toDiskEdit = (s: EditSnap | null): EditDraftLike | null =>
  s ? { path: s.base.path, hash: s.hash, text: s.text, project: s.project, kind: s.kind, remind: s.remind, ...(s.title !== undefined ? { title: s.title } : {}) } : null;

/** Odložení úpravy: nová verze téže poznámky nahradí starou, jinak se přidá nakonec (nejnovější = `top`). */
export const parkInto = <T extends { base: { path: string } }>(list: T[], x: T): T[] => [...list.filter((y) => !samePath(y.base.path, x.base.path)), x];
