// Stav editoru poznámky, který přežije odchod ze stránky: koncept NOVÉ poznámky (paměť + disk
// `<root>\.draft\`, s debounce) a rozepsaná ÚPRAVA existující (paměť + `.draft\edit.json`).
// Win+Shift+S ukradne fokus a šuplík se zavře — rozepsané nesmí zmizet. Plus obrázky ze schránky.

import { createSignal } from "solid-js";
import { api, type EditDraft, type NoteDetail } from "../api";
import { say } from "../ui/toast";
import { fromMs } from "./remind";
import { DEFAULT_KIND } from "./kinds";
import { planOther, planSame, trimBody } from "./editdraft";
import { samePath } from "../pin/model";
import { debounce } from "../util/time";
import { t } from "../i18n";

// ---------- nová poznámka ----------
export const [draft, setDraft] = createSignal("");
export const [draftImgs, setDraftImgs] = createSignal<string[]>([]);
export const [draftProj, setDraftProj] = createSignal("");
export const [projTouched, setProjTouched] = createSignal(false);
export const [draftRemind, setDraftRemind] = createSignal("");
export const [draftKind, setDraftKind] = createSignal(DEFAULT_KIND);
/** vlastní nadpis (`title:`); prázdný = název je první řádek */
export const [draftTitle, setDraftTitle] = createSignal("");

export const draftMeta = () => ({ text: draft(), project: draftProj(), projectTouched: projTouched(), remind: draftRemind(), kind: draftKind() === DEFAULT_KIND ? "" : draftKind(), title: draftTitle() });
export const b64Of = (urls: string[]) => urls.map((u) => u.slice(u.indexOf(",") + 1));

/** Zapsat koncept na disk; prázdný koncept (nic napsáno, nic vybráno) = smazat. */
function persistDraft() {
  const m = draftMeta();
  const empty = !m.text.trim() && !draftImgs().length && !m.remind && !m.projectTouched && !m.kind && !m.title.trim();
  (empty ? api.noteDraftClear() : api.noteDraftSave(m)).catch(() => {});
}

const draftSave = debounce(persistDraft, 500);
export const scheduleDraft = () => draftSave();

/** Zapsat hned, pokud čeká debounce (zavření stránky do 500 ms od psaní). */
export function flushDraft() {
  draftSave.flush();
  flushEdit();
}

export function resetDraft() {
  draftSave.cancel();
  setDraft("");
  setDraftImgs([]);
  setDraftProj("");
  setProjTouched(false);
  setDraftRemind("");
  setDraftKind(DEFAULT_KIND);
  setDraftTitle("");
  api.noteDraftClear().catch(() => {});
}

/** Koncept z disku (WebView mohl být zrušený), jen když v paměti nic není. */
export async function restoreDraft() {
  if (draft() || draftImgs().length) return;
  const d = await api.noteDraftLoad().catch(() => null);
  if (!d || draft() || draftImgs().length) return;
  setDraft(d.text);
  setDraftProj(d.project);
  setProjTouched(d.projectTouched);
  setDraftRemind(d.remind);
  setDraftKind(d.kind || DEFAULT_KIND);
  setDraftTitle(d.title ?? "");
  setDraftImgs(d.images.map((b) => `data:image/png;base64,${b}`));
}

// ---------- úprava existující poznámky ----------
type Fields = { text: string; project: string; kind: string; remind: string; title: string };
export const [edBase, setEdBase] = createSignal<NoteDetail | null>(null);
/** otisk verze, nad kterou se upravuje (z `note_read` nebo z obnovené úpravy) */
export const [edHash, setEdHash] = createSignal("");
export const [edText, setEdText] = createSignal("");
export const [edProj, setEdProj] = createSignal("");
export const [edKind, setEdKind] = createSignal(DEFAULT_KIND);
export const [edTitle, setEdTitle] = createSignal("");
export const [edRemind, setEdRemind] = createSignal("");
export const [edImgs, setEdImgs] = createSignal<string[]>([]);
const [orig, setOrig] = createSignal<Fields | null>(null);

export const edDirty = () => {
  const o = orig();
  return !!o && (edImgs().length > 0 || o.text !== edText() || o.project !== edProj() || o.kind !== edKind() || o.remind !== edRemind() || o.title !== edTitle());
};

