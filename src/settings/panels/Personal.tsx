// Panel „Osobní“: místo pro počasí (Open-Meteo; pole Město s našeptávačem `CityPicker.tsx`, souřadnice v Pokročilé,
// bez polohy podle IP), kalendáře (ICS jako první volba; přihlášení účtem Microsoft / Google je pod nimi v Pokročilé,
// `accounts/AccountsSettings.tsx`), přehrávač, časovač a historie schránky (`cliphist/ClipSettings.tsx`). Nahoře
// jsou sekce karet, které jsou v rozložení šuplíku; ostatní za „Další sekce…“ (`personalModel.ts`; hledání najde
// vše a rozbalí). Rychlé odkazy jsou od vlny 6 akce (panel Projekty, akce a odkazy); na které záložce která karta je, řídí `layout`.
import { For, Show, type JSX } from "solid-js";
import { t } from "../../i18n";
import { useSettings } from "../ctx";
import { Num, Text, Toggle } from "../fields/controls";
import { Advanced } from "../fields/Advanced";
import { FieldErr, PanelHead, Row, Section } from "../fields/Row";
import { fieldHint, fieldLabel } from "../labels";
import { layoutCardIds, splitPersonal, type PersonalSectionId } from "../personalModel";
import type { PanelProps } from "../types";
import { parsePresets } from "../../timer/model";
import CalendarSettings from "../../calendar/CalendarSettings";
import AccountsSettings from "../../accounts/AccountsSettings";
import MediaSettings from "../../media/MediaSettings";
import SuggestSettings from "../../suggest/SuggestSettings";
import { ClipSettings } from "../../cliphist/ClipSettings";
import { GithubSettings, SystemSettings } from "../../integrations/IntegrationsSettings";
import { CityPicker } from "./CityPicker";

/** Pár měst pro rychlé vyplnění (souřadnice jsou veřejné, nic se nezjišťuje). */
const PLACES: [string, number, number][] = [
  ["Praha", 50.08, 14.42],
  ["Brno", 49.2, 16.61],
  ["Ostrava", 49.82, 18.26],
  ["Plzeň", 49.75, 13.38],
  ["Bratislava", 48.15, 17.11],
];

function WeatherSection() {
  const c = useSettings();
  const w = () => c.cfg.personal.weather;
  const placed = () => w().lat != null && w().lon != null;
  return (
    <Section title={fieldLabel("personal.weather")} desc={fieldHint("personal.weather")}>
      <Row id="personal.weather" label={t("settings.personal.place", "Město")} hint={t("settings.personal.place.hint", "Napiš název a vyber ze seznamu — souřadnice se doplní samy. Hledá se jen při psaní.")} stack nested>
        <CityPicker />
        <p class="st-note">
          {placed()
            ? t("settings.personal.place.now", "Teď: {name} ({lat}, {lon})", { name: w().label || t("settings.personal.place.unnamed", "bez názvu"), lat: w().lat, lon: w().lon })
            : t("settings.personal.place.none", "Místo není nastavené — počasí se nestahuje.")}
        </p>
        <div class="st-quick">
          <For each={PLACES}>{([n, la, lo]) => <button class="mini" onClick={() => c.set("personal.weather", { lat: la, lon: lo, label: n })}>{n}</button>}</For>
          <Show when={w().lat != null || w().lon != null || w().label}>
            <button class="mini" onClick={() => c.set("personal.weather", { lat: null, lon: null, label: "" })}>{t("settings.personal.clear", "Vymazat")}</button>
          </Show>
        </div>
      </Row>
      <Advanced id="osobni.weather" inline hint={t("settings.personal.adv.hint", "Zeměpisná šířka a délka")}>
        <Row id="personal.weather.lat" label={t("settings.personal.coords", "Zeměpisná šířka a délka")} hint={t("settings.personal.coords.hint", "Doplní se výběrem města; ručně je najdeš třeba na mapy.cz (pravý klik na místo).")}>
          <Num path="personal.weather.lat" nullable step={0.0001} min={-90} max={90} label={t("settings.personal.lat", "Zeměpisná šířka")} placeholder={t("settings.personal.lat.ph", "šířka")} width="96px" />
          <Num path="personal.weather.lon" nullable step={0.0001} min={-180} max={180} label={t("settings.personal.lon", "Zeměpisná délka")} placeholder={t("settings.personal.lon.ph", "délka")} width="96px" />
          <FieldErr path="personal.weather.lon" />
        </Row>
        <Row id="personal.weather.label" label={t("settings.personal.label", "Název místa")} hint={t("settings.personal.label.hint", "Jen popisek na kartě počasí; po výběru města se doplní sám.")}>
          <Text path="personal.weather.label" label={t("settings.personal.label", "Název místa")} placeholder={t("settings.personal.label", "Název místa")} maxLength={40} />
        </Row>
      </Advanced>
    </Section>
  );
}

