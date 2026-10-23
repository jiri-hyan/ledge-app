// „Vzhled karty…“ (⋯ na kartě): akcent, pozadí, skrytá hlavička, hustota a u karet, kde to dává smysl,
// varianta zobrazení. Panel se kreslí v toku stránky pod mřížkou (okno šuplíku je jen tak vysoké jako
// obsah — plovoucí panel by se ořízl); karta nad ním je živý náhled (`lookFor` → `Deck` atributy
// `data-cl-*`). Hotovo / klik mimo = uložit (v režimu úprav do draftu), Esc = zahodit, Vrátit, Obnovit
// podle stylu. Obecné pro každou kartu (i nové karty jiných modulů) — řeší to `.deck-item`, ne karta.
// Panel (UI) — stav a `openCardLook` / `lookFor` jsou v malém `lookState.ts` (eager); tenhle soubor se načítá líně.
// Jediný panel vzhledu karty (kolo 13): u karty Právě hraje v něm je i výběr varianty přehrávače
// (`media.variant`, miniatury z `media/VariantPicker.tsx`; ukládá se hned jako ikonka v rohu karty a
// Nastavení → Osobní — jeden zdroj v configu), u karet se zástupci (Projekty, Akce) výběr dlaždice, jejíž
// vzhled (`tilelook/*`) se má upravit. Ikonka v rohu přehrávače a pravý klik na dlaždici jsou zkratky.
import { For, onCleanup, onMount, Show } from "solid-js";
import { t } from "../i18n";
import { MenuChoice } from "../cards/CardMenu";
import type { Snapshot } from "../api";
import { Thumb, saveVariant, shownVariant, variantHint, variantName } from "../media/VariantPicker";
import { VARIANTS } from "../media/timeline";
import { openLook } from "../tilelook/state";
import { closeCardLook, draft, setDraft, target, type Target } from "./lookState";
import {
  ACCENT_NAMES, accentCss, BG_NAMES, BGS, CARD_ACCENTS, DENSITIES, DENSITY_NAMES, patchCardLook, VIEW_CARDS, VIEW_NAMES, VIEWS,
  type CardLook,
} from "./look";
import { cardName, tx } from "./text";
import type { CardId } from "./model";
import "../tilelook/tilelook.css";
import "../media/media.css";

/** Panel pod mřížkou (MainPage), jen pro kartu na zobrazené záložce. */
export default function CardLookPanel(p: { page: string; s?: Snapshot }) {
  return (
    <Show when={target()?.page === p.page ? target() : null} keyed>
      {(tg) => <Panel tg={tg} s={p.s} />}
    </Show>
  );
}

function Panel(p: { tg: Target; s?: Snapshot }) {
  let box!: HTMLDivElement;
  const look = (): CardLook => draft() ?? {};
  const patch = (x: Partial<Record<keyof CardLook, unknown>>) => setDraft(patchCardLook(draft(), p.tg.card, x));
  const view = VIEW_CARDS[p.tg.card];
  onMount(() => {
    const down = (e: PointerEvent) => {
      const el = e.target as HTMLElement | null;
      // klik mimo panel = uložit; ⋯ jiné karty si panel přepne sám
      if (!el || box.contains(el) || el.closest("[data-cmenu]")) return;
      void closeCardLook(true);
    };
    const key = (e: KeyboardEvent) => {
      if (e.key !== "Escape") return;
      e.stopPropagation();
      e.preventDefault();
      void closeCardLook(false);
    };
    document.addEventListener("pointerdown", down, true);
    window.addEventListener("keydown", key, true);
    onCleanup(() => {
      document.removeEventListener("pointerdown", down, true);
      window.removeEventListener("keydown", key, true);
    });
    box.scrollIntoView({ block: "nearest" });
  });
  const accent = () => look().accent;
  return (
    <div class="tl-ed tl-card" ref={box} role="dialog" aria-label={t("cards.cardlook.aria", "Vzhled karty {name}", { name: cardName(p.tg.card as CardId) })}>
      <div class="tl-ed-head">
        <div class="tl-ed-title">
          <b>{t("cards.cardlook.title", "Vzhled karty: {name}", { name: cardName(p.tg.card as CardId) })}</b>
          <small>{t("cards.cardlook.hint", "Změny vidíš hned na kartě. Jen tahle karta na téhle záložce.")}</small>
        </div>
      </div>
      <div class="tl-ed-row" role="group" aria-label={t("cards.cardlook.accent", "Akcent")}>
        <span class="tl-ed-cap">{t("cards.cardlook.accent", "Akcent")}</span>
        <button class="tl-sw tl-sw-auto" classList={{ on: !accent() }} onClick={() => patch({ accent: undefined })}>{t("cards.cardlook.byStyle", "Podle stylu")}</button>
        <For each={CARD_ACCENTS}>
          {(a) => (
            <button class="tl-sw" classList={{ on: accent() === a }} style={{ "--tl-c": accentCss(a) }} title={tx(ACCENT_NAMES[a])} aria-label={tx(ACCENT_NAMES[a])} onClick={() => patch({ accent: a })} />
          )}
        </For>
        <label class="tl-custom" title={t("cards.tile.colorOwn", "Vlastní barva")}>
          <input type="color" value={accent()?.startsWith("#") ? accent() : "#a78bfa"} aria-label={t("cards.tile.colorOwn", "Vlastní barva")} onInput={(e) => patch({ accent: e.currentTarget.value })} />
          {t("cards.tile.colorOwnShort", "Vlastní")}
        </label>
      </div>
      <MenuChoice label={t("cards.cardlook.bg", "Pozadí")} items={BGS.map((b) => ({ v: b, text: tx(BG_NAMES[b]) }))} value={look().bg ?? "style"} onPick={(v) => patch({ bg: v })} />
      <MenuChoice label={t("cards.cardlook.density", "Hustota")} items={DENSITIES.map((d) => ({ v: d, text: tx(DENSITY_NAMES[d]) }))} value={look().density ?? "style"} onPick={(v) => patch({ density: v })} />
      <Show when={view}>
        {(def) => (
          <MenuChoice
            label={t("cards.cardlook.view", "Zobrazení")}
            items={VIEWS.map((v) => ({ v, text: tx(VIEW_NAMES[v]) }))}
            value={look().view ?? def()}
            onPick={(v) => patch({ view: v === def() ? undefined : v })}
          />
        )}
      </Show>
      <Show when={p.tg.card === "media" && p.s}>
        {(s) => <MediaVariants s={s()} cfgError={p.tg.cfgError} />}
      </Show>
      <Show when={(p.tg.card === "projects" || p.tg.card === "actions") && p.s}>
        {(s) => <TileLooks s={s()} card={p.tg.card} />}
      </Show>
      <div class="tl-ed-row">
        <label class="tl-ed-check">
          <input type="checkbox" checked={!!look().hideHead} onChange={(e) => patch({ hideHead: e.currentTarget.checked })} />
          {t("cards.cardlook.hideHead", "Skrýt hlavičku (⋯ zůstane při najetí)")}
        </label>
      </div>
      <div class="tl-ed-foot">
        <button class="mini" onClick={() => setDraft(undefined)}>{t("cards.cardlook.reset", "Obnovit podle stylu")}</button>
        <button class="mini" onClick={() => setDraft(p.tg.initial)}>{t("cards.tile.undo", "Vrátit")}</button>
        <span class="grow" />
        <button class="mini ok" onClick={() => void closeCardLook(true)}>{t("cards.tile.done", "Hotovo")}</button>
      </div>
    </div>
  );
}

