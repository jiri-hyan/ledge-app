// HTML náhled jednoho vzhledu zavřeného Ledge (`strip.look`) na „horní hraně monitoru“ pro galerii kroku
// Zavřený v panelu Vzhled. Kreslí se z rozpracovaného vzhledu (`.theme-scope` + `scopeStyle`: barvy, akcent,
// písmo), pozice a velikost proužku z `appearance.strip`, krytí ze `strip.opacity`; Klasický bere tvar
// `appearance.strip.style`. Data jsou UKÁZKOVÁ (přepínač „Co se děje“) — skutečný proužek kreslí Rust.
// V klidu se nic nehýbe (žádná nekonečná animace; „pulz“ = statický prstenec, točení prstenců `rings` =
// statická značka). Tvar kapsle (`strip.lookShape`, kolo 22) = `data-lshape` u stylů s `SHAPEABLE_LOOKS`.
import { For, Match, Show, Switch, type JSX } from "solid-js";
import { t } from "../../i18n";
import type { Appearance, StripLookId } from "../../types";
import { STRIP_SIZE, stripShape } from "../../theme/appearance";
import { scopeStyle } from "../../theme/apply";
import type { PreviewState } from "../appearanceModel";
import { SHAPEABLE_LOOKS, type LookShape } from "../../strip/looks";
import ComposePreview from "./ComposePreview";
import "./previews.css";

/** Barva stavu (tokeny motivu přes `data-c` v previews.css). */
type Tone = "run" | "wait" | "ok" | "acc" | "music" | "off";
export const Dot = (q: { c: Tone; ring?: boolean }) => <i class="apv-d" data-c={q.c} classList={{ "apv-ring": !!q.ring }} />;
export const Glyph = (q: { g: string; c?: Tone }) => <i class="apv-g" data-c={q.c ?? "off"}>{q.g}</i>;

/** Ukázkové texty (žádná skutečná data). */
export const sample = {
  work: () => t("appearance.sample.work", "Claude pracuje · 2"),
  wait: () => t("appearance.sample.wait", "Čeká na schválení"),
  music: () => t("appearance.sample.music", "Skladba · Interpret"),
  timer: () => t("appearance.sample.timer", "18 min"),
  done: () => t("appearance.sample.done", "Hotovo: ledge"),
  mode: () => t("appearance.sample.mode", "Práce"),
};
const TIME = "14:32";
/** Baterie jako tvar (`--p` = nabití). */
const Batt = (q: { p: number }) => <i class="apv-batt" style={{ "--p": `${q.p}%` }} />;
/** Prstenec limitu (`--p` = čerpání, `data-c` = barva; `spin` = statická značka „pracuje“). */
const Ring = (q: { p: number | null; c: Tone | "call" | "warm"; spin?: boolean }) => (
  <i class="apv-ring2" data-c={q.c} classList={{ "apv-ring-spin": !!q.spin }} style={{ "--p": `${q.p ?? 0}%` }} />
);
/** Tečka / ikona stavu před časem (kapsle kola 22). */
const lead = (s: PreviewState): JSX.Element =>
  ({ idle: null, work: <Dot c="run" ring />, wait: <Dot c="wait" ring />, music: null, timer: <Glyph g="⏱" c="acc" />, done: <Dot c="ok" /> })[s];