function CalendarSection() {
  return (
    <>
      <Section id="personal.calendars" title={fieldLabel("personal.calendars")} desc={fieldHint("personal.calendars")}>
        <CalendarSettings />
      </Section>
      <Advanced id="osobni.accounts" title={t("settings.accounts.adv", "Pokročilé: připojit účet Microsoft / Google")} hint={t("settings.accounts.adv.hint", "Přihlášení vlastní registrací aplikace — jen když ti odkaz ICS nestačí")}>
        <AccountsSettings />
      </Advanced>
    </>
  );
}

function TimerSection() {
  const c = useSettings();
  return (
    <Section title={t("settings.timer.section", "Časovač")} desc={t("settings.timer.desc", "Odpočet a stopky v kartě Časovač, v paletě (Ctrl+K) a v proužku. Běží i se zavřeným šuplíkem a přes restart Ledge.")}>
      <Row id="timer.presets">
        <input
          type="text"
          class="st-text"
          spellcheck={false}
          aria-label={fieldLabel("timer.presets")}
          placeholder={t("settings.timer.presets.ph", "5, 15, 25, 50")}
          value={(c.cfg.timer?.presets ?? []).join(", ")}
          onChange={(e) => {
            const v = parsePresets(e.currentTarget.value);
            c.set("timer.presets", v);
            e.currentTarget.value = v.join(", ");
          }}
        />
      </Row>
      <Row id="timer.toast"><Toggle path="timer.toast" label={fieldLabel("timer.toast")} /></Row>
    </Section>
  );
}

/** Sekce panelu podle id z `personalModel.ts`. */
const SECTIONS: Record<PersonalSectionId, () => JSX.Element> = {
  weather: () => <WeatherSection />,
  calendars: () => <CalendarSection />,
  media: () => (
    <Section title={t("settings.media.section", "Přehrávač")} desc={t("settings.media.desc", "Jak vypadá karta Právě hraje. Vzhled přepneš i přímo v kartě — ikonkou v jejím rohu.")}>
      <MediaSettings />
    </Section>
  ),
  timer: () => <TimerSection />,
  github: () => (
    <Section title={t("settings.github.section", "GitHub")} desc={t("settings.github.desc", "Karta GitHub a položka v proužku: PR čekající na tvoji revizi, tvoje PR s CI a notifikace. Výchozí vypnuto; karta se přidá v katalogu karet.")}>
      <GithubSettings />
    </Section>
  ),
  system: () => (
    <Section title={t("settings.system.section", "Systém")} desc={t("settings.system.desc", "Karta Systém: CPU, paměť, disky a síť. Přidáš ji v katalogu karet (✎ v šuplíku).")}>
      <SystemSettings />
    </Section>
  ),
  suggest: () => (
    <Section title={t("settings.suggest.section", "Kontextové návrhy")} desc={t("settings.suggest.desc", "Malý pruh nahoře v šuplíku (až 3 chipy) — jen z reálných událostí, bez zdroje se nic neukáže. Chip jde zavřít na 24 hodin.")}>
      <SuggestSettings />
    </Section>
  ),
  clipboard: () => <ClipSettings />,
};

export default function PersonalPanel(_p: PanelProps) {
  const c = useSettings();
  const split = () => splitPersonal(layoutCardIds(c.cfg.layout));
  return (
    <>
      <PanelHead title={t("settings.nav.osobni", "Osobní")} desc={t("settings.personal.desc", "Věci mimo práci: počasí, kalendář, hudba, časovač, historie schránky a karty GitHub a Systém.")} />
      <p class="st-note">{t("settings.personal.layoutNote", "Na které záložce která karta je (i jestli záložka Osobní vůbec je), upravíš v panelu Šuplík a karty nebo přímo v šuplíku přes ✎. Odkazy najdeš v panelu Projekty, akce a odkazy.")}</p>
      <For each={split().shown}>{(id) => SECTIONS[id]()}</For>
      <Show when={split().rest.length}>
        <Advanced id="osobni.more" title={t("settings.personal.more", "Další sekce…")} hint={t("settings.personal.more.hint", "Karty, které teď nejsou v šuplíku, a pomocné volby")}>
          <For each={split().rest}>{(id) => SECTIONS[id]()}</For>
        </Advanced>
      </Show>
    </>
  );
}
