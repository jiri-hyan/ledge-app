// Krátká nenásilná prohlídka při prvním otevření šuplíku (nový uživatel po průvodci, stávající jednou po
// aktualizaci): 3 bubliny u ✎ (Uprav si šuplík), ⋯ první karty (Každá karta má menu) a 🔍 (Ctrl+K — Najdi
// cokoli). Žádný modal, nic nebere fokus (jen tlačítka na klik), Další / Přeskočit; dokončení i přeskočení
// zapíše `ui.tips` (`tips.ts` `withTip`) → už nikdy znovu. Šuplík zavřený = bublina pryč, dokončí se příště.
// Viditelnost šuplíku z událostí `view` / `closing` (jako skořápka), žádné časovače kromě jednoho odkladu
// po otevření (morf doběhne).
import { createEffect, createSignal, on as track, onCleanup, Show } from "solid-js";
import { Portal } from "solid-js/web";
import { api, on, type ViewMsg } from "../api";
import { t } from "../i18n";
import { wizardOpen } from "../onboarding/state";
import { sayError } from "../ui/toast";
import { bubblePos, shouldShow, STEPS, withTip, type Step } from "./tips";
import { tx } from "./text";
import "./tour.css";

const START_MS = 700;
const [open, setOpen] = createSignal(false);
void on<ViewMsg>("view", (m) => setOpen(m.view === "full"));
void on<ViewMsg>("closing", () => setOpen(false));

/** config jednou za život WebView (jako `onboarding/state.ts`); po dokončení už nic */
const [pending, setPending] = createSignal<boolean | null>(null);
let asked = false;

function target(s: Step): HTMLElement | null {
  if (s.target === "menu") return document.querySelector<HTMLElement>(".deck .card-menu-btn");
  return document.querySelector<HTMLElement>(`[data-tip="${s.target}"]`);
}

async function done(cfgError: string | null) {
  setPending(false);
  if (cfgError) return;
  try {
    const cfg = await api.getConfig();
    if (cfg) await api.saveConfig(withTip(cfg));
  } catch (e) {
    sayError(e);
  }
}

export default function Tour(p: { cfgError: string | null }) {
  const [step, setStep] = createSignal(-1);
  const [pos, setPos] = createSignal<ReturnType<typeof bubblePos> | null>(null);
  let bubble: HTMLDivElement | undefined;
  let lit: HTMLElement | null = null;
  let timer: number | undefined;

  if (!asked) {
    asked = true;
    api.getConfig().then((cfg) => setPending(shouldShow(cfg, wizardOpen(), p.cfgError))).catch(() => setPending(false));
  }

  const light = (el: HTMLElement | null) => {
    lit?.classList.remove("tour-target");
    lit = el;
    lit?.classList.add("tour-target");
  };
  /** první krok s cílem od `from` (chybějící cíl — např. žádná karta — se přeskočí) */
  const go = (from: number) => {
    let i = from;
    while (i < STEPS.length && !target(STEPS[i])) i++;
    if (i >= STEPS.length) {
      light(null);
      setStep(-1);
      void done(p.cfgError);
      return;
    }
    setStep(i);
    requestAnimationFrame(place);
  };
  const place = () => {
    const s = STEPS[step()];
    const el = s ? target(s) : null;
    if (!el || !bubble) return setPos(null);
    light(el);
    const r = el.getBoundingClientRect();
    const b = bubble.getBoundingClientRect();
    setPos(bubblePos({ left: r.left, top: r.top, width: r.width, height: r.height }, b.width, b.height, window.innerWidth, window.innerHeight));
  };
  const skip = () => {
    light(null);
    setStep(-1);
    void done(p.cfgError);
  };

  // šuplík otevřený + prohlídka čeká + průvodce zavřený → po doběhnutí morfu začít; zavřený = schovat
  createEffect(
    track([open, pending, wizardOpen], ([o, pend, wiz]) => {
      clearTimeout(timer);
      if (!o || !pend || wiz) {
        light(null);
        setStep(-1);
        return;
      }
      timer = window.setTimeout(() => step() < 0 && go(0), START_MS);
    }),
  );
  window.addEventListener("resize", place);
  onCleanup(() => {
    clearTimeout(timer);
    window.removeEventListener("resize", place);
    light(null);
  });

  const cur = () => STEPS[step()];
  return (
    <Show when={step() >= 0 && cur()}>
      {(s) => (
        <Portal mount={document.querySelector(".shell") ?? document.body}>
          <div
            class="tour-bubble"
            classList={{ below: pos()?.below !== false, above: pos()?.below === false }}
            role="dialog"
            aria-live="polite"
            aria-label={tx(s().title)}
            ref={bubble}
            style={{ left: `${pos()?.left ?? -9999}px`, top: `${pos()?.top ?? -9999}px`, "--arrow": `${pos()?.arrow ?? 24}px` }}
          >
            <div class="tour-head">
              <b>{tx(s().title)}</b>
              <span class="tour-n">{t("cards.tour.step", "{i} / {n}", { i: step() + 1, n: STEPS.length })}</span>
            </div>
            <p>{tx(s().body)}</p>
            <div class="tour-btns">
              <button class="mini" onClick={skip}>{t("cards.tour.skip", "Přeskočit")}</button>
              <button class="mini ok" onClick={() => go(step() + 1)}>{step() + 1 < STEPS.length ? t("cards.tour.next", "Další") : t("cards.tour.done", "Hotovo")}</button>
            </div>
          </div>
        </Portal>
      )}
    </Show>
  );
}
