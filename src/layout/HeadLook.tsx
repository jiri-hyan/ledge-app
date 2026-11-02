// „Vzhled hlavičky…“ (✎ → klik na hlavičku nebo na přepínač záložek): díly hlavičky (zapnout / pořadí), styl hodin,
// štítků a přepínače záložek. Změny vidíš hned v hlavičce nad panelem; Vrátit = zpět na stav při otevření,
// Hotovo = uložit do `appearance.head` / `appearance.tabs`. Stav a uložení: `headState.ts`, čistá logika `headModel.ts`.
import { For, onCleanup, onMount, Show } from "solid-js";
import { t } from "../i18n";
import type { ChipStyle, ClockStyle, HeadLook as Head, HeadPart, TabsStyle } from "../types.ts";
import { CHIP_STYLES, CLOCK_STYLES, fullOrder, LOCKED, movePart, TABS_STYLES, withHidden, withOrder } from "./headModel";
import { closeHeadLook, headDraft, revertHeadLook, setHeadDraft } from "./headState";
import "../tilelook/tilelook.css";

const PART_NAMES = (): Record<HeadPart, string> => ({
  clock: t("cards.headlook.part.clock", "Hodiny"),
  chips: t("cards.headlook.part.chips", "Štítky (běží, neuloženo…)"),
  tabs: t("cards.headlook.part.tabs", "Přepínač záložek"),
  edit: t("cards.headlook.part.edit", "Úpravy ✎"),
  note: t("cards.headlook.part.note", "Poznámka"),
  search: t("cards.headlook.part.search", "Hledat"),
  settings: t("cards.headlook.part.settings", "Nastavení"),
  close: t("cards.headlook.part.close", "Zavřít ✕"),
});
const CLOCK_NAMES = (): Record<ClockStyle, string> => ({
  big: t("cards.headlook.clock.big", "Velké"),
  line: t("cards.headlook.clock.line", "Jednořádkové"),
  seconds: t("cards.headlook.clock.seconds", "S vteřinami"),
  analog: t("cards.headlook.clock.analog", "Analogové"),
  time: t("cards.headlook.clock.time", "Jen čas"),
});
const CHIP_NAMES = (): Record<ChipStyle, string> => ({
  auto: t("cards.headlook.chips.auto", "Auto"),
  pills: t("cards.headlook.chips.pills", "Pilulky"),
  merged: t("cards.headlook.chips.merged", "Jedna pilulka"),
  dots: t("cards.headlook.chips.dots", "Tečky"),
  icons: t("cards.headlook.chips.icons", "Ikonky"),
});
const TAB_NAMES = (): Record<TabsStyle, string> => ({
  segment: t("cards.headlook.tabs.segment", "Segment"),
  underline: t("cards.headlook.tabs.underline", "Podtržení"),
  pills: t("cards.headlook.tabs.pills", "Pilulky"),
  icons: t("cards.headlook.tabs.icons", "Jen ikony"),
  dots: t("cards.headlook.tabs.dots", "Tečky"),
});

function Choice<T extends string>(p: { label: string; items: readonly T[]; names: Record<T, string>; value: T; onPick: (v: T) => void }) {
  return (
    <div class="tl-ed-row" role="radiogroup" aria-label={p.label}>
      <span class="tl-ed-cap">{p.label}</span>
      <div class="seg hl-seg">
        <For each={p.items}>
          {(v) => <button classList={{ on: p.value === v }} role="radio" aria-checked={p.value === v} onClick={() => p.onPick(v)}>{p.names[v]}</button>}
        </For>
      </div>
    </div>
  );
}

