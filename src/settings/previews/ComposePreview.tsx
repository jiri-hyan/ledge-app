// HTML náhled vlastního zavřeného Ledge (`strip.look = "custom"`, `strip.compose`, kolo 23) na „horní hraně
// monitoru“: tvar, rozměry a položky ve slotech vlevo / uprostřed / vpravo ve zvoleném stylu. Data jsou
// UKÁZKOVÁ (přepínač „Co se děje“) — skutečný proužek kreslí Rust (`looks::compose`, `stripdraw`). Nic se
// nehýbe (střídání středu = první událost, točení prstence = statická značka). Kreslí jen viditelné.
// Kolo 25: výzdoba (`deco` linka / obrys / záře / přechod, `fill`, `sep` kapsle / ostrůvky / tečka / čárka, mono),
// nové styly (ikona + hodnota, pruh, průběh, podklad, Claude tečka + 5h, prstenec 5h) a barva dílu.
import { For, Show, type JSX } from "solid-js";
import { t } from "../../i18n";
import type { PreviewState } from "../appearanceModel";
import { type ComposeItem, type Piece, type StripCompose, SIZE_MULT, SPACE_MULT, drawKind, normCompose } from "../../strip/compose";
import type { LiveState } from "../../strip/edgefx";
import { EdgeLayer, edgeLook, liveOf } from "./EdgeFx";

const TONE: Record<Tone, string> = { run: "var(--blue)", wait: "var(--amber)", ok: "var(--green)", acc: "var(--accent)", music: "var(--amber)", off: "var(--t3)" };
/** Ukázková historie CPU / paměti pro „průběh“ (0–100). */
const SPARK = [18, 26, 22, 35, 30, 44, 38, 29, 33, 23];
const cssColor = (c: string | undefined) => (!c ? undefined : c === "accent" ? "var(--accent)" : c);
import "./previews.css";

type Tone = "run" | "wait" | "ok" | "acc" | "music" | "off";
/** Ukázková hodnota položky ve stavu (`null` = zdroj by nic neměl → položka se nekreslí). */
type Val = { g: string; text: string; num?: string; p?: number; c: Tone; ev?: boolean };

export const ITEM_GLYPH: Record<ComposeItem, string> = {
  time: "◷", date: "▦", claude: "✳", limit5h: "5h", limitWeek: "7d", music: "♪", timer: "⏱",
  cpu: "▤", memory: "▤", gpu: "▤", battery: "▭", weather: "☀", servers: "▣", repos: "⑂", activity: "✎",
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
    case "gpu": return { g: "▤", text: "GPU 14 %", num: "14", p: 14, c: "off" };
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
const Spark = () => (
  <svg class="apv-spark" viewBox="0 0 30 12" aria-hidden="true">
    <polyline points={SPARK.map((v, i) => `${(i * 30) / (SPARK.length - 1)},${12 - (v / 100) * 12}`).join(" ")} />
  </svg>
);

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
        const claude = q.piece.item === "claude";
        const word = <span class="apv-ct" data-c={x.c === "off" ? undefined : x.c}>{x.text}</span>;
        switch (drawKind(q.piece.item, q.piece.style, x.p !== undefined)) {
          case "badge": return icon(x, true);
          case "iconText":
            if (q.piece.item === "battery") return icon(x, true);
            return x.g ? <>{icon(x, false)}{word}</> : word;
          case "under":
            return <span class="apv-under" data-c={x.c}>{x.text}<i style={{ width: `${x.p ?? 0}%` }} /></span>;
          case "spark":
            return <span class="apv-sp" data-c="acc"><Spark />{word}</span>;
          case "dotPct":
            return <><i class="apv-d" data-c={x.ev || x.c !== "off" ? x.c : "off"} /><span class="apv-ct">{claude ? "42 %" : x.text}</span></>;
          case "text": return x.c === "wait" ? <b class="apv-ct" data-c="wait">{x.text}</b> : <span class="apv-ct" data-c={x.c === "off" ? undefined : x.c}>{x.text}</span>;
          case "dot": return <i class="apv-d" data-c={x.ev || x.c !== "off" ? x.c : "off"} />;
          case "ring":
            if (claude && q.piece.style === "ring5h") return <i class="apv-ring2" data-c={x.ev ? x.c : "acc"} style={{ "--p": "42%" }} />;
            return <i class="apv-ring2" data-c={x.c} classList={{ "apv-ring-spin": q.state === "work" }} style={{ "--p": `${x.p ?? 0}%` }} />;
          case "meter":
            return <span class="apv-meter" data-c={x.c}><i style={{ width: `${x.p}%` }} /></span>;
          default: return icon(x, false);
        }
      }}
    </Show>
  );
}