/** Naplní úpravu z poznámky na disku (po čtení, přepnutí úkolu). Tělo bez koncových zalomení. */
export function loadEditFrom(n: NoteDetail) {
  const f: Fields = { text: trimBody(n.body), project: n.project ?? "", kind: n.kind, remind: fromMs(n.remindMs), title: n.heading ?? "" };
  setEdBase(n);
  setEdHash(n.hash);
  setOrig(f);
  setEdText(f.text);
  setEdProj(f.project);
  setEdKind(f.kind);
  setEdRemind(f.remind);
  setEdTitle(f.title);
  setEdImgs([]);
}

/** Rozepsaná úprava, která se nenechá přepsat otevřením jiné poznámky: čeká v liště editoru
 *  („Máš rozepsanou úpravu «název» — Otevřít ji · Zahodit“), dokud šéf nerozhodne. Jen v paměti
 *  (na disku drží `edit.json` tu nejnovější) — po restartu se vrátí z disku jako jedna. */
export type ParkedEdit = { base: NoteDetail; hash: string; text: string; project: string; kind: string; remind: string; imgs: string[]; /** chybí = nadpis z disku */ title?: string };
export const [parked, setParked] = createSignal<ParkedEdit[]>([]);
export const parkedTop = (): ParkedEdit | null => {
  const a = parked();
  return a.length ? a[a.length - 1] : null;
};
const hasParked = (path: string) => parked().some((x) => samePath(x.base.path, path));
function park(x: ParkedEdit) {
  setParked([...parked().filter((y) => !samePath(y.base.path, x.base.path)), x]);
}

/** Rozepsaná úprava právě otevřené poznámky (null = nic, co by stálo za uložení: bez změny nebo bez textu). */
function snapshot(): ParkedEdit | null {
  const b = edBase();
  if (!b || !edDirty() || !edText().trim()) return null;
  return { base: b, hash: edHash(), text: edText(), project: edProj(), kind: edKind(), remind: edRemind(), imgs: edImgs(), title: edTitle() };
}

/** Co má být na disku v `edit.json`: rozepsaná otevřená úprava, jinak nejnovější odložená, jinak nic. */
function diskEdit(): EditDraft | null {
  const s = snapshot() ?? parkedTop();
  return s ? { path: s.base.path, hash: s.hash, text: s.text, project: s.project, kind: s.kind, remind: s.remind } : null;
}

function persistEdit() {
  const d = diskEdit();
  if (!edBase() && !d) return;
  api.noteEditDraftSave(d).catch(() => {});
}
const editSave = debounce(persistEdit, 500);
export const scheduleEdit = () => editSave();
const flushEdit = () => editSave.flush();

export function resetEdit() {
  editSave.cancel();
  setEdStale(false);
  setEdBase(null);
  setOrig(null);
  setEdImgs([]);
  api.noteEditDraftSave(diskEdit()).catch(() => {}); // odložená úprava na disku přežije
}

/** Úprava na disku je zastaralá: soubor se od ní změnil (lístek, sync) — editor ukáže konflikt hned. */
export const [edStale, setEdStale] = createSignal(false);
const dropDiskEdit = () => api.noteEditDraftSave(diskEdit()).catch(() => {});
const fieldsOf = (n: NoteDetail) => ({ path: n.path, hash: n.hash, text: trimBody(n.body), project: n.project ?? "", kind: n.kind, remind: fromMs(n.remindMs) });

/** Pořadí otevírání: dokončí se jen poslední čtení — pozdě doběhlé čtení jiné poznámky nesmí přepsat editor
 *  (a uložení pak zapsat do cizí poznámky). */
let openSeq = 0;
/** Zneplatní rozběhnutá čtení (odchod z editoru, otevření nové poznámky). */
export const cancelOpen = () => void ++openSeq;
/** Je čtení `id` už přebité novějším? (použije editor po vlastním `await`) */
export const openSuperseded = (id: number) => id !== openSeq;

/** Otevře úpravu `path`. Vrací cestu (otevřená), `null` (poznámku nejde přečíst) nebo `undefined` (čtení
 *  mezitím přebilo novější otevření — volající nedělá nic). Soubor se čte VŽDY znovu; rozepsaná úprava téže
 *  poznámky (paměť i disk) se obnoví až nad čerstvým čtením a otiskem (`planSame`): změnila-li se poznámka
 *  mezitím, zůstane tvoje úprava a editor ukáže konflikt. Rozepsaná úprava JINÉ poznámky se nikdy tiše
 *  nepřepne ani neztratí: požadovaná poznámka se otevře a ta rozepsaná čeká v `parked` (lišta editoru). */
