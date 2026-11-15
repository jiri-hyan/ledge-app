// HTML náhled jednoho chování po najetí myší (`drawer.hover`: celý šuplík / roztažený
// ostrůvek / dok) pro galerii kroku Po najetí v panelu Vzhled. Barvy a písmo z rozpracovaného vzhledu
// (`.theme-scope`), data UKÁZKOVÁ podle přepínače „Co se děje“ — skutečná okna kreslí Ledge. Tlačítka jsou
// jen obrázek (`span`), dlaždice sama je tlačítko.
import { For, type JSX } from "solid-js";
import { t } from "../../i18n";
import type { Appearance, HoverModeId } from "../../types";
import { scopeStyle } from "../../theme/apply";
import { deckCols, type PreviewState } from "../appearanceModel";
import { Dot, Glyph, sample } from "./StripLookPreview";
import "./previews.css";

const Btn = (q: { tone?: "ok" | "no"; children: JSX.Element }) => <span class="apv-btn" data-c={q.tone ?? ""}>{q.children}</span>;

function island(s: PreviewState) {
  const body: Record<PreviewState, JSX.Element> = {
    idle: <><b class="apv-big">14:32</b><span class="apv-dim">{t("appearance.sample.date", "ne 18. 10.")}</span></>,
    work: <><span><Dot c="run" ring /> {t("appearance.sample.workOn", "Claude pracuje na ledge")}</span><span class="apv-dim">4 min</span></>,
    wait: <><span><Glyph g="!" c="wait" /> <b>ledge</b> {t("appearance.sample.wantsRun", "chce spustit příkaz")}</span><span class="apv-btns"><Btn tone="ok">{t("appearance.sample.allow", "Povolit")}</Btn><Btn tone="no">{t("appearance.sample.deny", "Zamítnout")}</Btn></span></>,
    music: <><span><span class="apv-cover" /> {sample.music()}</span><span>⏮ ⏸ ⏭</span></>,
    timer: <><span><Glyph g="⏱" c="acc" /> {t("appearance.sample.left", "Zbývá 18:04")}</span><span class="apv-btns"><Btn>+5 min</Btn></span></>,
    done: <><span><Glyph g="✓" c="ok" /> {sample.done()}</span><span class="apv-btns"><Btn>{t("appearance.sample.open", "Otevřít")}</Btn></span></>,
  };
  return <span class="apv-peek apv-island">{body[s]}</span>;
}

function dock() {
  return (
    <span class="apv-peek apv-island">
      <span>{t("appearance.sample.dockNote", "📝 klikni a piš")}</span>
      <span><span class="apv-cover" /> ⏮ ⏸ ⏭</span>
      <span>📌</span>
    </span>
  );
}

function full(a: Appearance) {
  const cols = Math.min(4, deckCols(a.layout));
  return (
    <span class="apv-peek apv-full" style={{ "--c": String(cols) }}>
      <For each={Array.from({ length: cols * 2 })}>{(_, i) => <i class="apv-pcard" classList={{ "apv-wide": i() === 0 && cols > 2 }} />}</For>
    </span>
  );
}

export default function HoverPreview(q: { mode: HoverModeId; state: PreviewState; a: Appearance }) {
  return (
    <span class="apv-scr apv-tall theme-scope" style={scopeStyle(q.a)} data-pos={q.a.strip.position} aria-hidden="true">
      <span class="apv-win" />
      {q.mode === "island" ? island(q.state) : q.mode === "dock" ? dock() : full(q.a)}
    </span>
  );
}
