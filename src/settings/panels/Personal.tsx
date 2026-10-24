// Stránka „Obsah karet“ (Šuplík a karty; dřív Osobní) — kolo 21: PO KARTÁCH. Každá karta má jednu
// sbalovací skupinu (`fields/Group.tsx`) s lidským nadpisem, větou, co karta dělá, a všemi svými volbami
// pohromadě: Počasí (pole Město s našeptávačem `CityPicker.tsx`, souřadnice v Pokročilé, bez polohy podle IP),
// Kalendář (ICS jako první volba; přihlášení účtem Microsoft / Google pod ním v Pokročilé,
// `accounts/AccountsSettings.tsx`), Právě hraje, Časovač, GitHub, Systém; nakonec funkce bez vlastní karty
// (návrhy v šuplíku, historie schránky `cliphist/ClipSettings.tsx`). Pořadí a výchozí rozbalení podle
// rozložení (`personalModel.ts`): karty v šuplíku nahoře otevřené, ostatní sbalené — ale vidět vždy, i v režimu
// Základní (dřív schované za „Další sekce…“ jako Pokročilé). Hledání najde vše a skupinu rozbalí. Které karty
// šuplík ukazuje a na které záložce, řídí `layout` (Záložky a karty); odkazy jsou akce (Projekty a akce).
import { For, Show, type JSX } from "solid-js";
import { t } from "../../i18n";
import { gotoSettingsPanel } from "../../palette/goto";
import { useSettings } from "../ctx";
import { Num, Text, Toggle } from "../fields/controls";
import { Advanced } from "../fields/Advanced";
import { Group } from "../fields/Group";
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

function WeatherRows() {
  const c = useSettings();
  const w = () => c.cfg.personal.weather;
  const placed = () => w().lat != null && w().lon != null;
  return (
    <>
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
    </>
  );
}

function CalendarRows() {
  return (
    <>
      {/* kotva `f-personal.calendars` (skok z karty Kalendář a z Doporučených) */}
      <Section id="personal.calendars">
        <CalendarSettings />
      </Section>
      <Advanced id="osobni.accounts" inline title={t("settings.accounts.adv", "Pokročilé: připojit účet Microsoft / Google")} hint={t("settings.accounts.adv.hint", "Přihlášení vlastní registrací aplikace — jen když ti odkaz ICS nestačí")}>
        <AccountsSettings />
      </Advanced>
    </>
  );
}

function TimerRows() {
  const c = useSettings();
  return (
    <>
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
    </>
  );
}

/** Skupina na kartu: nadpis (název karty), jedna věta, co karta dělá, a obsah — klíč = id z `personalModel.ts`. */
type Part = { title: () => string; desc: () => string; body: () => JSX.Element };
const PARTS: Record<PersonalSectionId, Part> = {
  weather: { title: () => t("settings.personal.g.weather", "Počasí"), desc: () => fieldHint("personal.weather"), body: () => <WeatherRows /> },
  calendars: { title: () => t("settings.personal.g.calendar", "Kalendář"), desc: () => fieldHint("personal.calendars"), body: () => <CalendarRows /> },
  media: { title: () => t("settings.personal.g.media", "Právě hraje"), desc: () => t("settings.media.desc", "Jak vypadá karta Právě hraje. Vzhled přepneš i přímo v kartě — ikonkou v jejím rohu."), body: () => <MediaSettings /> },
  timer: { title: () => t("settings.timer.section", "Časovač"), desc: () => t("settings.timer.desc", "Odpočet a stopky v kartě Časovač, v paletě (Ctrl+K) a v proužku. Běží i se zavřeným šuplíkem a přes restart Ledge."), body: () => <TimerRows /> },
  github: { title: () => t("settings.github.section", "GitHub"), desc: () => t("settings.github.desc", "Karta GitHub a položka v proužku: PR čekající na tvoji revizi, tvoje PR s CI a notifikace. Výchozí vypnuto; karta se přidá v katalogu karet."), body: () => <GithubSettings /> },
  system: { title: () => t("settings.system.section", "Systém"), desc: () => t("settings.system.desc", "Karta Systém: CPU, paměť, disky a síť. Přidáš ji v katalogu karet (✎ v šuplíku)."), body: () => <SystemSettings /> },
  suggest: { title: () => t("settings.suggest.section", "Návrhy v šuplíku"), desc: () => t("settings.suggest.desc", "Malý pruh nahoře v šuplíku (až 3 chipy) — jen z reálných událostí, bez zdroje se nic neukáže. Chip jde zavřít na 24 hodin."), body: () => <SuggestSettings /> },
  clipboard: { title: () => t("settings.clip.title", "Historie schránky"), desc: () => t("settings.clip.desc", "V paletě (Ctrl+K) napiš v: a uvidíš, co jsi naposled zkopíroval. Pamatuje si jen text, jen v paměti Ledge — po restartu je pryč (kromě připnutých) a nikam se neposílá. Hesla zkopírovaná ze správce hesel nebo prohlížeče se neukládají."), body: () => <ClipSettings /> },
};

/** Skupiny jedné části stránky (`open` = výchozí rozbalení, uživatelova volba se pamatuje). */
function Parts(p: { ids: PersonalSectionId[]; open: boolean }) {
  return (
    <For each={p.ids}>
      {(id) => (
        <Group gid={`osobni.${id}`} title={PARTS[id].title()} desc={PARTS[id].desc()} open={p.open}>
          {PARTS[id].body()}
        </Group>
      )}
    </For>
  );
}

export default function PersonalPanel(_p: PanelProps) {
  const c = useSettings();
  const split = () => splitPersonal(layoutCardIds(c.cfg.layout));
  return (
    <>
      <PanelHead title={t("settings.nav.osobni", "Obsah karet")} desc={t("settings.personal.desc", "Co ukazují jednotlivé karty šuplíku — u každé karty všechny její volby pohromadě. Nahoře karty, které v šuplíku máš.")}>
        <button class="mini" onClick={() => gotoSettingsPanel("suplik")}>{t("settings.personal.toLayout", "Které karty šuplík ukazuje…")}</button>
      </PanelHead>
      <Show when={split().shown.length}>
        <h3 class="st-grp-sep">{t("settings.personal.sep.shown", "Karty v šuplíku")}</h3>
        <Parts ids={split().shown} open />
      </Show>
      <Show when={split().more.length}>
        <h3 class="st-grp-sep">{t("settings.personal.sep.more", "Další karty — teď v šuplíku nejsou")}</h3>
        <Parts ids={split().more} open={false} />
      </Show>
      <h3 class="st-grp-sep">{t("settings.personal.sep.helpers", "Funkce bez vlastní karty")}</h3>
      <Parts ids={split().helpers} open={false} />
    </>
  );
}
