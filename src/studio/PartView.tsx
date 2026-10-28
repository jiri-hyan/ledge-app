// Obrazovka části Ledge Studia (kolo 24): hlavička (název + jedna věta), pod ní otázky — každá otázka je
// jeden blok (dnešní panel nebo krok Vzhledu, `parts.ts`) s velkými volbami; technické volby panelů jsou
// v sbaleném „Doladit“ (`fields/Advanced.tsx`, `panels/apUi.tsx`). V průvodci jen hlavní otázky (`primary`)
// a bez Doladit, dokud uživatel neřekne „Ukázat vše v této části“. Panely se vykreslí jednou a žijí dál
// (přepnutí části je jen skryje — stav i posuv zůstanou).
import { For, Show, type Component, type JSX } from "solid-js";
import { Dynamic } from "solid-js/web";
import { t } from "../i18n";
import { lbl } from "../theme/label";
import type { Snapshot } from "../types";
import type { PanelProps } from "../settings/types";
import { cfg, setCfg } from "../settings/ctx";
import { PANELS } from "../settings/catalog";
import { AppearanceStep } from "../settings/panels/Appearance";
import { partById, visibleQuestions, type BlockId, type PartId } from "./parts";
import { PartIcon } from "./icons";

const step = (s: "closed" | "hover" | "open" | "motion"): Component<PanelProps> => (p) => <AppearanceStep {...p} step={s} />;
const VZHLED: Partial<Record<BlockId, Component<PanelProps>>> = { strip: step("closed"), hover: step("hover"), look: step("open"), motion: step("motion") };
const blockComponent = (b: BlockId): Component<PanelProps> | undefined => VZHLED[b] ?? PANELS.find((x) => x.id === b)?.component;

export default function PartView(p: {
  part: PartId;
  s: Snapshot;
  guide: boolean;
  showAll: boolean;
  onShowAll: () => void;
  scene?: JSX.Element;
  modified: number;
}) {
  const part = () => partById(p.part)!;
  const all = () => visibleQuestions(part(), cfg);
  const questions = () => all().filter((q) => !p.guide || p.showAll || q.primary);
  const hidden = () => all().length - questions().length;
  return (
    <div class="sp" data-part={p.part} data-trim={p.guide && !p.showAll ? "" : undefined}>
      <header class="sp-head">
        <span class="sp-icon"><PartIcon icon={part().icon} /></span>
        <div>
          <h1>{lbl(part().name)}</h1>
          <p>{lbl(part().line)}</p>
        </div>
        <Show when={p.modified}>
          <span class="sp-mod" title={t("settings.studio.modified.hint", "Volby, které se liší od výchozích")}>{t("settings.studio.modified", "{n} upraveno", { n: p.modified })}</span>
        </Show>
      </header>
      {p.scene}
      <For each={questions()}>
        {(q, i) => (
          <section class="sq" id={`q-${q.block}`}>
            <header class="sq-head">
              <i class="sq-n">{i() + 1}</i>
              <div>
                <h2>{lbl(q.q)}</h2>
                <p>{lbl(q.sub)}</p>
              </div>
            </header>
            <div class="sq-body" data-q>
              <Dynamic component={blockComponent(q.block)} cfg={cfg} setCfg={setCfg} s={p.s} />
            </div>
          </section>
        )}
      </For>
      <Show when={p.guide && !p.showAll}>
        <button class="sp-more" onClick={p.onShowAll}>
          {hidden()
            ? t("settings.studio.guide.showAllN", "Ukázat vše v této části (+{n} otázky a Doladit)", { n: hidden() })
            : t("settings.studio.guide.showAll", "Ukázat vše v této části (i Doladit)")}
        </button>
      </Show>
    </div>
  );
}
