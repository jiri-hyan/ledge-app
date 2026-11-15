// Potvrzení „Hotovo“ (L146) — ostrůvek i dok: Otevřít / ✕ potvrdí událost, `islandEvent` ji dál nevrací, dokud
// tatáž session nedokončí znovu (klíč = session + čas dokončení, `model.ts` `ackDone`). Jen lokální signál, bez příkazu a časovače.
import { createSignal } from "solid-js";
import { ackDone, type DoneAcked } from "./model";
import type { SessionView } from "../types";

const [acked, setAcked] = createSignal<DoneAcked>(new Map());
export const doneAcked = acked;
export const ackDoneSession = (s: Pick<SessionView, "id" | "since">, now: number) => setAcked((a) => ackDone(a, s, now));
