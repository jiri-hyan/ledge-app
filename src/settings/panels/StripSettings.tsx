// Sekce „Co proužek ukazuje“ v panelu Vzhled (dřív v panelu Šuplík a proužek): proužek zapnout,
// položky s náhledem (V klidu / Když se něco děje), pořadí přetažením, barva a „Jen při dění“ u položky
// (čistá logika `settings/strip.ts` + `strip.test.mjs`), vlastní text, časovač v proužku a monitor.
// Tvar, velikost, pozice u hrany, linka a krytí proužku jsou v kartě „Vzhled proužku“ hned nad tím
// (`Appearance.tsx`) — celý proužek je tak na jednom místě.
import { createSignal, For, Show } from "solid-js";
import { t } from "../../i18n";
import { useSettings } from "../ctx";
import { Seg, Select, Text, Toggle } from "../fields/controls";
import { Row, Section } from "../fields/Row";
import { SortList } from "../fields/SortList";
import { fieldLabel } from "../labels";
import { moveItem } from "../model";
import { clipCustom, COLOR_VAR, CUSTOM_MAX, customLen, previewSegments, QUIETABLE, STRIP_COLORS, withColor, withQuiet, type StripColorId } from "../strip";

const STRIP_ITEMS: { id: string; key: string; cs: string }[] = [
  { id: "mode", key: "settings.strip.item.mode", cs: "Záložka (Práce, Osobní nebo vlastní)" },
  { id: "clock", key: "settings.strip.item.clock", cs: "Čas a datum" },
  { id: "running", key: "settings.strip.item.running", cs: "Běžící servery" },
  { id: "claude", key: "settings.strip.item.claude", cs: "Claude (čekající žádosti, session)" },
  { id: "repos", key: "settings.strip.item.repos", cs: "Repa s neuloženou prací" },
  { id: "limits", key: "settings.strip.item.limits", cs: "Limity Claude 5h / týden" },
  { id: "waiting", key: "settings.strip.item.waiting", cs: "Kolik Claude session na tebe čeká" },
  { id: "custom", key: "settings.strip.item.custom", cs: "Vlastní text" },
  { id: "timer", key: "settings.strip.item.timer", cs: "Časovač (odpočet, stopky)" },
  { id: "media", key: "settings.strip.item.media", cs: "Právě hraje (skladba · interpret)" },
];

/** Název předvolby barvy (literály kvůli hlídači překladů). */
const colorLabel = (id: StripColorId): string =>
  ({
    accent: t("settings.strip.color.accent", "Akcent motivu"),
    text: t("settings.strip.color.text", "Jasná"),
    muted: t("settings.strip.color.muted", "Tlumená"),
    violet: t("settings.strip.color.violet", "Fialová"),
    blue: t("settings.strip.color.blue", "Modrá"),
    teal: t("settings.strip.color.teal", "Tyrkysová"),
    green: t("settings.strip.color.green", "Zelená"),
    amber: t("settings.strip.color.amber", "Jantarová"),
    red: t("settings.strip.color.red", "Červená"),
  })[id];

/** Ukázkové texty náhledu (ne skutečná data — náhled je označený „Ukázka“). Prázdné = položka by se neukázala. */
const sampleTexts = (custom: string, busy: boolean): Record<string, string> => ({
  mode: t("settings.strip.sample.mode", "Práce"),
  clock: t("settings.strip.sample.clock", "08:31 · st 15. 10."),
  running: busy ? t("settings.strip.sample.running", "web :5173") : "",
  claude: busy ? t("settings.strip.sample.claude", "Claude 1× pracuje") : "",
  repos: t("settings.strip.sample.repos", "2× neuloženo"),
  limits: t("settings.strip.sample.limits", "5h 42 %"),
  waiting: busy ? t("settings.strip.sample.waiting", "1 čeká") : "",
  custom: clipCustom(custom).trim(),
  timer: busy ? t("settings.strip.sample.timer", "⏱ 18 min") : "",
  media: busy ? t("settings.strip.sample.media", "♪ Skladba · Interpret") : "",
});
const stripLabel = (id: string) => {
  const x = STRIP_ITEMS.find((i) => i.id === id);
  return x ? t(x.key, x.cs) : id;
};

