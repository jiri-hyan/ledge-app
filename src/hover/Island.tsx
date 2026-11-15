// Roztažený ostrůvek po najetí (`drawer.hover: "island"`, Rust `View::Island`): černá kapsle s JEDNÍM
// nejdůležitějším děním (`model.ts` `islandEvent`) a tlačítky — čeká na tebe → Povolit / Zamítnout
// (`cards/Peek.tsx` `DecideButtons`; plán a otázka → celý šuplík), hotovo → Otevřít / ✕ (potvrzené zmizí, `doneAck.ts`), časovač → +5 min / Pauza,
// hudba → ⏮⏯⏭ (`media/Player.tsx`), schůzka za ≤ 10 min / právě teď → Připojit, limit 5h blízko → procenta a odhad, Claude pracuje → název a čas,
// klid → velký čas, datum, počasí, nebo jeden chip z `suggest/pick` (H2); pořadí dění je v `priority.ts`.
// Jen data ze snapshotu. Klik na prázdné místo nebo setrvání (`useExpand`) = celý šuplík.
import { Match, Show, Switch } from "solid-js";
import { api, type Snapshot } from "../api";
import type { SessionView } from "../types";
import { DecideButtons, pendingWhere } from "../cards/Peek";
import { IconShield } from "../icons";
import { Controls, Cover, useCover } from "../media/Player";
import { fmtDate, fmtClock, num, uptime, ago } from "../format";
import { pctText } from "../limits/model";
import { elapsedMs, fmtTimer, leftMs } from "../timer/model";
import { timerDo } from "../timer/TimerCard";
import { say, sayError } from "../ui/toast";
import { t, tp } from "../i18n";
import { tempIn, tempUnit } from "../cards/temp";
import { freshWeather, islandEvent } from "./model";
import { useChips } from "../suggest/chips";
import { chipLabel } from "../suggest/label";
import { closeSuggestion } from "../suggest/actions";
import { providerName } from "../calendar/model";
import { ackDoneSession, doneAcked } from "./doneAck";
import { Limit5h, sessionLabel } from "./parts";
import { useExpand } from "./expand";
import "../media/media.css";
import "./hover.css";

