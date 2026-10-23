// Panel „Poznámky“: zapnutí, složka, kategorie (`notes.kinds` — přidat, upravit, barva, ikona, pořadí
// přetažením, smazat s Vrátit) a synchronizace mezi PC přes git (zapnutí, repo, interval, „teď“).
import { createSignal, For, Show } from "solid-js";
import { api, type NoteKind } from "../../api";
import { ago } from "../../format";
import { t } from "../../i18n";
import { KindIcon } from "../../notes/kinds";
import { say, sayError } from "../../ui/toast";
import { base, offerUndo, useSettings } from "../ctx";
import { Advanced } from "../fields/Advanced";
import { Color, PathPick, Slider, Text, Toggle } from "../fields/controls";
import { FieldErr, PanelHead, Row, Section } from "../fields/Row";
import { SortList } from "../fields/SortList";
import { moveItem } from "../model";
import type { PanelProps } from "../types";

const ICONS = ["bulb", "chat", "checklist", "note", "flag", "star", "bug", "heart"];
const MAX_KINDS = 12;

const SYNC_TEXT: Record<string, [string, string]> = {
  off: ["settings.notes.sync.st.off", "Vypnutá"],
  ok: ["settings.notes.sync.st.ok", "V pořádku"],
  syncing: ["settings.notes.sync.st.syncing", "Synchronizuji…"],
  offline: ["settings.notes.sync.st.offline", "Bez připojení"],
  neprihlaseno: ["settings.notes.sync.st.auth", "Nepřihlášeno ke GitHubu — postup je v Pokročilé"],
  konflikt: ["settings.notes.sync.st.conflict", "Konflikt — vyřeš v repu poznámek"],
  chyba: ["settings.notes.sync.st.error", "Chyba"],
};

function KindsEditor() {
  const c = useSettings();
  /** rozbalený editor podle indexu (id se při úpravě mění) */
  const [open, setOpen] = createSignal<number | null>(null);
  const kinds = (): NoteKind[] => c.cfg.notes.kinds ?? [];
  const at = (i: number, f: string) => `notes.kinds.${i}.${f}`;
  function add() {
    let n = kinds().length + 1;
    while (kinds().some((k) => k.id === `kategorie-${n}`)) n++;
    const id = `kategorie-${n}`;
    c.update("notes.kinds", (all: NoteKind[]) => [...all, { id, label: t("settings.notes.kind.new", "Nová kategorie"), color: "", icon: "" }]);
    setOpen(kinds().length - 1);
  }
  function remove(i: number) {
    const before = structuredClone(kinds());
    const k = before[i];
    c.update("notes.kinds", (all: NoteKind[]) => all.filter((_, j) => j !== i));
    setOpen(null);
    offerUndo(t("settings.notes.kind.removed", "Kategorie „{l}“ odebrána", { l: k.label }), () => c.set("notes.kinds", before));
  }
  return (
    <Row id="notes.kinds" stack nested>
      <SortList items={kinds()} label={t("settings.f.notes.kinds", "Kategorie")} onMove={(a, b) => { setOpen(null); c.set("notes.kinds", moveItem(kinds(), a, b)); }}>
        {(k, i, handle) => (
          <div class="st-item-wrap">
            <div class="st-item">
              {handle()}
              <span class="st-kind-ic" style={{ color: k.color || "var(--t3)" }}><KindIcon icon={k.icon} /></span>
              <span class="grow">{k.label}<small class="mono"> {k.id}</small></span>
              <button class="mini" aria-expanded={open() === i()} onClick={() => setOpen(open() === i() ? null : i())}>{open() === i() ? t("settings.ui.done", "Hotovo") : t("settings.ui.edit", "Upravit")}</button>
              <button class="mini" disabled={kinds().length <= 1} title={kinds().length <= 1 ? t("settings.notes.kind.last", "Aspoň jedna kategorie musí zůstat") : undefined} onClick={() => remove(i())}>{t("settings.ui.remove", "Odebrat")}</button>
            </div>
            <Show when={open() === i()}>
              <div class="st-edit">
                <label class="st-sub">{t("settings.notes.kind.label", "Název")}<Text path={at(i(), "label")} label={t("settings.notes.kind.label", "Název")} maxLength={40} /><FieldErr path={at(i(), "label")} /></label>
                <label class="st-sub">
                  {t("settings.notes.kind.id", "Id v souborech")}
                  <Text path={at(i(), "id")} label={t("settings.notes.kind.id", "Id v souborech")} mono maxLength={32} />
                  <small>{t("settings.notes.kind.id.hint", "Zapisuje se do poznámek (kind:). Změna nepřejmenuje existující poznámky.")}</small>
                  <FieldErr path={at(i(), "id")} />
                </label>
                <label class="st-sub wide">{t("settings.notes.kind.color", "Barva")}<Color path={at(i(), "color")} label={t("settings.notes.kind.color", "Barva")} allowEmpty empty="" /><FieldErr path={at(i(), "color")} /></label>
                <div class="st-sub wide">
                  {t("settings.notes.kind.icon", "Ikona")}
                  <div class="st-icons" role="radiogroup" aria-label={t("settings.notes.kind.icon", "Ikona")}>
                    <For each={["", ...ICONS]}>
                      {(ic) => (
                        <button role="radio" aria-checked={k.icon === ic} classList={{ on: k.icon === ic }} title={ic || t("settings.notes.kind.dot", "Tečka")} style={{ color: k.color || "var(--t2)" }} onClick={() => c.set(at(i(), "icon"), ic)}>
                          <KindIcon icon={ic} />
                        </button>
                      )}
                    </For>
                  </div>
                </div>
              </div>
            </Show>
          </div>
        )}
      </SortList>
      <div class="st-actions">
        <button class="mini" disabled={kinds().length >= MAX_KINDS} onClick={add}>+ {t("settings.notes.kind.add", "Přidat kategorii")}</button>
        <Show when={kinds().length >= MAX_KINDS}><small>{t("settings.notes.kind.max", "Nejvýš {n} kategorií", { n: MAX_KINDS })}</small></Show>
      </div>
    </Row>
  );
}