export default function StripSettings() {
  const c = useSettings();
  // zapnuté v pořadí z configu, pak vypnuté
  const stripAll = () => {
    const on: string[] = c.cfg.strip.items ?? [];
    return [...on, ...STRIP_ITEMS.map((x) => x.id).filter((x) => !on.includes(x))];
  };
  const stripOn = (id: string) => (c.cfg.strip.items ?? []).includes(id);
  const setStrip = (all: string[], on: (id: string) => boolean) => c.set("strip.items", all.filter(on));
  const quiet = (id: string) => (c.cfg.strip.quiet ?? []).includes(id);
  const [pvMode, setPvMode] = createSignal<"calm" | "busy">("calm");

  return (
    <Section title={t("settings.strip.what", "Co proužek ukazuje")} desc={t("settings.strip.whatDesc", "Proužek je tenká lišta u horního okraje — zapni, co na ní chceš vidět.")}>
      <Row id="strip.enabled"><Toggle path="strip.enabled" label={fieldLabel("strip.enabled")} /></Row>
      <Show when={c.cfg.strip.enabled}>
        <Row id="strip.items" stack>
          <div class="st-strip-pv">
            <div class="st-strip-pv-head">
              <span>{t("settings.strip.preview", "Ukázka")}</span>
              <Seg
                value={pvMode()}
                onChange={setPvMode}
                label={t("settings.strip.preview", "Ukázka")}
                options={[
                  { v: "calm", label: t("settings.strip.preview.calm", "V klidu") },
                  { v: "busy", label: t("settings.strip.preview.busy", "Když se něco děje") },
                ]}
              />
            </div>
            <div class="st-strip-pv-bar">
              <For
                each={previewSegments(c.cfg.strip, sampleTexts(c.cfg.strip.custom ?? "", pvMode() === "busy"), pvMode() === "busy").filter((x) => !x.hidden)}
                fallback={<span class="st-strip-pv-empty">{t("settings.strip.preview.empty", "prázdný proužek")}</span>}
              >
                {(x) => <span class="st-strip-pv-seg" style={{ color: x.color ? COLOR_VAR[x.color as StripColorId] : undefined }}>{x.text}</span>}
              </For>
            </div>
          </div>
          <SortList items={stripAll()} label={fieldLabel("strip.items")} onMove={(from, to) => setStrip(moveItem(stripAll(), from, to), stripOn)}>
            {(id, _i, handle) => (
              <div class="st-item" classList={{ dim: !stripOn(id) }}>
                {handle()}
                <span class="grow">
                  {stripLabel(id)}
                  <Show when={stripOn(id) && !QUIETABLE.includes(id)}><small>{t("settings.strip.eventOnly", "jen když se něco děje")}</small></Show>
                </span>
                <Show when={stripOn(id) && QUIETABLE.includes(id)}>
                  <button
                    class="mini st-strip-quiet"
                    classList={{ "st-strip-on": quiet(id) }}
                    aria-pressed={quiet(id)}
                    title={t("settings.strip.quiet.hint", "V klidu skryté — ukáže se, jen když se něco děje (běží server, Claude, časovač, připomínka)")}
                    onClick={() => c.set("strip.quiet", withQuiet(c.cfg.strip.quiet, id, !quiet(id)))}
                  >
                    {t("settings.strip.quiet", "Jen při dění")}
                  </button>
                </Show>
                <Show when={stripOn(id)}>
                  <select
                    class="st-strip-color"
                    aria-label={t("settings.strip.color", "Barva: {item}", { item: stripLabel(id) })}
                    value={c.cfg.strip.colors?.[id] ?? ""}
                    style={{ color: c.cfg.strip.colors?.[id] ? COLOR_VAR[c.cfg.strip.colors[id] as StripColorId] : undefined }}
                    onChange={(e) => c.set("strip.colors", withColor(c.cfg.strip.colors, id, e.currentTarget.value || null))}
                  >
                    <option value="">{t("settings.strip.color.default", "Výchozí barva")}</option>
                    <For each={STRIP_COLORS}>{(k) => <option value={k}>{colorLabel(k)}</option>}</For>
                  </select>
                </Show>
                <Toggle value={stripOn(id)} onChange={(v) => setStrip(stripAll(), (x) => (x === id ? v : stripOn(x)))} label={stripLabel(id)} />
              </div>
            )}
          </SortList>
        </Row>
        <Show when={stripOn("custom")}>
          <Row id="strip.custom">
            <Text value={c.cfg.strip.custom ?? ""} onChange={(v) => c.set("strip.custom", clipCustom(v ?? ""))} label={fieldLabel("strip.custom")} placeholder={t("settings.strip.custom.ph", "🎯 Hluboká práce")} />
            <small class="st-strip-count">{customLen(c.cfg.strip.custom ?? "")}/{CUSTOM_MAX}</small>
          </Row>
        </Show>
        <Row id="timer.strip"><Toggle path="timer.strip" label={fieldLabel("timer.strip")} /></Row>
      </Show>
      <Row id="strip.monitor">
        <Select
          path="strip.monitor"
          label={fieldLabel("strip.monitor")}
          options={[
            { v: "primary", label: t("settings.drawer.monitor.primary", "Hlavní") },
            ...[0, 1, 2, 3].map((i) => ({ v: String(i), label: t("settings.drawer.monitor.n", "Monitor {n}", { n: i + 1 }) })),
          ]}
        />
      </Row>
    </Section>
  );
}
