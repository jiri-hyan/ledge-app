// „Připojit: <schůzka>“ v paletě: dnešní události kalendáře, které mají odkaz a ještě neskončily (`calendar/model.ts`
// `joinable` — stejné pravidlo jako tlačítko Připojit na kartě). Otevření jde stejnou cestou jako tlačítko
// (`api.calendarJoin`, ověří Rust). Čistá logika s vloženou akcí — `meet.test.mjs`; zapojení v `live.ts`.
import type { Snapshot } from "../api";
import { joinable, providerName } from "../calendar/model.ts";
import { t } from "../i18n.ts";
import type { PItem } from "./items";

const hhmm = (ms: number) => {
  const d = new Date(ms);
  return `${String(d.getHours()).padStart(2, "0")}:${String(d.getMinutes()).padStart(2, "0")}`;
};

export function meetItems(s: Pick<Snapshot, "personal">, now: number, join: (url: string) => unknown): PItem[] {
  const today = s.personal?.calendar?.days.find((d) => d.kind === "today");
  const out: PItem[] = [];
  for (const it of [...(today?.items ?? [])].sort((a, b) => a.start - b.start)) {
    const j = it.join;
    if (!j || !joinable(it, now)) continue;
    const name = it.title || t("cards.calendar.untitled", "(bez názvu)");
    out.push({
      id: `meet:${it.start}:${j.url}`,
      section: "app",
      order: 1,
      glyph: "📹",
      title: t("palette.meet.join", "Připojit: {name}", { name }),
      sub: [it.allDay ? "" : hhmm(it.start), providerName(j.provider)].filter(Boolean).join(" · "),
      keywords: "schuzka meeting pripojit join teams zoom meet webex jitsi hovor call kalendar",
      run: () => join(j.url),
    });
  }
  return out;
}
