// Editor vlastního zavřeného Ledge („Poskládat si vlastní“, `strip.compose`, kolo 23) v kroku Zavřený panelu
// Vzhled — styl náhledu „Kostky“: karusel předvoleb (15 hotových vzhledů → `composeOf`), tvar, rozměry
// (posuvníky + tabulka přesných čísel), přihrádky vlevo / uprostřed / vpravo s kostkami (přetažení z kostkárny
// nebo na náhled, klik = další styl, šipky = posun, × nebo vytažení ven = odebrat) a „Vrátit“ (zásobník
// změn v editoru). Vzhled = dnešní tokeny Nastavení; žádná trvalá animace (žádné kývání kostek). Úpravy jdou
// do rozpracovaného configu (živý náhled = `StripLookPreview` / PreviewDock), proužek se změní po Uložit.
import { For, Show, createSignal } from "solid-js";
import { lang, t } from "../../i18n";
import type { Appearance } from "../../types";
import { STRIP_LOOKS } from "../../strip/looks";
import {
  COMPOSE_ITEMS, COMPOSE_SHAPES, ITEM_STYLES, MAX_PIECES, SLOTS, W_MAX, W_MIN,
  type ComposeItem, type ComposeShape, type ItemStyle, type Slot, type StripCompose,
  composeOf, hRange, hasItem, normCompose, putPiece, removePiece, setAllStyles, setStyle, shiftPiece, total, withShape,
} from "../../strip/compose";
import StripLookPreview from "../previews/StripLookPreview";
import { ITEM_GLYPH } from "../previews/ComposePreview";
import type { PreviewState } from "../appearanceModel";
import { Slider } from "./apUi";
import "./compose.css";

const DRAG = "text/x-ledge-piece";

export const SHAPE_NAME: Record<ComposeShape, () => string> = {
  capsule: () => t("appearance.compose.shape.capsule", "Kapsle"),
  notch: () => t("appearance.compose.shape.notch", "Výřez"),
  line: () => t("appearance.compose.shape.line", "Tenká linka"),
  corner: () => t("appearance.compose.shape.corner", "Roh"),
  drop: () => t("appearance.compose.shape.drop", "Kapka"),
  island: () => t("appearance.compose.shape.island", "Ostrůvek"),
};
export const ITEM_NAME: Record<ComposeItem, () => string> = {
  time: () => t("appearance.compose.item.time", "Čas"),
  date: () => t("appearance.compose.item.date", "Datum"),
  claude: () => t("appearance.compose.item.claude", "Stav Clauda"),
  limit5h: () => t("appearance.compose.item.limit5h", "Limit 5 h"),
  limitWeek: () => t("appearance.compose.item.limitWeek", "Limit týden"),
  music: () => t("appearance.compose.item.music", "Hudba"),
  timer: () => t("appearance.compose.item.timer", "Časovač"),
  cpu: () => t("appearance.compose.item.cpu", "Výkon (CPU)"),
  memory: () => t("appearance.compose.item.memory", "Paměť"),
  battery: () => t("appearance.compose.item.battery", "Baterie"),
  weather: () => t("appearance.compose.item.weather", "Počasí"),
  servers: () => t("appearance.compose.item.servers", "Servery"),
  repos: () => t("appearance.compose.item.repos", "Repa"),
  activity: () => t("appearance.compose.item.activity", "Claude aktivita"),
};
/** Vysvětlivka u kostky (jen položky, kde je co říct). */
export const ITEM_HINT: Partial<Record<ComposeItem, () => string>> = {
  activity: () => t("appearance.compose.hint.activity", "Fáze (čtení, úpravy, testy, hotovo) je odhad podle posledního nástroje; čekající schválení má přednost."),
};
export const STYLE_NAME: Record<ItemStyle, () => string> = {
  icon: () => t("appearance.compose.style.icon", "Jen ikona"),
  badge: () => t("appearance.compose.style.badge", "Ikona s číslem"),
  text: () => t("appearance.compose.style.text", "Text"),
  ring: () => t("appearance.compose.style.ring", "Prstenec"),
  graph: () => t("appearance.compose.style.graph", "Malý graf"),
  dot: () => t("appearance.compose.style.dot", "Tečka"),
};
const SLOT_NAME: Record<Slot, () => string> = {
  left: () => t("appearance.compose.slot.left", "Vlevo"),
  center: () => t("appearance.compose.slot.center", "Uprostřed"),
  right: () => t("appearance.compose.slot.right", "Vpravo"),
};
const px = (v: number) => `${v} px`;
const pc = (v: number) => (lang() === "cs" ? `${v} %` : `${v}%`);

