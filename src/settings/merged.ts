// Sloučené volby Nastavení (kolo 33, L109): jeden Select místo několika přepínačů. Config na disku se NEMĚNÍ
// (starý config se čte beze změny chování, Rust čte dál tytéž klíče) — tady je jen mapování config ⇄ volba v UI.
// Čisté funkce, test `merged.test.mjs`.
import type { NotchStyle, NotifyKind } from "../types/notify";

/** Volba oznámení: „off“ = vypnuto, „on“ = zapnuto (když se ukazuje jen Windows toast, styl nemá smysl), jinak styl Notche. */
export type NotifyChoice = NotchStyle | "off" | "on";

const STYLE_DEFAULT: Record<NotifyKind, NotchStyle> = { waiting: "unfold", done: "sides", meeting: "drop", limit: "ticker", timer: "glow" };

/** Cesta vypínače události (meeting žádný nemá — zapíná se u každého kalendáře zvlášť). */
export const NOTIFY_SWITCH: Record<NotifyKind, string | null> = {
  waiting: "claude.toast",
  done: "claude.toastDone",
  meeting: null,
  limit: "claude.limitsAlert",
  timer: "timer.toast",
};

/** Hlásí se událost? (null = rozhoduje každý kalendář zvlášť). Výchozí hodnoty jako v Rustu: `toastDone` bez hodnoty
 *  se řídí „Claude čeká“, `timer.toast` je ve výchozím stavu zapnutý. */
export function notifyOn(cfg: any, k: NotifyKind): boolean | null {
  switch (k) {
    case "waiting":
      return !!cfg?.claude?.toast;
    case "done":
      return cfg?.claude?.toastDone ?? !!cfg?.claude?.toast;
    case "limit":
      return !!cfg?.claude?.limitsAlert;
    case "timer":
      return cfg?.timer?.toast !== false;
    case "meeting":
      return null;
  }
}

/** Co ukáže Select události: vypnuto → „off“; zapnuto → styl (v režimu Notch) nebo „on“ (jen toast Windows). */
export function notifyChoice(cfg: any, k: NotifyKind, notch: boolean): NotifyChoice {
  if (notifyOn(cfg, k) === false) return "off";
  if (!notch) return "on";
  return (cfg?.notify?.style?.[k] as NotchStyle | undefined) ?? STYLE_DEFAULT[k];
}

/** Zápisy do configu pro zvolenou hodnotu: vypínač + (u stylu) styl; vypnutí styl neztratí. */
export function notifyWrites(k: NotifyKind, v: NotifyChoice): [string, unknown][] {
  const sw = NOTIFY_SWITCH[k];
  const out: [string, unknown][] = [];
  if (sw) out.push([sw, v !== "off"]);
  if (v !== "off" && v !== "on") out.push([`notify.style.${k}`, v]);
  return out;
}

/** Zdroj limitů z aplikace / dotazu na Anthropic (dřív dva přepínače `claude.limitsDesktop` a `claude.limitsFetch`). */
export type LimitsSource = "none" | "desktop" | "fetch" | "both";

export function limitsSource(cfg: any): LimitsSource {
  const d = !!cfg?.claude?.limitsDesktop;
  const f = !!cfg?.claude?.limitsFetch;
  return d && f ? "both" : d ? "desktop" : f ? "fetch" : "none";
}

export function limitsWrites(v: LimitsSource): [string, boolean][] {
  return [
    ["claude.limitsDesktop", v === "desktop" || v === "both"],
    ["claude.limitsFetch", v === "fetch" || v === "both"],
  ];
}
