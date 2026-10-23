// Výběr animace otevírání v panelu Vzhled: 10 stylů jako malé živé ukázky (výřez nahoře + malý šuplík).
// Klik = vybrat a jednou přehrát tentýž plán, jaký použije šuplík (`theme/openAnim.ts` `openPlan`) —
// v klidu se nic nehýbe. Pod tím rychlost; při systémovém „omezit pohyb“ upozornění (Ledge pak
// nepřehrává nic, ani vysunutí okna v Rustu).
import { For, Show } from "solid-js";
import { t } from "../../i18n";
import type { MotionStyle } from "../../types";
import { MOTION_STYLES } from "../../theme/appearance";
import { openPlan } from "../../theme/openAnim";
import type { Pair } from "../../theme/looks";
import { lbl } from "../../theme/label";
import { Slider, lit, times, type Ap } from "./apUi";

const NAME: Record<MotionStyle, Pair> = {
  spring: ["appearance.motion.spring", "Pružina"],
  smooth: ["appearance.motion.smooth", "Plynulá"],
  snappy: ["appearance.motion.snappy", "Svižná"],
  bounce: ["appearance.motion.bounce", "Odskok"],
  fade: ["appearance.motion.fade", "Rozplynutí"],
  notch: ["appearance.motion.notch", "Z výřezu"],
  drop: ["appearance.motion.drop", "Pád"],
  unfold: ["appearance.motion.unfold", "Rozvinutí"],
  liquid: ["appearance.motion.liquid", "Tekutá kapsle"],
  none: ["appearance.motion.none", "Žádná"],
};
const DESC: Record<MotionStyle, Pair> = {
  spring: ["appearance.motion.springDesc", "přejede a usadí se"],
  smooth: ["appearance.motion.smoothDesc", "klidně, bez překmitu"],
  snappy: ["appearance.motion.snappyDesc", "rychle, pro netrpělivé"],
  bounce: ["appearance.motion.bounceDesc", "dopadne a dvakrát odskočí"],
  fade: ["appearance.motion.fadeDesc", "jen se objeví"],
  notch: ["appearance.motion.notchDesc", "rozbalí se z proužku"],
  drop: ["appearance.motion.dropDesc", "spadne a jednou se odrazí"],
  unfold: ["appearance.motion.unfoldDesc", "rozvine se, karty naskakují po jedné"],
  liquid: ["appearance.motion.liquidDesc", "pružně se roztáhne ze středu jako Dynamic Island"],
  none: ["appearance.motion.noneDesc", "okamžitě"],
};
/** tvary malé ukázky (výřez 44×8 px nahoře, šuplík s rádiusem 10 px) */
const GEO = {
  collapsed: "inset(0px calc(50% - 22px) calc(100% - 8px) calc(50% - 22px) round 0px 0px 6px 6px)",
  band: "inset(0px 0px calc(100% - 14px) 0px round 0px 0px 10px 10px)",
  full: "inset(0px 0px 0px 0px round 0px 0px 10px 10px)",
};

export default function AppearanceMotion(q: { ap: Ap; reduced: boolean }) {
  const m = () => q.ap.a().motion;
  const stages: Partial<Record<MotionStyle, HTMLSpanElement>> = {};
  const play = (style: MotionStyle) => {
    const el = stages[style];
    if (!el || q.reduced) return;
    el.getAnimations().forEach((x) => x.cancel());
    const plan = openPlan(style, m().speed, GEO);
    if (plan) {
      el.animate(plan.frames, { duration: plan.duration, easing: plan.easing });
      // rozvinutí: karty naskakují po jedné (jako v šuplíku, `motion.ts` revealContent)
      const step = plan.reveal?.step;
      if (step) el.querySelectorAll("i").forEach((c, i) => c.animate([{ opacity: 0, transform: "translateY(-4px)" }, { opacity: 1, transform: "none" }], { duration: 200, delay: 90 + i * step * 2, fill: "backwards" }));
    }
  };
  const choose = (style: MotionStyle) => {
    q.ap.set("motion", { ...m(), style });
    play(style);
  };
  return (
    <div class="card ap-wide" id="f-appearance.motion" classList={{ hit: lit("appearance.motion") }}>
      <h4>{t("appearance.motion.title", "Animace")}</h4>
      <Show when={q.reduced}><p class="ap-note warn">{t("appearance.motion.reduced", "Windows má zapnuté omezení animací — Ledge je proto nepřehrává.")}</p></Show>
      <div class="ap-anims" role="radiogroup" aria-label={t("appearance.motion.style", "Styl")}>
        <For each={MOTION_STYLES}>
          {(style) => (
            <button class="ap-anim" classList={{ on: m().style === style }} role="radio" aria-checked={m().style === style} onClick={() => choose(style)}>
              <span class="ap-astage" aria-hidden="true">
                <i class="ap-anub" />
                <span class="ap-adrawer" ref={(el) => (stages[style] = el)}><i /><i /><i /><i /></span>
              </span>
              <span class="ap-name">{lbl(NAME[style])}</span>
              <small class="ap-desc">{lbl(DESC[style])}</small>
            </button>
          )}
        </For>
      </div>
      <Show when={!q.reduced && m().style === "none"}><p class="ap-note">{t("appearance.motion.noneHint", "Šuplík se ukáže a schová hned, bez vysunutí; Nastavení se rozšíří bez přechodu.")}</p></Show>
      <Show when={m().style !== "none"}>
        <Slider label={t("appearance.motion.speed", "Rychlost")} min={0.5} max={2} step={0.1} value={m().speed} fmt={times}
          set={(v) => q.ap.set("motion", { ...m(), speed: +v.toFixed(1) })} />
      </Show>
    </div>
  );
}
