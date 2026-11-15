// Sekce „Oznámení“ na stránce Claude (kolo 26, Rust `notify/`): KDE (Notch v proužku / toast Windows,
// záložní toast) a JAK (doba, styl pro každou událost). Od kola 33 je vypínač události (`claude.toast`,
// `claude.toastDone`, `claude.limitsAlert`, `timer.toast`) sloučený se stylem do JEDNOHO Selectu
// (Vypnuto / styl; `merged.ts`, config na disku se nemění). Schůzka vypínač nemá — zapíná se u každého kalendáře
// (`remindToast`). „Vyzkoušet“ = `notify_preview` → ukázka v proužku hned (smyšlený obsah, tlačítko jen zavře).
import { For, Show } from "solid-js";
import { api } from "../../api";
import { t } from "../../i18n";
import { sayError } from "../../ui/toast";
import { useSettings } from "../ctx";
import { Select, Seg, Slider, Toggle } from "../fields/controls";
import { Row, Section } from "../fields/Row";
import { fieldLabel } from "../labels";
import { notifyChoice, notifyOn, notifyWrites, type NotifyChoice } from "../merged";
import { NOTIFY_DEFAULT, type NotchStyle, type NotifyKind } from "../../types/notify";

const STYLES: { v: NotchStyle; key: string; cs: string }[] = [
  { v: "unfold", key: "settings.notify.style.unfold", cs: "Rozbalení" },
  { v: "sides", key: "settings.notify.style.sides", cs: "Do stran" },
  { v: "drop", key: "settings.notify.style.drop", cs: "Kapka" },
  { v: "ticker", key: "settings.notify.style.ticker", cs: "Ticker" },
  { v: "glow", key: "settings.notify.style.glow", cs: "Jen záře" },
];

const KINDS: NotifyKind[] = ["waiting", "done", "meeting", "limit", "timer"];

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
      </Show>
      <For each={KINDS}>
        {(k) => {
          const id = `notify.style.${k}`;
          const switchable = () => notifyOn(c.cfg, k) !== null;
          const options = (): { v: NotifyChoice; label: string }[] => [
            ...(switchable() ? [{ v: "off" as NotifyChoice, label: t("settings.notify.choice.off", "Vypnuto") }] : []),
            ...(notch() ? STYLES.map((s) => ({ v: s.v as NotifyChoice, label: t(s.key, s.cs) })) : [{ v: "on" as NotifyChoice, label: t("settings.notify.choice.on", "Zapnuto (oznámení Windows)") }]),
          ];
          return (
            <Show when={notch() || switchable()}>
              <Row id={`notify.style.${k}`} stack dim={(k === "waiting" || k === "done") && !c.cfg.claude?.enabled}>
                <Select value={notifyChoice(c.cfg, k, notch())} onChange={(v: NotifyChoice) => notifyWrites(k, v).forEach(([p, x]) => c.set(p, x))} label={fieldLabel(id)} options={options()} />
                <Show when={notch()}>
                  <div class="st-actions">
                    <button type="button" class="mini" onClick={() => tryIt(k)}>
                      {t("settings.notify.try", "Vyzkoušet")}
                    </button>
                    <Show when={styleOf(k) !== NOTIFY_DEFAULT.style[k]}>
                      <button type="button" class="mini" onClick={() => c.set(id, NOTIFY_DEFAULT.style[k])}>
                        {t("settings.notify.reset", "Vrátit")}
                      </button>
                    </Show>
                    <Show when={k === "meeting"}>
                      <small class="st-note">{t("settings.notify.src.meeting", "Zapíná se u každého kalendáře (Obsah karet → Kalendář → připomínka).")}</small>
                    </Show>
                  </div>
                </Show>
              </Row>
            </Show>
          );
        }}
      </For>
    </Section>
  );
}
