// Osobní: karta „Právě hraje“ (média Windows přes Rust `media.rs`) + hlasitost výchozího výstupu
// (`volume.rs`). Pět vzhledů (`media.variant`, přepínač v rohu karty — `VariantPicker.tsx`): kompaktní
// řádek, karta s průběhem (výchozí), velký obal, vinyl, jen v proužku (v šuplíku tenký ovladač).
// Nic nehraje a hlasitost vypnutá / bez zařízení → karta zmizí; s hlasitostí zůstane a řekne „Nic
// nehraje“. Ovládání jen klikem / tahem, nic nebere fokus. Lišta průběhu jen se skutečnou timeline.
import { Match, Show, Switch, createSignal, onMount, type JSX } from "solid-js";
import { api, type MediaVariant, type MediaView, type Snapshot } from "../api";
import { sayError } from "../ui/toast";
import { t } from "../i18n";
import { sectionOn } from "../cards/sections";
import { throttle, volumeLevel } from "./model";
import { Controls, Cover, Equalizer, IcNote, Progress, TimeText, isPlaying, useCover } from "./Player";
import { VariantPicker, shownVariant } from "./VariantPicker";
import { drawerShown } from "./shown";
import { live } from "./timeline";
import "./media.css";

/** Záložka Osobní je vidět → Rust spustí odběry událostí (líně, jen poprvé). Nic nevykresluje. */
export function PersonalLive() {
  onMount(() => void api.personalLive());
  return null;
}

const stroke = { viewBox: "0 0 24 24", fill: "none", stroke: "currentColor", "stroke-width": 1.8, "stroke-linecap": "round", "stroke-linejoin": "round", "aria-hidden": true } as const;
const SPEAKER = "M4 9.5h3.5L12 5.5v13l-4.5-4H4z";
const VOL_ICON: Record<ReturnType<typeof volumeLevel>, () => JSX.Element> = {
  mute: () => <svg {...stroke}><path d={SPEAKER} /><path d="M16 9.5l5 5M21 9.5l-5 5" /></svg>,
  low: () => <svg {...stroke}><path d={SPEAKER} /><path d="M15.5 10a3 3 0 0 1 0 4" /></svg>,
  mid: () => <svg {...stroke}><path d={SPEAKER} /><path d="M15.5 10a3 3 0 0 1 0 4M18 8a6 6 0 0 1 0 8" /></svg>,
  high: () => <svg {...stroke}><path d={SPEAKER} /><path d="M15.5 10a3 3 0 0 1 0 4M18 8a6 6 0 0 1 0 8M20.5 5.5a9.5 9.5 0 0 1 0 13" /></svg>,
};

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

const titleOf = (m: MediaView) => m.title || t("cards.media.untitled", "Bez názvu");

/** Název + interpret (dva řádky, nebo jeden „Název · Interpret“). */
function Text(p: { m: MediaView; inline?: boolean }) {
  return (
    <div class="med-text">
      <Show
        when={!p.inline}
        fallback={
          <div class="med-title" title={[p.m.title, p.m.artist].filter(Boolean).join(" · ")}>
            {titleOf(p.m)}
            <Show when={p.m.artist}><span class="med-by"> · {p.m.artist}</span></Show>
          </div>
        }
      >
        <div class="med-title" title={p.m.title}>{titleOf(p.m)}</div>
        <Show when={p.m.artist}><div class="sub" title={p.m.artist}>{p.m.artist}</div></Show>
      </Show>
    </div>
  );
}

/** Pauza u vzhledů bez velkého tlačítka stavu. */
const Paused = (p: { m: MediaView }) => (
  <Show when={p.m.status === "paused"}><span class="pill n med-state">{t("cards.media.paused", "pauza")}</span></Show>
);

