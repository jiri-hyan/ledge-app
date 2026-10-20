// Jednorázový tip „Podrž kartu a přetáhni ji“ (kolo 10): při prvním najetí na hlavičku karty (až po prohlídce
// `Tour.tsx`, jinak by se bubliny pletly) vyskočí malá bublina u hlavičky; „Rozumím“, uplynutí 9 s nebo první
// tah podržením ji zavře a zapíše `ui.tips` (`tips.ts` `HOLD_TIP`) → už nikdy znovu. Žádný modal, nic
// nebere fokus. V klidu jediný posluchač `pointerover`, a to jen dokud tip není vyřízený; config se čte
// nejvýš jednou za pár sekund a jen při najetí na hlavičku.
import { createEffect, createSignal, on as track, onCleanup, onMount, Show } from "solid-js";
import { Portal } from "solid-js/web";
import { api } from "../api";
import { wizardOpen } from "../onboarding/state";
import { sayError } from "../ui/toast";
import { HEADER_SELECTOR } from "./dnd";
import { editing, holdDrags } from "./edit";
import { bubblePos, HOLD_TEXT, HOLD_TIP, seen, shouldShowHold, withTip } from "./tips";
import { tx } from "./text";
import "./tour.css";

const SHOW_MS = 9000;
const RECHECK_MS = 4000;
/** tip už je vyřízený (viděl ho / použil podržení) — platí do restartu WebView, další čtení configu netřeba */
let spent = false;

async function markSeen(cfgError: string | null) {
  spent = true;
  if (cfgError) return;
  try {
    const cfg = await api.getConfig();
    if (cfg && !seen(cfg, HOLD_TIP)) await api.saveConfig(withTip(cfg, HOLD_TIP));
  } catch (e) {
    sayError(e);
  }
}

export default function HoldTip(p: { cfgError: string | null }) {
  const [target, setTarget] = createSignal<HTMLElement | null>(null);
  const [pos, setPos] = createSignal<ReturnType<typeof bubblePos> | null>(null);
  let bubble: HTMLDivElement | undefined;
  let timer: number | undefined;
  let checking = false;
  let lastCheck = 0;

  const close = () => {
    clearTimeout(timer);
    setTarget(null);
    void markSeen(p.cfgError);
  };
  const place = () => {
    const el = target();
    if (!el || !bubble) return setPos(null);
    const r = el.getBoundingClientRect();
    const b = bubble.getBoundingClientRect();
    setPos(bubblePos({ left: r.left, top: r.top, width: r.width, height: r.height }, b.width, b.height, window.innerWidth, window.innerHeight));
  };
  const over = (e: PointerEvent) => {
    if (spent) return document.removeEventListener("pointerover", over);
    if (checking || editing() || target() || Date.now() - lastCheck < RECHECK_MS) return;
    const head = (e.target as Element | null)?.closest<HTMLElement>(HEADER_SELECTOR);
    if (!head || document.querySelector(".tour-bubble")) return;
    checking = true;
    lastCheck = Date.now();
    api
      .getConfig()
      .then((cfg) => {
        if (cfg && seen(cfg, HOLD_TIP)) spent = true;
        else if (shouldShowHold(cfg, wizardOpen(), p.cfgError) && head.isConnected) {
          setTarget(head);
          requestAnimationFrame(place);
          timer = window.setTimeout(close, SHOW_MS);
        }
      })
      .catch(() => {})
      .finally(() => (checking = false));
  };

  onMount(() => {
    if (!spent) document.addEventListener("pointerover", over);
    onCleanup(() => {
      document.removeEventListener("pointerover", over);
      clearTimeout(timer);
    });
  });
  // uživatel to objevil sám (tah podržením, tužka v záložkách, ✎) → tip už nepotřebuje
  createEffect(track(holdDrags, () => (target() ? close() : void (spent || markSeen(p.cfgError))), { defer: true }));
  createEffect(track(editing, (e) => e && target() && close(), { defer: true }));

  return (
    <Show when={target()}>
      <Portal mount={document.querySelector(".shell") ?? document.body}>
        <div
          class="tour-bubble"
          classList={{ below: pos()?.below !== false, above: pos()?.below === false }}
          role="status"
          aria-live="polite"
          ref={bubble}
          style={{ left: `${pos()?.left ?? -9999}px`, top: `${pos()?.top ?? -9999}px`, "--arrow": `${pos()?.arrow ?? 24}px` }}
        >
          <div class="tour-head">
            <b>{tx(HOLD_TEXT.title)}</b>
          </div>
          <p>{tx(HOLD_TEXT.body)}</p>
          <div class="tour-btns">
            <button class="mini ok" onClick={close}>{tx(HOLD_TEXT.ok)}</button>
          </div>
        </div>
      </Portal>
    </Show>
  );
}
