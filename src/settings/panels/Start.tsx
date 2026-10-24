// Stránka „Jazyk a průvodce“ (Aplikace; dřív Začínáme): úplně nahoře jazyk rozhraní, pak průvodce prvním
// spuštěním (stav + ruční spuštění znovu, vždy od začátku; samotný průvodce je stránka šuplíku `welcome`,
// `src/onboarding/Wizard.tsx`), „Upravit šuplík“ a přehled kláves a gest (`ui/shortcuts.ts`). Tipy na skryté
// funkce a „Co je nového“ jsou od kola 21 jen na stránce Doporučené (`Recommended.tsx`) — žádná duplicita.
import { For } from "solid-js";
import { t, type Lang } from "../../i18n";
import { startWizard } from "../../onboarding/state";
import { askEdit } from "../../layout/edit";
import { dirty } from "../ctx";
import { Seg } from "../fields/controls";
import { PanelHead, Row, Section } from "../fields/Row";
import { fieldLabel } from "../labels";
import type { PanelProps } from "../types";
import { groupName, SHORTCUT_GROUPS, shortcutsOf, shortcutText } from "../../ui/shortcuts";

/** Názvy jazyků vždy v jejich vlastním jazyce (ať se k češtině vrátí i ten, kdo anglicky neumí). */
const LANGS: { v: Lang; label: string }[] = [{ v: "cs", label: "Čeština" }, { v: "en", label: "English" }];

/** Klávesy a gesta z jediné tabulky `ui/shortcuts.ts` (stejný seznam otevírá v paletě „?“). */
function KeysList() {
  return (
    <For each={SHORTCUT_GROUPS}>
      {(g) => (
        <>
          <h4 class="st-keys-group">{groupName(g)}</h4>
          <ul class="st-tips">
            <For each={shortcutsOf(g.id)}>
              {(x) => (
                <li>
                  <kbd classList={{ "st-kbd-mouse": !x.keys }}>{x.keys || "🖱"}</kbd>
                  <span class="grow"><small>{shortcutText(x)}</small></span>
                </li>
              )}
            </For>
          </ul>
        </>
      )}
    </For>
  );
}

export default function StartPanel(p: PanelProps) {
  const done = () => p.cfg?.onboarding?.done !== false;
  const uiLang = (): Lang => (p.cfg.ui?.lang === "en" ? "en" : "cs");
  const setUiLang = (v: Lang) => (p.cfg.ui ? p.setCfg("ui", "lang", v) : p.setCfg("ui", { lang: v }));
  return (
    <>
      <PanelHead title={t("settings.nav.zaciname", "Jazyk a průvodce")} desc={t("settings.start.desc", "Jazyk Ledge, průvodce prvním spuštěním a přehled kláves a gest. Tipy a novinky najdeš v Doporučených.")} />
      <Section>
        <Row id="ui.lang">
          <Seg<Lang> label={fieldLabel("ui.lang")} value={uiLang()} onChange={setUiLang} options={LANGS} />
        </Row>
        <Row
          id="onboarding.wizard"
          hint={`${done() ? t("settings.welcome.done", "Proběhl nebo byl přeskočen") : t("settings.welcome.pending", "Ještě neproběhl — ukáže se při dalším otevření šuplíku")}. ${t("settings.welcome.lead", "Projde s tebou jazyk, projekty na disku, aplikace ze Start menu, Claude Code, odkazy a vzhled. Nic nemaže — jen přidává; co už v Ledge je, přeskočí.")}`}
        >
          <button class="mini ok" onClick={() => startWizard()}>{t("settings.welcome.run", "Spustit průvodce znovu")}</button>
        </Row>
        <Row id="layout.editNow" label={t("settings.start.editNow", "Upravit šuplík")} hint={dirty() ? t("settings.start.editNow.dirty", "Nejdřív ulož nebo zahoď rozpracované změny (lišta dole).") : t("settings.start.editNow.hint", "Přepne na šuplík v režimu úprav — co uděláš tam, uložíš tlačítkem Hotovo.")}>
          <button class="mini" disabled={dirty()} onClick={askEdit}>✎ {t("settings.start.editNow.btn", "Upravit")}</button>
        </Row>
      </Section>
      <Section id="onboarding.keys" title={t("ui.keys.title", "Klávesy a gesta")}>
        <KeysList />
      </Section>
    </>
  );
}
