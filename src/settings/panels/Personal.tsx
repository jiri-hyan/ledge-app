// Panel „Osobní“: místo pro počasí (Open-Meteo, bez polohy podle IP), kalendáře (ICS), přehrávač,
// časovač a historie schránky (`cliphist/ClipSettings.tsx`, dřív v panelu Šuplík a proužek). Rychlé odkazy
// jsou od vlny 6 akce (panel Projekty, akce a odkazy); na které záložce která karta je, řídí `layout`.
import { For, Show } from "solid-js";
import { t } from "../../i18n";
import { useSettings } from "../ctx";
import { Num, Text, Toggle } from "../fields/controls";
import { PanelHead, Row, Section } from "../fields/Row";
import { fieldHint, fieldLabel } from "../labels";
import type { PanelProps } from "../types";
import { parsePresets } from "../../timer/model";
import CalendarSettings from "../../calendar/CalendarSettings";
import MediaSettings from "../../media/MediaSettings";
import { ClipSettings } from "../../cliphist/ClipSettings";
import { GithubSettings, SystemSettings } from "../../integrations/IntegrationsSettings";

/** Pár měst pro rychlé vyplnění (souřadnice jsou veřejné, nic se nezjišťuje). */
const PLACES: [string, number, number][] = [
  ["Praha", 50.08, 14.42],
  ["Brno", 49.2, 16.61],
  ["Ostrava", 49.82, 18.26],
  ["Plzeň", 49.75, 13.38],
  ["Bratislava", 48.15, 17.11],
];

export default function PersonalPanel(_p: PanelProps) {
  const c = useSettings();
  const w = () => c.cfg.personal.weather;
  return (
    <>
      <PanelHead title={t("settings.nav.osobni", "Osobní")} desc={t("settings.personal.desc", "Věci mimo práci: počasí, kalendář, hudba, časovač, historie schránky a karty GitHub a Systém.")} />
      <p class="st-note">{t("settings.personal.layoutNote", "Na které záložce která karta je (i jestli záložka Osobní vůbec je), upravíš v panelu Šuplík a karty nebo přímo v šuplíku přes ✎. Odkazy najdeš v panelu Projekty, akce a odkazy.")}</p>
      <>
        <Section title={fieldLabel("personal.weather")} desc={fieldHint("personal.weather")}>
          <Row id="personal.weather" label={t("settings.personal.place", "Místo")} hint={t("settings.personal.place.hint", "Název je jen popisek na kartě. Souřadnice najdeš třeba na mapy.cz (pravý klik na místo).")} stack nested>
            <div class="st-inline">
              <Text path="personal.weather.label" label={t("settings.personal.label", "Název místa")} placeholder={t("settings.personal.label", "Název místa")} maxLength={40} />
              <Num path="personal.weather.lat" nullable step={0.0001} min={-90} max={90} label={t("settings.personal.lat", "Zeměpisná šířka")} placeholder={t("settings.personal.lat.ph", "šířka")} width="96px" />
              <Num path="personal.weather.lon" nullable step={0.0001} min={-180} max={180} label={t("settings.personal.lon", "Zeměpisná délka")} placeholder={t("settings.personal.lon.ph", "délka")} width="96px" />
              <Show when={w().lat != null || w().lon != null || w().label}>
                <button class="mini" onClick={() => c.set("personal.weather", { lat: null, lon: null, label: "" })}>{t("settings.personal.clear", "Vymazat")}</button>
              </Show>
            </div>
            <div class="st-quick">
              <For each={PLACES}>{([n, la, lo]) => <button class="mini" onClick={() => c.set("personal.weather", { lat: la, lon: lo, label: n })}>{n}</button>}</For>
            </div>
          </Row>
        </Section>
        <Section id="personal.calendars" title={fieldLabel("personal.calendars")} desc={fieldHint("personal.calendars")}>
          <CalendarSettings />
        </Section>
        <Section title={t("settings.media.section", "Přehrávač")} desc={t("settings.media.desc", "Jak vypadá karta Právě hraje. Vzhled přepneš i přímo v kartě — ikonkou v jejím rohu.")}>
          <MediaSettings />
        </Section>
        <Section title={t("settings.github.section", "GitHub")} desc={t("settings.github.desc", "Karta GitHub a položka v proužku: PR čekající na tvoji revizi, tvoje PR s CI a notifikace. Výchozí vypnuto; karta se přidá v katalogu karet.")}>
          <GithubSettings />
        </Section>
        <Section title={t("settings.system.section", "Systém")} desc={t("settings.system.desc", "Karta Systém: CPU, paměť, disky a síť. Přidáš ji v katalogu karet (✎ v šuplíku).")}>
          <SystemSettings />
        </Section>
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
        <ClipSettings />
      </>
    </>
  );
}
