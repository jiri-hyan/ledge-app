/** Právě hraje (Rust `media::MediaView`, GSMTC). */
export interface MediaView {
  title: string;
  artist: string;
  /** čitelný název aplikace; "" = neznámá */
  app: string;
  status: "playing" | "paused" | "stopped" | "changing";
  canToggle: boolean;
  canPrev: boolean;
  canNext: boolean;
  /** klíč obalu pro `api.mediaCover`; null = bez obalu */
  coverKey: string | null;
  /** průběh; null = zdroj timeline nedává (lišta se neukáže); chybí jen ve starém jádře */
  timeline?: import("../media/timeline").MediaTimeline | null;
  /** náhodně; null = zdroj nepodporuje (tlačítko se neukáže) */
  shuffle?: boolean | null;
  /** opakovat; null = zdroj nepodporuje */
  repeat?: import("../media/timeline").MediaRepeat | null;
}

/** Hlasitost výchozího výstupu (Rust `volume::VolumeView`). */
export interface VolumeView {
  /** 0–100 */
  level: number;
  muted: boolean;
  /** název zařízení; "" = nevíme */
  device: string;
  /** výchozí mikrofon ztlumený; null / chybí = žádný mikrofon (nebo starší jádro) */
  micMuted?: boolean | null;
  /** aktivní výstupy pro přepínač (`active` = výchozí); chybí = starší jádro */
  devices?: { id: string; name: string; active: boolean }[];
}

/** Baterie (Rust `battery::BatteryView`); jen stroj s baterií. */
export interface BatteryView {
  percent: number;
  charging: boolean;
  plugged: boolean;
  /** odhad Windows v sekundách, jen na baterii; null = Windows neví */
  remainingS: number | null;
}

// --- media: průběh skladby a vzhled karty Právě hraje ---
// Zrcadlo Rust `mediatime::Timeline` / `mediaconf::MediaConfig` (typy a logika v `media/timeline.ts`).
export type { MediaLook, MediaRepeat, MediaStripTime, MediaTimeline, MediaVariant } from "../media/timeline";
/** Akce `api.mediaControl` (Rust `media::Control`). */
export type MediaAction = "toggle" | "next" | "prev" | { seek: number } | { shuffle: boolean } | { repeat: import("../media/timeline").MediaRepeat };
