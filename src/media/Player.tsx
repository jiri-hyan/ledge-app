// Stavebnice karet Právě hraje (vzhledy v MediaCard.tsx): obal (jedna kopie na kartu), lišta průběhu
// s přetáčením, tlačítka přehrávání + náhodně / opakovat (jen když je zdroj umí), ekvalizér (ozdoba).
// Nic nebere fokus; čas tiká jen při přehrávání s otevřeným šuplíkem (`shown.ts`).
import { createEffect, createSignal, onCleanup, Show, type Accessor, type JSX } from "solid-js";
import { api, type MediaAction, type MediaRepeat, type MediaTimeline, type MediaView } from "../api";
import { sayError } from "../ui/toast";
import { t } from "../i18n";
import { drawerShown } from "./shown";
import { fmtTime, fraction, live, nextRepeat, nextTickIn, seekKey, seekTarget, shownPosition } from "./timeline";

const svg = { viewBox: "0 0 24 24", fill: "currentColor", "aria-hidden": true } as const;
const stroke = { viewBox: "0 0 24 24", fill: "none", stroke: "currentColor", "stroke-width": 1.9, "stroke-linecap": "round", "stroke-linejoin": "round", "aria-hidden": true } as const;
export const IcPrev = () => <svg {...svg}><path d="M6 5h2v14H6zM20 5v14L9.5 12z" /></svg>;
export const IcNext = () => <svg {...svg}><path d="M16 5h2v14h-2zM4 5v14l10.5-7z" /></svg>;
export const IcPlay = () => <svg {...svg}><path d="M7 4.5v15L19.5 12z" /></svg>;
export const IcPause = () => <svg {...svg}><path d="M6.5 5h4v14h-4zM13.5 5h4v14h-4z" /></svg>;
export const IcNote = () => <svg {...svg}><path d="M10 4v10.6A3.5 3.5 0 1 0 12 18V8h6V4z" /></svg>;
const IcShuffle = () => <svg {...stroke}><path d="M4 7h3.5c4 0 5 10 9 10H20M4 17h3.5c1.6 0 2.7-1.6 3.6-3.6M13 9.6C13.9 8 15 7 16.5 7H20M17.5 4.5 20 7l-2.5 2.5M17.5 14.5 20 17l-2.5 2.5" /></svg>;
const IcRepeat = () => <svg {...stroke}><path d="M5 11V9.5A2.5 2.5 0 0 1 7.5 7H19M16 4l3 3-3 3M19 13v1.5a2.5 2.5 0 0 1-2.5 2.5H5M8 20l-3-3 3-3" /></svg>;

export const control = (a: MediaAction) => void api.mediaControl(a).catch(sayError);
export const isPlaying = (m: MediaView) => m.status === "playing";

/** Obal na `coverKey` — jedna data URL na kartu (velký obal ji použije dvakrát, ale načte jednou). */
export function useCover(key: Accessor<string | null>): Accessor<string | null> {
  const [url, setUrl] = createSignal<string | null>(null);
  createEffect(() => {
    const k = key();
    setUrl(null);
    if (!k) return;
    let ok = true;
    onCleanup(() => (ok = false));
    void api.mediaCover(k).then((u) => ok && setUrl(u ?? null), () => {});
  });
  return url;
}

/** Čtvercový obal (bez obalu nota). `size`: malý (řádek), velký (velký obal), kulatý štítek desky. */
export function Cover(p: { url: string | null; size?: "sm" | "lg" | "label" }) {
  return (
    <div class="med-cover" classList={{ "med-cover-sm": p.size === "sm", "med-cover-lg": p.size === "lg", "med-cover-label": p.size === "label" }}>
      <Show when={p.url} fallback={<IcNote />}>{(u) => <img src={u()} alt="" draggable={false} />}</Show>
    </div>
  );
}

/** Pozice „teď“ pro lištu i čas: tiká na celé sekundy, jen když hraje a šuplík je vidět. */
function usePosition(m: Accessor<MediaView>, tl: Accessor<MediaTimeline>, seek: Accessor<{ ms: number; at: number } | null>) {
  const [now, setNow] = createSignal(Date.now());
  createEffect(() => {
    const playing = isPlaying(m());
    const line = tl();
    setNow(Date.now());
    if (!live(playing, drawerShown())) return;
    let h: number | undefined;
    const step = () => {
      const n = Date.now();
      setNow(n);
      h = window.setTimeout(step, nextTickIn(shownPosition(line, n, true, seek()), line.rate, true) ?? 1000);
    };
    h = window.setTimeout(step, nextTickIn(shownPosition(line, Date.now(), true, seek()), line.rate, true) ?? 1000);
    onCleanup(() => clearTimeout(h));
  });
  return () => shownPosition(tl(), now(), isPlaying(m()), seek());
}

/**
 * Lišta průběhu: klik / tah = přetočení (jen když to přehrávač umí, jinak jen ukazuje), šipky ±5 s.
 * `times` = čas vlevo a délka vpravo; `thin` = tenká lišta (kompaktní řádek).
 */
