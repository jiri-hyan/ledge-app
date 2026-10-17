// Osobní: baterie jako malý údaj v kartě Čas (Rust `battery.rs`). Jen na stroji s baterií — jinak
// (desktop, vypnuto v Nastavení) Rust pošle `null` a nic se nevykreslí. Zbývající čas jen, když ho
// Windows odhadne.
import { Show } from "solid-js";
import type { BatteryView } from "../api";
import { t } from "../i18n";
import { batteryLow, remaining } from "./model";
import "./media.css";

export default function Battery(p: { b: BatteryView | null }) {
  const left = () => {
    const r = remaining(p.b?.remainingS);
    if (!r) return "";
    if (r.h && r.m) return t("cards.battery.leftHM", "zbývá {h} h {m} min", r);
    if (r.h) return t("cards.battery.leftH", "zbývá {h} h", r);
    return r.m ? t("cards.battery.leftM", "zbývá {m} min", r) : t("cards.battery.leftLess", "zbývá méně než minuta");
  };
  return (
    <Show when={p.b}>
      {(b) => (
        <div class="med-bat" classList={{ "med-bat-low": batteryLow(b().percent, b().plugged) }}>
          <span class="med-bat-icon" aria-hidden="true">
            <span class="med-bat-fill" style={{ width: `${b().percent}%` }} />
          </span>
          <span class="med-bat-pct">{t("cards.battery.pct", "{n} %", { n: b().percent })}</span>
          <span class="med-bat-state">
            {b().charging ? t("cards.battery.charging", "nabíjí se") : b().plugged ? t("cards.battery.plugged", "v síti") : left() || t("cards.battery.onBattery", "na baterii")}
          </span>
        </div>
      )}
    </Show>
  );
}
