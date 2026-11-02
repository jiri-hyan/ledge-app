// Panel sekce lišty notche v režimu ✎ (kolo 27), vzor `layout/CardLook.tsx`: vzhledy s náhledy (`DesignPicker`),
// barva, u Systému tloušťka / vyhlazení / metriky, u analogových hodin vteřinovka; Odebrat, Vrátit, Hotovo. Změny
// jsou hned vidět v liště nad panelem (živý náhled z `barState.ts`), uloží se do draftu úprav (Hotovo / klik
// mimo), Esc zahodí. Bez rozpracované sekce ukazuje nabídku „+“ = sekce, které na liště ještě nejsou.
import { For, Show, onCleanup, onMount } from "solid-js";
import { t } from "../i18n";
import { MenuChoice } from "../cards/CardMenu";
import DesignPicker from "../layout/DesignPicker";
import type { LayoutPage } from "../layout/model";
import { tx } from "../layout/text";
import { addable, colorOk, metricsOf, toggleMetric, METRICS, WEIGHTS, type NotchBar } from "./bar";
import { BAR_NAMES, barDesignOf, barDesigns } from "./designs";
import { barThumb } from "./barThumbs";
import { addBarSection, barDraft, barTarget, closeSection, patchDraft, removeBarSection, setBarAdding, setBarDraft } from "./barState";
import "../tilelook/tilelook.css";

const WEIGHT_NAMES: Record<string, readonly [string, string]> = {
  thin: ["cards.notch.w.thin", "Tenká"],
  mid: ["cards.notch.w.mid", "Střední"],
  bold: ["cards.notch.w.bold", "Silná"],
};
const METRIC_NAMES: Record<string, readonly [string, string]> = {
  cpu: ["cards.notch.m.cpu", "CPU"],
  ram: ["cards.notch.m.ram", "RAM"],
  disk: ["cards.notch.m.disk", "Disk"],
  net: ["cards.notch.m.net", "Síť"],
};
/** Sekce, kde dává smysl barva podle zátěže / přechod (mají procenta nebo pruhy). */
const LOADED = ["system", "limits", "volume"];

