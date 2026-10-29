// Výběr vzhledu karty (kolo 25, `CardLook.design`, registr `designs.ts`): řada klikacích dlaždic s mini náhledem
// a názvem, jeden klik platí hned. Společné pro panel „Vzhled karty…“ (`CardLook.tsx`) a editor v Nastavení
// (`LayoutEditor.tsx`). Náhled je statická kresba (žádná data, žádná animace); neznámé id dostane obecný náhled.
import { For, Show, type JSX } from "solid-js";
import { t } from "../i18n";
import { designOf, designsOf } from "./designs";
import { tx } from "./text";
import "./designs.css";

const P = (d: string, extra?: Record<string, string | number>) => <path d={d} fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" {...extra} />;
const R = (x: number, y: number, w: number, h: number, o = 1) => <rect x={x} y={y} width={w} height={h} rx="1.5" fill="currentColor" opacity={o} />;

const ARC = "M7 19 A13 13 0 0 1 33 19";
/** Mini náhledy podle id vzhledu (40×24). */
const THUMBS: Record<string, () => JSX.Element> = {
  classic: () => <>{R(3, 4, 34, 3)}{R(3, 10, 34, 3, 0.6)}{R(3, 16, 22, 3, 0.35)}</>,
  lines: () => <>{R(3, 3, 8, 3)}{P("M14 5 L20 3 L26 6 L37 2")}{R(3, 11, 8, 3, 0.6)}{P("M14 13 L22 12 L37 13", { opacity: 0.6 })}{R(3, 19, 34, 2.5, 0.35)}</>,
  rings: () => <><circle cx="20" cy="12" r="10" fill="none" stroke="currentColor" stroke-width="2.4" opacity="0.35" /><circle cx="20" cy="12" r="10" fill="none" stroke="currentColor" stroke-width="2.4" stroke-dasharray="40 63" /><circle cx="20" cy="12" r="5.5" fill="none" stroke="currentColor" stroke-width="2.4" stroke-dasharray="14 35" /></>,
  pace: () => <>{R(3, 9, 34, 5, 0.3)}{R(3, 9, 20, 5)}<path d="M27 6 V17" stroke="currentColor" stroke-width="1.6" />{R(3, 19, 22, 2, 0.5)}</>,
  hero: () => <>{R(5, 4, 12, 12)}{R(20, 4, 12, 12)}{R(5, 19, 28, 2, 0.4)}</>,
  heroWeek: () => <>{R(5, 4, 12, 12)}{R(20, 4, 12, 12)}{R(5, 19, 28, 2, 0.4)}</>,
  heroSpend: () => <>{R(5, 4, 12, 12)}{R(20, 4, 12, 12)}{R(5, 19, 28, 2, 0.4)}</>,
  spend: () => <>{R(3, 4, 12, 7)}{R(18, 14, 3, 7, 0.4)}{R(23, 10, 3, 11, 0.4)}{R(28, 7, 3, 14, 0.4)}{R(33, 12, 3, 9)}</>,
  sessions: () => <>{R(3, 3, 16, 7, 0.8)}{R(21, 3, 16, 7, 0.5)}{R(3, 12, 16, 7, 0.5)}{R(21, 21, 16, 1.6)}</>,
  gauge: () => <>{P(ARC, { opacity: 0.3 })}{P("M7 19 A13 13 0 0 1 22 6.4")}<path d="M20 19 L27 10" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" /></>,
  gauges: () => <>{P("M3 19 A8 8 0 0 1 19 19", { opacity: 0.3 })}{P("M21 19 A8 8 0 0 1 37 19", { opacity: 0.3 })}<path d="M11 19 L15 13 M29 19 L26 12" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" /></>,
  bars: () => <>{R(4, 14, 3, 7, 0.5)}{R(9, 9, 3, 12, 0.6)}{R(14, 12, 3, 9, 0.5)}{R(19, 5, 3, 16)}{R(24, 8, 3, 13, 0.8)}{R(29, 13, 3, 8, 0.5)}{R(34, 10, 3, 11, 0.6)}</>,
  terminal: () => <>{R(3, 4, 3, 4)}{R(7, 4, 3, 4)}{R(11, 4, 3, 4)}{R(15, 4, 3, 4, 0.3)}{R(19, 4, 3, 4, 0.3)}{R(3, 11, 3, 4)}{R(7, 11, 3, 4)}{R(11, 11, 3, 4, 0.3)}{R(15, 11, 3, 4, 0.3)}{R(19, 11, 3, 4, 0.3)}{R(3, 18, 3, 3)}{R(7, 18, 3, 3, 0.3)}</>,
  tiles: () => <>{R(3, 3, 10, 12)}{R(15, 3, 10, 12, 0.7)}{R(27, 3, 10, 12, 0.5)}{P("M3 20 L9 18 L13 21 M15 20 L22 19 L25 21 M27 19 L33 21 L37 18", { "stroke-width": 1.2 })}</>,
  net: () => <><path d="M12 3 V16 M7 12 L12 17 L17 12" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" /><path d="M28 21 V8 M23 12 L28 7 L33 12" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" opacity="0.6" /></>,
};

