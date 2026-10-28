// Stránka „Spouštění a hry“ (sekce Aplikace, kolo 21; dřív konec stránky Otevírání a klávesy): jak Ledge
// běží jako aplikace — spouštění po startu Windows s upozorněním na bublinu MSIX (`pkgctx/BubbleNotice.tsx`),
// „Šetřit paměť“ (`drawer.keepWarmS`: uvolnění paměti WebView po minutě zavřeného šuplíku, s animacemi
// nesouvisí), hry a video přes celou obrazovku a herní režim. Zkratka herního režimu zůstává u ostatních
// zkratek (Otevírání a klávesy). Jako všechny stránky mění jen draft — zapíše společná lišta.
import { Show } from "solid-js";
import { api } from "../../api";
import { t } from "../../i18n";
import { useSettings } from "../ctx";
import { Toggle } from "../fields/controls";
import { Row, Section } from "../fields/Row";
import { fieldLabel } from "../labels";
import type { PanelProps } from "../types";
import BubbleNotice, { bubble } from "../../pkgctx/BubbleNotice";

export default function StartupPanel(_p: PanelProps) {
  const c = useSettings();
  return (
    <>
      <div class="st-cols">
        <Section title={t("settings.drawer.startMem", "Spouštění a paměť")}>
          <Row id="autostart" hint={bubble() ? t("settings.pkg.autostartBlocked", "Teď nelze — Ledge běží uvnitř balíčku {family}; Windows by zápis při přihlášení neviděl. Po restartu mimo se zapíše sám.", { family: bubble()!.family }) : undefined}>
            <Toggle path="autostart" label={fieldLabel("autostart")} />
          </Row>
          <BubbleNotice />
          <Row id="drawer.keepWarmS">
            <Toggle value={c.cfg.drawer.keepWarmS > 0} onChange={(v) => c.set("drawer.keepWarmS", v ? 60 : 0)} label={fieldLabel("drawer.keepWarmS")} />
          </Row>
        </Section>
        <Section title={t("settings.behavior.games", "Hry a celá obrazovka")}>
          <Row id="fullscreen" />
          <Row id="gameMode" hint={c.s.gameMode ? t("settings.drawer.gameMode.isOn", "Zapnutý — Ledge je teď schovaný. Vypneš ho tady nebo zkratkou herního režimu.") : t("settings.drawer.gameMode.isOff", "Vypnutý — Ledge se chová normálně.")}>
            <span class="st-status">
              <span class="st-dot" classList={{ warn: !!c.s.gameMode }} />
              <Show
                when={c.s.gameMode}
                fallback={<button class="mini" onClick={() => void api.setGameMode(true)}>{t("settings.drawer.gameMode.on", "Zapnout")}</button>}
              >
                <button class="mini ok" onClick={() => void api.setGameMode(false)}>{t("settings.drawer.gameMode.off", "Vypnout")}</button>
              </Show>
            </span>
          </Row>
        </Section>
      </div>
    </>
  );
}
