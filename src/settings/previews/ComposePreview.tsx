// HTML náhled vlastního zavřeného Ledge (`strip.look = "custom"`, `strip.compose`, kolo 23) na „horní hraně
// monitoru“: tvar, rozměry a položky ve slotech vlevo / uprostřed / vpravo ve zvoleném stylu. Data jsou
// UKÁZKOVÁ (přepínač „Co se děje“) — skutečný proužek kreslí Rust (`looks::compose`, `stripdraw`). Nic se
// nehýbe (střídání středu = první událost, točení prstence = statická značka). Kreslí jen viditelné.
import { For, Show, type JSX } from "solid-js";
import { t } from "../../i18n";
import type { PreviewState } from "../appearanceModel";
import { type ComposeItem, type Piece, type StripCompose, drawKind, normCompose } from "../../strip/compose";
import "./previews.css";

type Tone = "run" | "wait" | "ok" | "acc" | "music" | "off";
/** Ukázková hodnota položky ve stavu (`null` = zdroj by nic neměl → položka se nekreslí). */
type Val = { g: string; text: string; num?: string; p?: number; c: Tone; ev?: boolean };

export const ITEM_GLYPH: Record<ComposeItem, string> = {
  time: "◷", date: "▦", claude: "✳", limit5h: "5h", limitWeek: "7d", music: "♪", timer: "⏱",
  cpu: "▤", memory: "▤", battery: "▭", weather: "☀", servers: "▣", repos: "⑂", activity: "✎",
};

export function sampleVal(item: ComposeItem, s: PreviewState): Val | null {
  switch (item) {
    case "time": return { g: "◷", text: "14:32", c: "off" };
    case "date": return { g: "▦", text: t("appearance.sample.date", "ne 18. 10."), c: "off" };
    case "claude":
      return ({
        idle: { g: "✳", text: t("appearance.compose.sampleIdle", "Klid"), c: "off" },
        work: { g: "✳", text: t("appearance.compose.sampleWork", "Pracuje · 2"), num: "2", c: "run", ev: true },
        wait: { g: "✳", text: t("appearance.sample.waitYou", "Čeká na tebe"), num: "1", c: "wait", ev: true },
        music: { g: "✳", text: t("appearance.compose.sampleIdle", "Klid"), c: "off" },
        timer: { g: "✳", text: t("appearance.compose.sampleIdle", "Klid"), c: "off" },
        done: { g: "✓", text: t("appearance.sample.doneShort", "Hotovo"), num: "1", c: "ok", ev: true },
      } as Record<PreviewState, Val>)[s];
    case "limit5h": return { g: "", text: "5h 42 %", num: "42", p: 42, c: "acc" };
    case "limitWeek": return { g: "", text: t("appearance.compose.sampleWeek", "Týden 61 %"), num: "61", p: 61, c: "acc" };
    case "music": return s === "music" ? { g: "♪", text: t("appearance.sample.music", "Skladba · Interpret"), p: 35, c: "music", ev: true } : null;
    case "timer": return s === "timer" ? { g: "⏱", text: t("appearance.sample.timer", "18 min"), num: "18", p: 70, c: "acc", ev: true } : null;
    case "cpu": return { g: "▤", text: "CPU 23 %", num: "23", p: 23, c: "off" };
    case "memory": return { g: "▤", text: "RAM 46 %", num: "46", p: 46, c: "off" };
    case "battery": return { g: "", text: "82 %", num: "82", p: 82, c: "off" };
    case "weather": return { g: "☀", text: "21°", c: "off" };
    case "servers": return { g: "▣", text: t("appearance.compose.sampleServers", "Servery 3"), num: "3", c: "ok" };
    case "repos": return { g: "⑂", text: t("appearance.compose.sampleRepos", "Repa 1"), num: "1", c: "off" };
    case "activity": return activitySample(s);
  }
}

/** „Claude aktivita“: ukázka fáze (Rust `compose::activity_value`); v klidu nic. */
function activitySample(s: PreviewState): Val | null {
  if (s === "wait") return { g: "✳", text: t("appearance.sample.waitYou", "Čeká na tebe"), num: "1", c: "wait", ev: true };
  if (s === "work") return { g: "✳", text: t("appearance.compose.sampleActivity", "Úpravy · main.rs"), p: 50, c: "run", ev: true };
  if (s === "done") return { g: "✓", text: t("appearance.sample.doneShort", "Hotovo"), p: 100, c: "ok", ev: true };
  return null;
}

const Batt = (q: { p: number }) => <i class="apv-batt" style={{ "--p": `${q.p}%` }} />;

