// Export poznámky ven: Markdown / HTML / PDF do souboru (dialog Uložit jako v Rustu), tělo do schránky a „Otevřít v …“
// aplikacích, které má Windows registrované pro .md. Společné pro menu editoru (⋯) i lístku; výsledek vždy krátká
// hláška, chyba textem z jádra.

import { createSignal } from "solid-js";
import { api } from "../api";
import { say, sayError } from "../ui/toast";
import { t } from "../i18n";
import { trimBody } from "./editdraft";

/** Uloží poznámku jako `.md` / `.html` / `.pdf` (PDF tiskne skryté WebView2 v Rustu, pár sekund); zrušený dialog nic neřekne. */
export async function exportNote(path: string, format: "md" | "html" | "pdf") {
  try {
    const out = await api.noteExport(path, format);
    if (out) say(t("notes.export.saved", "Uloženo do {path}", { path: out }));
  } catch (e) {
    sayError(e);
  }
}

/** Tělo poznámky (Markdown) do schránky. */
export async function copyMarkdown(body: string) {
  try {
    await navigator.clipboard.writeText(trimBody(body));
    say(t("notes.export.copied", "Zkopírováno"));
  } catch {
    say(t("notes.export.copyFailed", "Do schránky se nepodařilo zkopírovat."), true);
  }
}

/** Aplikace pro „Otevřít v …“ — načtou se jednou za okno, při prvním otevření menu. */
const [apps, setApps] = createSignal<{ id: string; name: string }[]>([]);
let appsAsked = false;
function ensureApps() {
  if (appsAsked) return;
  appsAsked = true;
  api.noteOpenWithApps().then(setApps).catch(() => {});
}

/** Otevře poznámku v registrované aplikaci; chyba = česká hláška z jádra. */
export async function openWith(path: string, app: string) {
  try {
    await api.noteOpenWith(path, app);
  } catch (e) {
    sayError(e);
  }
}

/** Volby exportu pro poznámku `path` (`body` = co se zkopíruje jako Markdown). Sdílí editor i lístek. */
export function exportItems(path: string, body: () => string): { label: string; run: () => void; /** kopie do schránky (lístek ji má zvlášť) */ copy?: boolean }[] {
  ensureApps();
  return [
    { label: t("notes.export.md", "Uložit jako Markdown…"), run: () => void exportNote(path, "md") },
    { label: t("notes.export.html", "Uložit jako HTML…"), run: () => void exportNote(path, "html") },
    { label: t("notes.export.pdf", "Uložit jako PDF…"), run: () => void exportNote(path, "pdf") },
    { label: t("notes.export.copy", "Kopírovat jako Markdown"), run: () => void copyMarkdown(body()), copy: true },
    ...apps().map((a) => ({ label: t("notes.export.openWith", "Otevřít v {name}", { name: a.name }), run: () => void openWith(path, a.id) })),
  ];
}