const calmInIdle = (item: ComposeItem) => item === "claude" || item === "activity" || item === "music" || item === "timer" || item === "servers" || item === "repos" || item === "cpu" || item === "memory" || item === "gpu";

/** Díl v obálce: barevný podklad (`chip`), vlastní barva dílu (`--pcol`), kapsle / oddělovače přes CSS. */
function PieceBox(q: { piece: Piece; state: PreviewState }) {
  const v = () => (q.piece.onlyActive && q.state === "idle" && calmInIdle(q.piece.item) ? null : sampleVal(q.piece.item, q.state));
  // kostka kolem dílu: `bg` má přednost před stylem „podklad“ (jako Rust `chip_of`)
  const chip = () => cssColor(q.piece.bg) ?? (q.piece.style === "chip" ? cssColor(q.piece.color) ?? (v() && v()!.c !== "off" ? TONE[v()!.c] : "var(--accent)") : undefined);
  // velikost dílu = `zoom` (jako Rust: části, mezery i písmo dílu v jednom měřítku); `ink` barví jen text
  const zoom = () => (q.piece.size && q.piece.size !== "normal" ? SIZE_MULT[q.piece.size] : undefined);
  return (
    <Show when={v()}>
      <span class="apv-pc" classList={{ pcol: !!q.piece.color }} data-chip={chip() ? "" : undefined} style={{ "--pcol": cssColor(q.piece.color), "--chip": chip(), color: cssColor(q.piece.ink), zoom: zoom() }}>
        <PieceView piece={q.piece} state={q.state} />
      </span>
    </Show>
  );
}

/** Výzdoba jako CSS proměnné: barvy hrany (kolo 28 `EdgeFx.edgeLook`), pozadí (černá, sklo, tón, přechod, nic). */
export function decoStyle(c: StripCompose, s: PreviewState, fx?: LiveState): JSX.CSSProperties {
  const tint = "color-mix(in srgb, var(--accent) 22%, #000)";
  const dir = { right: "90deg", down: "180deg", diag: "135deg" }[c.fill.dir];
  const fill = {
    black: "#000",
    glass: "rgba(23, 26, 33, 0.62)",
    tint,
    gradient: `linear-gradient(${dir}, ${c.fill.from || tint}, ${c.fill.via ? `${c.fill.via}, ` : ""}${c.fill.to || "#000"})`,
    none: "transparent",
  }[c.fill.kind];
  return { ...edgeLook(c.deco, fx ?? liveOf(s), !!fx).vars, "--fill": fill };
}

/** Střed se střídáním (`rotate`): ukáže první událost, jinak první položku (jako Rust bez časovače). */
function centerOf(c: StripCompose, s: PreviewState): Piece[] {
  if (!c.rotate || c.center.length < 2) return c.center;
  const ev = c.center.find((p) => sampleVal(p.item, s)?.ev);
  return [ev ?? c.center.find((p) => sampleVal(p.item, s)) ?? c.center[0]];
}

