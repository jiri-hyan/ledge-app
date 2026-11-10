// Kolo 27 (oblast prouzek), editor vlastního zavřeného Ledge: sekce „Písmo“ (velikost, tloušťka, proložení,
// nezávisle na výšce proužku) a rozbalení „Velikost · Pozadí · Barva textu“ u každého dílu. Čisté úpravy
// jsou v `strip/compose.ts` (`setText`, `setPieceLook`); živý náhled = `StripLookPreview` po každé změně.
import { Show } from "solid-js";
import { t } from "../../i18n";
import {
  TEXT_MAX, TEXT_MIN, TEXT_SPACINGS, TEXT_WEIGHTS, PIECE_SIZES,
  type ComposeItem, type Piece, type PieceSize, type StripCompose, type TextSpacing, type TextWeight,
  setPieceLook, setText,
} from "../../strip/compose";
import { Seg, Slider } from "./apUi";

type Q = { c: StripCompose; commit: (c: StripCompose, key?: string) => void };

const WEIGHT_NAME: Record<TextWeight, () => string> = {
  normal: () => t("appearance.compose.text.w.normal", "Normální"),
  semi: () => t("appearance.compose.text.w.semi", "Polotučné"),
  bold: () => t("appearance.compose.text.w.bold", "Tučné"),
};
const SPACING_NAME: Record<TextSpacing, () => string> = {
  tight: () => t("appearance.compose.text.s.tight", "Těsné"),
  normal: () => t("appearance.compose.text.s.normal", "Normální"),
  wide: () => t("appearance.compose.text.s.wide", "Široké"),
};
const SIZE_NAME: Record<PieceSize, () => string> = {
  small: () => t("appearance.compose.piece.size.small", "Malý"),
  normal: () => t("appearance.compose.piece.size.normal", "Normální"),
  large: () => t("appearance.compose.piece.size.large", "Velký"),
};

/** Sekce „Písmo“: tři volby, výchozí = dnešní vzhled (násobek 1, normální, normální). */
export function ComposeTextBlock(q: Q) {
  return (
    <section class="cx-blk">
      <div class="cx-bh">
        <span>{t("appearance.compose.text", "Písmo")}<small>{t("appearance.compose.textHint", "Velikost písma nezávisle na výšce proužku. Proložení mění mezery mezi díly.")}</small></span>
      </div>
      <Slider label={t("appearance.compose.text.size", "Velikost písma")} min={TEXT_MIN} max={TEXT_MAX} step={0.05} value={q.c.text.size} fmt={(v) => `${Math.round(v * 100)} %`}
        anchor="strip.compose.text" set={(v) => q.commit(setText(q.c, { size: v }), "textSize")} />
      <div class="opt">
        <span>{t("appearance.compose.text.weight", "Tloušťka")}</span>
        <Seg label={t("appearance.compose.text.weight", "Tloušťka")} value={q.c.text.weight} options={TEXT_WEIGHTS.map((v) => [v, WEIGHT_NAME[v]()] as [TextWeight, string])} set={(v) => q.commit(setText(q.c, { weight: v }))} />
      </div>
      <div class="opt">
        <span>{t("appearance.compose.text.spacing", "Proložení")}</span>
        <Seg label={t("appearance.compose.text.spacing", "Proložení")} value={q.c.text.spacing} options={TEXT_SPACINGS.map((v) => [v, SPACING_NAME[v]()] as [TextSpacing, string])} set={(v) => q.commit(setText(q.c, { spacing: v }))} />
      </div>
    </section>
  );
}

/** Výběr barvy: nic / akcent / vlastní #rrggbb (pozadí a barva textu dílu). */
function ColorPick(q: { label: string; value: string; none: string; set: (v: string) => void }) {
  return (
    <label class="cx-lp">
      <span>{q.label}</span>
      <select aria-label={q.label} value={q.value.startsWith("#") ? "#" : q.value} onChange={(e) => q.set(e.currentTarget.value === "#" ? "#ff8800" : e.currentTarget.value)}>
        <option value="">{q.none}</option>
        <option value="accent">{t("appearance.compose.colorAccent", "Akcent")}</option>
        <option value="#">{t("appearance.compose.colorCustom", "Vlastní")}</option>
      </select>
      <Show when={q.value.startsWith("#")}>
        <input type="color" value={q.value} aria-label={q.label} onInput={(e) => q.set(e.currentTarget.value)} />
      </Show>
    </label>
  );
}

/** Rozbalení u dílu: Velikost · Pozadí · Barva textu. */
export function PieceLook(q: Q & { p: Piece; itemName: string }) {
  const item = (): ComposeItem => q.p.item;
  return (
    <details class="cx-pl">
      <summary>{t("appearance.compose.piece.look", "Velikost · Pozadí · Barva textu")}</summary>
      <label class="cx-lp">
        <span>{t("appearance.compose.piece.size", "Velikost")}</span>
        <select aria-label={t("appearance.compose.piece.sizeOf", "Velikost: {name}", { name: q.itemName })} value={q.p.size ?? "normal"}
          onChange={(e) => q.commit(setPieceLook(q.c, item(), { size: e.currentTarget.value as PieceSize }))}>
          {PIECE_SIZES.map((s) => <option value={s}>{SIZE_NAME[s]()}</option>)}
        </select>
      </label>
      <ColorPick label={t("appearance.compose.piece.bg", "Pozadí")} none={t("appearance.compose.piece.bgNone", "Žádné")} value={q.p.bg ?? ""} set={(v) => q.commit(setPieceLook(q.c, item(), { bg: v }))} />
      <ColorPick label={t("appearance.compose.piece.ink", "Barva textu")} none={t("appearance.compose.colorAuto", "Podle hodnoty")} value={q.p.ink ?? ""} set={(v) => q.commit(setPieceLook(q.c, item(), { ink: v }))} />
      <label class="cx-lp">
        <span>{t("appearance.compose.piece.onlyActive", "Jen při aktivitě")}</span>
        <input
          type="checkbox"
          checked={!!q.p.onlyActive}
          aria-label={t("appearance.compose.piece.onlyActive", "Jen při aktivitě")}
          title={t("appearance.compose.piece.onlyActiveHint", "V klidu se kostka schová a uvolní v proužku místo (např. klidný Claude, 0 serverů nebo nízké vytížení).")}
          onChange={(e) => q.commit(setPieceLook(q.c, item(), { onlyActive: e.currentTarget.checked }))}
        />
      </label>
    </details>
  );
}
