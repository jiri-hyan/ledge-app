// Roztažený ostrůvek po najetí (`drawer.hover: "island"`, Rust `View::Island`): černá kapsle s JEDNÍM
// nejdůležitějším děním (`model.ts` `islandEvent`) a tlačítky — čeká na tebe → Povolit / Zamítnout
// (`cards/Peek.tsx` `DecideButtons`; plán a otázka → celý šuplík), hotovo → Otevřít, časovač → +5 min / Pauza,
// hudba → ⏮⏯⏭ (`media/Player.tsx`), Claude pracuje → název a čas, klid → velký čas, datum, počasí.
// Jen data ze snapshotu. Klik na prázdné místo nebo setrvání (`useExpand`) = celý šuplík.
import { Match, Show, Switch } from "solid-js";
import { api, type Snapshot } from "../api";
import { DecideButtons, pendingWhere } from "../cards/Peek";
import { IconShield } from "../icons";
import { Controls, Cover, useCover } from "../media/Player";
import { fmtDate, fmtTime, num, uptime, ago } from "../format";
import { elapsedMs, fmtDuration, leftMs } from "../timer/model";
import { timerDo } from "../timer/TimerCard";
import { say } from "../ui/toast";
import { t, tp } from "../i18n";
import { freshWeather, islandEvent } from "./model";
import { Limit5h, sessionLabel } from "./Compact";
import { useExpand } from "./expand";
import "../media/media.css";
import "./hover.css";

