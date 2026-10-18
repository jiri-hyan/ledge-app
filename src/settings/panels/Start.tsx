// Panel „Začínáme“ (dřív „Průvodce“): průvodce prvním spuštěním (stav + ruční spuštění, vždy od začátku;
// samotný průvodce je stránka šuplíku `welcome`, `src/onboarding/Wizard.tsx`), tipy, které nejsou vidět na
// první pohled (Ctrl+K, ✎ úpravy šuplíku, ⋯ na kartě, hledání v Nastavení), a co je nového — každá
// položka s odkazem na místo v Nastavení (`gotoSettingsPanel`). Texty jsou jen popis funkcí, žádná čísla.
import { For } from "solid-js";
import { t } from "../../i18n";
import { startWizard } from "../../onboarding/state";
import { askEdit } from "../../layout/edit";
import { gotoSettingsPanel } from "../../palette/goto";
import { dirty } from "../ctx";
import { PanelHead, Row, Section } from "../fields/Row";
import type { PanelProps } from "../types";

type Item = { k: string; title: () => string; text: () => string; go?: string };

const TIPS: Item[] = [
  { k: "Ctrl+K", title: () => t("settings.start.tip.palette", "Paleta příkazů"), text: () => t("settings.start.tip.palette.text", "Ctrl+K nebo 🔍 v hlavičce: najde projekty, akce, poznámky i volby Nastavení, spočítá příklad a převede jednotky."), go: "f-palette.compact" },
  { k: "✎", title: () => t("settings.start.tip.edit", "Úpravy přímo v šuplíku"), text: () => t("settings.start.tip.edit.text", "Tužka v hlavičce šuplíku: karty přetáhneš, změníš jim velikost, přidáš z katalogu a založíš vlastní záložky.") },
  { k: "⋯", title: () => t("settings.start.tip.card", "Nabídka karty"), text: () => t("settings.start.tip.card.text", "Tři tečky v rohu každé karty: skrýt kartu nebo jen její část, nebo rovnou upravit šuplík.") },
  { k: "Ctrl+F", title: () => t("settings.start.tip.search", "Hledání v Nastavení"), text: () => t("settings.start.tip.search.text", "Najde každou volbu bez háčků a čárek — i podle starých názvů z dřívějších verzí.") },
];

const NEWS: Item[] = [
  { k: "▭", title: () => t("settings.start.new.closed", "Vzhled po krocích"), text: () => t("settings.start.new.closed.text", "Zavřený Ledge ve 12 podobách, co se stane po najetí myší a monitor, na kterém je vidět skutečná šířka a hustota."), go: "f-strip.look" },
  { k: "◐", title: () => t("settings.start.new.styles", "Design styly"), text: () => t("settings.start.new.styles.text", "Hotové balíčky vzhledu jedním klikem, s náhledem na celém šuplíku."), go: "f-appearance.style" },
  { k: "↧", title: () => t("settings.start.new.motion", "Nové animace otevírání"), text: () => t("settings.start.new.motion.text", "Vyber, jak šuplík vyjede — nebo žádnou animaci."), go: "f-appearance.motion" },
  { k: "▦", title: () => t("settings.start.new.layout", "Karty podle sebe"), text: () => t("settings.start.new.layout.text", "Vlastní záložky, velikost karet a přetahování přímo v šuplíku."), go: "f-layout" },
  { k: "♪", title: () => t("settings.start.new.media", "Přehrávač v několika podobách"), text: () => t("settings.start.new.media.text", "Karta Právě hraje jako řádek, karta, velký obal nebo vinyl."), go: "f-media.variant" },
  { k: "⚙", title: () => t("settings.start.new.settings", "Přehlednější Nastavení"), text: () => t("settings.start.new.settings.text", "Odkazy jsou teď akce (jeden seznam), celý proužek je ve Vzhledu a vše o chování v panelu Chování."), go: "akce" },
];

function List(p: { items: Item[] }) {
  return (
    <ul class="st-tips">
      <For each={p.items}>
        {(x) => (
          <li>
            <kbd>{x.k}</kbd>
            <span class="grow">
              <strong>{x.title()}</strong>
              <small>{x.text()}</small>
            </span>
            {x.go ? <button class="mini" onClick={() => gotoSettingsPanel(x.go!)}>{t("settings.start.show", "Ukázat")}</button> : null}
          </li>
        )}
      </For>
    </ul>
  );
}

export default function StartPanel(p: PanelProps) {
  const done = () => p.cfg?.onboarding?.done !== false;
  return (
    <>
      <PanelHead title={t("settings.nav.zaciname", "Začínáme")} desc={t("settings.start.desc", "Rychlý start, pár tipů a co je v Ledge nového. Vlevo je Nastavení rozdělené podle toho, co chceš změnit.")} />
      <Section>
        <Row
          id="onboarding.wizard"
          hint={`${done() ? t("settings.welcome.done", "Proběhl nebo byl přeskočen") : t("settings.welcome.pending", "Ještě neproběhl — ukáže se při dalším otevření šuplíku")}. ${t("settings.welcome.lead", "Projde s tebou jazyk, projekty na disku, aplikace ze Start menu, Claude Code, odkazy a vzhled. Nic nemaže — jen přidává; co už v Ledge je, přeskočí.")}`}
        >
          <button class="mini ok" onClick={() => startWizard()}>{t("settings.welcome.run", "Spustit průvodce")}</button>
        </Row>
        <Row id="layout.editNow" label={t("settings.start.editNow", "Upravit šuplík")} hint={dirty() ? t("settings.start.editNow.dirty", "Nejdřív ulož nebo zahoď rozpracované změny (lišta dole).") : t("settings.start.editNow.hint", "Přepne na šuplík v režimu úprav — co uděláš tam, uložíš tlačítkem Hotovo.")}>
          <button class="mini" disabled={dirty()} onClick={askEdit}>✎ {t("settings.start.editNow.btn", "Upravit")}</button>
        </Row>
        <Row id="appearance.pick" label={t("settings.start.look", "Vyber si vzhled")} hint={t("settings.start.look.hint", "Jak Ledge vypadá zavřený u horní hrany, po najetí myší a otevřený — galerie s náhledy.")}>
          <button class="mini" onClick={() => gotoSettingsPanel("f-strip.look")}>◐ {t("settings.start.look.btn", "Vybrat")}</button>
        </Row>
      </Section>
      <Section title={t("settings.start.tips", "Tipy")}>
        <List items={TIPS} />
      </Section>
      <Section title={t("settings.start.news", "Co je nového")}>
        <List items={NEWS} />
      </Section>
    </>
  );
}
