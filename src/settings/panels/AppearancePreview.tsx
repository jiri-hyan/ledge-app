// Živý náhled vzhledu pro panel Vzhled: mini „obrazovka“ s proužkem (pill/line/dot, pozice) a malým
// šuplíkem s kartou, vykreslené v DOM pod `.theme-scope` s proměnnými rozpracovaného vzhledu
// (`scopeStyle`). „Přehrát animaci“ zahraje otevření stejným plánem jako šuplík (`theme/openAnim.ts`,
// styl a rychlost rozpracovaného vzhledu); po změně stylu/rychlosti se přehraje sám.
import { For, Show, createEffect, on } from "solid-js";
import type { Appearance } from "../../types";
import { t } from "../../i18n";
import { spring } from "../../motion";
import { RADIUS, STRIP_SIZE, effectiveMotion, stripShape } from "../../theme/appearance";
import { openPlan } from "../../theme/openAnim";
import { scopeStyle } from "../../theme/apply";

type Props = { a: Appearance; stripOpacity: number; blur: boolean };

/** Barvy linky proužku v náhledu — stejná pravidla jako `geom::line_stops` (prázdné = přechod akcentu). */
function lineBackground(a: Appearance): string {
  const c = a.strip.color;
  if (c === "accent") return "var(--accent)";
  if (c) return c;
  return "linear-gradient(90deg, transparent, var(--accent) 35%, var(--accent-2) 65%, transparent)";
}

export default function AppearancePreview(p: Props) {
  let drawer!: HTMLDivElement;
  let body!: HTMLDivElement;

  function replay() {
    const shape = stripShape(p.a.strip.style, p.a.strip.size);
    const w = p.a.strip.style === "dot" ? 56 : 150;
    const r = 26 * RADIUS[p.a.radius];
    const reduced = typeof matchMedia === "function" && matchMedia("(prefers-reduced-motion: reduce)").matches;
    const plan = openPlan(effectiveMotion(p.a.motion.style, reduced), p.a.motion.speed, {
      collapsed: `inset(0px calc(50% - ${w / 2}px) calc(100% - ${shape.h}px) calc(50% - ${w / 2}px) round 0px 0px ${shape.r}px ${shape.r}px)`,
      band: `inset(0px 0px calc(100% - ${Math.round(shape.h * 1.6)}px) 0px round 0px 0px ${r}px ${r}px)`,
      full: `inset(0px 0px 0px 0px round 0px 0px ${r}px ${r}px)`,
    });
    drawer.getAnimations().forEach((x) => x.cancel());
    if (plan) drawer.animate(plan.frames, { duration: plan.duration, easing: plan.easing });
    const pop = spring(420, 22);
    body.animate([{ opacity: 0, transform: "translateY(-8px)" }, { opacity: 1, transform: "none" }], {
      duration: pop.duration,
      easing: pop.easing,
      delay: Math.min(60, pop.duration),
      fill: "backwards",
    });
  }
  // změna stylu nebo rychlosti animací → hned ukázat, jak vypadá
  createEffect(on(() => [p.a.motion.style, p.a.motion.speed], () => queueMicrotask(replay), { defer: true }));

  const dots = ["var(--green)", "var(--violet)", "var(--amber)"];
  return (
    <div class="ap-preview theme-scope" style={scopeStyle(p.a)} aria-label={t("appearance.preview.label", "Náhled vzhledu")}>
      <div class="ap-screen" data-pos={p.a.strip.position}>
        <div class="ap-axis">
          <div
            class="ap-strip"
            classList={{
              line: p.a.strip.style === "line", dot: p.a.strip.style === "dot", "ap-s-term": p.a.strip.style === "term", "ap-s-tag": p.a.strip.style === "tag",
              "ap-s-glow": p.a.strip.style === "glow", "ap-s-block": p.a.strip.style === "block", "ap-s-drop": p.a.strip.style === "drop",
            }}
            style={{ "--k": String(STRIP_SIZE[p.a.strip.size]), "--so": String(p.stripOpacity) }}
          >
            <Show
              when={p.a.strip.style !== "dot"}
              fallback={<For each={dots}>{(c) => <i class="ap-dot" style={{ background: c }} />}</For>}
            >
              <b>{t("appearance.preview.work", "Práce")}</b>
              <span class="ap-sep" />
              <span class="ap-dim">{t("appearance.preview.time", "14:32 · čt 16. 10.")}</span>
              <span class="ap-sep" />
              <span class="ap-pillseg"><i class="ap-dot" style={{ background: dots[0] }} />web :5173</span>
            </Show>
            <span class="ap-sig" style={{ background: lineBackground(p.a) }} />
          </div>
          <div class="ap-drawer" classList={{ blur: p.blur, "ap-float": p.a.layout === "float" && !p.blur }} ref={drawer}>
            <div ref={body} class="ap-body">
              <div class="ap-head">
                <b>{t("appearance.preview.work", "Práce")}</b>
                <div class="seg"><button class="on">{t("appearance.preview.work", "Práce")}</button><button>{t("appearance.preview.personal", "Osobní")}</button></div>
              </div>
              <div class="card">
                <h4><span class="ic" style={{ background: "var(--violet)" }} />Claude <span class="n">2</span></h4>
                <div class="row"><span class="name">ledge</span><span class="pill v">{t("appearance.preview.working", "pracuje")}</span></div>
                <div class="row"><span class="grow sub">{t("appearance.preview.meta", "před 3 min · feat/appearance")}</span><span class="pill g">{t("appearance.preview.done", "hotovo")}</span></div>
              </div>
              <div class="ap-actions">
                <button class="mini" tabindex="-1">{t("appearance.preview.mini", "Otevřít")}</button>
                <button class="btn pri" tabindex="-1">{t("appearance.preview.primary", "Spustit")}</button>
              </div>
            </div>
          </div>
        </div>
      </div>
      <button class="mini ap-replay" onClick={replay}>{t("appearance.preview.replay", "▶ Přehrát animaci")}</button>
    </div>
  );
}