export default function Island(p: { s: Snapshot; now: number; expandMs: number }) {
  const ev = () => islandEvent(p.s, p.now);
  const waiting = () => ev().kind === "approval" || ev().kind === "question";
  const ex = useExpand(() => p.expandMs, waiting);
  const media = () => { const e = ev(); return e.kind === "music" ? e.media : null; };
  const cover = useCover(() => media()?.coverKey ?? null);
  const more = (label: string) => <button class="btn" onClick={() => void api.expandDrawer()}>{label}</button>;
  async function open(id: string, canOpen: boolean, hint: string) {
    if (!canOpen) return say(hint, true);
    try {
      await api.sessionOpen(id);
    } catch (e) {
      say(String(e), true);
    }
  }
  return (
    <div class="isl" {...ex}>
      <Switch>
        <Match when={(() => { const e = ev(); return e.kind === "approval" ? p.s.pending.find((x) => x.id === e.pendingId) : undefined; })()}>
          {(pd) => (
            <>
              <span class="isl-ico wait"><IconShield /></span>
              <span class="isl-tx">
                <b>{pendingWhere(p.s, pd())}</b>
                <span class="isl-sub" title={pd().summary}>{pd().plan ? t("cards.hover.island.plan", "chce schválit plán") : `${pd().tool}: ${pd().summary}`}</span>
              </span>
              <span class="isl-act">
                <Show when={!pd().plan} fallback={more(t("cards.hover.island.showPlan", "Zobrazit plán"))}><DecideButtons pd={pd()} /></Show>
              </span>
            </>
          )}
        </Match>
        <Match when={(() => { const e = ev(); return e.kind === "question" ? p.s.questions.find((x) => x.id === e.askId) : undefined; })()}>
          {(a) => (
            <>
              <span class="isl-ico wait"><IconShield /></span>
              <span class="isl-tx">
                <b>{t("cards.hover.island.asks", "Claude se ptá")}</b>
                <span class="isl-sub">{a().questions[0]?.question ?? ""}</span>
              </span>
              <span class="isl-act">{more(t("cards.hover.island.answer", "Odpovědět"))}</span>
            </>
          )}
        </Match>
        <Match when={(() => { const e = ev(); return e.kind === "done" ? e.session : undefined; })()}>
          {(ss) => (
            <>
              <span class="isl-ico ok">✓</span>
              <span class="isl-tx">
                <b>{t("cards.hover.island.done", "Hotovo: {name}", { name: sessionLabel(ss()) })}</b>
                <span class="isl-sub">{ago(ss().since ?? ss().updatedAt, p.now)}</span>
              </span>
              <span class="isl-act">
                <Show when={ss().canOpen}>
                  <button class="btn pri" onClick={() => void open(ss().id, ss().canOpen, ss().openHint)}>{t("cards.hover.island.open", "Otevřít")}</button>
                </Show>
              </span>
            </>
          )}
        </Match>
        <Match when={(() => { const e = ev(); return e.kind === "timer" ? e.what : undefined; })()}>
          {(what) => (
            <Show when={what() === "countdown" && p.s.timer?.countdown} fallback={
              <>
                <span class="isl-ico tm">⏱</span>
                <span class="isl-tx"><b>{t("cards.hover.island.stopwatch", "Stopky {time}", { time: fmtDuration(p.s.timer?.stopwatch ? elapsedMs(p.s.timer.stopwatch, p.now) : 0) })}</b></span>
                <span class="isl-act"><button class="btn" onClick={() => void timerDo({ op: "swPause" })}>{t("cards.timer.pause", "Pauza")}</button></span>
              </>
            }>
              {(c) => (
                <>
                  <span class="isl-ico tm">⏱</span>
                  <span class="isl-tx">
                    <b>
                      <Switch fallback={t("cards.hover.island.left", "Zbývá {time}", { time: fmtDuration(leftMs(c(), p.now), true) })}>
                        <Match when={c().state === "done"}>{t("cards.timer.done", "Čas vypršel")}</Match>
                        <Match when={c().state === "paused"}>{t("cards.hover.island.paused", "Pozastaveno {time}", { time: fmtDuration(leftMs(c(), p.now), true) })}</Match>
                      </Switch>
                    </b>
                  </span>
                  <span class="isl-act">
                    <button class="btn" onClick={() => void timerDo({ op: "add", minutes: 5 })}>{t("cards.timer.plus5", "+5 min")}</button>
                    <Switch>
                      <Match when={c().state === "running"}><button class="btn" onClick={() => void timerDo({ op: "pause" })}>{t("cards.timer.pause", "Pauza")}</button></Match>
                      <Match when={c().state === "paused"}><button class="btn pri" onClick={() => void timerDo({ op: "resume" })}>{t("cards.timer.resume", "Pokračovat")}</button></Match>
                      <Match when={c().state === "done"}><button class="btn" onClick={() => void timerDo({ op: "cancel" })}>{t("cards.timer.dismiss", "Zavřít")}</button></Match>
                    </Switch>
                  </span>
                </>
              )}
            </Show>
          )}
        </Match>
        <Match when={media()}>
          {(m) => (
            <>
              <Cover url={cover()} size="sm" />
              <span class="isl-tx">
                <b>{m().title || t("cards.hover.unknownTrack", "Neznámá skladba")}</b>
                <Show when={m().artist}><span class="isl-sub">{m().artist}</span></Show>
              </span>
              <span class="isl-act"><Controls m={m()} tight /></span>
            </>
          )}
        </Match>
        <Match when={(() => { const e = ev(); return e.kind === "working" ? e : undefined; })()}>
          {(w) => (
            <>
              <span class="hov-dot hov-dot-run hov-pulse" />
              <span class="isl-tx">
                <b>{t("cards.hover.island.working", "Claude pracuje: {name}", { name: sessionLabel(w().session) })}</b>
                <span class="isl-sub">
                  {[w().session.since ? uptime(w().session.since!, p.now) : "", w().count > 1 ? tp("cards.hover.island.moreWorking", w().count - 1, ["+{n} další", "+{n} další", "+{n} dalších"]) : ""].filter(Boolean).join(" · ")}
                </span>
              </span>
              <span class="isl-act"><Limit5h s={p.s} now={p.now} /></span>
            </>
          )}
        </Match>
        <Match when={ev().kind === "idle"}>
          <span class="isl-time">{fmtTime(new Date(p.now))}</span>
          <span class="isl-tx">
            <span class="isl-sub">{fmtDate(new Date(p.now))}</span>
            <Show when={freshWeather(p.s.personal.weather, p.now)}>
              {(w) => <span class="isl-sub">{w().icon} {num(w().temp)} °C · {w().desc}</span>}
            </Show>
          </span>
        </Match>
      </Switch>
    </div>
  );
}
