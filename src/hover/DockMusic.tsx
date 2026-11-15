// Slot hudby v docku: obal, název (interpret jen v tooltipu — název má místo, L148), ⏮⏯⏭ a hlasitost. Kolečko nad slotem = ±5 % (`api.volumeSet`), klik na ikonu =
// ztlumit, po kliku do slotu (aktivace okna, `focusDrawer`) šipky: ↑↓ hlasitost, ←→ seek ±10 s. Jen data ze snapshotu;
// při tahu kolečkem drží slot poslední zadanou hodnotu, než dorazí snapshot (jinak by číslo poskakovalo).
import { createEffect, createSignal, on, onCleanup, Show } from "solid-js";
import { api, type MediaView, type VolumeView } from "../api";
import { Controls, Cover, isPlaying, useCover, control } from "../media/Player";
import { sayError } from "../ui/toast";
import { t } from "../i18n";
import { arrowPlan, volumeStep, wheelToStep } from "./model";

const HOLD_MS = 1200;

export default function DockMusic(p: { m: MediaView; vol: VolumeView | null }) {
  const cover = useCover(() => p.m.coverKey);
  const [pend, setPend] = createSignal<number | null>(null);
  let hold: number | undefined;
  const level = () => pend() ?? p.vol?.level ?? 0;
  // snapshot dohnal zadanou hodnotu → zase věřit jádru
  createEffect(on(() => p.vol?.level, (l) => { if (l === pend()) setPend(null); }, { defer: true }));
  onCleanup(() => clearTimeout(hold));

  function setLevel(steps: number) {
    if (!p.vol || steps === 0) return;
    const to = volumeStep(level(), steps);
    setPend(to);
    clearTimeout(hold);
    hold = window.setTimeout(() => setPend(null), HOLD_MS);
    void api.volumeSet(to).catch(sayError);
  }
  const muted = () => !!p.vol?.muted;
  const toggleMute = () => void api.volumeMute(!muted()).catch(sayError);

  return (
    <div
      class="isl-slot isl-music"
      data-noexpand
      tabIndex={0}
      role="group"
      aria-label={t("cards.hover.dock.music", "Hudba")}
      aria-keyshortcuts="ArrowUp ArrowDown ArrowLeft ArrowRight"
      title={p.vol ? t("cards.hover.dock.wheelHint", "Kolečko mění hlasitost · po kliknutí šipky: ↑↓ hlasitost, ←→ přetočení") : undefined}
      onPointerDown={() => void api.focusDrawer()?.catch(() => {})}
      onWheel={(e) => {
        if (!p.vol) return;
        e.preventDefault();
        setLevel(wheelToStep(e.deltaY));
      }}
      onKeyDown={(e) => {
        const plan = arrowPlan(e.key, p.m.timeline, isPlaying(p.m), Date.now());
        if (!plan || (plan.kind === "volume" && !p.vol)) return;
        e.preventDefault();
        if (plan.kind === "volume") setLevel(plan.steps);
        else control({ seek: plan.ms });
      }}
    >
      <Cover url={cover()} size="sm" />
      <span class="isl-tx" title={[p.m.artist, p.m.title].filter(Boolean).join(" — ") || undefined}>
        <b>{p.m.title || t("cards.hover.unknownTrack", "Neznámá skladba")}</b>
      </span>
      <Controls m={p.m} tight />
      <Show when={p.vol}>
        <button
          class="mini isl-vol"
          classList={{ "isl-vol-off": muted() }}
          aria-pressed={muted()}
          title={muted() ? t("cards.hover.dock.unmute", "Zapnout zvuk") : t("cards.hover.dock.mute", "Ztlumit")}
          aria-label={t("cards.hover.dock.muteVol", "Ztlumit, hlasitost {n} %", { n: level() })}
          onClick={toggleMute}
        >
          <span aria-hidden="true">{muted() ? "🔇" : "🔊"}</span><span aria-hidden="true">{muted() ? "" : level()}</span>
        </button>
      </Show>
    </div>
  );
}
