// Hlasitost výchozího výstupu v kartě Právě hraje / Zvuk (Rust `volume.rs` `VolumeView`): ztlumit, posuvník,
// procenta; kolo 18 navíc ztlumení výchozího mikrofonu (jen když nějaký je) a výběr
// výstupu (sluchátka / reproduktory / HDMI — menu `ui/Popover` `PopMenu`, jen když je víc než jeden výstup).
// Stav drží jádro událostmi Core Audio; tady se nic nedotazuje. Ovládání jen klikem / tahem, fokus nekrade.
import { For, Show, createSignal, type JSX } from "solid-js";
import { api, type VolumeView } from "../api";
import { say, sayError } from "../ui/toast";
import { t } from "../i18n";
import { createMenu, PopCap, PopItem, PopMenu } from "../ui/Popover";
import { outputPickerShown, throttle, volumeLevel } from "./model";

const stroke = { viewBox: "0 0 24 24", fill: "none", stroke: "currentColor", "stroke-width": 1.8, "stroke-linecap": "round", "stroke-linejoin": "round", "aria-hidden": true } as const;
const SPEAKER = "M4 9.5h3.5L12 5.5v13l-4.5-4H4z";
const VOL_ICON: Record<ReturnType<typeof volumeLevel>, () => JSX.Element> = {
  mute: () => <svg {...stroke}><path d={SPEAKER} /><path d="M16 9.5l5 5M21 9.5l-5 5" /></svg>,
  low: () => <svg {...stroke}><path d={SPEAKER} /><path d="M15.5 10a3 3 0 0 1 0 4" /></svg>,
  mid: () => <svg {...stroke}><path d={SPEAKER} /><path d="M15.5 10a3 3 0 0 1 0 4M18 8a6 6 0 0 1 0 8" /></svg>,
  high: () => <svg {...stroke}><path d={SPEAKER} /><path d="M15.5 10a3 3 0 0 1 0 4M18 8a6 6 0 0 1 0 8M20.5 5.5a9.5 9.5 0 0 1 0 13" /></svg>,
};
const MIC = "M12 3a3 3 0 0 0-3 3v6a3 3 0 0 0 6 0V6a3 3 0 0 0-3-3zM18.5 11a6.5 6.5 0 0 1-13 0M12 17.5V21M8.5 21h7";
const IcMic = (p: { off: boolean }) => <svg {...stroke}><path d={MIC} /><Show when={p.off}><path d="M4 4l16 16" /></Show></svg>;

function Outputs(p: { v: VolumeView }) {
  const menu = createMenu();
  const current = () => p.v.devices?.find((d) => d.active)?.name || p.v.device || t("cards.sound.device", "Zvukový výstup");
  const pick = async (id: string) => {
    menu.close();
    try {
      await api.volumeSetDevice(id);
      say(t("cards.sound.deviceChanged", "Výstup přepnut"));
    } catch (e) {
      sayError(e);
    }
  };
  return (
    <>
      <button class="mini med-out" aria-haspopup="menu" aria-expanded={menu.open()} title={t("cards.sound.deviceSelect", "Vybrat zvukový výstup")} onClick={menu.toggle}>
        <span class="med-out-name">{current()}</span>
        <span class="pz-caret">▾</span>
      </button>
      <Show when={menu.at()}>
        {(at) => (
          <PopMenu anchor={at().anchor} keyboard={at().keyboard} trigger={menu.trigger()} onClose={menu.close} label={t("cards.sound.devices", "Výstupní zařízení")}>
            <PopCap>{t("cards.sound.devices", "Výstupní zařízení")}</PopCap>
            <For each={p.v.devices ?? []}>
              {(d) => <PopItem radio on={d.active} onClick={() => (d.active ? menu.close() : void pick(d.id))}>{d.name || d.id}</PopItem>}
            </For>
          </PopMenu>
        )}
      </Show>
    </>
  );
}

export default function VolumeBar(p: { v: VolumeView }) {
  // při tahu platí místní hodnota (Rust ji potvrdí událostí; mezitím by posuvník poskakoval)
  const [drag, setDrag] = createSignal<number | null>(null);
  const send = throttle((v: number) => void api.volumeSet(v).catch(sayError), 50);
  const shown = () => drag() ?? p.v.level;
  const end = () => {
    send.flush();
    setDrag(null);
  };
  const Icon = () => VOL_ICON[volumeLevel(shown(), p.v.muted)]();
  const mic = () => p.v.micMuted;
  const toggleMic = async () => {
    try {
      const muted = await api.volumeMicMute();
      say(muted ? t("cards.sound.micMuteNotice", "Mikrofon ztlumen") : t("cards.sound.micUnmuteNotice", "Mikrofon zapnut"));
    } catch (e) {
      sayError(e);
    }
  };
  return (
    <>
      <div class="med-vol">
        <button
          class="mini med-ico"
          classList={{ on: p.v.muted }}
          title={p.v.muted ? t("cards.sound.unmute", "Zrušit ztlumení") : t("cards.sound.mute", "Ztlumit")}
          aria-label={p.v.muted ? t("cards.sound.unmute", "Zrušit ztlumení") : t("cards.sound.mute", "Ztlumit")}
          onClick={() => void api.volumeMute(!p.v.muted).catch(sayError)}
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
          title={p.v.device}
          onPointerDown={() => setDrag(p.v.level)}
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
        <Show when={mic() != null}>
          <button
            class="mini med-ico"
            classList={{ on: !!mic() }}
            aria-pressed={!!mic()}
            title={mic() ? t("cards.sound.micUnmute", "Zapnout mikrofon") : t("cards.sound.micMute", "Ztlumit mikrofon")}
            aria-label={mic() ? t("cards.sound.micUnmute", "Zapnout mikrofon") : t("cards.sound.micMute", "Ztlumit mikrofon")}
            onClick={() => void toggleMic()}
          >
            <IcMic off={!!mic()} />
          </button>
        </Show>
      </div>
      <Show when={outputPickerShown(p.v.devices?.length ?? 0)}>
        <div class="med-outs">
          <Outputs v={p.v} />
        </div>
      </Show>
    </>
  );
}