const C = (cx: number, cy: number, r: number, o = 1) => <circle cx={cx} cy={cy} r={r} fill="currentColor" opacity={o} />;
const SUN = (cx: number, cy: number) => <>{C(cx, cy, 3.2)}<path d={`M${cx} ${cy - 6} V${cy - 4.6} M${cx - 6} ${cy} H${cx - 4.6} M${cx + 4.6} ${cy} H${cx + 6} M${cx - 4.2} ${cy - 4.2} L${cx - 3.2} ${cy - 3.2} M${cx + 3.2} ${cy - 3.2} L${cx + 4.2} ${cy - 4.2}`} stroke="currentColor" stroke-width="1.3" stroke-linecap="round" /></>;
const CLOUD = (x: number, y: number, o = 1) => <path d={`M${x} ${y + 6} a3.6 3.6 0 0 1 1-7 a5 5 0 0 1 9.4 1.2 a3 3 0 0 1 .6 5.8 Z`} fill="currentColor" opacity={o} />;
/** Náhledy karet Počasí a Týden (klíč `card:id`; id `classic` / `week` / `rings` se mezi kartami opakují). */
const CARD_THUMBS: Record<string, () => JSX.Element> = {
  "weather:classic": () => <>{SUN(9, 8)}{R(18, 4, 19, 4)}{R(18, 11, 12, 2.5, 0.5)}{R(3, 18, 34, 3, 0.35)}</>,
  "weather:sky": () => <><rect x="2" y="2" width="36" height="20" rx="3" fill="currentColor" opacity="0.18" />{CLOUD(6, 5)}{CLOUD(21, 10, 0.55)}{C(32, 6, 1, 0.9)}{C(28, 4, 0.7, 0.6)}<path d="M9 19 l-1 2 M14 19 l-1 2 M25 19 l-1 2" stroke="currentColor" stroke-width="1.2" stroke-linecap="round" opacity="0.7" /></>,
  "weather:thermo": () => <>{C(7, 7, 3)}{R(13, 5, 24, 4, 0.2)}{R(17, 5, 14, 4, 0.6)}<path d="M24 3 V11" stroke="currentColor" stroke-width="1.6" />{R(3, 16, 8, 2.5, 0.5)}{R(29, 16, 8, 2.5, 0.5)}</>,
  "weather:poster": () => <>{R(3, 3, 22, 10)}{R(3, 16, 34, 2.5, 0.5)}{R(3, 20.5, 20, 2, 0.3)}{SUN(31, 8)}</>,
  "weather:curve": () => <>{R(5, 15, 3, 6, 0.35)}{R(11, 17, 3, 4, 0.35)}{R(17, 13, 3, 8, 0.35)}{R(23, 18, 3, 3, 0.35)}{R(29, 16, 3, 5, 0.35)}{P("M3 12 C9 4 14 4 20 8 S31 12 37 5")}</>,
  "weather:week": () => <>{[3, 8, 13, 18].map((y, i) => <>{R(3, y, 7, 2.2, 0.6)}{R(14 + i * 2, y, 14 - i, 2.2, 0.25)}{R(17 + i * 2, y, 6, 2.2)}</>)}</>,
  "weather:arc": () => <>{P("M4 20 A16 16 0 0 1 36 20", { opacity: 0.3, "stroke-dasharray": "2 3" })}{C(12, 9.6, 3)}{R(2, 20.5, 36, 1.4, 0.4)}</>,
  "week:strip": () => <>{R(3, 3, 14, 3)}{[0, 1, 2, 3, 4, 5, 6].map((i) => R(3 + i * 5, 10, 4, 7, i === 2 ? 1 : 0.35))}{R(3, 20, 30, 2, 0.4)}</>,
  "week:month": () => <>{[0, 1, 2, 3, 4].map((r) => [0, 1, 2, 3, 4, 5, 6].map((c) => C(6 + c * 4.6, 4.5 + r * 4.3, r === 2 && c === 3 ? 1.7 : 1, (r + c) % 4 === 0 ? 1 : 0.35)))}</>,
  "week:tearoff": () => <>{R(9, 2, 22, 4, 0.5)}<rect x="9" y="7" width="22" height="14" rx="2" fill="none" stroke="currentColor" stroke-width="1.6" />{R(15, 11, 10, 6)}</>,
  "week:timeline": () => <>{R(3, 11, 34, 1.6, 0.35)}{R(6, 5, 9, 5)}{R(18, 14, 11, 5, 0.6)}{R(31, 5, 5, 5, 0.4)}<path d="M23 3 V21" stroke="currentColor" stroke-width="1.4" /></>,
  "week:dial": () => <><circle cx="20" cy="12" r="9.5" fill="none" stroke="currentColor" stroke-width="2" opacity="0.3" /><path d="M20 12 L20 2.5 A9.5 9.5 0 0 1 27.4 6 Z" fill="currentColor" opacity="0.8" /><path d="M20 12 L14 6" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" /></>,
  "week:agenda": () => <>{[3, 9, 15].map((y, i) => <>{C(5, y + 1.5, 1.5, 1 - i * 0.25)}{R(10, y, 20 - i * 3, 3, 0.8 - i * 0.2)}{R(32, y, 5, 3, 0.35)}</>)}{R(3, 21, 34, 1.5, 0.3)}</>,
  "notes:list": () => <>{[3, 9, 15].map((y, i) => <>{R(3, y, 3, 3, 0.9 - i * 0.2)}{R(9, y, 26 - i * 4, 2.2, 0.8 - i * 0.2)}{R(9, y + 3, 14, 1.4, 0.3)}</>)}</>,
  "notes:compact": () => <>{[2, 6.5, 11, 15.5, 20].map((y, i) => R(3, y, 34 - (i % 3) * 6, 2.2, 0.8 - i * 0.1))}</>,
  "notes:cards": () => <>{[0, 1].map((r) => [0, 1, 2].map((c) => <rect x={3 + c * 11.7} y={3 + r * 10} width="10" height="8.5" rx="1.5" fill="currentColor" opacity={0.25 + ((r + c) % 3) * 0.2} />))}</>,
  "notes:bento": () => <>{R(3, 3, 22, 12, 0.6)}{R(27, 3, 10, 5.5, 0.3)}{R(27, 9.5, 10, 5.5, 0.3)}{R(3, 17, 34, 3.5, 0.2)}</>,
  "notes:kanban": () => <>{R(3, 3, 16, 3, 0.7)}{R(21, 3, 16, 3, 0.35)}{[9, 14, 19].map((y) => R(3, y, 16, 3.6, 0.45))}{[9, 14].map((y) => R(21, y, 16, 3.6, 0.2))}</>,
  "notes:timeline": () => <><path d="M9 2 V22" stroke="currentColor" stroke-width="1.4" opacity="0.4" />{[5, 12, 19].map((y, i) => <>{C(9, y, 2, 1 - i * 0.25)}{R(15, y - 1.6, 22 - i * 4, 3.2, 0.7 - i * 0.2)}</>)}</>,
};

