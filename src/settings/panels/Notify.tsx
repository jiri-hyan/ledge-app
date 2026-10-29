// Sekce „Oznámení“ na stránce Claude (kolo 26, Rust `notify/`): KDE (Notch v proužku / toast Windows,
// záložní toast) a JAK (doba, styl pro každou událost). ZDA se událost hlásí, zapínají dnešní vypínače
// (`claude.toast`, `claude.limitsAlert`, `timer.toast`, `remindToast` u kalendáře) — tady se jen ukazuje jejich
// stav. „Vyzkoušet“ = `notify_preview` → ukázka v proužku hned (smyšlený obsah, tlačítko jen zavře).
import { For, Show } from "solid-js";
import { api } from "../../api";
import { t } from "../../i18n";
import { sayError } from "../../ui/toast";
import { useSettings } from "../ctx";
import { Select, Seg, Slider, Toggle } from "../fields/controls";
import { Row, Section } from "../fields/Row";
import { fieldLabel } from "../labels";
import { NOTIFY_DEFAULT, type NotchStyle, type NotifyKind } from "../../types/notify";

const STYLES: { v: NotchStyle; key: string; cs: string }[] = [
  { v: "unfold", key: "settings.notify.style.unfold", cs: "Rozbalení" },
  { v: "sides", key: "settings.notify.style.sides", cs: "Do stran" },
  { v: "drop", key: "settings.notify.style.drop", cs: "Kapka" },
  { v: "ticker", key: "settings.notify.style.ticker", cs: "Ticker" },
  { v: "glow", key: "settings.notify.style.glow", cs: "Jen záře" },
];

const KINDS: NotifyKind[] = ["waiting", "done", "meeting", "limit", "timer"];

/** Zapíná událost dnešní vypínač? (null = rozhoduje každý kalendář zvlášť) */
function sourceOn(cfg: any, k: NotifyKind): boolean | null {
  switch (k) {
    case "waiting":
      return !!cfg.claude?.enabled && !!cfg.claude?.toast;
    case "done":
      return !!cfg.claude?.enabled && (cfg.claude?.toastDone ?? !!cfg.claude?.toast);
    case "limit":
      return !!cfg.claude?.limitsAlert;
    case "timer":
      return cfg.timer?.toast !== false;
    case "meeting":
      return null;
  }
}

const SOURCE: Record<NotifyKind, [string, string]> = {
  waiting: ["settings.notify.src.claude", "Zapíná „Claude čeká“ v sekci Claude výš."],
  done: ["settings.notify.src.claudeDone", "Zapíná přepínač výše; dokud ho nezměníš, řídí se „Claude čeká“."],
  limit: ["settings.notify.src.limit", "Zapíná „Upozornit na 80 % a 90 % limitu“ v Limitech výš."],
  timer: ["settings.notify.src.timer", "Zapíná „Oznámení po doběhnutí“ v Obsahu karet → Časovač."],
  meeting: ["settings.notify.src.meeting", "Zapíná se u každého kalendáře (Obsah karet → Kalendář → připomínka)."],
};

export default function NotifySection() {
  const c = useSettings();
  const notch = () => (c.cfg.notify?.surface ?? NOTIFY_DEFAULT.surface) === "notch";
  const styleOf = (k: NotifyKind): NotchStyle => c.cfg.notify?.style?.[k] ?? NOTIFY_DEFAULT.style[k];
  const tryIt = (k: NotifyKind) => {
    api.notifyPreview(k, styleOf(k)).catch(sayError);
  };
  return (
    <Section title={t("settings.notify.title", "Oznámení")} desc={t("settings.notify.desc", "Událost rozbalí proužek nahoře a po chvíli ho zase složí. Nebere fokus; ve hře a při Nerušit nic.")}>
      <Row id="notify.surface" stack>
        <Seg
          path="notify.surface"
          label={fieldLabel("notify.surface")}
          options={[
            { v: "notch", label: t("settings.notify.surface.notch", "V proužku (Notch)") },
            { v: "toast", label: t("settings.notify.surface.toast", "Jen oznámení Windows") },
          ]}
        />
      </Row>
      <Show when={notch()}>
        <Row id="notify.fallbackToast" nested>
          <Toggle path="notify.fallbackToast" label={fieldLabel("notify.fallbackToast")} />
        </Row>
        <Row id="notify.durationS" nested>
          <Slider path="notify.durationS" min={2} max={30} step={1} label={fieldLabel("notify.durationS")} fmt={(v) => `${v} s`} />
        </Row>
        <For each={KINDS}>
          {(k) => {
            const id = `notify.style.${k}`;
            const on = () => sourceOn(c.cfg, k);
            return (
              <>
              <Show when={k === "done"}>
                <Row id="claude.toastDone" dim={!c.cfg.claude?.enabled}>
                  <Toggle value={on() === true} onChange={(v) => c.set("claude.toastDone", v)} label={fieldLabel("claude.toastDone")} />
                </Row>
              </Show>
              <Row id={`notify.style.${k}`} stack>
                <Select path={id} label={fieldLabel(id)} options={STYLES.map((s) => ({ v: s.v, label: t(s.key, s.cs) }))} />
                <div class="st-actions">
                  <button type="button" class="mini" onClick={() => tryIt(k)}>
                    {t("settings.notify.try", "Vyzkoušet")}
                  </button>
                  <Show when={styleOf(k) !== NOTIFY_DEFAULT.style[k]}>
                    <button type="button" class="mini" onClick={() => c.set(id, NOTIFY_DEFAULT.style[k])}>
                      {t("settings.notify.reset", "Vrátit")}
                    </button>
                  </Show>
                  <small class="st-note">
                    <Show when={on() === false}>{t("settings.notify.off", "Teď vypnuto.")} </Show>
                    {t(...SOURCE[k])}
                  </small>
                </div>
              </Row>
              </>
            );
          }}
        </For>
      </Show>
    </Section>
  );
}
