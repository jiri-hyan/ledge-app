// Osobní: karta „Právě hraje“ (média Windows přes Rust `media.rs`) + hlasitost výchozího výstupu
// (`volume.rs`). Nic nehraje a hlasitost vypnutá / bez zařízení → karta zmizí; s hlasitostí zůstane
// a řekne „Nic nehraje“ (posuvník je pořád k užitku). Ovládání jen klikem / tahem, nic nebere fokus.
import { Show, createEffect, createSignal, onCleanup, onMount, type JSX } from "solid-js";
import { api, type Snapshot } from "../api";
import { sayError } from "../ui/toast";
import { t } from "../i18n";
import { sectionOn } from "../cards/sections";
import { throttle, volumeLevel } from "./model";
import "./media.css";

/** Záložka Osobní je vidět → Rust spustí odběry událostí (líně, jen poprvé). Nic nevykresluje. */
export function PersonalLive() {
  onMount(() => void api.personalLive());
  return null;
}

const svg = { viewBox: "0 0 24 24", fill: "currentColor", "aria-hidden": true } as const;
const IcPrev = () => <svg {...svg}><path d="M6 5h2v14H6zM20 5v14L9.5 12z" /></svg>;
const IcNext = () => <svg {...svg}><path d="M16 5h2v14h-2zM4 5v14l10.5-7z" /></svg>;
const IcPlay = () => <svg {...svg}><path d="M7 4.5v15L19.5 12z" /></svg>;
const IcPause = () => <svg {...svg}><path d="M6.5 5h4v14h-4zM13.5 5h4v14h-4z" /></svg>;
const IcNote = () => <svg {...svg}><path d="M10 4v10.6A3.5 3.5 0 1 0 12 18V8h6V4z" /></svg>;
const stroke = { viewBox: "0 0 24 24", fill: "none", stroke: "currentColor", "stroke-width": 1.8, "stroke-linecap": "round", "stroke-linejoin": "round", "aria-hidden": true } as const;
const SPEAKER = "M4 9.5h3.5L12 5.5v13l-4.5-4H4z";
const VOL_ICON: Record<ReturnType<typeof volumeLevel>, () => JSX.Element> = {
  mute: () => <svg {...stroke}><path d={SPEAKER} /><path d="M16 9.5l5 5M21 9.5l-5 5" /></svg>,
  low: () => <svg {...stroke}><path d={SPEAKER} /><path d="M15.5 10a3 3 0 0 1 0 4" /></svg>,
  mid: () => <svg {...stroke}><path d={SPEAKER} /><path d="M15.5 10a3 3 0 0 1 0 4M18 8a6 6 0 0 1 0 8" /></svg>,
  high: () => <svg {...stroke}><path d={SPEAKER} /><path d="M15.5 10a3 3 0 0 1 0 4M18 8a6 6 0 0 1 0 8M20.5 5.5a9.5 9.5 0 0 1 0 13" /></svg>,
};

function Cover(p: { coverKey: string | null }) {
  const [url, setUrl] = createSignal<string | null>(null);
  createEffect(() => {
    const key = p.coverKey;
    setUrl(null);
    if (!key) return;
    let live = true;
    onCleanup(() => (live = false));
    void api.mediaCover(key).then((u) => live && setUrl(u ?? null), () => {});
  });
  return (
    <div class="med-cover">
      <Show when={url()} fallback={<IcNote />}>{(u) => <img src={u()} alt="" />}</Show>
    </div>
  );
}

