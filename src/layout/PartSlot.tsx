// Části (sekce) uvnitř karty: obal `PartsBox` a jedna část `PartSlot`. Karta s částmi (Claude: limity, obsah,
// útrata; Právě hraje: skladba, hlasitost; Čas: obsah, baterie) je skládá do `PartsBox` a každou zabalí do
// `PartSlot` — pořadí (`layout…cards[].parts`), skrytí (`hiddenParts`), lišta pro přetažení v režimu úprav i
// sbalení klikem na nadpis dělá tenhle soubor; řadič tahu je `partDrag.ts`, spojení s rozložením dělá `Deck`
// přes `PartsCtx`. Bez kontextu (karta vykreslená mimo mřížku, třeba náhled vzhledu) se části kreslí v pořadí,
// v jakém je karta napsala — beze změny.
// Ve výchozím pořadí a mimo úpravy obaly nemají box (`display: contents`) — vzhled karty se nemění. Pořadí
// jinak řeší CSS `order` (DOM se nepřeskládá, takže během tahu nepřijde ukazatel ani fokus o prvek).
import { createContext, createSignal, Show, useContext, type JSX } from "solid-js";
import { t } from "../i18n";
import { foldKey, toggleFold } from "./parts";
import { slotName } from "./text";
import type { CardId } from "./model";
import "./dnd.css";

/** Co obaly potřebují od mřížky (`Deck` ho dává přes `PartsCtx.Provider`). */
export type PartsCtl = {
  editing: () => boolean;
  /** pořadí částí karty (při tahu náhled) */
  order: (card: CardId) => string[];
  /** karta potřebuje box (úpravy nebo jiné než výchozí pořadí) */
  flex: (card: CardId) => boolean;
  /** část je právě „v ruce“ (tažená) */
  lifted: (card: CardId, k: string) => boolean;
  down: (e: PointerEvent, card: CardId, k: string) => void;
  key: (e: KeyboardEvent, card: CardId, k: string) => void;
  /** ukázat skrytou část (z lišty skryté části v úpravách) */
  show: (k: string) => void;
};
export const PartsCtx = createContext<PartsCtl>();

// sbalení částí klikem na nadpis: v paměti (přežije přepínání záložek a zavření šuplíku, ne restart)
const [folded, setFolded] = createSignal<Record<string, boolean>>({});

export function PartsBox(p: { card: CardId; children: JSX.Element }) {
  const ctl = useContext(PartsCtx);
  return (
    <div class="lp-box" data-pcard={p.card} data-flex={ctl?.flex(p.card) ? "" : undefined}>
      {p.children}
    </div>
  );
}

/** Jedna část karty `card` s klíčem `k` (`model.ts` `CARD_SLOTS`). `hidden` = část je skrytá (nic se nekreslí a
 *  nic se nespouští; v úpravách ji připomene lišta s „Ukázat“). `titled` = s nadpisem a sbalením klikem;
 *  `empty` = část teď nemá co ukázat (nadpis bez obsahu by byl divadlo, tak se mimo úpravy nekreslí). */
export function PartSlot(p: { card: CardId; k: string; hidden?: boolean; titled?: boolean; empty?: boolean; children: JSX.Element }) {
  const ctl = useContext(PartsCtx);
  const editing = () => !!ctl?.editing();
  const fk = () => foldKey(p.card, p.k);
  const open = () => editing() || !folded()[fk()];
  const style = () => (ctl ? { order: String(ctl.order(p.card).indexOf(p.k)) } : undefined);
  const name = () => slotName(p.k);
  return (
    <div class="lp-slot" data-part={p.k} style={style()} classList={{ "lp-gap": !!ctl?.lifted(p.card, p.k) }}>
      <Show when={editing() && ctl}>
        {(c) => (
          <div class="lp-edit" classList={{ "lp-off": !!p.hidden }} onPointerDown={(e) => !p.hidden && c().down(e, p.card, p.k)}>
            <button
              class="lp-grip"
              disabled={!!p.hidden}
              title={t("cards.layout.partGrip", "Přetáhni část v kartě (Esc zruší) nebo dolů na „Skrýt“. Klávesy: šipky přesun, Delete skrýt")}
              aria-label={t("cards.layout.partGrip.aria", "Přesunout část {name}: šipky, Delete skryje", { name: name() })}
              onKeyDown={(e) => c().key(e, p.card, p.k)}
            >⠿</button>
            <span class="lp-name">{name()}</span>
            <Show when={p.hidden}>
              <small>{t("cards.layout.partOff", "skrytá")}</small>
              <button class="lp-show" onClick={() => c().show(p.k)}>{t("cards.layout.partShow", "Ukázat")}</button>
            </Show>
          </div>
        )}
      </Show>
      <Show when={!p.hidden}>
        <Show when={p.titled && !editing() && !p.empty}>
          <button class="lp-head" aria-expanded={open()} onClick={() => setFolded((f) => toggleFold(f, fk()))}>
            <span class="lp-chev" classList={{ on: open() }}>›</span>
            {name()}
          </button>
        </Show>
        <Show when={p.titled} fallback={p.children}>
          <div class="lp-fold" data-open={open() ? "true" : "false"}>
            <div class="lp-fold-in">{p.children}</div>
          </div>
        </Show>
      </Show>
    </div>
  );
}
