// Co se s rozpracovanými úpravami rozložení smí stát (čistě, test `keep.test.mjs`): zavření šuplíku
// (odjetí myši, blur, skrytí okna) je NEzahodí — draft i historie Zpět zůstávají v modulu `edit.ts`
// (WebView zůstává teplý) a po znovuotevření lišta připomene, že úpravy čekají. Zahodit je smí jen
// výslovné Zrušit; Esc mimo tah se při změnách nejdřív zeptá.
import { sameLayout, type Layout } from "./model.ts";

/** co udělalo šuplíkem: skrytí / blur nic nezahazuje, Zrušit ano, odchod na jinou stránku taky (výslovná navigace) */
export type EditEvent = "hide" | "blur" | "reopen" | "cancel" | "page-away";

/** Zahodí tahle událost rozpracované úpravy? Jen Zrušit a výslovný odchod ze stránky. */
export const discards = (ev: EditEvent): boolean => ev === "cancel" || ev === "page-away";

/** Liší se rozpracované rozložení od toho, se kterým se začínalo? */
export const isDirty = (before: Layout | null, draft: Layout | null): boolean => !!draft && !!before && !sameLayout(before, draft);

/** Esc mimo tah: beze změn rovnou zrušit, se změnami se zeptat. */
export const escAction = (before: Layout | null, draft: Layout | null): "cancel" | "ask" => (isDirty(before, draft) ? "ask" : "cancel");

/** Po znovuotevření šuplíku s nedokončenými změnami se ukáže připomínka (bez změn není co připomínat). */
export const showResume = (ev: EditEvent, before: Layout | null, draft: Layout | null): boolean => ev === "reopen" && isDirty(before, draft);
