// Sdílené hodiny šuplíku: `now` tiká 1 Hz, dokud je šuplík vidět (smyčku drží `App.tsx`, zavřený = stojí). Čtou ho jen
// místa, která opravdu ukazují sekundy (hlavička, časovač, „pracuje 1:23“, odpočet otázky Clauda). Co ukazuje nejvýš
// minuty, bere `nowMin` / `minuteOf` (`util/time.ts`) — jinak by se přepočítávalo 60× víc, než je vidět.
import { createSignal } from "solid-js";

export const [now, setNow] = createSignal(Date.now());
