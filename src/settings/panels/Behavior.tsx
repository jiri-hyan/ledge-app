// Panel „Chování“: jak se Ledge chová, ne jak vypadá — otevírání šuplíku (najetí / přitlačení /
// podržení, prodlevy), klávesové zkratky (`Hotkeys.tsx`), paleta příkazů, hry a herní režim, šuplík nad
// prohlížečem (`drawer.barWindows`, dřív „Okna s taby nahoře“), spouštění po startu Windows s upozorněním
// na bublinu MSIX (`pkgctx/BubbleNotice.tsx`) a „Šetřit paměť“ (`drawer.keepWarmS`, dřív „Uvolnit paměť
// při nečinnosti“). Vzhled (pozadí, šířka, animace, proužek) je v panelu Vzhled.
import { Show } from "solid-js";
import { api } from "../../api";
import { lang, t } from "../../i18n";
import { useSettings } from "../ctx";
import { Chips, Seg, Slider, Toggle } from "../fields/controls";
import { PanelHead, Row, Section } from "../fields/Row";
import { fieldHint, fieldLabel } from "../labels";
import type { PanelProps } from "../types";
import BubbleNotice, { bubble } from "../../pkgctx/BubbleNotice";
import { HotkeysSection } from "./Hotkeys";

const ms = (v: number) => (v >= 1000 ? `${(v / 1000).toLocaleString(lang() === "cs" ? "cs-CZ" : "en-US", { maximumFractionDigits: 2 })} s` : `${v} ms`);

export default function BehaviorPanel(_p: PanelProps) {
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

  return (
    <>
      <PanelHead title={t("settings.nav.chovani", "Chování")} desc={t("settings.behavior.desc", "Jak se Ledge otevírá, které klávesy poslouchá a kdy se schová.")} />

      <Section title={t("settings.drawer.open", "Otevírání a zavírání")}>
        <Row id="drawer.trigger" hint={triggerHint()} stack>
          <Seg
            path="drawer.trigger"
            label={fieldLabel("drawer.trigger")}
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
      </Section>

      <HotkeysSection />

      <Section title={t("settings.drawer.palette", "Paleta příkazů")}>
        <Row id="palette.compact"><Toggle path="palette.compact" label={fieldLabel("palette.compact")} /></Row>
        <Row id="palette.winSettings"><Toggle path="palette.winSettings" label={fieldLabel("palette.winSettings")} /></Row>
      </Section>

      <Section title={t("settings.smart.title", "Klávesnice, myš a přetahování")} desc={t("settings.smart.desc", "Rychlejší ovládání otevřeného šuplíku. Nic z toho se nestane mimo šuplík.")}>
        <Row id="smart.quickKeys"><Toggle path="smart.quickKeys" label={fieldLabel("smart.quickKeys")} /></Row>
        <Row id="smart.dropShortcut"><Toggle path="smart.dropShortcut" label={fieldLabel("smart.dropShortcut")} /></Row>
        <Row id="smart.tabWheel"><Toggle path="smart.tabWheel" label={fieldLabel("smart.tabWheel")} /></Row>
      </Section>

      <Section title={t("settings.behavior.games", "Hry a celá obrazovka")}>
        <Row id="fullscreen" />
        <Row id="gameMode">
          <button class="mini" onClick={() => void api.setGameMode(true)}>{t("settings.drawer.gameMode.on", "Zapnout")}</button>
        </Row>
      </Section>

      <Section id="drawer.barWindows" title={fieldLabel("drawer.barWindows")} desc={fieldHint("drawer.barWindows")}>
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

      <Section title={t("settings.drawer.startMem", "Spouštění a paměť")}>
        <Row id="autostart" hint={bubble() ? t("settings.pkg.autostartBlocked", "Teď nelze — Ledge běží uvnitř balíčku {family}; Windows by zápis při přihlášení neviděl. Po restartu mimo se zapíše sám.", { family: bubble()!.family }) : undefined}>
          <Toggle path="autostart" label={fieldLabel("autostart")} />
        </Row>
        <BubbleNotice />
        <Row id="drawer.keepWarmS">
          <Toggle value={d().keepWarmS > 0} onChange={(v) => c.set("drawer.keepWarmS", v ? 60 : 0)} label={fieldLabel("drawer.keepWarmS")} />
        </Row>
      </Section>
    </>
  );
}
