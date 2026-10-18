// Panel „Upravit vzhled…“ jednoho zástupce (projekt / akce) — kreslí se UVNITŘ karty pod dlaždicemi
// (okno šuplíku je jen tak vysoké jako obsah, plovoucí panel by se ořízl). Každá volba hned mění živý
// náhled (v panelu i na samotné dlaždici, `state.ts` `lookOf`): ikona Auto / Emoji / Systémová (hledání) /
// Obrázek, barva podkladu (předvolby + vlastní), tvar, vlastní popisek, schovat popisek. Hotovo nebo klik
// mimo = uložit, Esc = zahodit, Vrátit = jak bylo při otevření, Výchozí = automatický vzhled.
import { createMemo, createSignal, For, onCleanup, onMount, Show } from "solid-js";
import { t } from "../i18n";
import { MenuChoice } from "../cards/CardMenu";
import { tx } from "../layout/text";
import { glyphChar, searchGlyphs } from "./glyphs";
import { EMOJI, MAX_EMOJI, MAX_LABEL, patchLook, SHAPE_NAMES, SHAPES, TILE_COLORS, tileLabel, type TileLook } from "./model";
import { closeLook, lookDraft, lookTarget, pickLookImage, setLook, wantKeys } from "./state";
import TileIcon, { type AutoIcon } from "./TileIcon";

type Mode = "auto" | "emoji" | "glyph" | "image";
const MODES: readonly { v: Mode; name: readonly [string, string] }[] = [
  { v: "auto", name: ["cards.tile.icon.auto", "Auto"] },
  { v: "emoji", name: ["cards.tile.icon.emoji", "Emoji"] },
  { v: "glyph", name: ["cards.tile.icon.glyph", "Ikona"] },
  { v: "image", name: ["cards.tile.icon.image", "Obrázek"] },
];