/** Varianta karty Právě hraje (`media.variant`) — tytéž miniatury jako ikonka v rohu karty a Nastavení. */
function MediaVariants(p: { s: Snapshot; cfgError: string | null }) {
  const cur = () => shownVariant(p.s.personal.mediaLook);
  return (
    <div class="tl-ed-col" role="group" aria-label={t("cards.cardlook.variant", "Vzhled přehrávače")}>
      <span class="tl-ed-cap">{t("cards.cardlook.variant", "Vzhled přehrávače")}</span>
      <div class="med-looks" role="radiogroup" aria-label={t("cards.cardlook.variant", "Vzhled přehrávače")}>
        <For each={VARIANTS}>
          {(v) => (
            <button class="med-look" classList={{ "med-look-on": cur() === v }} role="radio" aria-checked={cur() === v} onClick={() => void saveVariant(v, p.cfgError)}>
              <Thumb v={v} lg />
              <b>{variantName(v)}</b>
              <small>{variantHint(v)}</small>
            </button>
          )}
        </For>
      </div>
    </div>
  );
}

/** Zástupci karty (projekty / akce): vybraná dlaždice → její panel „Upravit vzhled…“ (`tilelook/*`). */
function TileLooks(p: { s: Snapshot; card: string }) {
  const items = () =>
    p.card === "projects"
      ? p.s.projects.map((x) => ({ id: x.id, name: x.name, kind: "project" as const, look: x.look }))
      : (p.s.actions ?? []).filter((a) => !a.script).map((a) => ({ id: a.id, name: a.label, kind: "action" as const, look: a.look }));
  const open = async (id: string) => {
    const it = items().find((x) => x.id === id);
    if (!it) return;
    await closeCardLook(true);
    openLook({ kind: it.kind, id: it.id, name: it.name, initial: it.look }, p.s.cfgError);
  };
  return (
    <Show when={items().length}>
      <div class="tl-ed-row">
        <span class="tl-ed-cap">{t("cards.cardlook.tiles", "Zástupci")}</span>
        <select aria-label={t("cards.cardlook.tilesAria", "Upravit vzhled zástupce")} onChange={(e) => { const v = e.currentTarget.value; e.currentTarget.value = ""; void open(v); }}>
          <option value="">{t("cards.cardlook.tilesPick", "Upravit vzhled zástupce…")}</option>
          <For each={items()}>{(x) => <option value={x.id}>{x.name}</option>}</For>
        </select>
        <small class="tl-ed-note">{t("cards.cardlook.tilesHint", "Zkratka: pravý klik na zástupce → Upravit vzhled.")}</small>
      </div>
    </Show>
  );
}
