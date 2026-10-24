// Kreslit jen viditelné (kolo 22): graf se vykreslí, až když je jeho obal v zorném poli (IntersectionObserver)
// A šuplík je otevřený (`media/shown.ts` `drawerShown` — se zavřeným šuplíkem nic). Bez časovačů: jen události
// observeru. Bez IntersectionObserveru (starý WebView, test) = viditelné hned, ať graf nikdy tiše nezmizí.
import { createSignal, onCleanup, onMount, type Accessor } from "solid-js";
import { drawerShown } from "../media/shown";

/** `true`, když je prvek (`ref`) vidět v okně a šuplík je otevřený. `always` = bez observeru, jen stav šuplíku
 *  (malé miniatury, které jsou vidět vždy, když je vidět jejich panel). */
export function useVisible(el: () => Element | undefined, opts: { always?: boolean } = {}): Accessor<boolean> {
  const [seen, setSeen] = createSignal(!!opts.always || typeof IntersectionObserver === "undefined");
  onMount(() => {
    const node = el();
    if (opts.always || !node || typeof IntersectionObserver === "undefined") return;
    const io = new IntersectionObserver((entries) => {
      for (const e of entries) setSeen(e.isIntersecting);
    });
    io.observe(node);
    onCleanup(() => io.disconnect());
  });
  return () => seen() && drawerShown();
}