export default function LookEditor(p: { auto: AutoIcon; editing: boolean }) {
  let box!: HTMLDivElement;
  const tg = () => lookTarget()!;
  const look = (): TileLook => lookDraft() ?? {};
  const [mode, setMode] = createSignal<Mode>(look().icon?.kind ?? "auto");
  const [q, setQ] = createSignal("");
  const glyphs = createMemo(() => searchGlyphs(q(), (x) => tx(x.name)));
  const patch = (x: Partial<Record<keyof TileLook, unknown>>) => setLook(patchLook(lookDraft(), x));
  const pickMode = (m: Mode) => {
    setMode(m);
    if (m === "auto") patch({ icon: undefined });
  };
  const pickImage = async () => {
    const key = await pickLookImage();
    if (key) patch({ icon: { kind: "image", value: key } });
  };
  const close = (save: boolean) => void closeLook(save, p.editing);

  onMount(() => {
    // klik mimo panel = uložit (klik na jinou dlaždici otevře její panel a tenhle uloží sám)
    const down = (e: PointerEvent) => {
      const el = e.target as HTMLElement | null;
      if (!el || box.contains(el) || el.closest("[data-tl-tile]")) return;
      close(true);
    };
    const key = (e: KeyboardEvent) => {
      if (e.key !== "Escape") return;
      e.stopPropagation();
      e.preventDefault();
      close(false);
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
    <div class="tl-ed" ref={box} role="dialog" aria-label={t("cards.tile.editAria", "Vzhled: {name}", { name: tg().name })}>
      <div class="tl-ed-head">
        <TileIcon look={look()} auto={p.auto} size="44px" />
        <div class="tl-ed-title">
          <b>{look().hideLabel ? tg().name : tileLabel(tg().name, look())}</b>
          <small>{t("cards.tile.editTitle", "Vzhled dlaždice — změny vidíš hned")}</small>
        </div>
      </div>

      <MenuChoice label={t("cards.tile.icon", "Ikona")} items={MODES.map((m) => ({ v: m.v, text: tx(m.name) }))} value={mode()} onPick={pickMode} />
      <Show when={mode() === "emoji"}>
        <div class="tl-ed-grid" role="listbox" aria-label={t("cards.tile.icon.emoji", "Emoji")}>
          <For each={EMOJI}>
            {(e) => (
              <button class="tl-ed-e" role="option" aria-selected={look().icon?.value === e} classList={{ on: look().icon?.value === e }} onClick={() => patch({ icon: { kind: "emoji", value: e } })}>{e}</button>
            )}
          </For>
        </div>
        <div class="tl-ed-row">
          <input
            class="tl-ed-in"
            type="text"
            maxLength={MAX_EMOJI}
            placeholder={t("cards.tile.emojiOwn", "Jiné emoji (Win + .)")}
            aria-label={t("cards.tile.emojiOwn", "Jiné emoji (Win + .)")}
            value={look().icon?.kind === "emoji" ? look().icon!.value : ""}
            onFocus={wantKeys}
            onInput={(e) => e.currentTarget.value.trim() && patch({ icon: { kind: "emoji", value: e.currentTarget.value.trim() } })}
          />
        </div>
      </Show>
      <Show when={mode() === "glyph"}>
        <div class="tl-ed-row">
          <input class="tl-ed-in" type="search" placeholder={t("cards.tile.glyphSearch", "Hledat ikonu (domů, kód, hra…)")} aria-label={t("cards.tile.glyphSearch", "Hledat ikonu (domů, kód, hra…)")} value={q()} onFocus={wantKeys} onInput={(e) => setQ(e.currentTarget.value)} />
        </div>
        <div class="tl-ed-grid" role="listbox" aria-label={t("cards.tile.icon.glyph", "Ikona")}>
          <For each={glyphs()} fallback={<span class="tl-ed-note">{t("cards.tile.glyphNone", "Nic nenalezeno — zkus jiné slovo.")}</span>}>
            {(x) => (
              <button
                class="tl-ed-g"
                role="option"
                aria-selected={look().icon?.value === x.cp}
                classList={{ on: look().icon?.value === x.cp }}
                title={tx(x.name)}
                aria-label={tx(x.name)}
                onClick={() => patch({ icon: { kind: "glyph", value: x.cp } })}
              >{glyphChar(x.cp)}</button>
            )}
          </For>
        </div>
      </Show>
      <Show when={mode() === "image"}>
        <div class="tl-ed-row">
          <button class="mini" onClick={() => void pickImage()}>{t("cards.tile.pick", "Vybrat obrázek…")}</button>
          <span class="tl-ed-note">{look().icon?.kind === "image" ? t("cards.tile.picked", "Obrázek je uložený v Ledge — originál můžeš přesunout.") : t("cards.tile.pickHint", "PNG, JPG, ICO, WebP nebo SVG; zmenší se na ikonu.")}</span>
        </div>
      </Show>

      <div class="tl-ed-row" role="group" aria-label={t("cards.tile.color", "Barva")}>
        <span class="tl-ed-cap">{t("cards.tile.color", "Barva")}</span>
        <button class="tl-sw tl-sw-auto" classList={{ on: !look().color }} onClick={() => patch({ color: undefined })}>{t("cards.tile.icon.auto", "Auto")}</button>
        <For each={TILE_COLORS}>
          {(c) => <button class="tl-sw" classList={{ on: look().color === c }} style={{ "--tl-c": c }} aria-label={c} title={c} onClick={() => patch({ color: c })} />}
        </For>
        <label class="tl-custom" title={t("cards.tile.colorOwn", "Vlastní barva")}>
          <input type="color" value={look().color ?? p.auto.color} aria-label={t("cards.tile.colorOwn", "Vlastní barva")} onInput={(e) => patch({ color: e.currentTarget.value })} />
          {t("cards.tile.colorOwnShort", "Vlastní")}
        </label>
      </div>
      <MenuChoice label={t("cards.tile.shape", "Tvar")} items={SHAPES.map((s) => ({ v: s, text: tx(SHAPE_NAMES[s]) }))} value={look().shape ?? "rounded"} onPick={(v) => patch({ shape: v === "rounded" ? undefined : v })} />
      <div class="tl-ed-row">
        <span class="tl-ed-cap">{t("cards.tile.label", "Popisek")}</span>
        <input
          class="tl-ed-in"
          type="text"
          maxLength={MAX_LABEL}
          placeholder={tg().name}
          aria-label={t("cards.tile.label", "Popisek")}
          value={look().label ?? ""}
          onFocus={wantKeys}
          onInput={(e) => patch({ label: e.currentTarget.value })}
        />
        <label class="tl-ed-check"><input type="checkbox" checked={!!look().hideLabel} onChange={(e) => patch({ hideLabel: e.currentTarget.checked })} />{t("cards.tile.hideLabel", "Schovat popisek")}</label>
      </div>

      <div class="tl-ed-foot">
        <button class="mini" onClick={() => { setLook(undefined); setMode("auto"); }}>{t("cards.tile.reset", "Výchozí")}</button>
        <button class="mini" onClick={() => { setLook(tg().initial); setMode(tg().initial?.icon?.kind ?? "auto"); }}>{t("cards.tile.undo", "Vrátit")}</button>
        <span class="grow" />
        <button class="mini ok" onClick={() => close(true)}>{t("cards.tile.done", "Hotovo")}</button>
      </div>
    </div>
  );
}