export default function BarPanel(p: { page: LayoutPage; bar: NotchBar }) {
  let box!: HTMLDivElement;
  onMount(() => {
    const down = (e: PointerEvent) => {
      const el = e.target as HTMLElement | null;
      // klik mimo panel a lištu = uložit / zavřít nabídku (klik na jinou sekci si panel přepne sám)
      if (!el || box.contains(el) || el.closest(".nb-edit")) return;
      setBarAdding(null);
      closeSection(p.page, true);
    };
    const key = (e: KeyboardEvent) => {
      if (e.key !== "Escape") return;
      e.stopPropagation();
      e.preventDefault();
      setBarAdding(null);
      closeSection(p.page, false);
    };
    document.addEventListener("pointerdown", down, true);
    window.addEventListener("keydown", key, true);
    onCleanup(() => {
      document.removeEventListener("pointerdown", down, true);
      window.removeEventListener("keydown", key, true);
    });
    box.scrollIntoView({ block: "nearest" });
  });
  return (
    <div class="tl-ed tl-card nb-panel" ref={box} role="dialog" aria-label={t("cards.notch.panelAria", "Sekce lišty výřezu")}>
      <Show when={barDraft()} fallback={<AddList page={p.page} bar={p.bar} />}>
        {(d) => {
          const sec = () => d().id;
          const color = () => d().color;
          return (
            <>
              <div class="tl-ed-head">
                <div class="tl-ed-title">
                  <b>{t("cards.notch.panelTitle", "Sekce lišty: {name}", { name: tx(BAR_NAMES[sec()]) })}</b>
                  <small>{t("cards.notch.panelHint", "Změny vidíš hned v liště nahoře. Pořadí změníš tahem sekce.")}</small>
                </div>
              </div>
              <DesignPicker
                card={`notch:${sec()}`}
                list={barDesigns(sec())}
                title={t("cards.notch.design", "Vzhled sekce")}
                thumb={barThumb(sec())}
                look={{ design: barDesignOf(sec(), d().design) }}
                onPick={(id) => patchDraft({ design: id })}
              />
              <div class="tl-ed-row" role="group" aria-label={t("cards.notch.color", "Barva")}>
                <span class="tl-ed-cap">{t("cards.notch.color", "Barva")}</span>
                <button class="tl-sw tl-sw-auto" classList={{ on: !color() }} onClick={() => patchDraft({ color: undefined })}>{t("cards.notch.c.accent", "Akcent")}</button>
                <Show when={LOADED.includes(sec())}>
                  <button class="tl-sw tl-sw-auto" classList={{ on: color() === "load" }} title={t("cards.notch.c.loadHint", "Zelená → jantarová → červená podle procent")} onClick={() => patchDraft({ color: "load" })}>{t("cards.notch.c.load", "Podle zátěže")}</button>
                  <button class="tl-sw tl-sw-auto" classList={{ on: color() === "gradient" }} onClick={() => patchDraft({ color: "gradient" })}>{t("cards.notch.c.gradient", "Přechod")}</button>
                </Show>
                <label class="tl-custom" title={t("cards.tile.colorOwn", "Vlastní barva")}>
                  <input type="color" value={color()?.startsWith("#") ? color() : "#a78bfa"} aria-label={t("cards.tile.colorOwn", "Vlastní barva")} onInput={(e) => colorOk(e.currentTarget.value) && patchDraft({ color: e.currentTarget.value })} />
                  {t("cards.tile.colorOwnShort", "Vlastní")}
                </label>
              </div>
              <Show when={sec() === "system"}>
                <MenuChoice label={t("cards.notch.weight", "Tloušťka čáry")} items={WEIGHTS.map((w) => ({ v: w, text: tx(WEIGHT_NAMES[w]) }))} value={d().weight ?? "mid"} onPick={(v) => patchDraft({ weight: v })} />
                <div class="tl-ed-row" role="group" aria-label={t("cards.notch.metrics", "Metriky")}>
                  <span class="tl-ed-cap">{t("cards.notch.metrics", "Metriky")}</span>
                  <For each={METRICS}>
                    {(m) => (
                      <label class="tl-ed-check">
                        <input type="checkbox" checked={metricsOf(d()).includes(m)} onChange={() => patchDraft({ metrics: toggleMetric(d(), m) })} />
                        {tx(METRIC_NAMES[m])}
                      </label>
                    )}
                  </For>
                </div>
                <div class="tl-ed-row">
                  <label class="tl-ed-check">
                    <input type="checkbox" checked={!!d().smooth} onChange={(e) => patchDraft({ smooth: e.currentTarget.checked || undefined })} />
                    {t("cards.notch.smooth", "Vyhlazené křivky")}
                  </label>
                </div>
              </Show>
              <Show when={sec() === "days" && barDesignOf("days", d().design) === "analog"}>
                <div class="tl-ed-row">
                  <label class="tl-ed-check" title={t("cards.notch.secondsHint", "Tiká jen s otevřeným šuplíkem")}>
                    <input type="checkbox" checked={!!d().seconds} onChange={(e) => patchDraft({ seconds: e.currentTarget.checked || undefined })} />
                    {t("cards.notch.seconds", "Vteřinová ručička")}
                  </label>
                </div>
              </Show>
              <div class="tl-ed-foot">
                <button class="mini" onClick={() => removeBarSection(p.page, sec())}>{t("cards.notch.remove", "Odebrat z lišty")}</button>
                <button class="mini" onClick={() => setBarDraft(barTarget()?.initial ?? d())}>{t("cards.tile.undo", "Vrátit")}</button>
                <span class="grow" />
                <button class="mini ok" onClick={() => closeSection(p.page, true)}>{t("cards.tile.done", "Hotovo")}</button>
              </div>
            </>
          );
        }}
      </Show>
    </div>
  );
}

function AddList(p: { page: LayoutPage; bar: NotchBar }) {
  return (
    <div class="tl-ed-col" role="group" aria-label={t("cards.notch.add", "Přidat sekci do lišty")}>
      <span class="tl-ed-cap">{t("cards.notch.add", "Přidat sekci do lišty")}</span>
      <div class="dz-grid">
        <For each={addable(p.bar)}>
          {(id) => (
            <button class="dz-tile" title={tx(barDesigns(id)[0].line)} onClick={() => addBarSection(p.page, id)}>
              <svg class="dz-thumb" viewBox="0 0 40 24" aria-hidden="true">{barThumb(id)(barDesigns(id)[0].id)}</svg>
              <b>{tx(BAR_NAMES[id])}</b>
            </button>
          )}
        </For>
      </div>
      <div class="tl-ed-foot">
        <span class="grow" />
        <button class="mini" onClick={() => setBarAdding(null)}>{t("cards.notch.close", "Zavřít")}</button>
      </div>
    </div>
  );
}
