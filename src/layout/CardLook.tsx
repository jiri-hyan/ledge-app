// „Vzhled karty…“ (⋯ na kartě): akcent, pozadí, skrytá hlavička, hustota a u karet, kde to dává smysl,
// varianta zobrazení. Panel se kreslí v toku stránky pod mřížkou (okno šuplíku je jen tak vysoké jako
// obsah — plovoucí panel by se ořízl); karta nad ním je živý náhled (`lookFor` → `Deck` atributy
// `data-cl-*`). Hotovo / klik mimo = uložit (v režimu úprav do draftu), Esc = zahodit, Vrátit, Obnovit
// podle stylu. Obecné pro každou kartu (i nové karty jiných modulů) — řeší to `.deck-item`, ne karta.
import { createSignal, For, onCleanup, onMount, Show } from "solid-js";
import { t } from "../i18n";
import { MenuChoice } from "../cards/CardMenu";
import { change, editing, saveLayout } from "./edit";
import {
  ACCENT_NAMES, accentCss, BG_NAMES, BGS, CARD_ACCENTS, DENSITIES, DENSITY_NAMES, normalizeCardLook, patchCardLook, setCardLook, VIEW_CARDS, VIEW_NAMES, VIEWS,
  type CardLook,
} from "./look";
import { cardName, tx } from "./text";
import type { CardId } from "./model";
import "../tilelook/tilelook.css";

type Target = { page: string; card: string; initial: CardLook | undefined; cfgError: string | null };
const [target, setTarget] = createSignal<Target | null>(null);
const [draft, setDraft] = createSignal<CardLook | undefined>(undefined);

/** Vzhled, který má karta kreslit: rozpracovaný (živý náhled), jinak uložený. */
export const lookFor = (page: string, card: string, saved: CardLook | undefined): CardLook | undefined => {
  const tg = target();
  return tg && tg.page === page && tg.card === card ? draft() : saved;
};

/** ⋯ → „Vzhled karty…“. */
export function openCardLook(page: string, card: string, initial: CardLook | undefined, cfgError: string | null) {
  const cur = target();
  if (cur) void closeCardLook(true);
  const init = normalizeCardLook(initial, card);
  setTarget({ page, card, initial: init, cfgError });
  setDraft(init);
}

export async function closeCardLook(save: boolean) {
  const tg = target();
  const l = draft();
  setTarget(null);
  setDraft(undefined);
  if (!save || !tg || JSON.stringify(l ?? null) === JSON.stringify(tg.initial ?? null)) return;
  if (editing()) change((x) => setCardLook(x, tg.page, tg.card, l));
  else await saveLayout((x) => setCardLook(x, tg.page, tg.card, l), tg.cfgError);
}

/** Panel pod mřížkou (MainPage), jen pro kartu na zobrazené záložce. */
export default function CardLookPanel(p: { page: string }) {
  return (
    <Show when={target()?.page === p.page ? target() : null} keyed>
      {(tg) => <Panel tg={tg} />}
    </Show>
  );
}

function Panel(p: { tg: Target }) {
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
