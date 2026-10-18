// Nastavení Script Commands (panel „Projekty, akce a odkazy“, sekce pod akcemi): zapnutí, složka, dlaždice v kartě, „Otevřít složku
// skriptů“ (založí ji), co Ledge ve složce našel (s problémy) a formát hlavičky `@ledge.*`.
// Config přes `useSettings()` (`Toggle`/`PathPick` s `path`), seznam z `scripts_list` (sken jen na
// otevření panelu a na „Načíst znovu“).
import { createSignal, For, onMount, Show } from "solid-js";
import type { ScriptsListing } from "../types";
import { scriptsApi } from "./scripts";
import { ActionIcon } from "./ui";
import { PathPick, Toggle } from "../settings/fields/controls";
import { Row } from "../settings/fields/Row";
import { useSettings } from "../settings/ctx";
import { say, sayError } from "../ui/toast";
import { t, tp } from "../i18n";

export default function ScriptsSettings() {
  const c = useSettings();
  const [list, setList] = createSignal<ScriptsListing | null>(null);
  const load = () => scriptsApi.list().then((l) => setList(l ?? null), sayError);
  onMount(load);

  async function open() {
    try {
      const dir = await scriptsApi.openDir();
      if (dir) say(t("actions.scripts.opened", "Složka skriptů: {dir}", { dir }));
      await load();
    } catch (e) {
      sayError(e);
    }
  }

  return (
    <div class="card wide" data-anim>
      <h4>
        <span class="ic" style="background:var(--violet)" />
        {t("actions.scripts.title", "Skripty")}
        <span class="n">{list()?.scripts.length || ""}</span>
        <button class="mini" onClick={load}>{t("actions.scripts.reload", "Načíst znovu")}</button>
        <button class="mini" onClick={open}>{t("actions.scripts.open", "Otevřít složku skriptů")}</button>
      </h4>
      <div class="empty" style="padding-top:0">
        {t("actions.scripts.intro", "Vlastní skripty (.ps1, .cmd, .bat, .py, .js, .mjs) ze složky skriptů se objeví v paletě (Ctrl+K). Skript je cizí kód: Ledge ho spustí až po tvém potvrzení — poprvé a po každé změně souboru znovu.")}
      </div>
      <Row id="scripts.enabled">
        <Toggle path="scripts.enabled" label={t("settings.f.scripts.enabled", "Skripty v paletě")} />
      </Row>
      <Row id="scripts.dir" stack>
        <PathPick path="scripts.dir" kind="folder" nullable label={t("settings.f.scripts.dir", "Složka skriptů")} placeholder={list()?.dir || "%USERPROFILE%\\Ledge\\scripts"} />
      </Row>
      <Row id="scripts.inActions" dim={c.cfg.scripts?.enabled === false}>
        <Toggle path="scripts.inActions" label={t("settings.f.scripts.inActions", "Skripty i v kartě Rychlé akce")} />
      </Row>
      <Show when={list()}>
        {(l) => (
          <Show
            when={l().exists}
            fallback={<div class="empty">{l().enabled ? t("actions.scripts.noDir", "Složka zatím neexistuje — „Otevřít složku skriptů“ ji založí.") : t("actions.scripts.off", "Skripty jsou vypnuté.")}</div>}
          >
            <div class="act-list">
              <For each={l().scripts} fallback={<div class="empty">{t("actions.scripts.none", "Ve složce zatím není žádný skript s hlavičkou @ledge.title.")}</div>}>
                {(s) => (
                  <div class="act-row">
                    <ActionIcon a={s} size={26} />
                    <div class="grow">
                      <div class="name">{s.label}</div>
                      <div class="sub" classList={{ err: !!s.problem }}>{s.problem ?? [s.target, s.description].filter(Boolean).join(" · ")}</div>
                    </div>
                    <span class="pill n">{s.mode}</span>
                  </div>
                )}
              </For>
              <Show when={l().skipped > 0}>
                <div class="empty">{tp("actions.scripts.skipped", l().skipped, ["Další {n} soubor se nenačetl (strop 200).", "Další {n} soubory se nenačetly (strop 200).", "Dalších {n} souborů se nenačetlo (strop 200)."])}</div>
              </Show>
              <Show when={l().error}>{(e) => <div class="sub err">{e()}</div>}</Show>
            </div>
          </Show>
        )}
      </Show>
      <details class="act-help">
        <summary>{t("actions.scripts.format", "Formát hlavičky")}</summary>
        <pre class="log">{t("actions.scripts.example", "# @ledge.title Vyčistit temp\n# @ledge.description Smaže dočasné soubory starší než týden\n# @ledge.icon 🧹\n# @ledge.mode output\n# @ledge.confirm\n# @ledge.args -Days 7\n# @ledge.timeout 120")}</pre>
        <div class="empty">
          {t("actions.scripts.help", "Řádky @ledge.* v komentáři na začátku souboru (prvních 4 KB): # u .ps1 a .py, :: nebo REM u .cmd/.bat, // u .js/.mjs. Povinný je jen title. mode: output (výchozí, výstup v šuplíku), hidden (jen hotovo / chyba), terminal (viditelné okno). confirm = ptát se před každým spuštěním. args = pevné argumenty (mezery v uvozovkách). timeout = sekundy bez okna (výchozí 60). Python a Node musí být na PATH; .ps1 s diakritikou ulož jako UTF-8 s BOM.")}
        </div>
      </details>
    </div>
  );
}
