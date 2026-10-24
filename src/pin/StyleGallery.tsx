// Galerie stylů lístku (Lístky 2.0): dlaždice s živým náhledem malého lístku (`styles.ts` `pinLook`) — v okně
// lístku (⋯ → Vzhled) i v Nastavení → Poznámky → Lístky (výchozí styl). Jedním klikem, vybraný je označený.
// Vlastní styly v `gallery.css` (okno lístku nenačítá styly šuplíku a naopak).
import { For } from "solid-js";
import { t } from "../i18n";
import { PIN_STYLES, pinLook, type PinStyleId } from "./styles";
import "./gallery.css";

export const styleName = (s: PinStyleId): string =>
  ({
    classic: t("notes.pin.style.classic", "Tmavé sklo"),
    paper: t("notes.pin.style.paper", "Papír"),
    glass: t("notes.pin.style.glass", "Sklo"),
    stripe: t("notes.pin.style.stripe", "Barevný proužek"),
    minimal: t("notes.pin.style.minimal", "Minimal"),
    dark: t("notes.pin.style.dark", "Tmavý"),
  })[s];

/** Lidský název barvy z palety (`notes/meta.ts` `NOTE_COLORS`) — ⋯ lístku i Nastavení. */
export const colorName = (id: string): string =>
  ({
    yellow: t("notes.color.yellow", "Žlutá"),
    orange: t("notes.color.orange", "Oranžová"),
    red: t("notes.color.red", "Červená"),
    pink: t("notes.color.pink", "Růžová"),
    purple: t("notes.color.purple", "Fialová"),
    blue: t("notes.color.blue", "Modrá"),
    teal: t("notes.color.teal", "Tyrkysová"),
    green: t("notes.color.green", "Zelená"),
  })[id] ?? id;

export default function StyleGallery(p: { value: string; bg: number; text: number; color: string | null; onPick: (s: PinStyleId) => void }) {
  return (
    <div class="pin-gal" role="radiogroup" aria-label={t("notes.pin.style.title", "Styl lístku")}>
      <For each={PIN_STYLES}>
        {(s) => (
          <button type="button" class="pin-gal-tile" classList={{ on: p.value === s }} role="radio" aria-checked={p.value === s} title={styleName(s)} onClick={() => p.onPick(s)}>
            <span class="pin-gal-prev" data-style={s} style={pinLook(s, p.bg, p.text, p.color)}>
              <span class="pin-gal-bar" />
              <span class="pin-gal-line" />
              <span class="pin-gal-line pin-gal-short" />
            </span>
            <span class="pin-gal-name">{styleName(s)}</span>
          </button>
        )}
      </For>
    </div>
  );
}
