// Jediný vzorek `system_view` pro kartu Systém i lištu Systém (notch/SecSystem): obě čtou stejný signál a smyčka
// po 2 s (`util/poll.ts`) běží jednou, dokud je aspoň jedna z nich v šuplíku vidět. Poslední vzorek přežije
// přemontování (přepnutí záložky) — jinak by karta bliknula prázdná.
import { createSignal } from "solid-js";
import { sharedPoll } from "../util/poll";
import { pollWhileShown } from "../util/pollShown";
import { intApi, type SystemView } from "./api";

/** Odstup vzorků (Rust nevezme dva vzorky blíž než 1,5 s). */
const EVERY_MS = 2000;

const [view, setView] = createSignal<SystemView | null>(null);
const poll = sharedPoll(() => intApi.systemView(), setView, EVERY_MS);

/** Odebírá vzorky systému (jen s otevřeným šuplíkem) a vrací poslední. Volej v těle komponenty. */
export function useSystemView() {
  pollWhileShown(poll);
  return view;
}
