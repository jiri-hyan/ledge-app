// Je šuplík vidět? Pro karty Právě hraje: tikání času, točení desky a ekvalizér běží jen s otevřeným
// šuplíkem (se zavřeným nic — idle CPU). Čte stejné události jako skořápka (`view` / `closing` v App.tsx)
// plus `visibilitychange` WebView2; posluchače vzniknou až s první kartou, která se ptá.
import { createRoot, createSignal } from "solid-js";
import { on, type ViewMsg } from "../api";

let read: (() => boolean) | null = null;

function start(): () => boolean {
  return createRoot(() => {
    // karta se vykresluje = šuplík je (skoro jistě) otevřený; další `view` / `closing` to upřesní
    const [view, setView] = createSignal(true);
    const [visible, setVisible] = createSignal(typeof document === "undefined" || document.visibilityState !== "hidden");
    void on<ViewMsg>("view", (m) => setView(m.view !== "hidden"));
    void on<unknown>("closing", () => setView(false));
    if (typeof document !== "undefined") document.addEventListener("visibilitychange", () => setVisible(document.visibilityState !== "hidden"));
    return () => view() && visible();
  });
}

/** true = šuplík je otevřený a WebView není skryté. */
export function drawerShown(): boolean {
  read ??= start();
  return read();
}