export function DesignThumb(p: { id: string; card?: string }) {
  return (
    <svg class="dz-thumb" viewBox="0 0 40 24" aria-hidden="true">
      {(CARD_THUMBS[`${p.card}:${p.id}`] ?? THUMBS[p.id] ?? THUMBS.classic)()}
    </svg>
  );
}

/** Dlaždice vzhledů karty `card`; `look` = aktuální vzhled karty, `onPick` dostane id (výchozí také — volající ho normalizuje pryč). */
export default function DesignPicker(p: { card: string; look: { design?: string } | undefined; onPick: (id: string) => void }) {
  const list = () => designsOf(p.card);
  const cur = () => designOf(p.card, p.look)?.id;
  return (
    <Show when={list().length > 1}>
      <div class="tl-ed-col dz-box" role="group" aria-label={t("cards.design.title", "Vzhled karty")}>
        <span class="tl-ed-cap">{t("cards.design.title", "Vzhled karty")}</span>
        <div class="dz-grid" role="radiogroup" aria-label={t("cards.design.title", "Vzhled karty")}>
          <For each={list()}>
            {(d) => (
              <button class="dz-tile" classList={{ on: cur() === d.id }} role="radio" aria-checked={cur() === d.id} title={tx(d.line)} onClick={() => p.onPick(d.id)}>
                <DesignThumb card={p.card} id={d.id} />
                <b>{tx(d.name)}</b>
              </button>
            )}
          </For>
        </div>
      </div>
    </Show>
  );
}
