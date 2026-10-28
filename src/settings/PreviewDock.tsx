// Náhled DOLE pod editorem Nastavení (kolo 21): živá ukázka toho, co vyjede — Zavřený (proužek u hrany),
// Po najetí a Otevřený (miniatura šuplíku) z rozpracovaného vzhledu. Najetí na dlaždici stylu / motivu,
// vzhledu zavřeného Ledge nebo „Po najetí“ (`apUi.tsx` `peek` / `peekLook` / `peekHover`) se ukáže JEN tady;
// skutečný Ledge se mění až kliknutím (rozpracovaný vzhled) a Uložit. Data jsou ukázková; „Co se zrovna
// děje“ (`StateSwitch`) je JEN tady a řídí i miniatury galerií v krocích. Sbalení si pamatuje úložiště
// prohlížeče (try/catch).
// Žádný časovač ani animace v klidu — jen překreslení, když se změní vzhled.
import { For, Show, createSignal } from "solid-js";
import { t } from "../i18n";
import type { Appearance } from "../types";
import { normalizeAppearance, stripShape } from "../theme/appearance";
import { scopeStyle } from "../theme/apply";
import { LAYOUTS } from "../theme/looks";
import { cfg } from "./ctx";
import { StateSwitch, peek, peekHover, peekLook, peeking, previewState, withThemeDraft } from "./panels/apUi";
import { normLook } from "./panels/AppearanceClosed";
import { normShape } from "../strip/looks";
import { normHover } from "./panels/AppearanceHover";
import StripLookPreview from "./previews/StripLookPreview";
import HoverPreview from "./previews/HoverPreview";
import { CardsMini, peekingCards } from "../layout/CardsGallery";
import "./panels/appearance.css";

const DOCK_KEY = "ledge.settings.dock";
const readOpen = () => {
  try {
    return localStorage.getItem(DOCK_KEY) !== "0";
  } catch {
    return true;
  }
};
const [open, setOpenSig] = createSignal(readOpen());
function setOpen(v: boolean) {
  setOpenSig(v);
  try {
    localStorage.setItem(DOCK_KEY, v ? "1" : "0");
  } catch {
    // paměť je jen pohodlí
  }
}

/** podíl šířky miniatury šuplíku podle předvolby šířky (jako galerie stylů) */
const MINI_W: Record<string, string> = { custom: "64%", narrow: "48%", medium: "64%", wide: "82%", full: "100%" };

/** Otevřený šuplík v malém: proužek, hodiny a karty ve sloupcích rozložení, barvy a tvar vzhledu. */
function DrawerMini(q: { a: Appearance }) {
  const cols = () => Math.min(5, LAYOUTS[q.a.layout]?.cols ?? 4);
  const sh = () => stripShape(q.a.strip.style, q.a.strip.size);
  return (
    <span class="ap-mini st-dock-mini theme-scope" style={scopeStyle(q.a)} data-layout={q.a.layout} aria-hidden="true">
      <i class="ap-mstrip" style={{ height: `${Math.round(sh().h * 0.5)}px`, "border-radius": `0 0 ${sh().r * 0.5}px ${sh().r * 0.5}px` }} />
      <span class="ap-mdrawer" style={{ width: MINI_W[q.a.width] ?? "64%" }}>
        <b class="ap-mt">14:32</b>
        <span class="ap-mgrid" style={{ "--mc": String(cols()) }}>
          <span class="ap-mcard wide"><i class="ap-mh" /><i class="ap-ml" /></span>
          <For each={Array.from({ length: cols() * 2 })}>{() => <span class="ap-mcard"><i class="ap-mh" /><i class="ap-ml" /><i class="ap-ml short" /></span>}</For>
        </span>
      </span>
    </span>
  );
}

export default function PreviewDock() {
  /** co ukázat: dlaždice pod myší, jinak rozpracovaný vzhled */
  const a = (): Appearance => peek() ?? withThemeDraft(normalizeAppearance(cfg.appearance));
  const look = () => peekLook() ?? normLook(cfg.strip?.look);
  const hover = () => peekHover() ?? normHover(cfg.drawer?.hover);
  const opacity = () => cfg.strip?.opacity ?? 0.82;
  return (
    <section class="st-dock" classList={{ shut: !open(), peeking: peeking() || peekingCards() }} aria-label={t("settings.dock.title", "Náhled — takhle to vyjede")}>
      <header class="st-dock-head">
        <b>{t("settings.dock.title", "Náhled — takhle to vyjede")}</b>
        <small>{peeking() || peekingCards() ? t("settings.dock.peek", "Náhled dlaždice pod myší — klikni pro výběr") : t("settings.dock.hint", "Najetí na dlaždici se ukáže jen tady; skutečný Ledge se změní po kliknutí a Uložit.")}</small>
        <button class="mini" aria-expanded={open()} onClick={() => setOpen(!open())}>
          {open() ? t("settings.dock.hide", "Skrýt náhled") : t("settings.dock.show", "Ukázat náhled")}
        </button>
      </header>
      <Show when={open()}>
        <StateSwitch />
        <div class="st-dock-grid">
          <figure>
            <StripLookPreview look={look()} state={previewState()} a={a()} opacity={opacity()} shape={normShape(cfg.strip?.lookShape)} compose={cfg.strip?.compose} />
            <figcaption>{t("settings.dock.closed", "Zavřený")}</figcaption>
          </figure>
          <figure>
            <HoverPreview mode={hover()} state={previewState()} a={a()} />
            <figcaption>{t("settings.dock.hover", "Po najetí")}</figcaption>
          </figure>
          <figure>
            <DrawerMini a={a()} />
            <figcaption>{t("settings.dock.open", "Otevřený")}</figcaption>
          </figure>
          {/* karty 2026 / bento (kolo 22, `layout/CardsGallery.tsx`): dlaždice pod myší, jinak vybraná záložka galerie */}
          <figure>
            <CardsMini a={a()} />
            <figcaption>{t("settings.dock.cards", "Karty")}</figcaption>
          </figure>
        </div>
      </Show>
    </section>
  );
}