export default function HeadLookPanel() {
  let box!: HTMLDivElement;
  const d = () => headDraft()!;
  const head = () => d().head;
  const patch = (f: (h: Head) => Head) => setHeadDraft({ ...d(), head: f(head()) });
  onMount(() => {
    const key = (e: KeyboardEvent) => e.key === "Escape" && (e.stopPropagation(), void closeHeadLook(false));
    window.addEventListener("keydown", key, true);
    onCleanup(() => window.removeEventListener("keydown", key, true));
    box.scrollIntoView({ block: "nearest" });
  });
  const names = PART_NAMES;
  return (
    <Show when={headDraft()}>
      <div class="tl-ed tl-card hl-panel" ref={box} role="dialog" aria-label={t("cards.headlook.aria", "Vzhled hlavičky")}>
        <div class="tl-ed-head">
          <div class="tl-ed-title">
            <b>{t("cards.headlook.title", "Vzhled hlavičky")}</b>
            <small>{t("cards.headlook.hint", "Změny vidíš hned v hlavičce nad panelem.")}</small>
          </div>
        </div>
        <div class="hl-parts" role="group" aria-label={t("cards.headlook.parts", "Díly hlavičky")}>
          <For each={fullOrder(head())}>
            {(part) => {
              const locked = LOCKED.includes(part);
              const on = () => !(head().hidden ?? []).includes(part);
              return (
                <div class="hl-part" classList={{ off: !on() }}>
                  <label class="hl-check">
                    <input type="checkbox" checked={on()} disabled={locked} onChange={(e) => patch((h) => withHidden(h, part, !e.currentTarget.checked))} />
                    {names()[part]}
                  </label>
                  <button class="mini" disabled={part === "close"} title={t("cards.headlook.left", "Posunout doleva")} aria-label={t("cards.headlook.left", "Posunout doleva")} onClick={() => patch((h) => withOrder(h, movePart(h, part, -1)))}>←</button>
                  <button class="mini" disabled={part === "close"} title={t("cards.headlook.right", "Posunout doprava")} aria-label={t("cards.headlook.right", "Posunout doprava")} onClick={() => patch((h) => withOrder(h, movePart(h, part, 1)))}>→</button>
                </div>
              );
            }}
          </For>
        </div>
        <Choice label={t("cards.headlook.clock", "Hodiny")} items={CLOCK_STYLES} names={CLOCK_NAMES()} value={head().clock} onPick={(v) => patch((h) => ({ ...h, clock: v }))} />
        <div class="tl-ed-row">
          <label class="tl-ed-check">
            <input type="checkbox" checked={head().showDate !== false} onChange={(e) => patch(({ showDate: _s, ...h }) => (e.currentTarget.checked ? h : { ...h, showDate: false }))} />
            {t("cards.headlook.date", "Ukazovat datum")}
          </label>
        </div>
        <Choice label={t("cards.headlook.chips", "Štítky")} items={CHIP_STYLES} names={CHIP_NAMES()} value={head().chips} onPick={(v) => patch((h) => ({ ...h, chips: v }))} />
        <div class="tl-ed-row">
          <label class="tl-ed-check" title={t("cards.headlook.zeroHint", "Jinak se štítek s nulou (třeba „0 běží“) schová")}>
            <input type="checkbox" checked={!!head().showZero} onChange={(e) => patch(({ showZero: _s, ...h }) => (e.currentTarget.checked ? { ...h, showZero: true } : h))} />
            {t("cards.headlook.zero", "Ukazovat i štítky s nulou")}
          </label>
        </div>
        <Choice label={t("cards.headlook.tabs", "Záložky")} items={TABS_STYLES} names={TAB_NAMES()} value={d().tabs.style} onPick={(v) => setHeadDraft({ ...d(), tabs: { style: v } })} />
        <div class="tl-ed-foot">
          <button class="mini" onClick={() => revertHeadLook()}>{t("cards.tile.undo", "Vrátit")}</button>
          <span class="grow" />
          <button class="mini ok" onClick={() => void closeHeadLook(true)}>{t("cards.tile.done", "Hotovo")}</button>
        </div>
      </div>
    </Show>
  );
}
