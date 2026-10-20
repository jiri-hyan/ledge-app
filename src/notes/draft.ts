// Stav editoru poznámky, který přežije odchod ze stránky: koncept NOVÉ poznámky (paměť + disk
// `<root>\.draft\`, s debounce) a rozepsaná ÚPRAVA existující (paměť + `.draft\edit.json`).
// Win+Shift+S ukradne fokus a šuplík se zavře — rozepsané nesmí zmizet. Plus obrázky ze schránky.

import { createSignal } from "solid-js";
import { api, type NoteDetail } from "../api";
import { say } from "../ui/toast";
import { fromMs } from "./remind";
import { DEFAULT_KIND } from "./kinds";
import { planOther, planSame } from "./editdraft";
import { t } from "../i18n";

// ---------- nová poznámka ----------
export const [draft, setDraft] = createSignal("");
export const [draftImgs, setDraftImgs] = createSignal<string[]>([]);
export const [draftProj, setDraftProj] = createSignal("");
export const [projTouched, setProjTouched] = createSignal(false);
export const [draftRemind, setDraftRemind] = createSignal("");
export const [draftKind, setDraftKind] = createSignal(DEFAULT_KIND);

let draftTimer: number | undefined;
export const draftMeta = () => ({ text: draft(), project: draftProj(), projectTouched: projTouched(), remind: draftRemind(), kind: draftKind() === DEFAULT_KIND ? "" : draftKind() });
export const b64Of = (urls: string[]) => urls.map((u) => u.slice(u.indexOf(",") + 1));

/** Zapsat koncept na disk; prázdný koncept (nic napsáno, nic vybráno) = smazat. */
function persistDraft() {
  const m = draftMeta();
  const empty = !m.text.trim() && !draftImgs().length && !m.remind && !m.projectTouched && !m.kind;
  (empty ? api.noteDraftClear() : api.noteDraftSave(m)).catch(() => {});
}

export function scheduleDraft() {
  clearTimeout(draftTimer);
  draftTimer = window.setTimeout(() => {
    draftTimer = undefined;
    persistDraft();
  }, 500);
}

/** Zapsat hned, pokud čeká debounce (zavření stránky do 500 ms od psaní). */
export function flushDraft() {
  if (draftTimer !== undefined) {
    clearTimeout(draftTimer);
    draftTimer = undefined;
    persistDraft();
  }
  flushEdit();
}

export function resetDraft() {
  clearTimeout(draftTimer);
  draftTimer = undefined;
  setDraft("");
  setDraftImgs([]);
  setDraftProj("");
  setProjTouched(false);
  setDraftRemind("");
  setDraftKind(DEFAULT_KIND);
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
  setDraftImgs(d.images.map((b) => `data:image/png;base64,${b}`));
}

// ---------- úprava existující poznámky ----------
type Fields = { text: string; project: string; kind: string; remind: string };
export const [edBase, setEdBase] = createSignal<NoteDetail | null>(null);
/** otisk verze, nad kterou se upravuje (z `note_read` nebo z obnovené úpravy) */
export const [edHash, setEdHash] = createSignal("");
export const [edText, setEdText] = createSignal("");
export const [edProj, setEdProj] = createSignal("");
export const [edKind, setEdKind] = createSignal(DEFAULT_KIND);
export const [edRemind, setEdRemind] = createSignal("");
export const [edImgs, setEdImgs] = createSignal<string[]>([]);
const [orig, setOrig] = createSignal<Fields | null>(null);

export const edDirty = () => {
  const o = orig();
  return !!o && (edImgs().length > 0 || o.text !== edText() || o.project !== edProj() || o.kind !== edKind() || o.remind !== edRemind());
};

/** Naplní úpravu z poznámky na disku (po čtení, přepnutí úkolu). */
export function loadEditFrom(n: NoteDetail) {
  const f: Fields = { text: n.body, project: n.project ?? "", kind: n.kind, remind: fromMs(n.remindMs) };
  setEdBase(n);
  setEdHash(n.hash);
  setOrig(f);
  setEdText(f.text);
  setEdProj(f.project);
  setEdKind(f.kind);
  setEdRemind(f.remind);
  setEdImgs([]);
}

