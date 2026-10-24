// Široký notch (rozložení `notch`, kolo 22): lišta sekcí nahoře v šuplíku — přehrávač s obalem | pás dnů
// s velkým měsícem a časem | systém s mini grafy | svislý posuvník hlasitosti. Sekce skládá z EXISTUJÍCÍCH
// dat (snapshot Právě hraje / hlasitost, `intApi.systemView`) a jen z karet, které záložka má
// (`model.ts` `notchSections`); co chybí, se nekreslí. Systém se vzorkuje po 2 s JEN s otevřeným šuplíkem
// (jako karta Systém, kterou lišta převzala), zavřený = žádný časovač. Fokus nekrade (jen klik).
import { For, Match, Show, Switch, createEffect, createSignal, onCleanup } from "solid-js";
import { api, type Snapshot } from "../api";
import { t } from "../i18n";
import { fmtClock } from "../format";
import { sayError } from "../ui/toast";
import { drawerShown } from "../media/shown";
import { Cover, IcNext, IcPause, IcPlay, IcPrev, control, isPlaying, useCover } from "../media/Player";
import { throttle } from "../media/model";
import { intApi, type SystemView } from "../integrations/api";
import { fmtPct, loadOf } from "../integrations/format";
import { volumeOf, weekStrip, type NotchSection } from "./model";
import Sparkline from "../charts/Sparkline";
import "./notch.css";

/** Odstup vzorků systému (stejně jako karta Systém). */
const EVERY_MS = 2000;

const MONTHS = () => [
  t("cards.notch.month.jan", "led"), t("cards.notch.month.feb", "úno"), t("cards.notch.month.mar", "bře"), t("cards.notch.month.apr", "dub"),
  t("cards.notch.month.may", "kvě"), t("cards.notch.month.jun", "čvn"), t("cards.notch.month.jul", "čvc"), t("cards.notch.month.aug", "srp"),
  t("cards.notch.month.sep", "zář"), t("cards.notch.month.oct", "říj"), t("cards.notch.month.nov", "lis"), t("cards.notch.month.dec", "pro"),
];
/** Písmena dnů od neděle (`Date.getDay()`). */
const DOW = () => [
  t("cards.notch.dow.sun", "Ne"), t("cards.notch.dow.mon", "Po"), t("cards.notch.dow.tue", "Út"), t("cards.notch.dow.wed", "St"),
  t("cards.notch.dow.thu", "Čt"), t("cards.notch.dow.fri", "Pá"), t("cards.notch.dow.sat", "So"),
];

function Player(p: { s: Snapshot }) {
  const m = () => p.s.personal.media;
  const cover = useCover(() => m()?.coverKey ?? null);
  return (
    <Show when={m()}>
      {(mv) => (
        <section class="nb-sec nb-player" aria-label={t("cards.media.title", "Právě hraje")}>
          <span class="nb-cover"><Cover url={cover()} /></span>
          <div class="nb-pl">
            <b class="nb-title">{mv().title || t("cards.notch.untitled", "Bez názvu")}</b>
            <small class="nb-by">{mv().artist}</small>
            <div class="nb-ctl">
              <button class="nb-btn" disabled={!mv().canPrev} aria-label={t("cards.notch.prev", "Předchozí")} onClick={() => control("prev")}><IcPrev /></button>
              <button class="nb-btn nb-main" disabled={!mv().canToggle} aria-label={isPlaying(mv()) ? t("cards.notch.pause", "Pozastavit") : t("cards.notch.play", "Přehrát")} onClick={() => control("toggle")}>
                <Show when={isPlaying(mv())} fallback={<IcPlay />}><IcPause /></Show>
              </button>
              <button class="nb-btn" disabled={!mv().canNext} aria-label={t("cards.notch.next", "Další")} onClick={() => control("next")}><IcNext /></button>
            </div>
          </div>
        </section>
      )}
    </Show>
  );
}

function Days(p: { now: number }) {
  const d = () => new Date(p.now);
  return (
    <section class="nb-sec nb-days" aria-label={t("cards.clock.title", "Čas")}>
      <b class="nb-month">{MONTHS()[d().getMonth()]}</b>
      <div class="nb-dright">
        <div class="nb-week">
          <For each={weekStrip(d())}>
            {(w) => (
              <span class="nb-day" classList={{ "nb-today": w.today, "nb-weekend": w.weekend }}>
                <i>{DOW()[w.dow]}</i>
                <b>{w.date}</b>
              </span>
            )}
          </For>
        </div>
        <div class="nb-time">{fmtClock(d())}</div>
      </div>
    </section>
  );
}

