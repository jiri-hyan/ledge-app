// Potvrzení „otázku Clauda je opravdu vidět“ pro pojistku `claude.questionUnseenS` (Rust `claude/unseen.rs`):
// karta je aspoň částí v záběru (IntersectionObserver), šuplík otevřený a WebView viditelné (`drawerShown`).
// Jednou na id; když jádro odmítne (šuplík je podle něj zavřený), zkusí se to znovu při další změně viditelnosti.
// Žádný časovač — jen pozorovatel a události `view` / `visibilitychange`.
import { createEffect, createSignal, onCleanup, onMount } from "solid-js";
import { invoke } from "@tauri-apps/api/core";
import { inTauri } from "../api";
import { drawerShown } from "../media/shown";
import { shouldReportSeen } from "./model";

export function useReportSeen(el: () => HTMLElement | undefined, id: () => number, answerable: () => boolean) {
  const [inView, setInView] = createSignal(false);
  let reported = -1;
  let busy = false;
  const io = typeof IntersectionObserver === "undefined" ? undefined : new IntersectionObserver((es) => setInView(es.some((e) => e.isIntersecting)));
  onMount(() => {
    const e = el();
    if (e && io) io.observe(e);
  });
  onCleanup(() => io?.disconnect());
  createEffect(() => {
    // všechny signály přečíst vždy (Solid sleduje jen to, co efekt opravdu přečetl)
    const cur = id();
    const go = shouldReportSeen({ answerable: answerable(), shown: drawerShown(), inView: inView(), reported: reported === cur });
    if (!inTauri || busy || !go) return;
    busy = true;
    invoke<boolean>("question_seen", { id: cur })
      .then((ok) => {
        if (ok) reported = cur;
      })
      .catch(() => {})
      .finally(() => {
        busy = false;
      });
  });
}
