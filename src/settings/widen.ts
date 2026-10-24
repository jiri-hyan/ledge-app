// Širší šuplík pro Nastavení: stránka si řekne o šířku (`wantWidth` → `report_size`), Rust ji ořízne
// na min(žádost, 90 % monitoru) (`drawerfit.rs`). Přechod: rozšíření = clip-path z původního tvaru
// na celé okno po `resize`; zúžení = clip-path se stáhne na původní šířku a teprve pak se okno zmenší
// (`holdWide`). Pohyb podle `appearance.motion` (`widenMotion`): styl „Bez animací“ i systémové
// `prefers-reduced-motion` = okamžitě bez přechodu, jinak délky / rychlost a křivka stylu; rozšíření
// pružinou `spring()` (ta styl a rychlost respektuje sama). Používá App (`report`) a Settings.
import { createSignal } from "solid-js";
import { OVERLAP, spring, collapsedClip, FULL_CLIP, motionNow, shellRadius } from "../motion";
import { widenMotion } from "../theme/appearance";

/** Šířka Nastavení (kolo 21 větší: postranní sekce + stránka + náhled dole vedle sebe). */
export const SETTINGS_WIDE = 1280;
const [holdWide, setHoldWide] = createSignal(false);
export { holdWide };

/** Šířka, o kterou si stránka řekne (0 = běžný šuplík podle configu). */
export const wantWidth = (page: string) => (page === "settings" || holdWide() ? SETTINGS_WIDE : 0);

const reduced = () => window.matchMedia("(prefers-reduced-motion: reduce)").matches;
/** Plán přechodu teď (`null` = bez přechodu). */
const plan = () => widenMotion(motionNow().style, motionNow().speed, reduced());
const shell = () => document.querySelector<HTMLElement>(".shell");
/** šířka okna před rozšířením — na ni se zužuje */
let narrowW = 0;

/** Po vstupu do Nastavení: až okno povyroste, rozvinout tvar z původní velikosti. */
export function widenIn() {
  const el = shell();
  narrowW = window.innerWidth;
  if (!el || !plan()) return;
  const fromW = window.innerWidth;
  const fromH = window.innerHeight - OVERLAP;
  const onResize = () => {
    stop();
    if (window.innerWidth <= fromW + 1) return;
    const sp = spring(260, 26);
    if (sp.duration <= 0) return;
    el.animate([{ clipPath: collapsedClip(fromW, fromH, shellRadius()) }, { clipPath: FULL_CLIP }], { duration: sp.duration, easing: sp.easing });
  };
  const timer = window.setTimeout(() => stop(), 700);
  const stop = () => {
    clearTimeout(timer);
    window.removeEventListener("resize", onResize);
  };
  window.addEventListener("resize", onResize);
}

/** Odchod z Nastavení: stáhnout tvar na původní šířku, pak teprve zmenšit okno. */
export function narrowOut() {
  const el = shell();
  const content = el?.firstElementChild as HTMLElement | null;
  // šuplík se právě zavírá (běží/drží animace zavření) nebo není kam zužovat → rovnou
  const m = plan();
  if (!el || !content || !m || !narrowW || el.getAnimations().length > 0 || window.innerWidth <= narrowW + 1) return;
  const h = window.innerHeight - OVERLAP;
  setHoldWide(true);
  const fade = content.animate([{ opacity: 1 }, { opacity: 0 }], { duration: m.fadeMs, fill: "forwards" });
  const shrink = el.animate([{ clipPath: FULL_CLIP }, { clipPath: collapsedClip(narrowW, h, shellRadius()) }], {
    duration: m.shrinkMs,
    easing: m.easing,
    fill: "forwards",
  });
  let done = false;
  const finish = () => {
    if (done) return;
    done = true;
    window.removeEventListener("resize", finish);
    shrink.cancel();
    fade.cancel();
    content.animate([{ opacity: 0 }, { opacity: 1 }], { duration: m.restoreMs });
  };
  shrink.finished.then(
    () => {
      window.addEventListener("resize", finish);
      window.setTimeout(finish, 400);
      setHoldWide(false);
    },
    () => {
      setHoldWide(false);
      finish();
    },
  );
}
