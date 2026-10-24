// Galerie karet 2026 v Nastavení → Vzhled → Otevřený (kolo 22): „Karty 2026“ (předvolby plochy / zrnění /
// hrdiny pro všechny karty jedné záložky, `look.ts` `LOOK_PRESETS`) a „Bento šablony“ (Pracovní, Minimal, Hudba
// jako NOVÁ záložka, `bento.ts`). Klik = rozpracovaný config (`layout`) + „Vrátit“; uloží společná lišta. Najetí
// na dlaždici = náhled JEN dole v `settings/PreviewDock.tsx` (`CardsMini`), šuplík se nemění. Bez časovačů.
import { createMemo, createSignal, For, Show } from "solid-js";
import { t } from "../i18n";
import type { Appearance } from "../types";
import { offerUndo, useSettings } from "../settings/ctx";
import { scopeStyle } from "../theme/apply";
import { say } from "../ui/toast";
import { applyLookPreset, LOOK_PRESETS, presetOf, normalizeCardLook, type CardLook } from "./look";
import { addTemplatePage, TEMPLATES, templateCards, type Template, type TemplateId } from "./bento";
import { normalizeLayout, pageOf, type CardId, type LayoutCard } from "./model";
import { cardName, pageName, tx } from "./text";
import "./gallery.css";

type Peek = { preset: string } | { template: TemplateId };
const [peekCards, setPeekCards] = createSignal<Peek | null>(null);
/** záložka, na kterou galerie předvolby použije (sdílí ji náhled dole) */
const [galleryPage, setGalleryPage] = createSignal("");
/** Ukazuje náhled dole teď dlaždici karet pod myší? */
export const peekingCards = () => !!peekCards();

const peekOn = (p: Peek) => ({ onPointerEnter: () => setPeekCards(p), onPointerLeave: () => setPeekCards(null), onFocus: () => setPeekCards(p), onBlur: () => setPeekCards(null) });

/** Malá karta v miniatuře: plocha / zrnění / hrdina z `look` (ukázka tvaru, žádná data). */
function MiniCard(q: { look?: CardLook; wide?: boolean; label?: string; style?: Record<string, string> }) {
  return (
    <span class="cg-mcard" classList={{ "cg-wide": !!q.wide }} data-s={q.look?.surface ?? "style"} data-grain={q.look?.grain ? "" : undefined} style={q.style}>
      <Show when={q.label} fallback={<i class="cg-mh" />}>
        <b class="cg-mlabel">{q.label}</b>
      </Show>
      <Show when={q.look?.hero} fallback={<><i class="cg-ml" /><i class="cg-ml cg-short" /></>}>
        <i class="cg-mnum" />
        <i class="cg-ml cg-short" />
      </Show>
    </span>
  );
}

/** Miniatura předvolby: šest karet se vzhledem předvolby (hrdina jen u karet, které ho umí). */
function PresetMini(q: { a?: Appearance; preset: string; tall?: boolean }) {
  const looks = () => {
    const pr = LOOK_PRESETS.find((x) => x.id === q.preset);
    const ids: CardId[] = ["claude", "projects", "weather", "notes", "calendar", "timer"];
    return ids.map((id) => normalizeCardLook(pr?.patch, id));
  };
  return (
    <span class="theme-scope cg-mini" classList={{ "cg-tall": !!q.tall }} style={q.a ? scopeStyle(q.a) : undefined} aria-hidden="true">
      <span class="cg-grid">
        <For each={looks()}>{(l, i) => <MiniCard look={l} wide={i() === 0} />}</For>
      </span>
    </span>
  );
}

/** Miniatura bento záložky: dlaždice v pevných řádcích s názvy karet (skutečné názvy, žádná data). */
function BentoMini(q: { a?: Appearance; cards: LayoutCard[]; tall?: boolean }) {
  return (
    <span class="theme-scope cg-mini" classList={{ "cg-tall": !!q.tall }} style={q.a ? scopeStyle(q.a) : undefined} aria-hidden="true">
      <span class="cg-grid cg-bento">
        <For each={q.cards}>
          {(c) => <MiniCard look={c.look} label={cardName(c.id)} style={{ "grid-column": `span ${c.size === "s" ? 1 : 2}`, "grid-row": `span ${c.h ?? 2}` }} />}
        </For>
      </span>
    </span>
  );
}

const tplCards = (tp: Template) => templateCards(tp);

