// Panel „Začínáme“ (dřív „Průvodce“): úplně nahoře jazyk rozhraní, pak průvodce prvním spuštěním (stav + ruční
// spuštění znovu, vždy od začátku; samotný průvodce je stránka šuplíku `welcome`, `src/onboarding/Wizard.tsx`),
// tipy, které nejsou vidět na první pohled (Ctrl+K, ✎ úpravy šuplíku, ⋯ na kartě, hledání v Nastavení), a co je
// nového — každá položka s odkazem na místo v Nastavení (`gotoSettingsPanel`). „Co je nového“ a zmínka o starých
// názvech v tipu o hledání jsou jen pro toho, kdo přešel z nižší verze (`startModel.ts`; poslední viděnou verzi
// drží panel v localStorage). Texty jsou jen popis funkcí, žádná čísla.
import { For, Show } from "solid-js";
import { t, type Lang } from "../../i18n";
import { startWizard } from "../../onboarding/state";
import { askEdit } from "../../layout/edit";
import { gotoSettingsPanel } from "../../palette/goto";
import { dirty } from "../ctx";
import { Seg } from "../fields/controls";
import { PanelHead, Row, Section } from "../fields/Row";
import { fieldLabel } from "../labels";
import { parseVersion, upgradedFrom } from "../startModel";
import type { PanelProps } from "../types";
import { groupName, SHORTCUT_GROUPS, shortcutsOf, shortcutText } from "../../ui/shortcuts";

type Item = { k: string; title: () => string; text: () => string; go?: string };

/** Názvy jazyků vždy v jejich vlastním jazyce (ať se k češtině vrátí i ten, kdo anglicky neumí). */
const LANGS: { v: Lang; label: string }[] = [{ v: "cs", label: "Čeština" }, { v: "en", label: "English" }];

const SEEN_KEY = "ledge.settings.seenVersion";

/** Rozhodnutí „přešel z nižší verze“ platí do konce běhu okna (panel se při přepínání vytváří znovu). */
let upgradedNow: boolean | null = null;

/** Přečte poslední viděnou verzi, rozhodne a zapíše současnou (úložiště nemusí být k dispozici = nový uživatel). */
function upgraded(current: string | undefined): boolean {
  if (upgradedNow !== null) return upgradedNow;
  let seen: string | null = null;
  try {
    seen = localStorage.getItem(SEEN_KEY);
  } catch {
    // bez úložiště se „Co je nového“ neukáže
  }
  upgradedNow = upgradedFrom(seen, current);
  if (current && parseVersion(current)) {
    try {
      localStorage.setItem(SEEN_KEY, current);
    } catch {
      // jen pohodlí
    }
  }
  return upgradedNow;
}

const tips = (old: boolean): Item[] => [
  { k: "Ctrl+K", title: () => t("settings.start.tip.palette", "Paleta příkazů"), text: () => t("settings.start.tip.palette.text", "Ctrl+K nebo 🔍 v hlavičce: najde projekty, akce, poznámky i volby Nastavení, spočítá příklad a převede jednotky."), go: "f-palette.compact" },
  { k: "✎", title: () => t("settings.start.tip.edit", "Úpravy přímo v šuplíku"), text: () => t("settings.start.tip.edit.text", "Tužka v hlavičce šuplíku: karty přetáhneš, změníš jim velikost, přidáš z katalogu a založíš vlastní záložky.") },
  { k: "⋯", title: () => t("settings.start.tip.card", "Nabídka karty"), text: () => t("settings.start.tip.card.text", "Tři tečky v rohu každé karty: skrýt kartu nebo jen její část, nebo rovnou upravit šuplík.") },
  {
    k: "Ctrl+F",
    title: () => t("settings.start.tip.search", "Hledání v Nastavení"),
    text: () => (old ? t("settings.start.tip.search.text", "Najde každou volbu bez háčků a čárek — i podle starých názvů z dřívějších verzí.") : t("settings.start.tip.search.new", "Najde každou volbu, i když nevíš, jak se jmenuje, a bez háčků a čárek.")),
  },
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
  const up = upgraded(p.s?.update?.current);
  const uiLang = (): Lang => (p.cfg.ui?.lang === "en" ? "en" : "cs");
  const setUiLang = (v: Lang) => (p.cfg.ui ? p.setCfg("ui", "lang", v) : p.setCfg("ui", { lang: v }));
  return (
    <>
      <PanelHead title={t("settings.nav.zaciname", "Začínáme")} desc={t("settings.start.desc", "Rychlý start, pár tipů a co je v Ledge nového. Vlevo je Nastavení rozdělené podle toho, co chceš změnit.")} />
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
        <Row id="appearance.pick" label={t("settings.start.look", "Vyber si vzhled")} hint={t("settings.start.look.hint", "Jak Ledge vypadá zavřený u horní hrany, po najetí myší a otevřený — galerie s náhledy.")}>
          <button class="mini" onClick={() => gotoSettingsPanel("f-strip.look")}>◐ {t("settings.start.look.btn", "Vybrat")}</button>
        </Row>
      </Section>
      <Section title={t("settings.start.tips", "Tipy")}>
        <List items={tips(up)} />
      </Section>
      <Section id="onboarding.keys" title={t("ui.keys.title", "Klávesy a gesta")}>
        <KeysList />
      </Section>
      <Show when={up}>
        <Section title={t("settings.start.news", "Co je nového")}>
          <List items={NEWS} />
        </Section>
      </Show>
    </>
  );
}