/** Obsah jednoho vzhledu v daném stavu. */
function body(look: StripLookId, s: PreviewState, a: Appearance): JSX.Element {
  switch (look) {
    case "island":
      return ({
        idle: <Dot c="off" />,
        work: <><Dot c="run" ring /> {sample.work()}</>,
        wait: <><Dot c="wait" ring /> <b>{sample.wait()}</b></>,
        music: <><Glyph g="♪" c="music" /> {sample.music()}</>,
        timer: <><Glyph g="⏱" c="acc" /> {sample.timer()}</>,
        done: <><Glyph g="✓" c="ok" /> {sample.done()}</>,
      })[s];
    case "dots": {
      const c: Record<PreviewState, Tone[]> = {
        idle: ["off", "off", "off"], work: ["run", "ok", "off"], wait: ["wait", "ok", "off"],
        music: ["off", "ok", "music"], timer: ["off", "ok", "acc"], done: ["ok", "ok", "off"],
      };
      return <For each={c[s]}>{(x, i) => <Dot c={x} ring={i() === 0 && (s === "work" || s === "wait")} />}</For>;
    }
    case "line": {
      const m: Record<PreviewState, [number, Tone][]> = {
        idle: [[1, "off"]], work: [[2, "run"], [5, "off"]], wait: [[1, "wait"]], music: [[3, "music"], [4, "off"]], timer: [[6, "acc"], [2, "off"]], done: [[1, "ok"]],
      };
      return <For each={m[s]}>{([f, c]) => <i class="apv-seg" data-c={c} style={{ flex: String(f) }} />}</For>;
    }
    case "icons": {
      const b = (g: string, n: string, c?: Tone) => <span class="apv-badge"><Glyph g={g} c={c} /><Show when={n}><sup>{n}</sup></Show></span>;
      return ({
        idle: <>{b("✳", "")}{b("▣", "")}{b("⑂", "")}</>,
        work: <>{b("✳", "2", "run")}{b("▣", "3", "ok")}{b("⑂", "1")}</>,
        wait: <>{b("!", "1", "wait")}{b("✳", "2", "run")}{b("▣", "3", "ok")}</>,
        music: <>{b("♪", "", "music")}{b("✳", "")}{b("▣", "3", "ok")}</>,
        timer: <>{b("⏱", "18", "acc")}{b("✳", "")}{b("▣", "3", "ok")}</>,
        done: <>{b("✓", "", "ok")}{b("✳", "")}{b("▣", "3", "ok")}</>,
      })[s];
    }
    case "clock":
      return <>{({ idle: null, work: <Dot c="run" ring />, wait: <Dot c="wait" ring />, music: <Glyph g="♪" c="music" />, timer: <Glyph g="⏱" c="acc" />, done: <Dot c="ok" /> })[s]}<b class="apv-big">{TIME}</b></>;
    case "corner":
      return ({
        idle: <><Dot c="off" /> {TIME}</>,
        work: <><Dot c="run" ring /> {t("appearance.sample.cornerWork", "2 pracují")}</>,
        wait: <><Dot c="wait" ring /> <b>{t("appearance.sample.cornerWait", "čeká 1")}</b></>,
        music: <><Glyph g="♪" c="music" /> {t("appearance.sample.cornerMusic", "hraje")}</>,
        timer: <><Glyph g="⏱" c="acc" /> 18:04</>,
        done: <><Glyph g="✓" c="ok" /> {t("appearance.sample.cornerDone", "hotovo")}</>,
      })[s];
    case "ghost":
      return s === "wait" ? <><Dot c="wait" ring /> {sample.wait()}</> : s === "done" ? <><Glyph g="✓" c="ok" /> {sample.done()}</> : null;
    case "bar": {
      const L: Record<PreviewState, string[]> = {
        idle: ["ledge", "—"],
        work: [t("appearance.sample.barWork", "claude ▸ 2 pracují"), "srv 3"],
        wait: [t("appearance.sample.barWait", "claude ▸ ČEKÁ 1"), "srv 3"],
        music: [t("appearance.sample.barMusic", "♪ skladba"), "srv 3"],
        timer: ["⏱ 18:04", "srv 3"],
        done: [t("appearance.sample.barDone", "✓ hotovo"), "srv 3"],
      };
      return <><span class="apv-bar-l"><For each={L[s]}>{(x) => <span class="apv-bar-seg">{x}</span>}</For></span><span>5h 42 % · {TIME}</span></>;
    }
    case "notch":
      return ({
        idle: null, work: <><Dot c="run" ring /> 2</>, wait: <><Dot c="wait" ring /> {t("appearance.sample.notchWait", "1 čeká")}</>,
        music: <Glyph g="♪" c="music" />, timer: <><Glyph g="⏱" c="acc" /> 18</>, done: <Glyph g="✓" c="ok" />,
      })[s];
    case "progress": {
      const m: Record<PreviewState, [string, string, number, Tone]> = {
        idle: [t("appearance.sample.limit", "Limit 5h"), "42 %", 42, "acc"],
        work: [t("appearance.sample.working", "Claude pracuje"), "5h 42 %", 42, "run"],
        wait: [t("appearance.sample.waitYou", "Čeká na tebe"), "1", 100, "wait"],
        music: [sample.music(), "1:42 / 4:03", 42, "music"],
        timer: [t("appearance.sample.timerName", "Časovač"), "18 min", 70, "acc"],
        done: [t("appearance.sample.doneShort", "Hotovo"), "ledge", 100, "ok"],
      };
      const [l, r, p, c] = m[s];
      return <><span class="apv-pr-row"><span>{l}</span><span class="apv-dim">{r}</span></span><span class="apv-pr-track"><i class="apv-seg" data-c={c} style={{ width: `${p}%` }} /></span></>;
    }
    case "ticker": {
      const x: Record<PreviewState, string> = {
        idle: `${TIME} · ${t("appearance.sample.date", "ne 18. 10.")}`,
        work: t("appearance.sample.tickWork", "Claude pracuje: ledge"),
        wait: t("appearance.sample.tickWait", "⚠ Claude čeká na schválení"),
        music: `♪ ${sample.music()}`,
        timer: t("appearance.sample.tickTimer", "⏱ Zbývá 18 min"),
        done: `✓ ${sample.done()}`,
      };
      // klíč = stav → jednorázový posun při změně (žádná smyčka)
      return <For each={[s]}>{() => <span class="apv-tick">{x[s]}</span>}</For>;
    }
    case "minimum":
      return <>{lead(s)}<Batt p={82} /><span>82 %</span><b>{TIME}</b></>;
    case "trio": {
      const mid = s === "wait" ? <><Dot c="wait" ring /> <b>{sample.wait()}</b></> : s === "done" ? <><Glyph g="✓" c="ok" /> {sample.done()}</> : <>{lead(s)}<b>{TIME}</b><span class="apv-dim">{t("appearance.sample.date", "ne 18. 10.")}</span></>;
      return <><span>☀ <b>21°</b></span><i class="apv-sep" />{mid}<i class="apv-sep" /><span>CPU 8 %</span><span>RAM 46 %</span><Batt p={82} /><span>82 %</span></>;
    }
    case "rings": {
      const wait = s === "wait";
      return <><Ring p={42} c={wait ? "call" : "acc"} spin={s === "work"} /><Ring p={86} c={wait ? "call" : "warm"} spin={s === "work"} /><Show when={s === "done"}><Dot c="ok" /></Show><b>{TIME}</b></>;
    }
    default: {
      // Klasický: dnešní proužek (tvar `appearance.strip.style`), položky podle stavu
      if (a.strip.style === "dot") return <For each={(["ok", s === "wait" ? "wait" : "run", "off"] as Tone[])}>{(c) => <Dot c={c} />}</For>;
      const ev: Record<PreviewState, JSX.Element> = {
        idle: null,
        work: <span class="apv-pill" data-c="run"><Dot c="run" />{sample.work()}</span>,
        wait: <span class="apv-pill" data-c="wait"><Dot c="wait" />{sample.wait()}</span>,
        music: <span>♪ {sample.music()}</span>,
        timer: <span>⏱ {sample.timer()}</span>,
        done: <span class="apv-pill" data-c="ok"><Dot c="ok" />{sample.done()}</span>,
      };
      return <><b>{sample.mode()}</b><i class="apv-sep" /><span class="apv-dim">{TIME}</span><Show when={s !== "idle"}><i class="apv-sep" />{ev[s]}</Show></>;
    }
  }
}