export async function openEdit(path: string): Promise<string | null | undefined> {
  const id = ++openSeq;
  const cur = edBase();
  let mem: ParkedEdit | null = null;
  if (cur) {
    const snap = snapshot();
    if (snap) {
      if (samePath(cur.path, path)) mem = snap;
      else park(snap);
    }
  }
  if (!mem) {
    // tuhle poznámku jsme předtím odložili (otevřeli jsme jinou) → rozepsané se vrací, ne ztrácí
    const pk = parked().find((x) => samePath(x.base.path, path));
    if (pk) {
      setParked(parked().filter((x) => x !== pk));
      mem = pk;
    }
  }
  setEdStale(false);
  let disk = mem ? null : ((await api.noteEditDraftLoad().catch(() => null)) ?? null);
  if (id !== openSeq) return undefined;
  const other = planOther(disk, path);
  if (other.do === "redirect" && disk) {
    if (!hasParked(other.path)) {
      const pn = (await api.noteRead(other.path).catch(() => null)) ?? null;
      if (id !== openSeq) return undefined;
      // rozepsaná poznámka mezitím zmizela (smazaná, archivovaná) → úprava už nemá komu patřit
      if (pn) park({ base: pn, hash: disk.hash, text: disk.text, project: disk.project, kind: disk.kind || DEFAULT_KIND, remind: disk.remind, imgs: [] });
      else void dropDiskEdit();
    }
    disk = null;
  } else if (other.do === "drop") {
    void dropDiskEdit();
    disk = null;
  }
  let n: NoteDetail | null = null;
  try {
    n = (await api.noteRead(path)) ?? null;
  } catch (e) {
    if (id !== openSeq) return undefined;
    say(String(e), true);
    if (disk && samePath(disk.path, path)) void dropDiskEdit();
    return null;
  }
  if (id !== openSeq) return undefined;
  if (!n) return null;
  loadEditFrom(n);
  const draft = mem ? { path: mem.base.path, hash: mem.hash, text: mem.text, project: mem.project, kind: mem.kind, remind: mem.remind } : disk;
  let plan = planSame(draft, fieldsOf(n));
  if (plan.do === "drop" && mem && mem.title !== undefined && mem.title !== (n.heading ?? "")) plan = { do: "restore", stale: mem.hash !== n.hash }; // změněný jen nadpis
  if (plan.do === "drop" && mem?.imgs.length) plan = { do: "restore", stale: mem.hash !== n.hash }; // změněný jen obrázky
  if (plan.do === "drop") {
    if (!mem) void dropDiskEdit();
  } else if (plan.do === "restore" && draft) {
    setEdHash(draft.hash);
    setEdText(draft.text);
    setEdProj(draft.project);
    setEdKind(draft.kind || DEFAULT_KIND);
    setEdRemind(draft.remind);
    if (mem?.title !== undefined) setEdTitle(mem.title);
    setEdImgs(mem?.imgs ?? []);
    setEdStale(plan.stale);
    if (!mem) say(plan.stale ? t("notes.draft.restoredStale", "Poznámka se od rozepsané úpravy změnila (třeba lístkem na ploše) — tvoje úprava zůstala tady, „Načíst znovu“ ukáže novou verzi.") : t("notes.draft.restored", "Obnovena rozepsaná úprava."));
  }
  return path;
}

/** Odložená úprava se vrátí do editoru (rozepsaná úprava právě otevřené poznámky se odloží místo ní).
 *  Vrací cestu k otevření — volající ji nastaví jako `editPath`, `openEdit` nad ní ověří otisk. */
export function resumeParked(): string | null {
  const pk = parkedTop();
  if (!pk) return null;
  const cur = snapshot();
  setParked(parked().filter((x) => x !== pk));
  if (cur) park(cur);
  loadEditFrom(pk.base);
  setEdHash(pk.hash);
  setEdText(pk.text);
  setEdProj(pk.project);
  setEdKind(pk.kind || DEFAULT_KIND);
  setEdRemind(pk.remind);
  if (pk.title !== undefined) setEdTitle(pk.title);
  setEdImgs(pk.imgs);
  return pk.base.path;
}

