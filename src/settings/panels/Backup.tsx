// Panel „Záloha a obnova“: export uloženého nastavení do souboru, import se souhrnem změn po sekcích
// (rizikové sekce nezaškrtnuté), obnova výchozích po sekcích s potvrzením. Import i obnova jdou jen
// do ROZPRACOVANÝCH změn (Vrátit / Zahodit) — zapíše je až Uložit, po ověření v Rustu.
import { createResource, createSignal, For, Show } from "solid-js";
import { api, type ImportPreview } from "../../api";
import { t } from "../../i18n";
import { say, sayError } from "../../ui/toast";
import { dirty, offerUndo, snapshotDraft, useSettings } from "../ctx";
import { Confirm } from "../fields/Confirm";
import { PanelHead, Row, Section } from "../fields/Row";
import type { PanelProps } from "../types";

/** Lidské názvy sekcí configu; neznámá sekce (nová oblast) = klíč. */
const SECTION_NAMES: Record<string, readonly [string, string]> = {
  projects: ["settings.section.projects", "Projekty"],
  roots: ["settings.section.roots", "Kde hledat repa"],
  drawer: ["settings.section.drawer", "Šuplík"],
  strip: ["settings.section.strip", "Proužek"],
  modes: ["settings.section.modes", "Záložky"],
  claude: ["settings.section.claude", "Claude Code"],
  hotkeys: ["settings.section.hotkeys", "Zkratky"],
  notes: ["settings.section.notes", "Poznámky"],
  personal: ["settings.section.personal", "Osobní"],
  autostart: ["settings.section.autostart", "Spouštění po startu Windows"],
  update: ["settings.section.update", "Aktualizace"],
  appearance: ["settings.section.appearance", "Vzhled a animace"],
  actions: ["settings.section.actions", "Rychlé akce"],
  questions: ["settings.section.questions", "Otázky Clauda"],
  onboarding: ["settings.section.onboarding", "Průvodce"],
  ui: ["settings.section.ui", "Jazyk"],
};
export const sectionName = (k: string) => {
  const n = SECTION_NAMES[k];
  return n ? t(n[0], n[1]) : k;
};

/** Sekce, které obnova výchozích nenabízí (projekty by zmizely celé — mažou se jednotlivě). */
const NO_RESET = new Set(["projects", "$schema"]);

const today = () => new Date().toISOString().slice(0, 10);

function ImportBox() {
  const c = useSettings();
  const [prev, setPrev] = createSignal<ImportPreview | null>(null);
  const [pick, setPick] = createSignal<Set<string>>(new Set());
  const [busy, setBusy] = createSignal(false);
  async function choose() {
    setBusy(true);
    try {
      const path = await api.pickPath("json", t("settings.backup.import.pick", "Soubor se zálohou nastavení"));
      if (!path) return;
      const p = await api.configImportPreview(path);
      if (!p) return;
      setPrev(p);
      setPick(new Set(p.sections.filter((s) => s.changed && !s.risk).map((s) => s.key)));
      if (!p.sections.some((s) => s.changed)) say(t("settings.backup.import.same", "Soubor je stejný jako tvoje nastavení — není co převzít."));
    } catch (e) {
      sayError(e);
    } finally {
      setBusy(false);
    }
  }
  function apply() {
    const p = prev();
    if (!p) return;
    const before = snapshotDraft();
    const keys = [...pick()];
    for (const k of keys) c.set(k, p.cfg[k]);
    setPrev(null);
    offerUndo(t("settings.backup.import.done", "Převzato {n} částí — zkontroluj a ulož", { n: keys.length }), () => {
      for (const k of keys) c.set(k, before[k]);
    });
  }
  const toggle = (k: string) => {
    const s = new Set(pick());
    if (s.has(k)) s.delete(k);
    else s.add(k);
    setPick(s);
  };
  return (
    <Row id="backup.import" stack>
      <div class="st-actions">
        <button class="mini" disabled={busy()} onClick={choose}>{busy() ? t("settings.backup.import.reading", "Načítám…") : t("settings.backup.import.btn", "Vybrat soubor…")}</button>
      </div>
      <Show when={prev()?.sections.some((s) => s.changed) ? prev() : null}>
        {(p) => (
          <div class="st-diff">
            <p class="st-note">{t("settings.backup.import.ask", "Co převzít? Soubor je ověřený; převzaté části půjdou do rozpracovaných změn.")}</p>
            <For each={p().sections.filter((s) => s.changed)}>
              {(s) => (
                <label class="st-diff-row" classList={{ risk: !!s.risk }}>
                  <input type="checkbox" checked={pick().has(s.key)} onChange={() => toggle(s.key)} />
                  <span class="grow">
                    <strong>{sectionName(s.key)}</strong>
                    <small class="mono">{s.detail}</small>
                    <Show when={s.risk}><small class="warn">{t("settings.backup.risk", "Pozor: {why}", { why: s.risk })}</small></Show>
                  </span>
                </label>
              )}
            </For>
            <Show when={p().sections.some((s) => !s.changed)}>
              <small class="st-note">{t("settings.backup.import.unchanged", "Beze změny: {list}", { list: p().sections.filter((s) => !s.changed).map((s) => sectionName(s.key)).join(", ") })}</small>
            </Show>
            <div class="st-actions">
              <button class="mini" onClick={() => setPrev(null)}>{t("settings.ui.cancel", "Zrušit")}</button>
              <button class="mini ok" disabled={!pick().size} onClick={apply}>{t("settings.backup.import.apply", "Převzít vybrané ({n})", { n: pick().size })}</button>
            </div>
          </div>
        )}
      </Show>
    </Row>
  );
}