export default function NotesPanel(_p: PanelProps) {
  const c = useSettings();
  const n = () => c.cfg.notes;
  const [syncing, setSyncing] = createSignal(false);
  /** synchronizuje se podle ULOŽENÉHO nastavení — rozpracovaná změna sync by tu mátla */
  const syncDirty = () => {
    try {
      return JSON.stringify(JSON.parse(base()).notes?.sync) !== JSON.stringify(n().sync);
    } catch {
      return false;
    }
  };
  async function syncNow() {
    setSyncing(true);
    try {
      say((await api.notesSyncNow()) ?? t("settings.notes.sync.noTauri", "Mimo Ledge nejde synchronizovat."));
    } catch (e) {
      sayError(e);
    } finally {
      setSyncing(false);
    }
  }
  const st = () => c.s.notesSync;
  return (
    <>
      <PanelHead id="notes.enabled" title={t("settings.nav.poznamky", "Poznámky")} desc={t("settings.notes.desc", "Rychlé poznámky a připomínky jsou obyčejné Markdown soubory ve tvé složce.")}>
        <Toggle path="notes.enabled" label={t("settings.f.notes.enabled", "Rychlé poznámky a připomínky")} />
      </PanelHead>
      <Show when={n().enabled} fallback={<p class="st-empty">{t("settings.notes.off", "Poznámky jsou vypnuté — karta, připomínky i zkratka zmizí. Soubory na disku zůstanou.")}</p>}>
        <Section>
          <Row id="notes.root" stack>
            <PathPick path="notes.root" kind="folder" label={t("settings.f.notes.root", "Složka poznámek")} placeholder={t("settings.notes.root.ph", "výchozí: Ledge\\notes v profilu")} />
          </Row>
          <KindsEditor />
        </Section>
        <Section title={t("settings.notes.sync", "Synchronizace mezi počítači")}>
          <Row id="notes.sync.enabled"><Toggle path="notes.sync.enabled" label={t("settings.f.notes.sync.enabled", "Synchronizace mezi počítači")} /></Row>
          <Show when={n().sync.enabled}>
            <Row
              id="notes.sync.now"
              label={t("settings.notes.sync.now", "Synchronizovat teď")}
              hint={syncDirty() ? t("settings.notes.sync.saveFirst", "Nejdřív ulož změny — synchronizuje se podle uloženého nastavení.") : undefined}
            >
              <span class="st-status">
                <span class="st-dot" classList={{ ok: st().state === "ok", warn: st().state === "offline" || st().state === "syncing", bad: ["neprihlaseno", "konflikt", "chyba"].includes(st().state) }} />
                <span>
                  {t(...(SYNC_TEXT[st().state] ?? ["", st().state]))}
                  <Show when={st().lastOkMs}><small>{t("settings.notes.sync.last", "naposledy {when}", { when: ago(st().lastOkMs!) })}</small></Show>
                  <Show when={st().message}><small>{st().message}</small></Show>
                </span>
                <button class="mini" disabled={syncing() || syncDirty()} onClick={syncNow}>{syncing() ? t("settings.notes.sync.running", "Synchronizuji…") : t("settings.notes.sync.now", "Synchronizovat teď")}</button>
              </span>
            </Row>
            <Advanced id="poznamky" inline hint={t("settings.notes.adv.hint", "Repo pro poznámky, přihlášení ke gitu a interval")}>
              <Row id="notes.sync.remote" stack>
                <Text path="notes.sync.remote" label={t("settings.f.notes.sync.remote", "Repo pro poznámky")} placeholder={t("settings.notes.sync.remote.ph", "https://github.com/ty/poznamky.git")} mono grow />
              </Row>
              <Row id="notes.sync.intervalMin">
                <Slider path="notes.sync.intervalMin" min={1} max={120} label={t("settings.f.notes.sync.intervalMin", "Jak často synchronizovat")} fmt={(v) => t("settings.notes.sync.every", "každých {v} min", { v })} />
              </Row>
            </Advanced>
          </Show>
        </Section>
      </Show>
    </>
  );
}
