// Příkazy časovače v paletě (Ctrl+K): „Časovač N min“ pro každou předvolbu, pauza / pokračovat / +5 min /
// zrušit podle stavu odpočtu, stopky start / pauza / vynulovat. Akce jdou přes `timer/TimerCard.timerDo`
// (Rust `timer.rs`); paleta se po akci zavře jako u ostatních příkazů appky.
import type { Snapshot } from "../api";
import { t } from "../i18n";
import { say } from "../ui/toast";
import { timerDo } from "../timer/TimerCard";
import { presetsOf, type TimerOp } from "../timer/model";
import type { PItem } from "./items";

const KW = "casovac timer odpocet countdown focus fokus soustredeni pomodoro minuty";
const SW_KW = "stopky stopwatch mereni casu";

export function timerItems(s: Snapshot): PItem[] {
  const out: PItem[] = [];
  const cd = s.timer?.countdown ?? null;
  const sw = s.timer?.stopwatch ?? null;
  const add = (id: string, title: string, keywords: string, op: TimerOp, ok: string) =>
    out.push({ id: `timer:${id}`, section: "app", order: 5, glyph: "⏱", title, keywords, run: () => timerDo(op).then((r) => r && say(ok)) });

  for (const m of presetsOf(s.timer)) add(`start${m}`, t("palette.timer.start", "Časovač {m} min", { m }), KW, { op: "start", minutes: m }, t("palette.timer.started", "Časovač {m} min běží", { m }));
  if (cd?.state === "running") add("pause", t("palette.timer.pause", "Pozastavit časovač"), KW, { op: "pause" }, t("palette.timer.paused", "Časovač pozastaven"));
  if (cd?.state === "paused") add("resume", t("palette.timer.resume", "Pokračovat v časovači"), KW, { op: "resume" }, t("palette.timer.resumed", "Časovač pokračuje"));
  if (cd && cd.state !== "done") add("plus5", t("palette.timer.plus5", "Časovač +5 min"), KW, { op: "add", minutes: 5 }, t("palette.timer.added", "Přidáno 5 min"));
  if (cd) add("cancel", t("palette.timer.cancel", "Zrušit časovač"), KW, { op: "cancel" }, t("palette.timer.cancelled", "Časovač zrušen"));
  if (sw?.state === "running") add("swPause", t("palette.timer.swPause", "Pozastavit stopky"), SW_KW, { op: "swPause" }, t("palette.timer.swPaused", "Stopky pozastaveny"));
  else add("swStart", sw ? t("palette.timer.swResume", "Pokračovat ve stopkách") : t("palette.timer.sw", "Stopky"), SW_KW, { op: "swStart" }, t("palette.timer.swStarted", "Stopky běží"));
  if (sw) add("swReset", t("palette.timer.swReset", "Vynulovat stopky"), SW_KW, { op: "swReset" }, t("palette.timer.swDone", "Stopky vynulovány"));
  return out;
}
