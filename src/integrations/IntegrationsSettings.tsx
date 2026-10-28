// Nastavení → Obsah karet → GitHub a Systém. GitHub je výchozí vypnutý a zapíná se jedním přepínačem s jasnou
// větou, co Ledge čte a kam to jde; Systém nic nevypíná (vzorkuje jen viditelná karta nebo položka proužku),
// jen volí disky a jednotky sítě. Mění se jen návrh Nastavení — platí hned (autosave Studia).
import { createResource, For, Show } from "solid-js";
import { t } from "../i18n";
import { useSettings } from "../settings/ctx";
import { Seg, Select, Toggle } from "../settings/fields/controls";
import { Row } from "../settings/fields/Row";
import { fieldLabel } from "../settings/labels";
import { intApi, type GithubConfig, type SystemConfig } from "./api";
import { diskOn, toggleDisk } from "./format";
import "./integrations.css";

/** Nabízené odstupy obnovy (min); jiná hodnota z ručně psaného configu se přidá do nabídky. */
const EVERY = [2, 5, 10, 15, 30, 60];

export function GithubSettings() {
  const c = useSettings();
  const g = (): GithubConfig => c.cfg.github ?? { enabled: false, review: true, mine: true, notifications: true, refreshMin: 5 };
  const options = () => {
    const cur = g().refreshMin;
    const all = EVERY.includes(cur) ? EVERY : [...EVERY, cur].sort((a, b) => a - b);
    return all.map((n) => ({ v: String(n), label: t("settings.github.everyN", "každých {n} min", { n }) }));
  };
  return (
    <>
      <Row id="github.enabled"><Toggle path="github.enabled" label={fieldLabel("github.enabled")} /></Row>
      <Row id="github.review" dim={!g().enabled}><Toggle path="github.review" label={fieldLabel("github.review")} /></Row>
      <Row id="github.mine" dim={!g().enabled}><Toggle path="github.mine" label={fieldLabel("github.mine")} /></Row>
      <Row id="github.notifications" dim={!g().enabled}><Toggle path="github.notifications" label={fieldLabel("github.notifications")} /></Row>
      <Row id="github.refreshMin" dim={!g().enabled}>
        <Select value={String(g().refreshMin)} onChange={(v) => c.set("github.refreshMin", Number(v))} options={options()} label={fieldLabel("github.refreshMin")} />
      </Row>
      <p class="int-note">{t("settings.github.note", "Přihlášení řeší příkaz gh auth login v terminálu — Ledge žádné heslo ani token nevidí a nic neukládá. Do proužku GitHub přidáš ve Vzhledu → Zavřený → Co proužek ukazuje (položka „GitHub“).")}</p>
    </>
  );
}

export function SystemSettings() {
  const c = useSettings();
  const s = (): SystemConfig => c.cfg.system ?? { disks: [], netUnit: "bytes" };
  // jeden vzorek při otevření panelu: odtud písmena disků, které ve stroji jsou (karta je ukazuje podle uloženého configu)
  const [probe] = createResource(async () => (await intApi.systemView()) ?? null);
  const letters = () => probe()?.allDisks ?? [];
  return (
    <>
      <Row id="system.disks" stack>
        <div class="int-disks" role="group" aria-label={fieldLabel("system.disks")}>
          <For each={letters()} fallback={<Show when={!probe.loading}><span class="int-note">{t("settings.system.noDisks", "Disky se nepodařilo zjistit.")}</span></Show>}>
            {(l) => (
              <button class="int-disk" classList={{ "int-disk-on": diskOn(s().disks, l) }} aria-pressed={diskOn(s().disks, l)} onClick={() => c.set("system.disks", toggleDisk(s().disks, l, letters()))}>
                {l}:
              </button>
            )}
          </For>
        </div>
      </Row>
      <Row id="system.netUnit">
        <Seg
          path="system.netUnit"
          label={fieldLabel("system.netUnit")}
          options={[
            { v: "bytes", label: t("settings.system.unit.bytes", "MB/s (bajty)") },
            { v: "bits", label: t("settings.system.unit.bits", "Mbit/s (bity)") },
          ]}
        />
      </Row>
      <p class="int-note">{t("settings.system.note", "Karta Systém měří, jen když je vidět a šuplík je otevřený. Položka „Systém“ v proužku měří každých 10 sekund. Teplotu ani grafickou kartu Ledge neukazuje — nejdou spolehlivě zjistit.")}</p>
    </>
  );
}