/** Odložená úprava se zahodí (lišta „Zahodit“). */
export function dropParked() {
  const pk = parkedTop();
  if (!pk) return;
  setParked(parked().filter((x) => x !== pk));
  api.noteEditDraftSave(diskEdit()).catch(() => {});
}

export type SyncResult = "same" | "reloaded" | "conflict" | "gone";
/** Poznámka se mezitím mohla změnit zvenku (psaní v lístku, sync z druhého PC): přečte ji a porovná otisk.
 *  Úprava beze změn se rovnou aktualizuje (`reloaded`), rozepsaná dostane konflikt (`conflict`). */
export async function syncEdit(): Promise<SyncResult> {
  const b = edBase();
  if (!b) return "same";
  const id = openSeq;
  let n: NoteDetail | null | undefined;
  try {
    n = await api.noteRead(b.path);
  } catch {
    return "gone";
  }
  if (!n) return "gone";
  if (id !== openSeq || edBase() !== b) return "same"; // mezitím se otevřelo něco jiného
  if (n.hash === edHash()) return "same";
  if (edDirty()) return "conflict";
  loadEditFrom(n);
  return "reloaded";
}

/** „Ponechat moje“: zůstane rozepsaný text, otisk se přepne na aktuální soubor — další uložení ho přepíše. */
export async function keepMine(): Promise<boolean> {
  const b = edBase();
  if (!b) return false;
  const n = await api.noteRead(b.path);
  if (!n || edBase() !== b) return false;
  setEdHash(n.hash);
  setEdStale(false);
  return true;
}

// ---------- obrázky ze schránky ----------
const MAX_IMG = 15 * 1024 * 1024;
const MAX_IMGS = 10;
const MAX_IMGS_TOTAL = 30 * 1024 * 1024;
const imgBytes = (url: string) => ((url.length - url.indexOf(",") - 1) * 3) / 4;

/** Obrázek ze schránky → PNG data URL (Win+Shift+S dává PNG; jiné formáty převede canvas). */
async function toPngDataUrl(file: File): Promise<string> {
  if (file.type === "image/png") {
    return new Promise((res, rej) => {
      const r = new FileReader();
      r.onload = () => res(String(r.result));
      r.onerror = () => rej(r.error);
      r.readAsDataURL(file);
    });
  }
  const bmp = await createImageBitmap(file);
  const cv = document.createElement("canvas");
  cv.width = bmp.width;
  cv.height = bmp.height;
  cv.getContext("2d")!.drawImage(bmp, 0, 0);
  bmp.close();
  return cv.toDataURL("image/png");
}

/** Vložení ze schránky: obrázky přidá do `imgs` (s limity hned, ne až při uložení). */
export async function pasteImages(e: ClipboardEvent, imgs: () => string[], setImgs: (v: string[]) => void) {
  const items = [...(e.clipboardData?.items ?? [])];
  const files = items.filter((i) => i.kind === "file" && i.type.startsWith("image/")).map((i) => i.getAsFile()).filter((f): f is File => !!f);
  if (!files.length) return;
  if (!items.some((i) => i.type === "text/plain")) e.preventDefault();
  for (const f of files) {
    if (imgs().length >= MAX_IMGS) return say(t("notes.draft.maxImages", "Nejvýš {max} obrázků na poznámku.", { max: MAX_IMGS }), true);
    const total = imgs().reduce((a, u) => a + imgBytes(u), 0);
    if (f.size > MAX_IMG || total + f.size > MAX_IMGS_TOTAL) return say(t("notes.draft.tooBigBoth", "Obrázky dohromady nesmí mít víc než 30 MB (jeden max 15 MB)."), true);
    try {
      const url = await toPngDataUrl(f);
      if (imgBytes(url) > MAX_IMG) {
        say(t("notes.draft.tooBigOne", "Obrázek je větší než 15 MB."), true);
        continue;
      }
      if (total + imgBytes(url) > MAX_IMGS_TOTAL) return say(t("notes.draft.tooBigTotal", "Obrázky dohromady nesmí mít víc než 30 MB."), true);
      setImgs([...imgs(), url]);
    } catch {
      say(t("notes.draft.unreadable", "Obrázek ze schránky nejde přečíst."), true);
    }
  }
}