export default function ComposeEditor(q: { value: unknown; set: (c: StripCompose) => void; a: Appearance; opacity: number; state: PreviewState }) {
  const c = () => normCompose(q.value);
  // „Vrátit“ v editoru: zásobník předchozích stavů; tažení jednoho posuvníku = jeden krok
  const [stack, setStack] = createSignal<StripCompose[]>([]);
  let lastKey = "";
  const commit = (next: StripCompose, key = "") => {
    const prev = c();
    if (JSON.stringify(next) === JSON.stringify(prev)) return;
    if (!key || key !== lastKey) setStack((s) => [...s.slice(-29), prev]);
    lastKey = key;
    q.set(next);
  };
  const undo = () => {
    const s = stack();
    if (!s.length) return;
    lastKey = "";
    setStack(s.slice(0, -1));
    q.set(s[s.length - 1]);
  };
  const line = () => c().shape === "line";
  const full = () => total(c()) >= MAX_PIECES;
  const [hRow0, hRow1] = [() => hRange(c().shape)[0], () => hRange(c().shape)[1]];
  let car: HTMLDivElement | undefined;
  const scroll = (dir: number) => car?.scrollBy({ left: dir * car.clientWidth * 0.8, behavior: "smooth" });

  // přetahování: data = id položky; puštění na přihrádku = přidat / přesunout, mimo přihrádky = odebrat
  const [over, setOver] = createSignal<Slot | null>(null);
  const dragStart = (e: DragEvent, item: ComposeItem) => {
    e.dataTransfer?.setData(DRAG, item);
    if (e.dataTransfer) e.dataTransfer.effectAllowed = "move";
  };
  const dropOn = (e: DragEvent, slot: Slot, at?: number) => {
    e.preventDefault();
    e.stopPropagation();
    setOver(null);
    const item = e.dataTransfer?.getData(DRAG) as ComposeItem;
    if (COMPOSE_ITEMS.includes(item)) commit(putPiece(c(), item, slot, at));
  };
  const allow = (e: DragEvent, slot: Slot | null) => {
    if (!e.dataTransfer?.types.includes(DRAG)) return;
    e.preventDefault();
    if (slot !== over()) setOver(slot);
  };
  /** puštění na náhled: slot podle místa (třetiny šířky) */
  const dropOnPreview = (e: DragEvent) => {
    const r = (e.currentTarget as HTMLElement).getBoundingClientRect();
    const x = (e.clientX - r.left) / Math.max(1, r.width);
    dropOn(e, x < 1 / 3 ? "left" : x > 2 / 3 ? "right" : "center");
  };
  // kostka z přihrádky vytažená mimo přihrádky (nikam nepuštěná) = pryč
  const dragEndRack = (e: DragEvent, item: ComposeItem) => {
    setOver(null);
    if (e.dataTransfer?.dropEffect === "none" && total(c()) > 1) commit(removePiece(c(), item));
  };

  return (
    <div class="card ap-top" id="f-strip.compose">
      <h4>{t("appearance.compose.title", "Poskládat si vlastní")}</h4>
      <p class="ap-note">{t("appearance.compose.lead", "Začni z hotového vzhledu a dolaď tvar, rozměry a kostky. Ukázková data; proužek se změní po Uložit.")}</p>

      <div class="cx-hero" onDragOver={(e) => allow(e, null)} onDrop={dropOnPreview} aria-label={t("appearance.compose.preview", "Náhled vlastního vzhledu")}>
        <StripLookPreview look="custom" compose={c()} state={q.state} a={q.a} opacity={q.opacity} />
      </div>
      <div class="cx-bar">
        <span class="cx-sum">{SHAPE_NAME[c().shape]()} · {line() ? t("appearance.compose.fullEdge", "celá hrana") : `${c().width} × ${c().height} px`} · {t("appearance.compose.count", "{n} z {max} položek", { n: total(c()), max: MAX_PIECES })}</span>
        <button class="btn" type="button" disabled={!stack().length} onClick={undo}>{t("appearance.compose.undo", "Vrátit")}</button>
      </div>

      <section class="cx-blk">
        <div class="cx-bh">
          <span>{t("appearance.compose.presets", "Začít z hotového vzhledu")}<small>{t("appearance.compose.presetsHint", "Klik naplní editor tím vzhledem; pak ho dolaď.")}</small></span>
          <span class="cx-arrows">
            <button class="btn" type="button" aria-label={t("appearance.compose.left", "Posunout doleva")} onClick={() => scroll(-1)}>‹</button>
            <button class="btn" type="button" aria-label={t("appearance.compose.right", "Posunout doprava")} onClick={() => scroll(1)}>›</button>
          </span>
        </div>
        <div class="cx-car" ref={car} role="list" aria-label={t("appearance.compose.presets", "Začít z hotového vzhledu")}>
          <For each={STRIP_LOOKS.filter((x) => x.id !== "custom")}>
            {(x) => (
              <button class="ap-style cx-pre" type="button" role="listitem" onClick={() => commit(composeOf(x.id))}>
                <StripLookPreview look={x.id} state={q.state} a={q.a} opacity={q.opacity} />
                <span class="ap-name">{lang() === "en" ? x.en : x.cs}</span>
              </button>
            )}
          </For>
        </div>
      </section>

      <section class="cx-blk">
        <div class="opt">
          <span>{t("appearance.compose.shape", "Tvar")}</span>
          <span class="cx-shapes" role="radiogroup" aria-label={t("appearance.compose.shape", "Tvar")}>
            <For each={COMPOSE_SHAPES}>
              {(v) => <button class="btn cx-shape" classList={{ on: c().shape === v }} type="button" role="radio" aria-checked={c().shape === v} onClick={() => commit(withShape(c(), v))}>{SHAPE_NAME[v]()}</button>}
            </For>
          </span>
        </div>
        <Show when={!line()}>
          <Slider label={t("appearance.compose.width", "Šířka")} min={W_MIN} max={W_MAX} step={2} value={c().width} fmt={px} set={(v) => commit({ ...c(), width: v }, "width")} />
        </Show>
        <Slider label={line() ? t("appearance.compose.thickness", "Tloušťka") : t("appearance.compose.height", "Výška")} min={hRow0()} max={hRow1()} step={1} value={c().height} fmt={px} set={(v) => commit({ ...c(), height: v }, "height")} />
        <Show when={!line()}>
          <Slider label={t("appearance.compose.round", "Zaoblení")} hint={t("appearance.compose.roundHint", "0 % ostré rohy, 100 % úplně kulaté.")} min={0} max={100} step={1} value={c().round} fmt={pc} set={(v) => commit({ ...c(), round: v }, "round")} />
        </Show>
        <table class="cx-num">
          <caption>{t("appearance.compose.exact", "Přesná čísla")}</caption>
          <thead><tr><th scope="col">{t("appearance.compose.what", "Rozměr")}</th><th scope="col">{t("appearance.compose.value", "Hodnota")}</th><th scope="col">{t("appearance.compose.range", "Rozsah")}</th></tr></thead>
          <tbody>
            <For each={[
              ["width", t("appearance.compose.width", "Šířka"), W_MIN, W_MAX, "px"],
              ["height", line() ? t("appearance.compose.thickness", "Tloušťka") : t("appearance.compose.height", "Výška"), hRow0(), hRow1(), "px"],
              ["round", t("appearance.compose.round", "Zaoblení"), 0, 100, "%"],
            ] as [keyof StripCompose & ("width" | "height" | "round"), string, number, number, string][]}>
              {([k, label, min, max, unit]) => (
                <tr classList={{ off: line() && k !== "height" }}>
                  <th scope="row">{label}</th>
                  <td><input type="number" min={min} max={max} step={1} value={c()[k]} disabled={line() && k !== "height"} aria-label={label}
                    onChange={(e) => commit(normCompose({ ...c(), [k]: +e.currentTarget.value }), k)} /> {unit}</td>
                  <td>{min}–{max} {unit}</td>
                </tr>
              )}
            </For>
          </tbody>
        </table>
      </section>

      <section class="cx-blk">
        <div class="cx-bh">
          <span>{t("appearance.compose.cubes", "Kostky: obsah a styl")}<small>{t("appearance.compose.cubesHint", "Přetáhni kostku do přihrádky nebo na náhled. 1 až 5 kostek. Klik přepne styl, tažení ven ji odebere.")}</small></span>
        </div>
        <div class="opt">
          <span>{t("appearance.compose.allStyles", "Styl všech kostek")}</span>
          <span class="cx-styles" role="group" aria-label={t("appearance.compose.allStyles", "Styl všech kostek")}>
            <For each={ITEM_STYLES}>{(s) => <button class="btn" type="button" onClick={() => commit(setAllStyles(c(), s))}>{STYLE_NAME[s]()}</button>}</For>
          </span>
        </div>
        <div class="cx-rack">
          <For each={SLOTS}>
            {(slot) => (
              <div class="cx-well" classList={{ over: over() === slot }} data-slot={slot} onDragOver={(e) => allow(e, slot)} onDragLeave={() => over() === slot && setOver(null)} onDrop={(e) => dropOn(e, slot)}>
                <span class="cx-wn">{SLOT_NAME[slot]()}</span>
                <Show when={c()[slot].length} fallback={<span class="cx-emp">{t("appearance.compose.dropHere", "Přetáhni sem kostku")}</span>}>
                  <For each={c()[slot]}>
                    {(p, i) => (
                      <div class="cx-cw" onDragOver={(e) => allow(e, slot)} onDrop={(e) => dropOn(e, slot, i())}>
                        <button class="cx-cube" type="button" draggable="true" onDragStart={(e) => dragStart(e, p.item)} onDragEnd={(e) => dragEndRack(e, p.item)}
                          aria-label={t("appearance.compose.cubeLabel", "{name}, styl {style}. Klik změní styl.", { name: ITEM_NAME[p.item](), style: STYLE_NAME[p.style]() })}
                          title={t("appearance.compose.cubeHint", "Klik změní styl, tažením přesuneš")} onClick={() => commit(setStyle(c(), p.item, "next"))}>
                          <span class="cx-ci" aria-hidden="true">{ITEM_GLYPH[p.item]}</span>
                          <span class="cx-cn">{ITEM_NAME[p.item]()}</span>
                          <span class="cx-cs">{STYLE_NAME[p.style]()}</span>
                        </button>
                        <span class="cx-cm">
                          <button type="button" class="cx-mini" aria-label={t("appearance.compose.moveLeft", "Posunout doleva")} onClick={() => commit(shiftPiece(c(), p.item, -1))}>‹</button>
                          <button type="button" class="cx-mini" aria-label={t("appearance.compose.moveRight", "Posunout doprava")} onClick={() => commit(shiftPiece(c(), p.item, 1))}>›</button>
                          <button type="button" class="cx-mini" disabled={total(c()) <= 1} aria-label={t("appearance.compose.remove", "Odebrat {name}", { name: ITEM_NAME[p.item]() })} onClick={() => commit(removePiece(c(), p.item))}>×</button>
                        </span>
                      </div>
                    )}
                  </For>
                </Show>
              </div>
            )}
          </For>
        </div>
        <div class="cx-trayh">{t("appearance.compose.tray", "Kostkárna")}<small>{full() ? t("appearance.compose.full", "Plno, nejdřív něco odeber.") : t("appearance.compose.trayHint", "Táhni, nebo klikni (přidá doprostřed).")}</small></div>
        <div class="cx-tray" onDragOver={(e) => allow(e, null)} onDrop={(e) => {
          // kostka z přihrádky puštěná zpět do kostkárny = odebrat
          e.preventDefault();
          setOver(null);
          const item = e.dataTransfer?.getData(DRAG) as ComposeItem;
          if (hasItem(c(), item)) commit(removePiece(c(), item));
        }}>
          <For each={COMPOSE_ITEMS.filter((i) => !hasItem(c(), i))}>
            {(item) => (
              <button class="cx-cube" type="button" draggable={!full()} disabled={full()} onDragStart={(e) => dragStart(e, item)} onClick={() => commit(putPiece(c(), item, "center"))}
                aria-label={t("appearance.compose.add", "Přidat {name}", { name: ITEM_NAME[item]() })} title={ITEM_HINT[item]?.()}>
                <span class="cx-ci" aria-hidden="true">{ITEM_GLYPH[item]}</span>
                <span class="cx-cn">{ITEM_NAME[item]()}</span>
              </button>
            )}
          </For>
        </div>
        <label class="opt"><span>{t("appearance.compose.hideIdle", "V klidu skrytý")}<small>{t("appearance.compose.hideIdleHint", "Ukáže se, až na tebe něco čeká nebo je hotovo.")}</small></span>
          <input type="checkbox" checked={c().hideIdle} onChange={(e) => commit({ ...c(), hideIdle: e.currentTarget.checked })} /></label>
        <label class="opt"><span>{t("appearance.compose.rotate", "Střídat střed")}<small>{t("appearance.compose.rotateHint", "Víc událostí uprostřed se střídá po 5 s; v klidu se nic nehýbe.")}</small></span>
          <input type="checkbox" checked={c().rotate} onChange={(e) => commit({ ...c(), rotate: e.currentTarget.checked })} /></label>
      </section>
    </div>
  );
}
