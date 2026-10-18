// Panel „Šuplík a proužek“: CHOVÁNÍ — jak se šuplík otevírá a zavírá, záložky a karty (`layout/LayoutEditor.tsx`), proužek (co
// ukazuje, monitor), paleta příkazů (kompaktní), okna s taby nahoře (tlak na hranu), uvolnění paměti, autostart. Vzhled (pozadí,
// rozmazání, šířka, animace, styl a pozice proužku) je JEN v panelu „Vzhled a animace“ (`Appearance.tsx`).
import { createSignal, For, Show } from "solid-js";
import { api } from "../../api";
import { lang, t } from "../../i18n";
import { useSettings } from "../ctx";
import { Chips, Seg, Select, Slider, Text, Toggle } from "../fields/controls";
import { PanelHead, Row, Section } from "../fields/Row";
import { SortList } from "../fields/SortList";
import { moveItem } from "../model";
import type { PanelProps } from "../types";
import LayoutEditor from "../../layout/LayoutEditor";
import { ClipSettings } from "../../cliphist/ClipSettings";
import BubbleNotice, { bubble } from "../../pkgctx/BubbleNotice";
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

const ms = (v: number) => (v >= 1000 ? `${(v / 1000).toLocaleString(lang() === "cs" ? "cs-CZ" : "en-US", { maximumFractionDigits: 2 })} s` : `${v} ms`);