/** `compose` = `strip.compose` (jen u `custom`, kolo 23; `ComposePreview`). */
export default function StripLookPreview(q: { look: StripLookId; state: PreviewState; a: Appearance; opacity: number; shape?: LookShape; compose?: unknown }) {
  const lshape = () => (SHAPEABLE_LOOKS.includes(q.look) && q.shape && q.shape !== "round" ? q.shape : undefined);
  const shape = () => stripShape(q.a.strip.style, q.a.strip.size);
  return (
    <span class="apv-scr theme-scope" style={scopeStyle(q.a)} data-pos={q.a.strip.position} aria-hidden="true">
      <span class="apv-win" />
      <Switch>
        <Match when={q.look === "classic"}>
          <span class="apv-sx" data-look="classic" data-shape={q.a.strip.style}
            style={{ "--k": String(STRIP_SIZE[q.a.strip.size] * 0.78), "--so": String(q.opacity), height: `${Math.round(shape().h * 0.78)}px`, "border-radius": `0 0 ${shape().r * 0.78}px ${shape().r * 0.78}px` }}>
            {body("classic", q.state, q.a)}
          </span>
        </Match>
        <Match when={q.look === "custom"}>
          <ComposePreview compose={q.compose} state={q.state} opacity={q.opacity} k={STRIP_SIZE[q.a.strip.size] * 0.78} />
        </Match>
        {/* Neviditelný do události: v klidu a při běžném dění není vidět vůbec nic */}
        <Match when={q.look !== "classic" && (q.look !== "ghost" || q.state === "wait" || q.state === "done")}>
          <span class="apv-sx" data-look={q.look} data-lshape={lshape()} style={{ "--k": String(STRIP_SIZE[q.a.strip.size] * 0.78), "--so": String(q.opacity) }}>{body(q.look, q.state, q.a)}</span>
        </Match>
      </Switch>
    </span>
  );
}
