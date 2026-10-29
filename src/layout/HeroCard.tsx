// Karta „číslo jako hrdina“ (kolo 22, `look.hero`): místo obsahu karty jedno velké skutečné číslo (`hero.ts`
// `heroFacts`), popisek, druhý řádek a sparkline (`charts/Stat.tsx`). Hlavička s názvem a ⋯ zůstává (zpět přes
// „Vzhled karty…“). Bez čísla z reálného zdroje se tahle karta vůbec nevykreslí — `Deck` ukáže kartu jako dřív.
import { Show, type JSX } from "solid-js";
import { num } from "../format";
import { t } from "../i18n";
import Stat from "../charts/Stat";
import type { HeroFacts } from "./hero";
import { cardName, tx } from "./text";
import type { CardId } from "./model";
import "./hero.css";

export default function HeroCard(p: { id: CardId; facts: HeroFacts; menu?: JSX.Element; onFix?: () => void }) {
  const sub = () => (p.facts.sub ? tx(p.facts.sub.text, p.facts.sub.vars) : undefined);
  return (
    <div class="card hero-card" data-anim>
      <h4>
        <span class="ic" style="background:var(--accent)" />
        {cardName(p.id)}
        {p.menu}
      </h4>
      <div class="hero-body">
        <Show when={p.facts.empty} fallback={
          <Stat value={num(p.facts.value, p.facts.digits)} unit={p.facts.unit} label={tx(p.facts.label)} sub={sub()} series={p.facts.series} min={p.facts.min} max={p.facts.max} />
        }>
          {(e) => (
            <div class="hero-empty" role="status">
              <span class="hero-empty-label">{tx(p.facts.label)}</span>
              <b>{tx(e().title)}</b>
              <small>{tx(e().why)}</small>
              {p.onFix && <button class="hint-link" onClick={p.onFix}>{t("cards.hero.fix", "Otevřít nastavení")}</button>}
            </div>
          )}
        </Show>
      </div>
    </div>
  );
}
