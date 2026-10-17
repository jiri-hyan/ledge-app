// Panel „Šuplík a proužek“: CHOVÁNÍ — jak se šuplík otevírá a zavírá, karty hlavní stránky, proužek (co
// ukazuje, monitor), paleta příkazů (kompaktní), okna s taby nahoře (tlak na hranu), uvolnění paměti, autostart. Vzhled (pozadí,
// rozmazání, šířka, animace, styl a pozice proužku) je JEN v panelu „Vzhled a animace“ (`Appearance.tsx`).
import { Show } from "solid-js";
import { api } from "../../api";
import { lang, t } from "../../i18n";
import { useSettings } from "../ctx";
import { Chips, Seg, Select, Slider, Toggle } from "../fields/controls";
import { PanelHead, Row, Section } from "../fields/Row";
import { SortList } from "../fields/SortList";
import { moveItem } from "../model";
import type { PanelProps } from "../types";

const STRIP_ITEMS: { id: string; key: string; cs: string }[] = [
  { id: "mode", key: "settings.strip.item.mode", cs: "Režim (Práce / Osobní)" },
  { id: "clock", key: "settings.strip.item.clock", cs: "Čas a datum" },
  { id: "running", key: "settings.strip.item.running", cs: "Běžící servery" },
  { id: "claude", key: "settings.strip.item.claude", cs: "Claude (čekající žádosti, session)" },
  { id: "repos", key: "settings.strip.item.repos", cs: "Repa s neuloženou prací" },
  { id: "limits", key: "settings.strip.item.limits", cs: "Limity Claude 5h / týden" },
  { id: "waiting", key: "settings.strip.item.waiting", cs: "Kolik Claude session na tebe čeká" },
];
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

      <Section id="drawer.sections" title={t("settings.f.drawer.sections", "Karty na hlavní stránce")} desc={t("settings.drawer.sections.desc", "Vypnutá karta zmizí ze šuplíku; data se dál nesbírají zbytečně.")}>
        <Row id="drawer.sections.projects" label={t("settings.drawer.sec.projects", "Projekty")} hint={t("settings.drawer.sec.projects.hint", "Dlaždice projektů nahoře")}>
          <Toggle path="drawer.sections.projects" label={t("settings.drawer.sec.projects", "Projekty")} />
        </Row>
        <Row id="drawer.sections.running" label={t("settings.drawer.sec.running", "Běžící servery")} hint={t("settings.drawer.sec.running.hint", "Ukáže se, jen když Ledge něco spustil")}>
          <Toggle path="drawer.sections.running" label={t("settings.drawer.sec.running", "Běžící servery")} />
        </Row>
        <Row id="drawer.sections.claude" label={t("settings.drawer.sec.claude", "Claude")} hint={t("settings.drawer.sec.claude.hint", "Session, subagenti a žádosti o schválení")}>
          <Toggle path="drawer.sections.claude" label={t("settings.drawer.sec.claude", "Claude")} />
        </Row>
        <Row
          id="drawer.sections.notes"
          label={t("settings.drawer.sec.notes", "Poznámky")}
          dim={c.cfg.notes?.enabled === false}
          hint={c.cfg.notes?.enabled === false ? t("settings.drawer.sec.notes.off", "Poznámky jsou vypnuté celé (panel Poznámky) — karta se neukáže") : t("settings.drawer.sec.notes.hint", "Poslední poznámky a připomínky na dnes")}
        >
          <Toggle path="drawer.sections.notes" label={t("settings.drawer.sec.notes", "Poznámky")} />
        </Row>
      </Section>

      <Section title={t("settings.drawer.palette", "Paleta příkazů")}>
        <Row id="palette.compact"><Toggle path="palette.compact" label={t("settings.f.palette.compact", "Kompaktní paleta příkazů")} /></Row>
        <Row id="palette.winSettings"><Toggle path="palette.winSettings" label={t("settings.f.palette.winSettings", "Nastavení Windows v paletě")} /></Row>
      </Section>

      <Section title={t("settings.drawer.strip", "Proužek")}>
        <Row id="strip.enabled"><Toggle path="strip.enabled" label={t("settings.f.strip.enabled", "Proužek nahoře")} /></Row>
        <Show when={c.cfg.strip.enabled}>
          <Row id="strip.items" stack>
            <SortList
              items={stripAll()}
              label={t("settings.f.strip.items", "Co proužek ukazuje")}
              onMove={(from, to) => setStrip(moveItem(stripAll(), from, to), stripOn)}
            >
              {(id, _i, handle) => (
                <div class="st-item" classList={{ dim: !stripOn(id) }}>
                  {handle()}
                  <span class="grow">{stripLabel(id)}</span>
                  <Toggle
                    value={stripOn(id)}
                    onChange={(v) => setStrip(stripAll(), (x) => (x === id ? v : stripOn(x)))}
                    label={stripLabel(id)}
                  />
                </div>
              )}
            </SortList>
          </Row>
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
        <Row id="autostart"><Toggle path="autostart" label={t("settings.f.autostart", "Spouštět po startu Windows")} /></Row>
        <Row id="drawer.keepWarmS">
          <Toggle value={d().keepWarmS > 0} onChange={(v) => c.set("drawer.keepWarmS", v ? 60 : 0)} label={t("settings.f.drawer.keepWarmS", "Uvolnit paměť při nečinnosti")} />
        </Row>
      </Section>
    </>
  );
}
