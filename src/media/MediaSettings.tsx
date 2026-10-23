// Nastavení → Osobní → Přehrávač: vzhled karty Právě hraje s náhledy (`media.variant`), čas skladby
// v proužku (`media.stripTime`) a ozdoba ekvalizéru (`media.equalizer`). Mění se jen návrh Nastavení —
// uloží (a vrátí) ho společná lišta Uložit / Zahodit / Vrátit. Pod tím oblíbené playlisty a stanice
// (`FavoritesSettings.tsx`). Totéž pole `media.variant` mění i panel
// „Vzhled karty…“ na kartě (`layout/CardLook.tsx`) a ikonka v jejím rohu — jeden zdroj v configu.
import { For, Show } from "solid-js";
import { t } from "../i18n";
import { useSettings } from "../settings/ctx";
import { Select, Toggle } from "../settings/fields/controls";
import { fieldLabel } from "../settings/labels";
import { Row } from "../settings/fields/Row";
import type { MediaStripTime, MediaVariant } from "../api";
import { VARIANTS, variantOf } from "./timeline";
import { Thumb, variantHint, variantName } from "./VariantPicker";
import FavoritesSettings from "./FavoritesSettings";
import "./media.css";

export default function MediaSettings() {
  const c = useSettings();
  const current = (): MediaVariant => variantOf(c.cfg.media);
  const timeOpts = (): { v: MediaStripTime; label: string }[] => [
    { v: "auto", label: t("settings.media.time.auto", "Jen u vzhledu „Jen v proužku“") },
    { v: "on", label: t("settings.media.time.on", "Vždy, když přehrávač čas hlásí") },
    { v: "off", label: t("settings.media.time.off", "Nikdy") },
  ];
  return (
    <>
      <Row id="media.variant" stack>
        <div class="med-looks" role="radiogroup" aria-label={fieldLabel("media.variant")}>
          <For each={VARIANTS}>
            {(v) => (
              <button class="med-look" classList={{ "med-look-on": current() === v }} role="radio" aria-checked={current() === v} onClick={() => c.set("media.variant", v)}>
                <Thumb v={v} lg />
                <b>{variantName(v)}</b>
                <small>{variantHint(v)}</small>
              </button>
            )}
          </For>
        </div>
      </Row>
      <p class="med-cardnote">{t("settings.media.cardLook", "Akcent, pozadí a hlavičku karty nastavíš na kartě: ⋯ → Vzhled karty… (tam je i tento výběr).")}</p>
      <Row id="media.stripTime">
        <Select path="media.stripTime" label={fieldLabel("media.stripTime")} options={timeOpts()} />
      </Row>
      <Show when={current() === "strip"}>
        <Row id="media.equalizer"><Toggle path="media.equalizer" label={fieldLabel("media.equalizer")} /></Row>
      </Show>
      <FavoritesSettings id="media.favorites" />
    </>
  );
}
