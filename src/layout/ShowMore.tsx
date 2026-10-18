// „Zobrazit další (k)“ ↔ „Méně“ pod kartou se stropem řádků (`cap.ts`). Rozbalení se pamatuje zvlášť pro
// každou záložku a kartu (`capKey`) po dobu běhu Ledge — v paměti, nic se nepočítá ani neukládá.
// `useCap` = sdílený háček pro karty: strop z rozložení, stav rozbalení, rozdělení řádků.
import { createSignal, Show } from "solid-js";
import { t } from "../i18n";
import { capKey, capList } from "./cap";

const [opened, setOpened] = createSignal<Record<string, boolean>>({});
const isOpen = (key: string) => !!opened()[key];
const toggle = (key: string) => setOpened((o) => ({ ...o, [key]: !o[key] }));

/** Strop řádků karty: `items` → viditelné / schované + tlačítko. `pinned` se nikdy neschová. */
export function useCap<T>(p: () => { page?: string; card: string; limit?: number }, items: () => readonly T[], pinned?: (x: T) => boolean) {
  const key = () => capKey(p().page ?? "", p().card);
  const split = () => capList(items(), p().limit ?? 0, isOpen(key()), pinned);
  return {
    shown: () => split().shown,
    hidden: () => split().hidden,
    open: () => isOpen(key()),
    /** dá se sbalit / rozbalit (strop je a řádků je víc) */
    toggles: () => split().hidden.length > 0 || (isOpen(key()) && (p().limit ?? 0) > 0 && items().length > (p().limit ?? 0)),
    toggle: () => toggle(key()),
  };
}

/** Tlačítko pod řádky: „Zobrazit další (6) · +5 nečinných, +1 pracující“ / „Méně“. */
export default function ShowMore(p: { open: boolean; hidden: number; summary?: string; onToggle: () => void }) {
  return (
    <button class="cap-more" aria-expanded={p.open} onClick={() => p.onToggle()}>
      <Show when={!p.open} fallback={t("cards.cap.less", "Méně")}>
        {t("cards.cap.more", "Zobrazit další ({n})", { n: p.hidden })}
        <Show when={p.summary}>{(s) => <span class="cap-sum"> · {s()}</span>}</Show>
      </Show>
    </button>
  );
}
