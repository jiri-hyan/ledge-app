// Sekce lišty notche „Přehrávač“ a „Hlasitost“ (kolo 27). Přehrávač: Obal s textem (dnešní) · Jen ovládání ·
// Běžící text (CSS animace jen s otevřeným šuplíkem a bez omezení pohybu — `prefers-reduced-motion` / pohyb
// „none“). Hlasitost: Svislý (dnešní) a Vodorovný posuvník · Otočné kolečko (kolečko myši / šipky) · Sloupky
// rozsvícené podle úrovně (statické — skutečné spektrum Ledge nemá, žádná falešná animace) · Ikona s číslem.
// Data jen ze snapshotu (Právě hraje, hlasitost); bez dat prázdný stav. Fokus nekrade (jen klik).
import { For, Match, Show, Switch } from "solid-js";
import { api, type Snapshot } from "../api";
import { t } from "../i18n";
import { sayError } from "../ui/toast";
import { drawerShown } from "../media/shown";
import { Cover, IcNext, IcPause, IcPlay, IcPrev, control, isPlaying, useCover } from "../media/Player";
import { throttle } from "../media/model";
import { motionNow } from "../motion";
import { reducedMotion } from "../layout/flip";
import { volumeOf } from "./model";
import { eqLit, knobAngle, nudge, solidOf } from "./look";
import type { BarSection } from "./bar";

function Controls(p: { m: NonNullable<Snapshot["personal"]["media"]>; big?: boolean }) {
  return (
    <div class="nb-ctl" classList={{ "nb-ctl-big": p.big }}>
      <button class="nb-btn" disabled={!p.m.canPrev} aria-label={t("cards.notch.prev", "Předchozí")} onClick={() => control("prev")}><IcPrev /></button>
      <button class="nb-btn nb-main" disabled={!p.m.canToggle} aria-label={isPlaying(p.m) ? t("cards.notch.pause", "Pozastavit") : t("cards.notch.play", "Přehrát")} onClick={() => control("toggle")}>
        <Show when={isPlaying(p.m)} fallback={<IcPlay />}><IcPause /></Show>
      </button>
      <button class="nb-btn" disabled={!p.m.canNext} aria-label={t("cards.notch.next", "Další")} onClick={() => control("next")}><IcNext /></button>
    </div>
  );
}

export function Player(p: { s: Snapshot; sec: BarSection; design: string }) {
  const m = () => p.s.personal.media;
  const cover = useCover(() => (p.design === "cover" ? m()?.coverKey ?? null : null));
  const title = () => m()?.title || t("cards.notch.untitled", "Bez názvu");
  // běžící text: jen viditelný šuplík a bez omezení pohybu (jinak stojí a ořízne se třemi tečkami)
  const run = () => drawerShown() && !reducedMotion() && motionNow().style !== "none";
  return (
    <section class="nb-sec nb-player" data-design={p.design} style={p.sec.color ? { "--nb-c": solidOf(p.sec.color) } : undefined} aria-label={t("cards.media.title", "Právě hraje")}>
      <Show when={m()} fallback={<small class="nb-empty">{t("cards.notch.nothing", "Nic nehraje")}</small>}>
        {(mv) => (
          <Switch>
            <Match when={p.design === "controls"}>
              <div class="nb-pl nb-plc">
                <b class="nb-title">{title()}</b>
                <small class="nb-by">{mv().artist}</small>
                <Controls m={mv()} big />
              </div>
            </Match>
            <Match when={p.design === "marquee"}>
              <div class="nb-pl nb-plm">
                <div class="nb-marq" classList={{ run: run() }} title={[mv().title, mv().artist].filter(Boolean).join(" · ")}>
                  <span>{title()}<Show when={mv().artist}> · {mv().artist}</Show></span>
                  <Show when={run()}><span aria-hidden="true">{title()}<Show when={mv().artist}> · {mv().artist}</Show></span></Show>
                </div>
                <Controls m={mv()} />
              </div>
            </Match>
            <Match when={true}>
              <span class="nb-cover"><Cover url={cover()} /></span>
              <div class="nb-pl">
                <b class="nb-title">{title()}</b>
                <small class="nb-by">{mv().artist}</small>
                <Controls m={mv()} />
              </div>
            </Match>
          </Switch>
        )}
      </Show>
    </section>
  );
}

const EQ_BARS = 10;