/** Odhad šířky obsahu při `k = 1`, aby miniatury v galerii („Hotové designy“) zmenšily měřítko `k` a neořízly kraje. */
function estContentW(c: StripCompose, s: PreviewState): number {
  const sp = SPACE_MULT[c.text.spacing];
  const pw = (p: Piece) => {
    const base =
      p.style === "dot" ? 14 :
      p.style === "icon" || p.style === "ring" || p.style === "ring5h" || p.style === "badge" ? 28 :
      p.style === "graph" ? 36 :
      p.style === "spark" ? 82 :
      p.item === "claude" ? 148 :
      p.item === "music" ? 116 :
      p.item === "time" ? 54 : 84;
    const pill = p.bg || p.style === "chip" || c.sep === "pills" || c.sep === "islands" ? 16 : 0;
    return base * SIZE_MULT[p.size ?? "normal"] * c.text.size + pill;
  };
  const slotW = (ps: Piece[]) => ps.reduce((sum, p) => sum + pw(p), 0) + Math.max(0, ps.length - 1) * 10 * sp;
  const lw = slotW(c.left);
  const cw = slotW(centerOf(c, s));
  const rw = slotW(c.right);
  const sides = cw > 0 ? 2 * Math.max(lw, rw) : lw + rw;
  return Math.round(sides + cw + 36 * sp);
}

/** Rozměry těla v px náhledu (`k` = měřítko; zaoblení jako Rust `compose::spec`). */
export function bodyStyle(c: StripCompose, k: number, opacity: number, minW?: number): JSX.CSSProperties {
  const h = c.height * k;
  const r = c.shape === "drop" ? (c.round / 100) * h : ((c.round / 100) * h) / 2;
  const pill = c.shape === "island" || c.shape === "corner";
  return {
    "--k": String(k),
    "--so": String(opacity),
    "min-width": `${(minW ?? c.width) * k}px`,
    height: `${h}px`,
    "border-radius": pill ? `${r}px` : `0 0 ${r}px ${r}px`,
    "--ear": `${Math.min(18, Math.max(6, c.height * 0.42)) * k}px`,
    "--rd": `${Math.min(30, Math.max(10, c.height * 0.6)) * k}px`,
    "font-size": `${Math.min(20, Math.max(9.5, c.height * 0.45)) * c.text.size * k}px`,
    "--sp": String(SPACE_MULT[c.text.spacing]),
    "font-weight": c.text.weight === "bold" ? 700 : c.text.weight === "semi" ? 600 : undefined,
  };
}

/** `fx` = „Vyzkoušet“ stavu živé hrany z editoru (i limit, který přepínač „Co se děje“ nemá). */
export default function ComposePreview(q: { compose: unknown; state: PreviewState; opacity: number; k?: number; pos?: string; fx?: LiveState; fitWidth?: number }) {
  const c = () => normCompose(q.compose);
  const ef = () => edgeLook(c().deco, q.fx ?? liveOf(q.state), !!q.fx);
  const naturalW = () => Math.max(c().width, estContentW(c(), q.state));
  const k = () => {
    const base = q.k ?? 0.78;
    const ear = c().shape === "notch" ? 28 : 0;
    return q.fitWidth ? Math.min(base, q.fitWidth / Math.max(160, naturalW() + ear)) : base;
  };
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
        <span class="apv-sx" data-look="custom" data-cshape={c().shape} data-edge={ef().fx} data-len={c().deco.len} data-run={ef().run || (c().fill.flow && (q.state === "work" || q.state === "wait" || c().deco.always)) ? "" : undefined} data-fflow={c().fill.flow && c().fill.kind === "gradient" ? "" : undefined} data-fill={c().fill.kind} data-sep={c().sep} data-mono={c().mono ? "" : undefined}
          style={{ ...bodyStyle(c(), k(), q.opacity, naturalW()), ...decoStyle(c(), q.state, q.fx) }}>
          <span class="apv-cs" data-slot="left"><For each={c().left}>{(p) => <PieceBox piece={p} state={q.state} />}</For></span>
          <span class="apv-cs" data-slot="center"><For each={centerOf(c(), q.state)}>{(p) => <PieceBox piece={p} state={q.state} />}</For></span>
          <span class="apv-cs" data-slot="right"><For each={c().right}>{(p) => <PieceBox piece={p} state={q.state} />}</For></span>
          <EdgeLayer fx={ef().fx} run={ef().run} />
        </span>
      </Show>
    </Show>
  );
}