function ResetBox() {
  const c = useSettings();
  const [defaults] = createResource(() => api.configDefaults());
  const [ask, setAsk] = createSignal<string | null>(null);
  const keys = () => Object.keys(defaults() ?? {}).filter((k) => !NO_RESET.has(k));
  const same = (k: string) => JSON.stringify(c.cfg[k]) === JSON.stringify(defaults()?.[k]);
  function reset(k: string) {
    setAsk(null);
    const before = snapshotDraft()[k];
    c.set(k, defaults()![k]);
    offerUndo(t("settings.backup.reset.done", "{s}: výchozí nastavení — ulož, nebo vrať", { s: sectionName(k) }), () => c.set(k, before));
  }
  return (
    <Row id="backup.reset" stack>
      <Show when={defaults()} fallback={<div class="st-skel" />}>
        <div class="st-reset">
          <For each={keys()}>
            {(k) => (
              <button class="mini" disabled={same(k)} title={same(k) ? t("settings.backup.reset.same", "Už je výchozí") : undefined} onClick={() => setAsk(k)}>
                {sectionName(k)}
              </button>
            )}
          </For>
        </div>
        <Confirm
          when={ask() !== null}
          danger
          text={t("settings.backup.reset.ask", "Obnovit výchozí: {s}?", { s: sectionName(ask() ?? "") })}
          detail={t("settings.backup.reset.detail", "Změní se jen rozpracovaná verze — zapíše se až po Uložit a do té doby jde vrátit.")}
          yes={t("settings.backup.reset.yes", "Obnovit")}
          onYes={() => reset(ask()!)}
          onNo={() => setAsk(null)}
        />
      </Show>
    </Row>
  );
}

export default function BackupPanel(_p: PanelProps) {
  const [busy, setBusy] = createSignal(false);
  async function exportCfg() {
    setBusy(true);
    try {
      const path = await api.pickSave(t("settings.backup.export.file", "ledge-nastaveni-{d}.json", { d: today() }));
      if (!path) return;
      await api.configExport(path);
      say(t("settings.backup.export.done", "Nastavení uloženo do {p}", { p: path }));
    } catch (e) {
      sayError(e);
    } finally {
      setBusy(false);
    }
  }
  return (
    <>
      <PanelHead title={t("settings.nav.zaloha", "Záloha a obnova")} desc={t("settings.backup.desc", "Přenes nastavení na jiný počítač, nebo se vrať k výchozímu.")} />
      <Section>
        <Row id="backup.export" hint={dirty() ? t("settings.backup.export.dirty", "Exportuje se uložené nastavení — rozpracované změny nejdřív ulož.") : undefined}>
          <button class="mini" disabled={busy()} onClick={exportCfg}>{t("settings.backup.export.btn", "Exportovat…")}</button>
        </Row>
        <ImportBox />
      </Section>
      <Section title={t("settings.f.backup.reset", "Obnovit výchozí")}>
        <ResetBox />
      </Section>
      <Section>
        <Row id="backup.file">
          <button class="mini" onClick={() => api.openConfigFile()?.catch(sayError)}>{t("settings.backup.file.btn", "Otevřít config.json")}</button>
        </Row>
      </Section>
    </>
  );
}