/** Jedna položka ve zvoleném stylu (stejné náhrady jako Rust `compose::parts`). */
export function PieceView(q: { piece: Piece; state: PreviewState }): JSX.Element {
  const v = () => sampleVal(q.piece.item, q.state);
  const icon = (val: Val, badge: boolean): JSX.Element => {
    if (q.piece.item === "battery") return <><Batt p={val.p ?? 0} />{badge ? <span>{val.text}</span> : null}</>;
    if (!val.g) return <span>{val.text}</span>;
    return badge && val.num ? <span class="apv-badge"><i class="apv-g" data-c={val.c}>{val.g}</i><sup>{val.num}</sup></span> : <i class="apv-g" data-c={val.c}>{val.g}</i>;
  };
  return (
    <Show when={v()}>
      {(val) => {
        const x = val();
        switch (drawKind(q.piece.item, q.piece.style, x.p !== undefined)) {
          case "badge": return icon(x, true);
          case "text": return x.c === "wait" ? <b class="apv-ct" data-c="wait">{x.text}</b> : <span class="apv-ct" data-c={x.c === "off" ? undefined : x.c}>{x.text}</span>;
          case "dot": return <i class="apv-d" data-c={x.ev || x.c !== "off" ? x.c : "off"} />;
          case "ring":
            return <i class="apv-ring2" data-c={x.c} classList={{ "apv-ring-spin": q.state === "work" }} style={{ "--p": `${x.p ?? 0}%` }} />;
          case "meter":
            return <span class="apv-meter" data-c={x.c}><i style={{ width: `${x.p}%` }} /></span>;
          default: return icon(x, false);
        }
      }}
    </Show>
  );
}

/** Střed se střídáním (`rotate`): ukáže první událost, jinak první položku (jako Rust bez časovače). */
function centerOf(c: StripCompose, s: PreviewState): Piece[] {
  if (!c.rotate || c.center.length < 2) return c.center;
  const ev = c.center.find((p) => sampleVal(p.item, s)?.ev);
  return [ev ?? c.center.find((p) => sampleVal(p.item, s)) ?? c.center[0]];
}

/** Rozměry těla v px náhledu (`k` = měřítko; zaoblení jako Rust `compose::spec`). */
export function bodyStyle(c: StripCompose, k: number, opacity: number): JSX.CSSProperties {
  const h = c.height * k;
  const r = c.shape === "drop" ? (c.round / 100) * h : ((c.round / 100) * h) / 2;
  const pill = c.shape === "island" || c.shape === "corner";
  return {
    "--k": String(k),
    "--so": String(opacity),
    "min-width": `${c.width * k}px`,
    height: `${h}px`,
    "border-radius": pill ? `${r}px` : `0 0 ${r}px ${r}px`,
    "--ear": `${Math.min(18, Math.max(6, c.height * 0.42)) * k}px`,
    "--rd": `${Math.min(30, Math.max(10, c.height * 0.6)) * k}px`,
    "font-size": `${Math.min(20, Math.max(9.5, c.height * 0.45)) * k}px`,
  };
}

export default function ComposePreview(q: { compose: unknown; state: PreviewState; opacity: number; k?: number; pos?: string }) {
  const c = () => normCompose(q.compose);
  const k = () => q.k ?? 0.78;
  const hidden = () => c().hideIdle && q.state !== "wait" && q.state !== "done";
  const all = () => [...c().left, ...c().center, ...c().right];
  return (
    <Show when={!hidden()}>
      <Show
        when={c().shape !== "line"}
        fallback={
          <span class="apv-sx" data-look="line" style={{ height: `${Math.max(2, c().height * k())}px` }}>
            <For each={all()}>{(p) => { const v = sampleVal(p.item, q.state); return <i class="apv-seg" data-c={v && (v.ev || v.p !== undefined) ? v.c : "off"} style={{ flex: "1" }} />; }}</For>
          </span>
        }
      >
        <span class="apv-sx" data-look="custom" data-cshape={c().shape} style={bodyStyle(c(), k(), q.opacity)}>
          <span class="apv-cs" data-slot="left"><For each={c().left}>{(p) => <PieceView piece={p} state={q.state} />}</For></span>
          <span class="apv-cs" data-slot="center"><For each={centerOf(c(), q.state)}>{(p) => <PieceView piece={p} state={q.state} />}</For></span>
          <span class="apv-cs" data-slot="right"><For each={c().right}>{(p) => <PieceView piece={p} state={q.state} />}</For></span>
        </span>
      </Show>
    </Show>
  );
}