let editTimer: number | undefined;
function persistEdit() {
  const b = edBase();
  if (!b) return;
  // prázdný text nejde uložit — takovou úpravu na disku nedržet (dřív přebila skutečný obsah poznámky)
  const d = edDirty() && edText().trim() ? { path: b.path, hash: edHash(), text: edText(), project: edProj(), kind: edKind(), remind: edRemind() } : null;
  api.noteEditDraftSave(d).catch(() => {});
}
export function scheduleEdit() {
  clearTimeout(editTimer);
  editTimer = window.setTimeout(() => {
    editTimer = undefined;
    persistEdit();
  }, 500);
}
function flushEdit() {
  if (editTimer === undefined) return;
  clearTimeout(editTimer);
  editTimer = undefined;
  persistEdit();
}

export function resetEdit() {
  clearTimeout(editTimer);
  editTimer = undefined;
  setEdStale(false);
  setEdBase(null);
  setOrig(null);
  setEdImgs([]);
  api.noteEditDraftSave(null).catch(() => {});
}

/** Úprava na disku je zastaralá: soubor se od ní změnil (lístek, sync) — editor ukáže konflikt hned. */
export const [edStale, setEdStale] = createSignal(false);
const dropDiskEdit = () => api.noteEditDraftSave(null).catch(() => {});
const fieldsOf = (n: NoteDetail) => ({ path: n.path, hash: n.hash, text: n.body, project: n.project ?? "", kind: n.kind, remind: fromMs(n.remindMs) });

/** Otevře úpravu `path`. Vrací cestu, která se má upravovat doopravdy: rozepsaná úprava JINÉ
 *  poznámky s nějakým textem má přednost (jinak by se tiše ztratila). Prázdná nebo zastaralá úprava
 *  obsah poznámky nepřebije (`editdraft.ts`) — dřív ji přebíjela a editor byl prázdný, ač lístek
 *  na ploše ukazoval text i obrázek. */
export async function openEdit(path: string): Promise<string | null> {
  const cur = edBase();
  if (cur && edDirty()) {
    if (cur.path !== path) say(t("notes.draft.otherEdit", "Máš rozepsanou úpravu jiné poznámky — dokonči ji, nebo ji Esc zahodí."));
    return cur.path;
  }
  setEdStale(false);
  let disk = (await api.noteEditDraftLoad().catch(() => null)) ?? null;
  let n: NoteDetail | null = null;
  const other = planOther(disk, path);
  if (other.do === "redirect") {
    n = (await api.noteRead(other.path).catch(() => null)) ?? null;
    if (n) {
      say(t("notes.draft.otherEdit", "Máš rozepsanou úpravu jiné poznámky — dokonči ji, nebo ji Esc zahodí."));
      path = other.path;
    } else {
      // rozepsaná poznámka mezitím zmizela (smazaná, archivovaná) → úprava už nemá komu patřit
      void dropDiskEdit();
      disk = null;
    }
  } else if (other.do === "drop") {
    void dropDiskEdit();
    disk = null;
  }
  try {
    n ??= (await api.noteRead(path)) ?? null;
    if (!n) return null;
    loadEditFrom(n);
  } catch (e) {
    say(String(e), true);
    if (disk && disk.path === path) void dropDiskEdit();
    return null;
  }
  const plan = planSame(disk, fieldsOf(n));
  if (plan.do === "drop") void dropDiskEdit();
  else if (plan.do === "restore" && disk) {
    setEdHash(disk.hash);
    setEdText(disk.text);
    setEdProj(disk.project);
    setEdKind(disk.kind || DEFAULT_KIND);
    setEdRemind(disk.remind);
    setEdStale(plan.stale);
    say(plan.stale ? t("notes.draft.restoredStale", "Poznámka se od rozepsané úpravy změnila (třeba lístkem na ploše) — tvoje úprava zůstala tady, „Načíst znovu“ ukáže novou verzi.") : t("notes.draft.restored", "Obnovena rozepsaná úprava."));
  }
  return path;
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