function MuteBtn(p: { muted: boolean }) {
  return (
    <button class="nb-btn nb-mute" aria-pressed={p.muted} aria-label={p.muted ? t("cards.notch.unmute", "Zapnout zvuk") : t("cards.notch.mute", "Ztlumit")} onClick={() => void api.volumeMute(!p.muted).catch(sayError)}>
      <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
        <path d="M4 9.5h3.5L12 5.5v13l-4.5-4H4z" />
        <Show when={p.muted} fallback={<path d="M15.5 10a3 3 0 0 1 0 4M18 8a6 6 0 0 1 0 8" />}><path d="M16 9.5l5 5M21 9.5l-5 5" /></Show>
      </svg>
    </button>
  );
}

export function Volume(p: { s: Snapshot; sec: BarSection; design: string }) {
  const v = () => p.s.personal.volume;
  const send = throttle((x: number) => void api.volumeSet(x).catch(sayError), 50);
  const level = () => (v()?.muted ? 0 : v()?.level ?? 0);
  const c = () => solidOf(p.sec.color, level());
  const set = (x: number | null) => x !== null && send.push(x);
  /** kolečko myši a šipky (kolečko, ikona, sloupky) — jen nad prvkem, fokus se nebere */
  const wheel = (e: WheelEvent) => {
    e.preventDefault();
    set(nudge(level(), -Math.sign(e.deltaY)));
  };
  const key = (e: KeyboardEvent) => {
    const dir = e.key === "ArrowUp" || e.key === "ArrowRight" ? 1 : e.key === "ArrowDown" || e.key === "ArrowLeft" ? -1 : 0;
    if (!dir) return;
    e.preventDefault();
    set(nudge(level(), dir));
  };
  const range = (vertical: boolean) => (
    <input
      class="nb-slider"
      classList={{ "nb-hslider": !vertical }}
      type="range"
      min="0"
      max="100"
      step="1"
      style={{ "accent-color": c() }}
      value={level()}
      aria-label={t("cards.notch.volume", "Hlasitost")}
      onInput={(e) => set(volumeOf(e.currentTarget.value))}
    />
  );
  const slider = (label: string) => ({ role: "slider", tabindex: 0, "aria-label": label, "aria-valuemin": 0, "aria-valuemax": 100, "aria-valuenow": level(), onWheel: wheel, onKeyDown: key }) as const;
  return (
    <section class="nb-sec nb-vol" data-design={p.design} aria-label={t("cards.sound.title", "Zvuk")}>
      <Show when={v()} fallback={<small class="nb-empty">{t("cards.notch.noDevice", "Bez zvukového zařízení")}</small>}>
        {(vol) => (
          <Switch>
            <Match when={p.design === "hslider"}>
              <div class="nb-vrow">{range(false)}<b class="nb-vnum">{level()}</b></div>
              <MuteBtn muted={vol().muted} />
            </Match>
            <Match when={p.design === "knob"}>
              <svg class="nb-knob" viewBox="0 0 60 60" {...slider(t("cards.notch.volume", "Hlasitost"))}>
                <circle cx="30" cy="30" r="24" class="nb-knobface" />
                <line x1="30" y1="30" x2="30" y2="11" stroke-width="3.5" stroke-linecap="round" style={{ stroke: c() }} transform={`rotate(${knobAngle(level())} 30 30)`} />
                <text x="30" y="56" text-anchor="middle" class="nb-knobv">{level()}</text>
              </svg>
              <MuteBtn muted={vol().muted} />
            </Match>
            <Match when={p.design === "eq"}>
              <div class="nb-eq" {...slider(t("cards.notch.volume", "Hlasitost"))}>
                <For each={Array.from({ length: EQ_BARS }, (_, i) => i)}>
                  {(i) => (
                    <i
                      classList={{ on: i < eqLit(level(), EQ_BARS) }}
                      style={{ height: `${30 + (i * 70) / (EQ_BARS - 1)}%`, background: i < eqLit(level(), EQ_BARS) ? c() : undefined }}
                      onClick={() => set((i + 1) * (100 / EQ_BARS))}
                    />
                  )}
                </For>
              </div>
              <MuteBtn muted={vol().muted} />
            </Match>
            <Match when={p.design === "icon"}>
              <div class="nb-vicon" {...slider(t("cards.notch.volume", "Hlasitost"))}>
                <MuteBtn muted={vol().muted} />
                <b class="nb-vbig" style={{ color: p.sec.color ? c() : undefined }}>{level()}</b>
              </div>
            </Match>
            <Match when={true}>
              {range(true)}
              <MuteBtn muted={vol().muted} />
            </Match>
          </Switch>
        )}
      </Show>
    </section>
  );
}