/** Náhled dole (`PreviewDock.tsx`): dlaždice pod myší, jinak vybraná záložka galerie (předvolba / bento). */
export function CardsMini(q: { a: Appearance }) {
  const c = useSettings();
  const L = createMemo(() => normalizeLayout(c.cfg.layout));
  const pg = () => pageOf(L(), galleryPage() || L().pages[0]?.id || "prace");
  const view = () => {
    const p = peekCards();
    if (p && "template" in p) {
      const tp = TEMPLATES.find((x) => x.id === p.template);
      return tp ? { bento: tplCards(tp) } : null;
    }
    if (p) return { preset: p.preset };
    if (pg().bento) return { bento: pg().cards };
    return { preset: presetOf(L(), pg().id) ?? "style" };
  };
  return (
    <Show when={view()} keyed>
      {(v) => ("bento" in v ? <BentoMini a={q.a} cards={v.bento!} tall /> : <PresetMini a={q.a} preset={v.preset!} tall />)}
    </Show>
  );
}

export default function CardsGallery() {
  const c = useSettings();
  const L = createMemo(() => normalizeLayout(c.cfg.layout));
  const pg = createMemo(() => pageOf(L(), galleryPage() || L().pages[0]?.id || "prace"));
  const cur = () => presetOf(L(), pg().id);
  const put = (l: ReturnType<typeof L>) => c.set("layout", l);

  const pickPreset = (id: string) => {
    setPeekCards(null);
    const prev = L();
    const pr = LOOK_PRESETS.find((x) => x.id === id);
    if (!pr) return;
    put(applyLookPreset(prev, pg().id, id));
    offerUndo(t("cards.gallery.presetDone", "Karty na záložce {tab}: {name}", { tab: pageName(pg()), name: tx(pr.name) }), () => put(prev));
  };
  const pickTemplate = (tp: Template) => {
    setPeekCards(null);
    const prev = L();
    const r = addTemplatePage(prev, tp, tx(tp.name));
    if (!r) {
      say(t("cards.gallery.full", "Víc záložek nejde — nejdřív nějakou smaž."));
      return;
    }
    put(r.layout);
    setGalleryPage(r.id);
    offerUndo(t("cards.gallery.tplDone", "Přidána záložka „{name}“ (bento) — v šuplíku ji uvidíš po Uložit", { name: tx(tp.name) }), () => put(prev));
  };

  return (
    <div class="card ap-top" id="f-appearance.cards2026">
      <h4>{t("cards.gallery.title", "Karty 2026")}</h4>
      <div class="cg-pages" role="radiogroup" aria-label={t("cards.gallery.page", "Pro záložku")}>
        <span class="cg-pages-l">{t("cards.gallery.page", "Pro záložku")}</span>
        <For each={L().pages}>
          {(p) => (
            <button class="mini" classList={{ "cg-on": p.id === pg().id }} role="radio" aria-checked={p.id === pg().id} onClick={() => setGalleryPage(p.id)}>
              {p.icon ? `${p.icon} ` : ""}{pageName(p)}
            </button>
          )}
        </For>
      </div>
      <div class="ap-styles" role="radiogroup" aria-label={t("cards.gallery.title", "Karty 2026")}>
        <For each={LOOK_PRESETS}>
          {(pr) => (
            <button class="ap-style" classList={{ on: cur() === pr.id }} role="radio" aria-checked={cur() === pr.id} title={tx(pr.desc)} {...peekOn({ preset: pr.id })} onClick={() => pickPreset(pr.id)}>
              <PresetMini preset={pr.id} />
              <span class="ap-name">{tx(pr.name)}</span>
              <small class="ap-desc">{tx(pr.desc)}</small>
            </button>
          )}
        </For>
      </div>
      <h4 class="cg-sub">{t("cards.gallery.bento", "Bento šablony")}</h4>
      <small class="cg-hint">{t("cards.gallery.bentoHint", "Šablona přidá novou záložku s dlaždicemi (největší = nejdůležitější); tvoje záložky zůstanou. Bento zapneš i na existující záložce v Šuplík a karty.")}</small>
      <div class="ap-styles" role="list">
        <For each={TEMPLATES}>
          {(tp) => (
            <button class="ap-style" role="listitem" title={tx(tp.desc)} {...peekOn({ template: tp.id })} onClick={() => pickTemplate(tp)}>
              <BentoMini cards={tplCards(tp)} />
              <span class="ap-name">{tp.icon} {tx(tp.name)}</span>
              <small class="ap-desc">{tx(tp.desc)}</small>
            </button>
          )}
        </For>
      </div>
    </div>
  );
}