function Volume(p: { level: number; muted: boolean; device: string }) {
  // při tahu platí místní hodnota (Rust ji potvrdí událostí; mezitím by posuvník poskakoval)
  const [drag, setDrag] = createSignal<number | null>(null);
  const send = throttle((v: number) => void api.volumeSet(v).catch(sayError), 50);
  const shown = () => drag() ?? p.level;
  const end = () => {
    send.flush();
    setDrag(null);
  };
  const Icon = () => VOL_ICON[volumeLevel(shown(), p.muted)]();
  return (
    <div class="med-vol">
      <button
        class="mini med-ico"
        classList={{ on: p.muted }}
        title={p.muted ? t("cards.sound.unmute", "Zrušit ztlumení") : t("cards.sound.mute", "Ztlumit")}
        aria-label={p.muted ? t("cards.sound.unmute", "Zrušit ztlumení") : t("cards.sound.mute", "Ztlumit")}
        onClick={() => void api.volumeMute(!p.muted).catch(sayError)}
      >
        <Icon />
      </button>
      <input
        class="med-range"
        type="range"
        min="0"
        max="100"
        step="1"
        value={shown()}
        aria-label={t("cards.sound.level", "Hlasitost")}
        title={p.device}
        onPointerDown={() => setDrag(p.level)}
        onInput={(e) => {
          const v = +e.currentTarget.value;
          setDrag(v);
          send.push(v);
        }}
        onChange={end}
        onPointerUp={end}
        onPointerCancel={end}
      />
      <span class="med-pct">{t("cards.sound.pct", "{n} %", { n: shown() })}</span>
    </div>
  );
}

export default function MediaCard(p: { s: Snapshot; menu?: JSX.Element }) {
  const playing = () => sectionOn(p.s.sections, "nowPlaying");
  const m = () => (playing() ? p.s.personal.media : null);
  const vol = () => (sectionOn(p.s.sections, "sound") ? p.s.personal.volume : null);
  const control = (a: "toggle" | "next" | "prev") => void api.mediaControl(a).catch(sayError);
  return (
    <Show when={m() || vol()}>
      <div class="card" data-anim>
        <h4>
          <span class="ic" style="background:var(--violet)" />
          {playing() ? t("cards.media.title", "Právě hraje") : t("cards.sound.title", "Zvuk")}
          <Show when={m()?.app}>{(a) => <span class="n">{a()}</span>}</Show>
          {p.menu}
        </h4>
        <Show when={playing()}>
          <Show when={m()} fallback={<div class="empty">{t("cards.media.none", "Nic nehraje")}</div>}>
            {(mm) => (
              <>
                <div class="med-now">
                  <Cover coverKey={mm().coverKey} />
                  <div class="med-text">
                    <div class="med-title" title={mm().title}>{mm().title || t("cards.media.untitled", "Bez názvu")}</div>
                    <Show when={mm().artist}><div class="sub" title={mm().artist}>{mm().artist}</div></Show>
                  </div>
                </div>
                <div class="med-ctl">
                  <button class="mini med-ico" disabled={!mm().canPrev} title={t("cards.media.prev", "Předchozí")} aria-label={t("cards.media.prev", "Předchozí")} onClick={() => control("prev")}>
                    <IcPrev />
                  </button>
                  <button
                    class="mini med-ico med-play"
                    disabled={!mm().canToggle}
                    title={mm().status === "playing" ? t("cards.media.pause", "Pozastavit") : t("cards.media.play", "Přehrát")}
                    aria-label={mm().status === "playing" ? t("cards.media.pause", "Pozastavit") : t("cards.media.play", "Přehrát")}
                    onClick={() => control("toggle")}
                  >
                    <Show when={mm().status === "playing"} fallback={<IcPlay />}><IcPause /></Show>
                  </button>
                  <button class="mini med-ico" disabled={!mm().canNext} title={t("cards.media.next", "Další")} aria-label={t("cards.media.next", "Další")} onClick={() => control("next")}>
                    <IcNext />
                  </button>
                  <Show when={mm().status === "paused"}><span class="pill n med-state">{t("cards.media.paused", "pauza")}</span></Show>
                </div>
              </>
            )}
          </Show>
        </Show>
        <Show when={vol()}>{(v) => <Volume level={v().level} muted={v().muted} device={v().device} />}</Show>
      </div>
    </Show>
  );
}