/** `full` = rozbalení rovnou do celého šuplíku (ostrůvek přebírá schválení v docku; dok jinak rozbaluje do středního panelu). */
export default function Island(p: { s: Snapshot; now: number; expandMs: number; full?: boolean }) {
  const ev = () => islandEvent(p.s, p.now, doneAcked());
  /** H2: v klidu nejvýš jeden chip z `pick()` (stejný výběr jako pruh v šuplíku; schránka se tu nečte) */
  const chips = useChips(() => p.s, () => p.now, () => null);
  const waiting = () => ev().kind === "approval" || ev().kind === "question";
  const ex = useExpand(() => p.expandMs, waiting, () => false, () => !!p.full);
  const media = () => { const e = ev(); return e.kind === "music" ? e.media : null; };
  const cover = useCover(() => media()?.coverKey ?? null);
  const more = (label: string) => <button class="btn" onClick={() => void api.expandDrawer(!!p.full)}>{label}</button>;
  async function open(ss: SessionView) {
    ackDoneSession(ss, p.now); // L146: Otevřít potvrdí „Hotovo“ (i když otevření selže, hláška se ukáže)
    if (!ss.canOpen) return say(ss.openHint, true);
    try {
      await api.sessionOpen(ss.id);
    } catch (e) {
      sayError(e);
    }
  }
  // střídmé hlášení čtečce: jen když se objeví schválení / otázka / hotovo (ne hodiny, ne odpočet)
  const said = () => {
    const e = ev();
    if (e.kind === "approval") return t("cards.hover.island.sayApproval", "Claude čeká na schválení");
    if (e.kind === "question") return t("cards.hover.island.asks", "Claude se ptá");
    return e.kind === "done" ? t("cards.hover.island.done", "Hotovo: {name}", { name: sessionLabel(e.session) }) : "";
  };
  return (
    <div class="isl" {...ex}>
      <span class="sr-only" role="status" aria-live="polite">{said()}</span>
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
        <Match when={(() => { const e = ev(); return e.kind === "meeting" ? e.meeting : undefined; })()}>
          {(m) => (
            <>
              <span class="isl-ico tm">▶</span>
              <span class="isl-tx">
                <b>{t("cards.hover.island.meeting", "Schůzka: {title}", { title: m().title || t("cards.calendar.untitled", "(bez názvu)") })}</b>
                <span class="isl-sub">
                  {m().ongoing ? t("cards.hover.island.meetingOn", "právě probíhá, do {end}", { end: fmtClock(new Date(m().end)) }) : t("cards.hover.island.meetingIn", "začíná za {n} min ({at})", { n: Math.max(1, Math.ceil((m().start - p.now) / 60_000)), at: fmtClock(new Date(m().start)) })}
                </span>
              </span>
              <span class="isl-act">
                <Show when={m().join}>
                  {(j) => <button class="btn pri" title={t("cards.calendar.joinTitle", "Připojit se přes {app}", { app: providerName(j().provider) })} onClick={() => void api.calendarJoin(j().url).catch(sayError)}>{t("cards.calendar.join", "Připojit")}</button>}
                </Show>
              </span>
            </>
          )}
        </Match>
        <Match when={(() => { const e = ev(); return e.kind === "limit" ? e.limit : undefined; })()}>
          {(l) => (
            <>
              <span class="isl-ico wait">%</span>
              <span class="isl-tx">
                <b>{t("cards.hover.island.limit", "Limit 5h: {pct} %", { pct: pctText(l().pct) })}</b>
                <span class="isl-sub">
                  {[
                    l().etaAt != null && l().etaAt! - p.now <= 3_600_000 ? t("cards.hover.island.limitEta", "dojde za ~{n} min", { n: Math.max(1, Math.round((l().etaAt! - p.now) / 60_000)) }) : "",
                    l().resetsAt != null ? t("cards.hover.island.limitReset", "obnova {at}", { at: fmtClock(new Date(l().resetsAt!)) }) : "",
                  ].filter(Boolean).join(" · ")}
                </span>
              </span>
              <span class="isl-act">{more(t("cards.hover.island.limitMore", "Detail"))}</span>
            </>
          )}
        </Match>
        <Match when={(() => { const e = ev(); return e.kind === "done" ? e.session : undefined; })()}>
          {(ss) => (
            <>
              <span class="isl-ico ok" aria-hidden="true">✓</span>
              <span class="isl-tx">
                <b>{t("cards.hover.island.done", "Hotovo: {name}", { name: sessionLabel(ss()) })}</b>
                <span class="isl-sub">{ago(ss().since ?? ss().updatedAt, p.now)}</span>
              </span>
              <span class="isl-act">
                <Show when={ss().canOpen}>
                  <button class="btn pri" onClick={() => void open(ss())}>{t("cards.hover.island.open", "Otevřít")}</button>
                </Show>
                <button class="mini" title={t("cards.timer.dismiss", "Zavřít")} aria-label={t("cards.timer.dismiss", "Zavřít")} onClick={() => ackDoneSession(ss(), p.now)}>✕</button>
              </span>
            </>
          )}
        </Match>
        <Match when={(() => { const e = ev(); return e.kind === "timer" ? e.what : undefined; })()}>
          {(what) => (
            <Show when={what() === "countdown" && p.s.timer?.countdown} fallback={
              <>
                <span class="isl-ico tm">⏱</span>
                <span class="isl-tx"><b>{t("cards.hover.island.stopwatch", "Stopky {time}", { time: fmtTimer(p.s.timer?.stopwatch ? elapsedMs(p.s.timer.stopwatch, p.now) : 0) })}</b></span>
                <span class="isl-act"><button class="btn" onClick={() => void timerDo({ op: "swPause" })}>{t("cards.timer.pause", "Pauza")}</button></span>
              </>
            }>
              {(c) => (
                <>
                  <span class="isl-ico tm">⏱</span>
                  <span class="isl-tx">
                    <b>
                      <Switch fallback={t("cards.hover.island.left", "Zbývá {time}", { time: fmtTimer(leftMs(c(), p.now), true) })}>
                        <Match when={c().state === "done"}>{t("cards.timer.done", "Čas vypršel")}</Match>
                        <Match when={c().state === "paused"}>{t("cards.hover.island.paused", "Pozastaveno {time}", { time: fmtTimer(leftMs(c(), p.now), true) })}</Match>
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
          <span class="isl-time">{fmtClock(new Date(p.now))}</span>
          <span class="isl-tx">
            <span class="isl-sub">{fmtDate(new Date(p.now))}</span>
            <Show when={chips()[0]} fallback={
              <Show when={freshWeather(p.s.personal.weather, p.now)}>
                {(w) => <span class="isl-sub">{w().icon} {num(tempIn(w().temp, p.s.personal.weather.fahrenheit))} {tempUnit(p.s.personal.weather.fahrenheit)} · {w().desc}</span>}
              </Show>
            }>
              {(c) => <span class="isl-sub isl-chip" title={chipLabel(c())}>{chipLabel(c())}{c().more > 0 ? ` +${c().more}` : ""}</span>}
            </Show>
          </span>
          <Show when={chips()[0]}>
            {(c) => (
              <span class="isl-act">
                {more(t("cards.hover.island.chipOpen", "Otevřít"))}
                <button class="mini" title={t("cards.suggest.dismiss", "Zavřít návrh na 24 hodin")} aria-label={t("cards.suggest.dismiss", "Zavřít návrh na 24 hodin")} onClick={() => closeSuggestion(c().key)}>✕</button>
              </span>
            )}
          </Show>
        </Match>
      </Switch>
    </div>
  );
}