export default function DrawerPanel(_p: PanelProps) {
  const c = useSettings();
  const d = () => c.cfg.drawer;
  const trigger = () => d().trigger as string;
  const triggerHint = () =>
    ({
      hover: t("settings.drawer.trigger.hover.hint", "Šuplík vyjede, když myš chvíli zůstane na proužku nahoře uprostřed."),
      edge: t("settings.drawer.trigger.edge.hint", "Šuplík vyjede, když myš přitlačíš k hornímu okraji v místě proužku."),
      dwell: t("settings.drawer.trigger.dwell.hint", "Šuplík vyjede až po delším podržení myši na proužku — nejméně náhodných otevření."),
      off: t("settings.drawer.trigger.off.hint", "Myš šuplík neotevírá vůbec, jen klávesová zkratka."),
    })[trigger()] ?? "";

  // proužek: zapnuté v pořadí z configu, pak vypnuté
  const stripAll = () => {
    const on: string[] = c.cfg.strip.items ?? [];
    return [...on, ...STRIP_ITEMS.map((x) => x.id).filter((x) => !on.includes(x))];
  };
  const stripOn = (id: string) => (c.cfg.strip.items ?? []).includes(id);
  const setStrip = (all: string[], on: (id: string) => boolean) => c.set("strip.items", all.filter(on));
  const quiet = (id: string) => (c.cfg.strip.quiet ?? []).includes(id);
  const [pvMode, setPvMode] = createSignal<"calm" | "busy">("calm");


  return (
    <>
      <PanelHead title={t("settings.nav.suplik", "Šuplík a proužek")} desc={t("settings.drawer.desc", "Jak se Ledge chová u horního okraje obrazovky.")} />

      <Section title={t("settings.drawer.open", "Otevírání a zavírání")}>
        <Row id="drawer.trigger" hint={triggerHint()} stack>
          <Seg
            path="drawer.trigger"
            label={t("settings.f.drawer.trigger", "Jak se šuplík otevírá")}
            options={[
              { v: "hover", label: t("settings.drawer.trigger.hover", "Najetím na proužek") },
              { v: "edge", label: t("settings.drawer.trigger.edge", "Přitlačením k okraji") },
              { v: "dwell", label: t("settings.drawer.trigger.dwell", "Podržením") },
              ...(c.cfg.hotkeys?.enabled ? [{ v: "off", label: t("settings.drawer.trigger.off", "Jen zkratkou") }] : []),
            ]}
          />
        </Row>
        <Row id="drawer.delays" stack>
          <div class="st-grid2">
            <Show when={trigger() === "hover"}>
              <label class="st-sub">{t("settings.drawer.hoverDelay", "Najetí")}<Slider path="drawer.hoverDelayMs" min={0} max={800} step={10} label={t("settings.drawer.hoverDelay", "Najetí")} fmt={ms} /></label>
            </Show>
            <Show when={trigger() === "edge"}>
              <label class="st-sub">{t("settings.drawer.edgeDelay", "Přitlačení")}<Slider path="drawer.edgeDelayMs" min={0} max={800} step={10} label={t("settings.drawer.edgeDelay", "Přitlačení")} fmt={ms} /></label>
            </Show>
            <Show when={trigger() === "dwell"}>
              <label class="st-sub">{t("settings.drawer.dwellDelay", "Podržení")}<Slider path="drawer.dwellDelayMs" min={150} max={2000} step={50} label={t("settings.drawer.dwellDelay", "Podržení")} fmt={ms} /></label>
            </Show>
            <label class="st-sub">{t("settings.drawer.closeDelay", "Zavření po odjetí myši")}<Slider path="drawer.closeDelayMs" min={100} max={2000} step={50} label={t("settings.drawer.closeDelay", "Zavření po odjetí myši")} fmt={ms} /></label>
          </div>
        </Row>
        <Row id="fullscreen" />
        <Row id="gameMode">
          <button class="mini" onClick={() => void api.setGameMode(true)}>{t("settings.drawer.gameMode.on", "Zapnout")}</button>
        </Row>
      </Section>

      <LayoutEditor />

      <Section title={t("settings.drawer.palette", "Paleta příkazů")}>
        <Row id="palette.compact"><Toggle path="palette.compact" label={t("settings.f.palette.compact", "Kompaktní paleta příkazů")} /></Row>
        <Row id="palette.winSettings"><Toggle path="palette.winSettings" label={t("settings.f.palette.winSettings", "Nastavení Windows v paletě")} /></Row>
      </Section>

      <ClipSettings />

      <Section title={t("settings.drawer.strip", "Proužek")}>
        <Row id="strip.enabled"><Toggle path="strip.enabled" label={t("settings.f.strip.enabled", "Proužek nahoře")} /></Row>
        <Row id="shelf.enabled"><Toggle path="shelf.enabled" label={t("settings.f.shelf.enabled", "Přihrádka na soubory")} /></Row>
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
            <SortList
              items={stripAll()}
              label={t("settings.f.strip.items", "Co proužek ukazuje")}
              onMove={(from, to) => setStrip(moveItem(stripAll(), from, to), stripOn)}
            >
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
                  <Toggle
                    value={stripOn(id)}
                    onChange={(v) => setStrip(stripAll(), (x) => (x === id ? v : stripOn(x)))}
                    label={stripLabel(id)}
                  />
                </div>
              )}
            </SortList>
          </Row>
          <Show when={stripOn("custom")}>
            <Row id="strip.custom">
              <Text
                value={c.cfg.strip.custom ?? ""}
                onChange={(v) => c.set("strip.custom", clipCustom(v ?? ""))}
                label={t("settings.f.strip.custom", "Vlastní text v proužku")}
                placeholder={t("settings.strip.custom.ph", "🎯 Hluboká práce")}
              />
              <small class="st-strip-count">{customLen(c.cfg.strip.custom ?? "")}/{CUSTOM_MAX}</small>
            </Row>
          </Show>
          <Row id="timer.strip"><Toggle path="timer.strip" label={t("settings.f.timer.strip", "Časovač v proužku, když běží")} /></Row>
        </Show>
        <Row id="strip.monitor">
          <Select
            path="strip.monitor"
            label={t("settings.f.strip.monitor", "Monitor")}
            options={[
              { v: "primary", label: t("settings.drawer.monitor.primary", "Hlavní") },
              ...[0, 1, 2, 3].map((i) => ({ v: String(i), label: t("settings.drawer.monitor.n", "Monitor {n}", { n: i + 1 }) })),
            ]}
          />
        </Row>
      </Section>

      <Section id="drawer.barWindows" title={t("settings.f.drawer.barWindows", "Okna s taby nahoře")} desc={t("settings.f.drawer.barWindows.hint", "U maximalizovaného prohlížeče by šuplík otvíralo přepínání tabů")}>
        <Row id="drawer.barWindows.trigger" label={t("settings.drawer.bar.trigger", "Jak tam šuplík otevřít")} stack>
          <Seg
            path="drawer.barWindows.trigger"
            label={t("settings.drawer.bar.trigger", "Jak tam šuplík otevřít")}
            options={[
              { v: "push", label: t("settings.drawer.bar.push", "Jen zatlačením na horní hranu") },
              { v: "hover", label: t("settings.drawer.bar.hover", "Stejně jako jinde") },
            ]}
          />
        </Row>
        <Show when={d().barWindows.trigger === "push"}>
          <Row id="drawer.barWindows.pushDistance" label={t("settings.drawer.bar.force", "Síla zatlačení")} hint={t("settings.drawer.bar.force.hint", "Pohyb myši nahoru za hranou (bez kliknutí a do stran). Víc = tlačit déle; proužek nad taby ustoupí na linku.")}>
            <Slider path="drawer.barWindows.pushDistance" min={10} max={400} step={10} label={t("settings.drawer.bar.force", "Síla zatlačení")} fmt={(v) => (v <= 40 ? t("settings.drawer.bar.light", "lehce ({v})", { v }) : v >= 150 ? t("settings.drawer.bar.hard", "silně ({v})", { v }) : String(v))} />
          </Row>
          <Row id="drawer.barWindows.pushMs" label={t("settings.drawer.bar.ms", "Tablet a vzdálená plocha")} hint={t("settings.drawer.bar.ms.hint", "Kde tlak měřit nejde: jak dlouho stát na hraně")}>
            <Slider path="drawer.barWindows.pushMs" min={100} max={2000} step={50} label={t("settings.drawer.bar.ms", "Tablet a vzdálená plocha")} fmt={ms} />
          </Row>
        </Show>
        <Row id="drawer.barWindows.processes" label={t("settings.drawer.bar.procs", "Programy s taby nahoře")} hint={t("settings.drawer.bar.procs.hint", "Název procesu, např. chrome.exe; přidat jde i Code.exe nebo WindowsTerminal.exe. Enter přidá.")} stack>
          <Chips path="drawer.barWindows.processes" label={t("settings.drawer.bar.procs", "Programy s taby nahoře")} placeholder={t("settings.drawer.bar.procs.ph", "appka.exe")} normalize={(s) => { const v = s.trim().replace(/^.*[\\/]/, ""); return v && !/\.exe$/i.test(v) ? `${v}.exe` : v; }} />
        </Row>
      </Section>

      <Section title={t("settings.drawer.startMem", "Spouštění a paměť")} desc={t("settings.drawer.lookMoved", "Pozadí, rozmazání, šířku a animace šuplíku nastavíš v panelu Vzhled a animace.")}>
        <Row id="autostart" hint={bubble() ? t("settings.pkg.autostartBlocked", "Teď nelze — Ledge běží uvnitř balíčku {family}; Windows by zápis při přihlášení neviděl. Po restartu mimo se zapíše sám.", { family: bubble()!.family }) : undefined}>
          <Toggle path="autostart" label={t("settings.f.autostart", "Spouštět po startu Windows")} />
        </Row>
        <BubbleNotice />
        <Row id="drawer.keepWarmS">
          <Toggle value={d().keepWarmS > 0} onChange={(v) => c.set("drawer.keepWarmS", v ? 60 : 0)} label={t("settings.f.drawer.keepWarmS", "Uvolnit paměť při nečinnosti")} />
        </Row>
      </Section>
    </>
  );
}