export function Progress(p: { m: MediaView; tl: MediaTimeline; times?: boolean; thin?: boolean }) {
  const [seek, setSeek] = createSignal<{ ms: number; at: number } | null>(null);
  const [drag, setDrag] = createSignal<number | null>(null);
  const pos = usePosition(() => p.m, () => p.tl, seek);
  const shown = () => drag() ?? pos();
  let bar!: HTMLDivElement;
  const at = (e: PointerEvent) => {
    const r = bar.getBoundingClientRect();
    return seekTarget(e.clientX - r.left, r.width, p.tl.durationMs);
  };
  const send = (ms: number) => {
    setSeek({ ms, at: Date.now() });
    control({ seek: ms });
  };
  const label = () => t("cards.media.position", "{pos} z {len}", { pos: fmtTime(shown()), len: fmtTime(p.tl.durationMs) });
  return (
    <div class="med-prog" classList={{ "med-prog-thin": p.thin }}>
      <div
        ref={bar}
        class="med-bar"
        classList={{ "med-bar-seek": p.tl.canSeek, "med-bar-drag": drag() != null }}
        role="slider"
        tabIndex={p.tl.canSeek ? 0 : -1}
        aria-label={t("cards.media.progress", "Průběh skladby")}
        aria-valuemin={0}
        aria-valuemax={Math.round(p.tl.durationMs / 1000)}
        aria-valuenow={Math.round(shown() / 1000)}
        aria-valuetext={label()}
        aria-readonly={!p.tl.canSeek}
        title={p.tl.canSeek ? t("cards.media.seek", "Klikni nebo táhni pro přetočení") : label()}
        onPointerDown={(e) => {
          if (!p.tl.canSeek || e.button !== 0) return;
          bar.setPointerCapture(e.pointerId);
          setDrag(at(e));
        }}
        onPointerMove={(e) => drag() != null && setDrag(at(e))}
        onPointerUp={(e) => {
          if (drag() == null) return;
          const to = at(e);
          setDrag(null);
          send(to);
        }}
        onPointerCancel={() => setDrag(null)}
        onKeyDown={(e) => {
          if (!p.tl.canSeek) return;
          const to = seekKey(e.key, shown(), p.tl.durationMs);
          if (to == null) return;
          e.preventDefault();
          send(to);
        }}
      >
        <i class="med-fill" style={{ transform: `scaleX(${fraction(shown(), p.tl.durationMs)})` }} />
      </div>
      <Show when={p.times}>
        <div class="med-times">
          <span>{fmtTime(shown())}</span>
          <span>{fmtTime(p.tl.durationMs)}</span>
        </div>
      </Show>
    </div>
  );
}

/** Jen čas „1:42“ (vzhled Jen v proužku) — tiká stejně jako lišta. */
export function TimeText(p: { m: MediaView; tl: MediaTimeline }) {
  const pos = usePosition(() => p.m, () => p.tl, () => null);
  return <span class="med-clock">{fmtTime(pos())}</span>;
}

function ModeButton(p: { on: boolean; label: string; badge?: string; onClick: () => void; children: JSX.Element }) {
  return (
    <button class="mini med-ico med-mode" classList={{ "med-mode-on": p.on }} aria-pressed={p.on} title={p.label} aria-label={p.label} onClick={p.onClick}>
      {p.children}
      <Show when={p.badge}><b class="med-badge">{p.badge}</b></Show>
    </button>
  );
}

const repeatLabel = (r: MediaRepeat) =>
  r === "track" ? t("cards.media.repeatTrack", "Opakovat skladbu") : r === "list" ? t("cards.media.repeatList", "Opakovat vše") : t("cards.media.repeatOff", "Opakovat: vypnuto");

/**
 * Předchozí / přehrát-pauza / další; `modes` = i náhodně a opakovat (jen když je zdroj hlásí — jinak se
 * tlačítka neukážou).
 */
export function Controls(p: { m: MediaView; modes?: boolean; tight?: boolean }) {
  const playing = () => isPlaying(p.m);
  const playLabel = () => (playing() ? t("cards.media.pause", "Pozastavit") : t("cards.media.play", "Přehrát"));
  return (
    <div class="med-ctl" classList={{ "med-ctl-tight": p.tight }}>
      <Show when={p.modes && typeof p.m.shuffle === "boolean"}>
        <ModeButton on={!!p.m.shuffle} label={p.m.shuffle ? t("cards.media.shuffleOn", "Náhodně: zapnuto") : t("cards.media.shuffleOff", "Náhodně: vypnuto")} onClick={() => control({ shuffle: !p.m.shuffle })}>
          <IcShuffle />
        </ModeButton>
      </Show>
      <button class="mini med-ico" disabled={!p.m.canPrev} title={t("cards.media.prev", "Předchozí")} aria-label={t("cards.media.prev", "Předchozí")} onClick={() => control("prev")}>
        <IcPrev />
      </button>
      <button class="mini med-ico med-play" disabled={!p.m.canToggle} title={playLabel()} aria-label={playLabel()} onClick={() => control("toggle")}>
        <Show when={playing()} fallback={<IcPlay />}><IcPause /></Show>
      </button>
      <button class="mini med-ico" disabled={!p.m.canNext} title={t("cards.media.next", "Další")} aria-label={t("cards.media.next", "Další")} onClick={() => control("next")}>
        <IcNext />
      </button>
      <Show when={p.modes && p.m.repeat}>
        {(r) => (
          <ModeButton on={r() !== "none"} label={repeatLabel(r())} badge={r() === "track" ? "1" : undefined} onClick={() => control({ repeat: nextRepeat(r()) })}>
            <IcRepeat />
          </ModeButton>
        )}
      </Show>
    </div>
  );
}

/** Ekvalizér — jen ozdoba (neukazuje skutečný zvuk); hýbe se jen při přehrávání s otevřeným šuplíkem. */
export function Equalizer(p: { playing: boolean }) {
  return (
    <span class="med-eq" classList={{ "med-eq-live": live(p.playing, drawerShown()) }} aria-hidden="true" title={t("cards.media.eqHint", "Jen ozdoba — neukazuje skutečný zvuk")}>
      <i />
      <i />
      <i />
    </span>
  );
}
