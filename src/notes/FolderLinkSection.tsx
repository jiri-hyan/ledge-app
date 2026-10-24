// Nastavení → Poznámky a lístky → Propojení → „Propojit se složkou“: výběr složky s `.md` (Obsidian vault,
// Logseq pages, Joplin, VS Code…), směr (jasná věta, co se kam zapisuje), přenášení mazání (v Pokročilé),
// „Synchronizovat teď“, poslední stav a konflikty. Skupinu Propojení dává `settings/panels/Notes.tsx`. Logika popisků je v `folderlink.ts` (test), Rust `notes/folder_sync*`.
import { createResource, createSignal, For, Show } from "solid-js";
import { api } from "../api";
import { ago } from "../format";
import { t } from "../i18n";
import { base, useSettings } from "../settings/ctx";
import { PathPick, Seg, Toggle } from "../settings/fields/controls";
import { Advanced } from "../settings/fields/Advanced";
import { Row, Section } from "../settings/fields/Row";
import { say, sayError } from "../ui/toast";
import { conflictSentence, deleteSentence, DIRECTION_IDS, directionHint, directionLabel, dirtyLink, linkCfg, openablePath, stateText, stateTone, writeSentence } from "./folderlink";

export default function FolderLinkSection() {
  const c = useSettings();
  const cfg = () => linkCfg(c.cfg.notes);
  const view = () => c.s.notesFolder;
  const [busy, setBusy] = createSignal(false);
  const dirty = () => dirtyLink(base(), cfg());
  /** po výběru složky: kolik poznámek v ní je, nebo proč ji propojit nejde */
  const [found] = createResource(
    () => (cfg().enabled && cfg().path.trim() ? cfg().path.trim() : null),
    async (path) => {
      try {
        const r = await api.noteFolderCheck(path);
        return r === undefined ? null : { n: r.n, warn: r.warn ?? null };
      } catch (e) {
        return { err: String(e) };
      }
    },
  );

  async function syncNow() {
    setBusy(true);
    try {
      say((await api.noteFolderSyncNow()) ?? t("notes.folder.noTauri", "Mimo Ledge nejde synchronizovat."));
    } catch (e) {
      sayError(e);
    } finally {
      setBusy(false);
    }
  }

  async function clear() {
    try {
      await api.noteFolderClearConflicts();
    } catch (e) {
      sayError(e);
    }
  }

  return (
    <Section title={t("notes.folder.title", "Propojit se složkou")}>
      <Row id="notes.folderLink.enabled" label={t("notes.folder.enabled", "Propojit poznámky se složkou")} hint={t("notes.folder.enabled.hint", "Poznámky se kopírují jako obyčejné .md soubory do složky a z ní, kterou umí jiné aplikace (vault Obsidianu, stránky Logseq, export Joplinu, VS Code…). Výchozí vypnuto, nic nejde po síti.")}>
        <Toggle path="notes.folderLink.enabled" label={t("notes.folder.enabled", "Propojit poznámky se složkou")} />
      </Row>
      <Show when={cfg().enabled}>
        <Row id="notes.folderLink.path" stack label={t("notes.folder.path", "Složka")}>
          <PathPick path="notes.folderLink.path" kind="folder" label={t("notes.folder.path", "Složka")} placeholder={t("notes.folder.path.ph", "např. D:\\Obsidian\\MujVault")} />
          <Show when={found()}>
            {(f) => (
              <Show when={"n" in f()} fallback={<small class="st-err" role="alert">{(f() as { err: string }).err}</small>}>
                <>
                  <small>{t("notes.folder.found", "Ve složce je {n} poznámek (.md).", { n: (f() as { n: number }).n })}</small>
                  <Show when={(f() as { warn: string | null }).warn}>{(w) => <small class="st-err" role="alert">{w()}</small>}</Show>
                </>
              </Show>
            )}
          </Show>
        </Row>
        <Row id="notes.folderLink.direction" stack label={t("notes.folder.dir", "Co se kopíruje")} hint={writeSentence(cfg().direction)}>
          <Seg path="notes.folderLink.direction" label={t("notes.folder.dir", "Co se kopíruje")} options={DIRECTION_IDS.map((id) => ({ v: id, label: directionLabel(id), hint: directionHint(id) }))} />
        </Row>
        <Advanced id="poznamky.folder" inline hint={t("notes.folder.adv.hint", "Přenášení mazání na druhou stranu")}>
          <Row id="notes.folderLink.propagateDeletes" label={t("notes.folder.deletes", "Přenášet mazání")} hint={deleteSentence(cfg().propagateDeletes)}>
            <Toggle path="notes.folderLink.propagateDeletes" label={t("notes.folder.deletes", "Přenášet mazání")} />
          </Row>
        </Advanced>
        <p class="st-sec-desc">{conflictSentence()}</p>
        <Show when={c.cfg.notes.sync?.enabled && cfg().direction !== "export"}>
          <p class="st-sec-desc">{t("notes.folder.git", "Zapnutá je i synchronizace mezi počítači — poznámky načtené ze složky se nahrají i do repa poznámek.")}</p>
        </Show>
        <Row id="notes.folderLink.now" label={t("notes.folder.now", "Synchronizovat teď")} hint={dirty() ? t("notes.folder.saveFirst", "Nejdřív ulož změny — synchronizuje se podle uloženého nastavení.") : undefined}>
          <span class="st-status">
            <span class="st-dot" classList={{ ok: stateTone(view()) === "ok", warn: stateTone(view()) === "warn", bad: stateTone(view()) === "bad" }} />
            <span>
              {stateText(view())}
              <Show when={view()?.lastMs}><small>{t("notes.folder.last", "naposledy {when}", { when: ago(view()!.lastMs!) })}</small></Show>
              <Show when={view()?.message}><small>{view()!.message}</small></Show>
            </span>
            <button class="mini" disabled={busy() || dirty() || !cfg().path.trim()} onClick={syncNow}>{busy() ? t("notes.folder.running", "Synchronizuji…") : t("notes.folder.now", "Synchronizovat teď")}</button>
          </span>
        </Row>
        <Show when={view()?.conflicts.length}>
          <Row id="notes.folderLink.conflicts" stack label={t("notes.folder.conflicts", "Konflikty")}>
            <For each={view()!.conflicts}>
              {(k) => (
                <div class="st-item">
                  <span class="grow">
                    {k.name}
                    <small>{k.kept === "folder" ? t("notes.folder.conflict.kept", "Zůstala verze ze složky, verze z Ledge je uložená jako kopie") : t("notes.folder.conflict.keptLedge", "Zůstala verze z Ledge, verze ze složky je uložená jako kopie")}</small>
                  </span>
                  <Show when={openablePath(k)}>{(p) => <button class="mini" onClick={() => api.noteOpen(p())}>{t("notes.folder.conflict.open", "Otevřít kopii")}</button>}</Show>
                </div>
              )}
            </For>
            <div class="st-actions"><button class="mini" onClick={clear}>{t("notes.folder.conflict.ok", "Rozumím")}</button></div>
          </Row>
        </Show>
      </Show>
    </Section>
  );
}
