// Položky palety, které závisí na dotazu nebo na živých datech a jdou přes jádro: hudba a hlasitost (`media.ts`),
// „Připojit: <schůzka>“ (`meet.ts`), Povolit / Zamítnout Clauda (`decide.ts`). Čistá logika je v modulech
// (testy bez IPC); tady je jen zapojení na `api` — stejné volání jako tlačítka karet (`media/Player.tsx`, `CalendarCard`,
// `cards/Peek.tsx`).
import { api, type Snapshot } from "../api";
import { say, sayError } from "../ui/toast";
import { t } from "../i18n";
import { decideItems } from "./decide";
import { mediaItems } from "./media";
import { meetItems } from "./meet";
import type { PItem } from "./items";

const fail = (p: unknown) => void Promise.resolve(p).catch(sayError);

async function decide(id: number, allow: boolean) {
  try {
    if (!(await api.decide(id, allow))) say(t("sessions.perm.gone", "Žádost už nečeká — vyřídila se jinde nebo vypršela."), true);
  } catch (e) {
    sayError(e);
  }
}

export function liveItems(s: Snapshot, q: string, now = Date.now()): PItem[] {
  return [
    ...mediaItems(s, q, { control: (a) => fail(api.mediaControl(a)), volume: (l) => fail(api.volumeSet(l)), mute: (m) => fail(api.volumeMute(m)) }),
    ...meetItems(s, now, (url) => fail(api.calendarJoin(url))),
    ...decideItems(s, q, (id, allow) => void decide(id, allow)),
  ];
}