function System() {
  const [view, setView] = createSignal<SystemView | null>(null);
  createEffect(() => {
    if (!drawerShown()) return;
    let alive = true;
    let timer: number | undefined;
    const tick = async () => {
      try {
        const v = await intApi.systemView();
        if (alive && v) setView(v);
      } catch {
        /* jeden vzorek navíc nic neřeší — příště */
      }
      if (alive) timer = window.setTimeout(tick, EVERY_MS);
    };
    void tick();
    onCleanup(() => {
      alive = false;
      if (timer !== undefined) window.clearTimeout(timer);
    });
  });
  const disk = () => {
    const d = view()?.disks.find((x) => x.system) ?? view()?.disks[0];
    return d && d.total > 0 ? ((d.total - d.free) / d.total) * 100 : null;
  };
  const Row = (q: { name: string; pct: number | null | undefined; hist?: number[] }) => (
    <Show when={q.pct != null}>
      <div class="nb-row" data-load={loadOf(q.pct)}>
        <span class="nb-rn">{q.name}</span>
        <Show when={q.hist} fallback={<span class="nb-bar"><i style={{ width: `${Math.min(100, q.pct ?? 0)}%` }} /></span>}>
          {(h) => (
            <span class="nb-spark" classList={{ "nb-hot": loadOf(q.pct) !== "ok" }}>
              <Sparkline values={h()} min={0} max={100} h={24} area={false} dot={false} fluid />
            </span>
          )}
        </Show>
        <b class="nb-rv">{fmtPct(q.pct ?? 0)}</b>
      </div>
    </Show>
  );
  return (
    <Show when={view()}>
      {(v) => (
        <section class="nb-sec nb-sys" aria-label={t("cards.notch.system", "Systém")}>
          <Row name="CPU" pct={v().cpu} hist={v().cpuHist} />
          <Row name="RAM" pct={v().memTotal > 0 ? v().memPct : null} hist={v().memHist} />
          <Row name={t("cards.notch.disk", "Disk")} pct={disk()} />
        </section>
      )}
    </Show>
  );
}

function Volume(p: { s: Snapshot }) {
  const v = () => p.s.personal.volume;
  const send = throttle((x: number) => void api.volumeSet(x).catch(sayError), 50);
  return (
    <Show when={v()}>
      {(vol) => (
        <section class="nb-sec nb-vol" aria-label={t("cards.sound.title", "Zvuk")}>
          <input
            class="nb-slider"
            type="range"
            min="0"
            max="100"
            step="1"
            value={vol().muted ? 0 : vol().level}
            aria-label={t("cards.notch.volume", "Hlasitost")}
            onInput={(e) => {
              const x = volumeOf(e.currentTarget.value);
              if (x !== null) send.push(x);
            }}
          />
          <button class="nb-btn nb-mute" aria-pressed={vol().muted} aria-label={vol().muted ? t("cards.notch.unmute", "Zapnout zvuk") : t("cards.notch.mute", "Ztlumit")} onClick={() => void api.volumeMute(!vol().muted).catch(sayError)}>
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
              <path d="M4 9.5h3.5L12 5.5v13l-4.5-4H4z" />
              <Show when={vol().muted} fallback={<path d="M15.5 10a3 3 0 0 1 0 4M18 8a6 6 0 0 1 0 8" />}><path d="M16 9.5l5 5M21 9.5l-5 5" /></Show>
            </svg>
          </button>
        </section>
      )}
    </Show>
  );
}

export default function NotchBar(p: { s: Snapshot; now: number; sections: NotchSection[] }) {
  return (
    <div class="nb" data-anim>
      <For each={p.sections}>
        {(sec) => (
          <Switch>
            <Match when={sec === "player"}><Player s={p.s} /></Match>
            <Match when={sec === "days"}><Days now={p.now} /></Match>
            <Match when={sec === "system"}><System /></Match>
            <Match when={sec === "volume"}><Volume s={p.s} /></Match>
          </Switch>
        )}
      </For>
    </div>
  );
}
