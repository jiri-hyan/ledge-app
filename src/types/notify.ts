// Zrcadlo `notify::conf::NotifyConfig` (Rust, config `notify`, kolo 26) — KDE a JAK se oznámení ukáže.
// ZDA rozhodují dál vypínače událostí (`claude.toast`, `claude.limitsAlert`, `timer.toast`, `remindToast`).

/** Kam jde oznámení: proužek nahoře (Notch, výchozí) nebo jen Windows toast. */
export type NotifySurface = "notch" | "toast";
/** Choreografie Notche. */
export type NotchStyle = "unfold" | "sides" | "drop" | "ticker" | "glow";
/** Události oznámení (id pro `notify_preview`). */
export type NotifyKind = "waiting" | "done" | "meeting" | "limit" | "timer";

export type NotifyConfig = {
  surface: NotifySurface;
  fallbackToast: boolean;
  /** 2–30 s, výchozí 6 („Hotovo“ 2/3) */
  durationS: number;
  style: Record<NotifyKind, NotchStyle>;
};

/** Výchozí stav (= `NotifyConfig::default` v Rustu). */
export const NOTIFY_DEFAULT: NotifyConfig = {
  surface: "notch",
  fallbackToast: true,
  durationS: 6,
  style: { waiting: "unfold", done: "sides", meeting: "drop", limit: "ticker", timer: "glow" },
};
