// Karta „Rychlé přepínače“ (kolo 22, id `toggles`, obr. 1 a 5): dlaždice jako v centru akcí — zvuk (klik =
// ztlumit / zrušit), mikrofon (jen když nějaký je), hlasitost − / + po 5 % a baterie, když stroj nějakou má.
// Stav je z Core Audio (`volume.rs`, události — nic se nedotazuje) a z `battery.rs`; ovládá tytéž příkazy jako
// karta Právě hraje. Jas tu záměrně není: Windows ho bez práv správce spolehlivě nastavit nedovolí (jen některé
// notebooky přes WMI), tak se neukazuje nic, co by nefungovalo.
import { Show, type JSX } from "solid-js";
import { api, type Snapshot } from "../api";
import { t } from "../i18n";
import { say, sayError } from "../ui/toast";
import { stepVolume } from "./model";
import "./board.css";

const STEP = 5;

export default function TogglesCard(p: { s: Snapshot; menu?: JSX.Element }) {
  const v = () => p.s.personal?.volume ?? null;
  const bat = () => p.s.personal?.battery ?? null;
  const mic = () => v()?.micMuted;
  const toggleMic = async () => {
    try {
      const muted = await api.volumeMicMute();
      say(muted ? t("cards.sound.micMuteNotice", "Mikrofon ztlumen") : t("cards.sound.micUnmuteNotice", "Mikrofon zapnut"));
    } catch (e) {
      sayError(e);
    }
  };
  const step = (delta: number) => {
    const x = v();
    if (x) void api.volumeSet(stepVolume(x.level, delta)).catch(sayError);
  };
  return (
    <div class="card bd-toggles" data-anim>
      <h4><span class="ic" style="background:var(--green)" />{t("cards.board.toggles.title", "Rychlé přepínače")}{p.menu}</h4>
      <Show when={v() || bat()} fallback={<div class="empty">{t("cards.board.toggles.none", "Žádné zvukové zařízení ani baterie.")}</div>}>
        <div class="bd-tiles">
          <Show when={v()}>
            {(x) => (
              <button class="bd-tile" classList={{ on: !x().muted }} aria-pressed={!x().muted} title={x().device || t("cards.sound.device", "Zvukový výstup")} onClick={() => void api.volumeMute(!x().muted).catch(sayError)}>
                <span class="bd-tile-ic" aria-hidden="true">{x().muted ? "🔇" : "🔊"}</span>
                <span class="bd-tile-n">{t("cards.board.toggles.sound", "Zvuk")}</span>
                <span class="bd-tile-v">{x().muted ? t("cards.board.toggles.muted", "ztlumeno") : t("cards.sound.pct", "{n} %", { n: x().level })}</span>
              </button>
            )}
          </Show>
          <Show when={mic() != null}>
            <button class="bd-tile" classList={{ on: !mic() }} aria-pressed={!mic()} onClick={() => void toggleMic()}>
              <span class="bd-tile-ic" aria-hidden="true">🎙️</span>
              <span class="bd-tile-n">{t("cards.board.toggles.mic", "Mikrofon")}</span>
              <span class="bd-tile-v">{mic() ? t("cards.board.toggles.micOff", "ztlumený") : t("cards.board.toggles.micOn", "zapnutý")}</span>
            </button>
          </Show>
          <Show when={bat()}>
            {(b) => (
              <div class="bd-tile bd-tile-ro" title={t("cards.board.origin.officialTip", "Číslo hlásí přímo zdroj (Windows / Claude Code), Ledge ho jen ukazuje.")}>
                <span class="bd-tile-ic" aria-hidden="true">{b().charging ? "⚡" : "🔋"}</span>
                <span class="bd-tile-n">{t("cards.board.toggles.battery", "Baterie")}</span>
                <span class="bd-tile-v">{t("cards.sound.pct", "{n} %", { n: b().percent })}</span>
              </div>
            )}
          </Show>
        </div>
        <Show when={v()}>
          {(x) => (
            <div class="bd-vol">
              <button class="mini" disabled={x().level <= 0} onClick={() => step(-STEP)} aria-label={t("cards.board.toggles.down", "Ztišit o {n} %", { n: STEP })} title={t("cards.board.toggles.down", "Ztišit o {n} %", { n: STEP })}>−</button>
              <span class="bd-vol-track" role="meter" aria-valuemin={0} aria-valuemax={100} aria-valuenow={x().level} aria-label={t("cards.sound.level", "Hlasitost")}>
                <i style={{ width: `${x().level}%` }} classList={{ "bd-vol-mute": x().muted }} />
              </span>
              <button class="mini" disabled={x().level >= 100} onClick={() => step(STEP)} aria-label={t("cards.board.toggles.up", "Zesílit o {n} %", { n: STEP })} title={t("cards.board.toggles.up", "Zesílit o {n} %", { n: STEP })}>+</button>
            </div>
          )}
        </Show>
      </Show>
    </div>
  );
}
