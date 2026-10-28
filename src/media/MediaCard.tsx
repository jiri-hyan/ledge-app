// Osobní: karta „Právě hraje“ (média Windows přes Rust `media.rs`) + hlasitost výchozího výstupu
// (`volume.rs`, ovladač `VolumeBar.tsx` — i mikrofon a výběr výstupu). Pět vzhledů (`media.variant`, přepínač v rohu karty — `VariantPicker.tsx`): kompaktní
// řádek, karta s průběhem (výchozí), velký obal, vinyl, jen v proužku (v šuplíku tenký ovladač).
// Nic nehraje a hlasitost vypnutá / bez zařízení → karta zmizí; s hlasitostí zůstane a řekne „Nic
// nehraje“. Ovládání jen klikem / tahem, nic nebere fokus. Lišta průběhu jen se skutečnou timeline.
import { Match, Show, Switch, createResource, onMount, type JSX } from "solid-js";
import { api, type MediaVariant, type MediaView, type Snapshot } from "../api";
import { t } from "../i18n";
import { sectionOn } from "../cards/sections";
import VolumeBar from "./VolumeBar";
import { coverAccent } from "./tint";
import { Controls, Cover, Equalizer, IcNote, Progress, TimeText, isPlaying, useCover } from "./Player";
import { VariantPicker, shownVariant } from "./VariantPicker";
import { drawerShown } from "./shown";
import { PartSlot, PartsBox } from "../layout/PartSlot";
import Favorites from "./FavoriteTiles";
import { live } from "./timeline";
import "./media.css";

/** Záložka Osobní je vidět → Rust spustí odběry událostí (líně, jen poprvé). Nic nevykresluje. */
export function PersonalLive() {
  onMount(() => void api.personalLive());
  return null;
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
  /** oblíbené: jen zapnutá část a (něco uložené, nebo něco hraje — je co přidat); jinak by byl nadpis bez obsahu */
  const favs = () => (sectionOn(p.s.sections, "favorites") ? p.s.personal.mediaLook?.favorites ?? [] : []);
  const favsOn = () => sectionOn(p.s.sections, "favorites") && (favs().length > 0 || !!m());
  const v = () => shownVariant(p.s.personal.mediaLook);
  const eq = () => p.s.personal.mediaLook?.equalizer !== false;
  const cover = useCover(() => m()?.coverKey ?? null);
  /** podbarvení barvou obalu (`media.tint`, výchozí vypnuté) — spočítá se jednou na obal */
  const [tint] = createResource(() => (p.s.personal.mediaLook?.tint ? cover() : null) || null, coverAccent);
  /** tenký ovladač (Jen v proužku) — jen se skladbou; bez ní zůstane obyčejná karta (Zvuk / Nic nehraje) */
  const slim = () => v() === "strip" && !!m();
  const picker = () => <VariantPicker current={v()} cfgError={p.s.cfgError} push={!slim() && !m()?.app} />;
  return (
    <Show when={m() || vol() || favsOn()}>
      <div
        class="card med-card"
        classList={{ "med-v-compact": v() === "compact", "med-v-cover": v() === "cover" && !!m(), "med-v-slim": slim(), "med-tinted": !!tint() && !!m() }}
        style={tint() && m() ? { "--media-tint": tint()! } : undefined}
        data-anim
      >
        <Show when={v() === "cover" && cover()}>{(u) => <span class="med-backdrop" aria-hidden="true"><img src={u()} alt="" draggable={false} /></span>}</Show>
        <Show when={!slim()}>
          <h4>
            <span class="ic" style="background:var(--violet)" />
            {playing() ? t("cards.media.title", "Právě hraje") : vol() ? t("cards.sound.title", "Zvuk") : t("cards.media.fav.title", "Oblíbené")}
            <Show when={m()?.app}>{(a) => <span class="n">{a()}</span>}</Show>
            <Show when={playing()}>{picker()}</Show>
            {p.menu}
          </h4>
        </Show>
        {/* skladba a hlasitost jsou části karty: pořadí a skrytí v úpravách (`layout/PartSlot.tsx`) */}
        <PartsBox card="media">
          <PartSlot card="media" k="nowPlaying" hidden={!playing()}>
            <Show when={m()} fallback={<div class="empty">{t("cards.media.none", "Nic nehraje")}</div>}>
              {(mm) => <Body m={mm()} v={v()} cover={cover()} eq={eq()} menu={slim() ? p.menu : undefined} picker={picker()} />}
            </Show>
          </PartSlot>
          <PartSlot card="media" k="sound" hidden={!sectionOn(p.s.sections, "sound")} titled={playing() && !slim() && !!m()} empty={!vol()}>
            <Show when={vol()}>{(x) => <VolumeBar v={x()} />}</Show>
          </PartSlot>
          <PartSlot card="media" k="favorites" hidden={!sectionOn(p.s.sections, "favorites")} titled={(!!m() || !!vol()) && !slim()} empty={!favsOn()}>
            <Show when={favsOn()}>
              <Favorites list={favs()} playing={m()} cfgError={p.s.cfgError} />
            </Show>
          </PartSlot>
        </PartsBox>
      </div>
    </Show>
  );
}
