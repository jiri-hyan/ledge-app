// Stránka „Doporučené“ (kolo 21, první po otevření Nastavení): nejčastější volby jako dlaždice s jednou
// větou, co dělají — klik skočí přímo na volbu a zvýrazní ji (`structure.ts` `RECOMMENDED`, skok přes
// `palette/goto.ts`). Pod nimi „Dobré vědět“ (funkce, které nejsou vidět na první pohled: Ctrl+K, ✎, ⋯,
// Ctrl+F) a přehled sekcí („kde co je“). Jen po přechodu z nižší verze (`startModel.ts`; poslední viděnou
// verzi drží stránka v localStorage) nahoře „Co je nového“ — a dlaždice, které jsou samy novinkou, dostanou
// štítek „Nové“ místo toho, aby se opakovaly v seznamu. Dřív byly tipy a novinky i v Jazyk a průvodce
// (duplicita). Nic needituje, jen vede dál. Texty jsou jen popis funkcí, žádná čísla.
import { For, Show } from "solid-js";
import { t } from "../../i18n";
import { gotoSettingsPanel } from "../../palette/goto";
import { PanelHead, Section } from "../fields/Row";
import { PanelIcon } from "../icons";
import { RECOMMENDED, SECTIONS } from "../structure";
import { sectionLabel, tr } from "../labels";
import { parseVersion, upgradedFrom } from "../startModel";
import type { PanelProps } from "../types";

type Item = { k: string; title: () => string; text: () => string; go?: string };

const SEEN_KEY = "ledge.settings.seenVersion";

/** Rozhodnutí „přešel z nižší verze“ platí do konce běhu okna (stránka může vzniknout znovu). */
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

/** Funkce, které nejsou vidět na první pohled (`old` = přešel z nižší verze → zmínka o starých názvech). */
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

/** Novinky, které NEjsou dlaždicí Doporučených (ty dostanou štítek „Nové“ — `NEW_TILES`). */
const NEWS: Item[] = [
  { k: "↧", title: () => t("settings.start.new.motion", "Nové animace otevírání"), text: () => t("settings.start.new.motion.text", "Vyber, jak šuplík vyjede — nebo žádnou animaci."), go: "f-appearance.motion" },
  { k: "♪", title: () => t("settings.start.new.media", "Přehrávač v několika podobách"), text: () => t("settings.start.new.media.text", "Karta Právě hraje jako řádek, karta, velký obal nebo vinyl."), go: "f-media.variant" },
  { k: "⚙", title: () => t("settings.start.new.settings", "Přehlednější Nastavení"), text: () => t("settings.start.new.settings.text", "Méně sekcí podle toho, co chceš udělat, Doporučené nahoře, chytré hledání (i anglicky a podle klíče configu) a přepínač Základní / Pokročilé.") },
];

/** Dlaždice Doporučených, které jsou samy novinkou (cíl `go` → věta do bublinky štítku „Nové“). */
const NEW_TILES: Record<string, () => string> = {
  "f-strip.look": () => t("settings.start.new.closed.text", "Zavřený Ledge ve 12 podobách, co se stane po najetí myší a monitor, na kterém je vidět skutečná šířka a hustota."),
  "f-appearance.style": () => t("settings.start.new.styles.text", "Hotové balíčky vzhledu jedním klikem, s náhledem na celém šuplíku."),
  "f-layout": () => t("settings.start.new.layout.text", "Vlastní záložky, velikost karet a přetahování přímo v šuplíku."),
};

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

export default function RecommendedPanel(p: PanelProps) {
  const up = upgraded(p.s?.update?.current);
  return (
    <>
      <PanelHead title={t("settings.rec.title", "Doporučené")} desc={t("settings.rec.desc", "Co lidé mění nejčastěji. Klikni na dlaždici a Nastavení skočí rovnou na volbu.")} />
      <Show when={up}>
        <Section title={t("settings.start.news", "Co je nového")} desc={t("settings.rec.newsDesc", "Novinky jsou označené i na dlaždicích níž štítkem „Nové“.")}>
          <List items={NEWS} />
        </Section>
      </Show>
      <div class="st-rec">
        <For each={RECOMMENDED}>
          {(x) => (
            <button class="st-rec-tile" onClick={() => gotoSettingsPanel(x.go)}>
              <span class="st-rec-ico" aria-hidden="true">{x.icon}</span>
              <span class="st-rec-txt">
                <b>
                  {tr(x.title)}
                  <Show when={up && NEW_TILES[x.go]}>
                    {(why) => <em class="st-rec-new" title={why()()}>{t("settings.rec.new", "Nové")}</em>}
                  </Show>
                </b>
                <small>{tr(x.text)}</small>
              </span>
            </button>
          )}
        </For>
      </div>
      <Section title={t("settings.rec.tips", "Dobré vědět")} desc={t("settings.rec.tipsDesc", "Funkce, které nejsou vidět na první pohled.")}>
        <List items={tips(up)} />
      </Section>
      <h3 class="st-grp-sep">{t("settings.rec.sections", "Kde co je")}</h3>
      <div class="st-rec">
        <For each={SECTIONS.filter((s) => s.id !== "doporucene")}>
          {(s) => (
            <button class="st-rec-tile" onClick={() => gotoSettingsPanel(s.pages[0])}>
              <span class="st-rec-ico" aria-hidden="true"><PanelIcon id={s.id} /></span>
              <span class="st-rec-txt">
                <b>{sectionLabel(s)}</b>
                <small>{tr(s.desc)}</small>
              </span>
            </button>
          )}
        </For>
      </div>
    </>
  );
}
