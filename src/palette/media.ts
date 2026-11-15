// Hudba a hlasitost v paletě (Ctrl+K): Přehrát / Pauza, Další, Předchozí, Hlasitost +10 / −10, Ztlumit a „hlasitost 40“
// (číslo v dotazu). Jen když existují data z jádra (`personal.media` — GSMTC, `personal.volume`); jinak nic.
// Čistá logika s vloženými akcemi (`MediaIO`) — ověřuje ji `media.test.mjs`; zapojení na `api` je v `live.ts`.
import type { Snapshot } from "../api";
import { t } from "../i18n.ts";
import { fold } from "./search.ts";
import { shownStatus } from "../media/model.ts";
import type { PItem } from "./items";

export interface MediaIO {
  control: (a: "toggle" | "next" | "prev") => unknown;
  volume: (level: number) => unknown;
  mute: (muted: boolean) => unknown;
}

const STEP = 10;
const KW = "hudba prehravac media pisnicka skladba music player spotify";
const VOL_KW = "hlasitost zvuk volume vol sound audio";

const clamp = (n: number) => Math.max(0, Math.min(100, Math.round(n)));

/** „hlasitost 40“ / „volume 40 %“ / „vol 40“ → 40 (0–100); jiný dotaz = null. */
export function volumeQuery(q: string): number | null {
  const m = /^(hlasitost|volume|vol|zvuk)\s+(\d{1,3})\s*%?$/.exec(fold(q).trim());
  return m ? clamp(Number(m[2])) : null;
}

export function mediaItems(s: Pick<Snapshot, "personal">, q: string, io: MediaIO): PItem[] {
  const out: PItem[] = [];
  const m = s.personal?.media ?? null;
  const v = s.personal?.volume ?? null;
  const add = (id: string, title: string, keywords: string, glyph: string, run: () => unknown, sub?: string) =>
    out.push({ id: `media:${id}`, section: "app", order: 5, glyph, title, sub, keywords, run });

  if (m && shownStatus(m)) {
    const sub = [m.title, m.artist].filter(Boolean).join(" — ") || m.app;
    if (m.canToggle) {
      const playing = m.status === "playing";
      add("toggle", playing ? t("palette.media.pause", "Pauza") : t("palette.media.play", "Přehrát"), `${KW} play pause pauza prehrat zastavit`, playing ? "⏸" : "▶", () => io.control("toggle"), sub);
    }
    if (m.canNext) add("next", t("palette.media.next", "Další skladba"), `${KW} next dalsi skip preskocit`, "⏭", () => io.control("next"), sub);
    if (m.canPrev) add("prev", t("palette.media.prev", "Předchozí skladba"), `${KW} previous predchozi zpet back`, "⏮", () => io.control("prev"), sub);
  }
  if (v) {
    const dev = v.device || undefined;
    if (v.level < 100) add("up", t("palette.media.up", "Hlasitost +{n}", { n: STEP }), `${VOL_KW} nahlas hlasiteji louder up plus`, "🔊", () => io.volume(clamp(v.level + STEP)), dev);
    if (v.level > 0) add("down", t("palette.media.down", "Hlasitost −{n}", { n: STEP }), `${VOL_KW} ztisit tise quieter down minus`, "🔉", () => io.volume(clamp(v.level - STEP)), dev);
    add("mute", v.muted ? t("palette.media.unmute", "Zapnout zvuk") : t("palette.media.mute", "Ztlumit"), `${VOL_KW} ztlumit mute unmute ticho`, v.muted ? "🔈" : "🔇", () => io.mute(!v.muted), dev);
    const want = volumeQuery(q);
    if (want !== null) add(`set${want}`, t("palette.media.set", "Hlasitost {n} %", { n: want }), `${VOL_KW} ${want}`, "🔊", () => io.volume(want), dev);
  }
  return out;
}
