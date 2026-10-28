// Scéna Ledge Studia (kolo 24): živý náhled JEN právě upravované části, vpravo vedle obsahu (sticky; v úzkém
// okně pod hlavičkou části). Proužek = horní hrana monitoru s proužkem + „Co se zrovna děje“; Otevírání =
// co vyjede po najetí; Vzhled šuplíku = mini šuplík a karty v motivu; Obsah = mapa záložek a karet; Claude =
// ukázka žádosti o povolení a limitů. Ostatní části scénu nemají (obsah dostane šířku). Najetí na dlaždici
// (`apUi.tsx` `peek*`, `layout/CardsGallery.tsx`) se ukáže JEN tady; skutečný Ledge mění až klik (Platí hned).
// Data v náhledech jsou ukázková a scéna to říká. Žádný časovač ani animace v klidu.
import { For, Match, Show, Switch, createMemo } from "solid-js";
import { t } from "../i18n";
import type { Appearance } from "../types";
import { normalizeAppearance, stripShape } from "../theme/appearance";
import { scopeStyle } from "../theme/apply";
import { LAYOUTS } from "../theme/looks";
import { cfg } from "../settings/ctx";
import { StateSwitch, peek, peekHover, peekLook, previewState, withThemeDraft } from "../settings/panels/apUi";
import { normLook } from "../settings/panels/AppearanceClosed";
import { normHover } from "../settings/panels/AppearanceHover";
import { STRIP_LOOKS, normShape } from "../strip/looks";
import { HOVER_MODES } from "../hover/modes";
import StripLookPreview from "../settings/previews/StripLookPreview";
import HoverPreview from "../settings/previews/HoverPreview";
import { CardsMini } from "../layout/CardsGallery";
import { normalizeLayout } from "../layout/model";
import { cardName, pageName } from "../layout/text";
import { cardPart } from "./anatomy";
import type { PartId } from "./parts";
import "../settings/panels/appearance.css";

/** Části, které scénu mají. */
export const SCENE_PARTS: readonly PartId[] = ["pruzek", "otevirani", "vzhled", "obsah", "claude"];

/** podíl šířky miniatury šuplíku podle předvolby šířky (jako galerie stylů) */
const MINI_W: Record<string, string> = { custom: "64%", narrow: "48%", medium: "64%", wide: "82%", full: "100%" };

/** Otevřený šuplík v malém: proužek, hodiny a karty ve sloupcích rozložení, barvy a tvar vzhledu. */
export function DrawerMini(q: { a: Appearance }) {
  const cols = () => Math.min(5, LAYOUTS[q.a.layout]?.cols ?? 4);
  const sh = () => stripShape(q.a.strip.style, q.a.strip.size);
  return (
    <span class="ap-mini sd-mini theme-scope" style={scopeStyle(q.a)} data-layout={q.a.layout} aria-hidden="true">
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

/** Horní hrana monitoru — rám pro proužek a „co vyjede“. */
const Edge = (q: { children: any; tall?: boolean }) => (
  <div class="sd-edge tall" classList={{ taller: q.tall }}>
    <i class="sd-bezel" />
    <div class="sd-edge-in">{q.children}</div>
  </div>
);

function ContentMap() {
  const L = createMemo(() => normalizeLayout(cfg.layout));
  return (
    <div class="sd-map">
      <For each={L().pages.slice(0, 6)}>
        {(pg) => (
          <section class="sd-map-page">
            <b>{pageName(pg)}</b>
            <div class="sd-map-cards">
              <For each={pg.cards.slice(0, 10)}>{(c) => <span class="sd-chip" data-part={cardPart(c.id)} data-size={c.size}>{cardName(c.id)}</span>}</For>
              <Show when={!pg.cards.length}><small>{t("settings.studio.scene.emptyPage", "Prázdná záložka")}</small></Show>
            </div>
          </section>
        )}
      </For>
    </div>
  );
}

function ClaudeDemo() {
  return (
    <div class="sd-claude">
      <div class="sd-ask">
        <header><i class="sd-pulse" /><b>{t("settings.studio.scene.askTitle", "Claude čeká na povolení")}</b><small>app · 0:12</small></header>
        <code>npm run test -- --watch=false</code>
        <div class="sd-ask-btns">
          <span class="sd-btn ok">{t("settings.studio.scene.allow", "Povolit")}</span>
          <span class="sd-btn">{t("settings.studio.scene.deny", "Zamítnout")}</span>
        </div>
      </div>
      <div class="sd-limits">
        <div><span>{t("settings.studio.scene.session", "Session")}</span><i style={{ "--v": "38%" }} /><small>38 %</small></div>
        <div><span>{t("settings.studio.scene.week", "Týden")}</span><i style={{ "--v": "64%" }} /><small>64 %</small></div>
      </div>
    </div>
  );
}

/** Souhrn aktuální volby pod náhledem — jen skutečné hodnoty z configu. */
function Sum(q: { rows: [string, string][] }) {
  return (
    <dl class="sd-sum">
      <For each={q.rows}>{([k, v]) => <><dt>{k}</dt><dd>{v}</dd></>}</For>
    </dl>
  );
}

export default function Scene(p: { part: PartId; onHide: () => void }) {
  const a = (): Appearance => peek() ?? withThemeDraft(normalizeAppearance(cfg.appearance));
  const look = () => peekLook() ?? normLook(cfg.strip?.look);
  const hover = () => peekHover() ?? normHover(cfg.drawer?.hover);
  const opacity = () => cfg.strip?.opacity ?? 0.82;
  return (
    <aside class="sd" data-scene aria-label={t("settings.studio.scene.title", "Ukázka")}>
      <header class="sd-head">
        <b>{t("settings.studio.scene.title", "Ukázka")}</b>
        <small>{t("settings.studio.scene.hint", "Ukázková data · najetí na volbu se ukáže tady")}</small>
        <button class="sd-x" onClick={p.onHide} title={t("settings.studio.scene.hide", "Skrýt ukázku")} aria-label={t("settings.studio.scene.hide", "Skrýt ukázku")}>×</button>
      </header>
      <Switch>
        <Match when={p.part === "pruzek"}>
          <Edge><StripLookPreview look={look()} state={previewState()} a={a()} opacity={opacity()} shape={normShape(cfg.strip?.lookShape)} compose={cfg.strip?.compose} /></Edge>
          <StateSwitch />
          <Sum rows={[
            [t("settings.studio.scene.sum.look", "Vzhled"), (() => { const x = STRIP_LOOKS.find((l) => l.id === look()); return x ? t(x.key, x.cs) : String(look()); })()],
            [t("settings.studio.scene.sum.opacity", "Krytí"), Math.round(opacity() * 100) + " %"],
          ]} />
        </Match>
        <Match when={p.part === "otevirani"}>
          <Edge tall><HoverPreview mode={hover()} state={previewState()} a={a()} /></Edge>
          <StateSwitch />
          <Sum rows={[[t("settings.studio.scene.sum.hover", "Po najetí"), (() => { const x = HOVER_MODES.find((l) => l.id === hover()); return x ? t(x.key, x.cs) : String(hover()); })()]]} />
        </Match>
        <Match when={p.part === "vzhled"}>
          <div class="sd-stack">
            <DrawerMini a={a()} />
            <CardsMini a={a()} />
          </div>
        </Match>
        <Match when={p.part === "obsah"}><ContentMap /></Match>
        <Match when={p.part === "claude"}><ClaudeDemo /></Match>
      </Switch>
    </aside>
  );
}