/** Obsah podle vzhledu (bez hlavičky a hlasitosti). */
function Body(p: { m: MediaView; v: MediaVariant; cover: string | null; eq: boolean; menu?: JSX.Element; picker: JSX.Element }) {
  const tl = () => p.m.timeline ?? null;
  const spin = () => live(isPlaying(p.m), drawerShown());
  const bar = (times: boolean) => <Show when={tl()}>{(x) => <Progress m={p.m} tl={x()} times={times} thin={!times} />}</Show>;
  const foot = () => (
    <div class="med-foot">
      <Controls m={p.m} modes />
      <Paused m={p.m} />
    </div>
  );
  return (
    <Switch>
      <Match when={p.v === "compact"}>
        <div class="med-now med-row">
          <Cover url={p.cover} size="sm" />
          <div class="med-grow">
            <Text m={p.m} inline />
            {bar(false)}
          </div>
          <Controls m={p.m} tight />
        </div>
      </Match>
      <Match when={p.v === "cover"}>
        <div class="med-now">
          <Cover url={p.cover} size="lg" />
          <Text m={p.m} />
        </div>
        {bar(true)}
        {foot()}
      </Match>
      <Match when={p.v === "vinyl"}>
        <div class="med-now">
          <div class="med-disc" classList={{ "med-disc-spin": spin() }} aria-hidden="true">
            <Cover url={p.cover} size="label" />
          </div>
          <div class="med-grow">
            <Text m={p.m} />
            {bar(true)}
          </div>
        </div>
        {foot()}
      </Match>
      <Match when={p.v === "strip"}>
        <div class="med-slim">
          <Show when={p.eq} fallback={<span class="med-note"><IcNote /></span>}>
            <Equalizer playing={isPlaying(p.m)} />
          </Show>
          <Text m={p.m} inline />
          <Show when={tl()}>{(x) => <TimeText m={p.m} tl={x()} />}</Show>
          <Controls m={p.m} tight />
          {p.picker}
          {p.menu}
        </div>
      </Match>
      <Match when={p.v === "card"}>
        <div class="med-now">
          <Cover url={p.cover} />
          <Text m={p.m} />
        </div>
        {bar(true)}
        {foot()}
      </Match>
    </Switch>
  );
}

export default function MediaCard(p: { s: Snapshot; menu?: JSX.Element }) {
  const playing = () => sectionOn(p.s.sections, "nowPlaying");
  const m = () => (playing() ? p.s.personal.media : null);
  const vol = () => (sectionOn(p.s.sections, "sound") ? p.s.personal.volume : null);
  const v = () => shownVariant(p.s.personal.mediaLook);
  const eq = () => p.s.personal.mediaLook?.equalizer !== false;
  const cover = useCover(() => m()?.coverKey ?? null);
  /** tenký ovladač (Jen v proužku) — jen se skladbou; bez ní zůstane obyčejná karta (Zvuk / Nic nehraje) */
  const slim = () => v() === "strip" && !!m();
  const picker = () => <VariantPicker current={v()} cfgError={p.s.cfgError} push={!slim() && !m()?.app} />;
  return (
    <Show when={m() || vol()}>
      <div
        class="card med-card"
        classList={{ "med-v-compact": v() === "compact", "med-v-cover": v() === "cover" && !!m(), "med-v-slim": slim() }}
        data-anim
      >
        <Show when={v() === "cover" && cover()}>{(u) => <span class="med-backdrop" aria-hidden="true"><img src={u()} alt="" draggable={false} /></span>}</Show>
        <Show when={!slim()}>
          <h4>
            <span class="ic" style="background:var(--violet)" />
            {playing() ? t("cards.media.title", "Právě hraje") : t("cards.sound.title", "Zvuk")}
            <Show when={m()?.app}>{(a) => <span class="n">{a()}</span>}</Show>
            <Show when={playing()}>{picker()}</Show>
            {p.menu}
          </h4>
        </Show>
        <Show when={playing()}>
          <Show when={m()} fallback={<div class="empty">{t("cards.media.none", "Nic nehraje")}</div>}>
            {(mm) => <Body m={mm()} v={v()} cover={cover()} eq={eq()} menu={slim() ? p.menu : undefined} picker={picker()} />}
          </Show>
        </Show>
        <Show when={vol()}>{(x) => <Volume level={x().level} muted={x().muted} device={x().device} />}</Show>
      </div>
    </Show>
  );
}
