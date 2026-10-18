// Panel „Osobní“: zapnutí záložky, místo pro počasí (Open-Meteo, bez polohy podle IP) a rychlé odkazy
// (web / složka / soubor / aplikace; řazení přetažením, výběr cíle dialogem, smazání s Vrátit).
import { createSignal, For, Show } from "solid-js";
import { api, type PickKind } from "../../api";
import { t } from "../../i18n";
import { sayError } from "../../ui/toast";
import { offerUndo, useSettings } from "../ctx";
import { Num, Text, Toggle } from "../fields/controls";
import { FieldErr, PanelHead, Row, Section } from "../fields/Row";
import { SortList } from "../fields/SortList";
import { moveItem } from "../model";
import type { PanelProps } from "../types";
import { parsePresets } from "../../timer/model";
import CalendarSettings from "../../calendar/CalendarSettings";

const MAX_LINKS = 24;

function Links() {
  const c = useSettings();
  const links = (): { name: string; target: string }[] => c.cfg.personal.links ?? [];
  const at = (i: number, f: string) => `personal.links.${i}.${f}`;
  const [picking, setPicking] = createSignal(false);
  async function pick(i: number, kind: PickKind) {
    setPicking(true);
    try {
      const got = await api.pickPath(kind, t("settings.links.pick", "Cíl odkazu"));
      if (!got) return;
      c.set(at(i, "target"), got);
      if (!links()[i].name.trim()) c.set(at(i, "name"), got.replace(/[\\/]+$/, "").split(/[\\/]/).pop()!.replace(/\.exe$/i, ""));
    } catch (e) {
      sayError(e);
    } finally {
      setPicking(false);
    }
  }
  function remove(i: number) {
    const before = structuredClone(links());
    c.update("personal.links", (all: unknown[]) => all.filter((_, j) => j !== i));
    offerUndo(t("settings.links.removed", "Odkaz „{n}“ odebrán", { n: before[i].name || before[i].target }), () => c.set("personal.links", before));
  }
  return (
    <Row id="personal.links" stack nested>
      <Show when={links().length} fallback={<p class="st-empty">{t("settings.links.empty", "Zatím žádné odkazy. Přidej web, složku nebo aplikaci, kterou otvíráš často.")}</p>}>
        <SortList items={links()} label={t("settings.f.personal.links", "Rychlé odkazy")} onMove={(a, b) => c.set("personal.links", moveItem(links(), a, b))}>
          {(_l, i, handle) => (
            <div class="st-item-wrap">
              <div class="st-item">
                {handle()}
                <Text path={at(i(), "name")} label={t("settings.links.name", "Název")} placeholder={t("settings.links.name", "Název")} maxLength={60} />
                <Text path={at(i(), "target")} label={t("settings.links.target", "Cíl")} placeholder={t("settings.links.target.ph", "https://… · C:\\cesta · appka.exe")} mono grow />
                <span class="st-pickers">
                  <button class="mini" disabled={picking()} title={t("settings.links.pick.folder", "Vybrat složku")} onClick={() => pick(i(), "folder")}>{t("settings.links.folder", "Složka")}</button>
                  <button class="mini" disabled={picking()} title={t("settings.links.pick.file", "Vybrat soubor nebo aplikaci")} onClick={() => pick(i(), "file")}>{t("settings.links.file", "Soubor")}</button>
                </span>
                <button class="mini" onClick={() => remove(i())}>{t("settings.ui.remove", "Odebrat")}</button>
              </div>
              <FieldErr path={at(i(), "name")} />
              <FieldErr path={at(i(), "target")} />
            </div>
          )}
        </SortList>
      </Show>
      <div class="st-actions">
        <button class="mini" disabled={links().length >= MAX_LINKS} onClick={() => c.update("personal.links", (l: unknown[]) => [...l, { name: "", target: "" }])}>+ {t("settings.links.add", "Přidat odkaz")}</button>
        <Show when={links().length >= MAX_LINKS}><small>{t("settings.links.max", "Nejvýš {n} odkazů", { n: MAX_LINKS })}</small></Show>
      </div>
    </Row>
  );
}

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
      <PanelHead id="modes.personalEnabled" title={t("settings.nav.osobni", "Osobní")} desc={t("settings.personal.desc", "Záložka pro běžné věci mimo práci: čas, počasí, rychlé odkazy a obecné poznámky.")}>
        <Toggle path="modes.personalEnabled" label={t("settings.f.modes.personalEnabled", "Záložka Osobní")} />
      </PanelHead>
      <Show when={c.cfg.modes.personalEnabled} fallback={<p class="st-empty">{t("settings.personal.off", "Záložka Osobní je vypnutá — šuplík ukazuje jen Práci.")}</p>}>
        <Section title={t("settings.f.personal.weather", "Místo pro počasí")} desc={t("settings.f.personal.weather.hint", "Open-Meteo, bez klíče. Prázdné = nic se nestahuje (polohu podle IP Ledge nezjišťuje).")}>
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
        <Section id="personal.calendars" title={t("settings.f.personal.calendars", "Kalendáře")} desc={t("settings.f.personal.calendars.hint", "Google nebo Outlook z tajného odkazu ICS — jen čtení, bez přihlášení. Vlož odkaz a klikni na Přidat.")}>
          <CalendarSettings />
        </Section>
        <Section title={t("settings.timer.section", "Časovač")} desc={t("settings.timer.desc", "Odpočet a stopky v kartě Časovač, v paletě (Ctrl+K) a v proužku. Běží i se zavřeným šuplíkem a přes restart Ledge.")}>
          <Row id="timer.presets">
            <input
              type="text"
              class="st-text"
              spellcheck={false}
              aria-label={t("settings.f.timer.presets", "Předvolby časovače")}
              placeholder={t("settings.timer.presets.ph", "5, 15, 25, 50")}
              value={(c.cfg.timer?.presets ?? []).join(", ")}
              onChange={(e) => {
                const v = parsePresets(e.currentTarget.value);
                c.set("timer.presets", v);
                e.currentTarget.value = v.join(", ");
              }}
            />
          </Row>
          <Row id="timer.toast"><Toggle path="timer.toast" label={t("settings.f.timer.toast", "Oznámení po doběhnutí")} /></Row>
        </Section>
        <Section title={t("settings.f.personal.links", "Rychlé odkazy")} desc={t("settings.f.personal.links.hint", "Web (jen http/https), složka, soubor nebo aplikace — vždy bez argumentů")}>
          <Links />
        </Section>
      </Show>
    </>
  );
}
