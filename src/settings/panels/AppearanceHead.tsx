// Studio → Vzhled šuplíku → Otevřený: hlavička šuplíku (kolo 27, `appearance.head` / `appearance.tabs`): které díly
// se ukazují, styl hodin a štítků, nulové štítky a vzhled přepínače záložek. Stejné volby jsou i přímo v šuplíku
// (✎ → klik na hlavičku, `layout/HeadLook.tsx`); pořadí dílů se mění tam (tlačítka ← →). Logika: `layout/headModel.ts`.
import { For } from "solid-js";
import { t } from "../../i18n";
import type { ChipStyle, ClockStyle, HeadLook, HeadPart, TabsStyle } from "../../types";
import { CHIP_STYLES, CLOCK_STYLES, LOCKED, TABS_STYLES, fullOrder, normalizeHead, normalizeTabs, withHidden } from "../../layout/headModel";
import { Toggle } from "../fields/controls";
import { lit, type Ap } from "./apUi";

const PARTS: [HeadPart, () => string][] = [
  ["clock", () => t("cards.headlook.part.clock", "Hodiny")],
  ["chips", () => t("cards.headlook.part.chips", "Štítky (běží, neuloženo…)")],
  ["tabs", () => t("cards.headlook.part.tabs", "Přepínač záložek")],
  ["edit", () => t("cards.headlook.part.edit", "Úpravy ✎")],
  ["note", () => t("cards.headlook.part.note", "Poznámka")],
  ["search", () => t("cards.headlook.part.search", "Hledat")],
];
const CLOCKS: Record<ClockStyle, () => string> = {
  big: () => t("cards.headlook.clock.big", "Velké"),
  line: () => t("cards.headlook.clock.line", "Jednořádkové"),
  seconds: () => t("cards.headlook.clock.seconds", "S vteřinami"),
  analog: () => t("cards.headlook.clock.analog", "Analogové"),
  time: () => t("cards.headlook.clock.time", "Jen čas"),
};
const CHIPS: Record<ChipStyle, () => string> = {
  auto: () => t("cards.headlook.chips.auto", "Auto"),
  pills: () => t("cards.headlook.chips.pills", "Pilulky"),
  merged: () => t("cards.headlook.chips.merged", "Jedna pilulka"),
  dots: () => t("cards.headlook.chips.dots", "Tečky"),
  icons: () => t("cards.headlook.chips.icons", "Ikonky"),
};
const TABS: Record<TabsStyle, () => string> = {
  segment: () => t("cards.headlook.tabs.segment", "Segment"),
  underline: () => t("cards.headlook.tabs.underline", "Podtržení"),
  pills: () => t("cards.headlook.tabs.pills", "Pilulky"),
  icons: () => t("cards.headlook.tabs.icons", "Jen ikony"),
  dots: () => t("cards.headlook.tabs.dots", "Tečky"),
};

export default function AppearanceHead(q: { ap: Ap }) {
  const head = () => normalizeHead(q.ap.a().head);
  const tabs = () => normalizeTabs(q.ap.a().tabs);
  const put = (f: (h: HeadLook) => HeadLook) => q.ap.set("head", f(head()));
  return (
    <div class="card ap-top">
      <h4>{t("appearance.head.title", "Hlavička šuplíku")}</h4>
      <p class="ap-note">{t("appearance.head.hint", "Hodiny, štítky, záložky a nástroje nahoře v šuplíku. Přímo v šuplíku: ✎ a pak klik na hlavičku. Nastavení ⚙ a zavření ✕ vypnout nejdou.")}</p>
      <div class="opt" id="f-appearance.head.parts" classList={{ hit: lit("appearance.head.parts") }}>
        <span>{t("appearance.head.parts", "Díly hlavičky")}<small>{t("appearance.head.parts.hint", "Co se v hlavičce ukazuje; pořadí měníš v šuplíku (✎ → hlavička)")}</small></span>
        <span class="ap-range">
          <For each={PARTS.filter(([p]) => fullOrder(head()).includes(p) && !LOCKED.includes(p))}>
            {([part, name]) => (
              <label class="tl-ed-check">
                <Toggle label={name()} value={!(head().hidden ?? []).includes(part)} onChange={(v) => put((h) => withHidden(h, part, !v))} />
                {name()}
              </label>
            )}
          </For>
        </span>
      </div>
      <div class="opt" id="f-appearance.head.clock" classList={{ hit: lit("appearance.head.clock") }}>
        <span>{t("appearance.head.clock", "Hodiny v hlavičce")}<small>{t("appearance.head.clock.hint", "Velké, jednořádkové, s vteřinami, analogové nebo jen čas; vteřiny a ručičky běží jen s otevřeným šuplíkem")}</small></span>
        <select aria-label={t("appearance.head.clock", "Hodiny v hlavičce")} value={head().clock} onChange={(e) => put((h) => ({ ...h, clock: e.currentTarget.value as ClockStyle }))}>
          <For each={CLOCK_STYLES}>{(c) => <option value={c}>{CLOCKS[c]()}</option>}</For>
        </select>
        <label class="tl-ed-check">
          <Toggle label={t("cards.headlook.date", "Ukazovat datum")} value={head().showDate !== false} onChange={(v) => put(({ showDate: _s, ...h }) => (v ? h : { ...h, showDate: false }))} />
          {t("cards.headlook.date", "Ukazovat datum")}
        </label>
      </div>
      <div class="opt" id="f-appearance.head.chips" classList={{ hit: lit("appearance.head.chips") }}>
        <span>{t("appearance.head.chips", "Štítky v hlavičce")}<small>{t("appearance.head.chips.hint", "Vždy v jedné řadě. Auto zhušťuje, dokud se štítky nevejdou (pilulky → sloučená → tečky → ikonky), až pak „+N“. Nulové štítky se jinak schovají.")}</small></span>
                <select aria-label={t("appearance.head.chips", "Štítky v hlavičce")} value={head().chips} onChange={(e) => put((h) => ({ ...h, chips: e.currentTarget.value as ChipStyle }))}>
          <For each={CHIP_STYLES}>{(c) => <option value={c}>{CHIPS[c]()}</option>}</For>
        </select>
        <label class="tl-ed-check">
          <Toggle label={t("cards.headlook.zero", "Ukazovat i štítky s nulou")} value={!!head().showZero} onChange={(v) => put(({ showZero: _s, ...h }) => (v ? { ...h, showZero: true } : h))} />
          {t("cards.headlook.zero", "Ukazovat i štítky s nulou")}
        </label>
      </div>
      <div class="opt" id="f-appearance.tabs" classList={{ hit: lit("appearance.tabs") }}>
        <span>{t("appearance.tabs", "Přepínač záložek")}<small>{t("appearance.tabs.hint", "Segment, podtržení, pilulky, jen ikony nebo tečky; úpravy záložek v ✎ fungují ve všech")}</small></span>
        <select aria-label={t("appearance.tabs", "Přepínač záložek")} value={tabs().style} onChange={(e) => q.ap.set("tabs", { style: e.currentTarget.value as TabsStyle })}>
          <For each={TABS_STYLES}>{(s) => <option value={s}>{TABS[s]()}</option>}</For>
        </select>
      </div>
    </div>
  );
}
