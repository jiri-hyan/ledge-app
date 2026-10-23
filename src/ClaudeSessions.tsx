// Karta Claude v šuplíku: všechny Claude Code session na PC (Rust `sessions.rs` → `Snapshot.sessions`,
// seřazené čeká > pracuje > nečinná > skončila s prací), schvalování (`PendingRow` + „Vždy povolit“ /
// „Zamítnout s důvodem…“ v `sessions/PermExtra.tsx`, náhled diffu `sessions/DiffPreview.tsx`, review plánu
// `sessions/PlanReview.tsx`), otázky Clauda
// (`QuestionCard` pod řádkem session), subagenti, panel Git session (`sessions/GitPanel.tsx`) a klik = otevřít session (`api.sessionOpen`, Win32 v `winfocus.rs`).
// Strop řádků (vlna 9, `layout/cap.ts`): nejvýš `limit` session (výchozí 4, ⋯ karty / Nastavení), pod nimi
// „Zobrazit další (k) · +3 nečinné“ ↔ „Méně“ (rozbalení zvlášť pro každou záložku). Kdo čeká na tebe
// (schválení, otázka), je vždy nahoře a nikdy se neschová; pak pracující, nečinné podle poslední aktivity
// a skončené (`sessions/order.ts`).
import { createSignal, For, Show, type JSX } from "solid-js";
import { api, type Snapshot, type SessionView, type Pending, type SubagentDone, type Ask } from "./api";
import { ago, uptime } from "./format";
import { say } from "./ui/toast";
import { logoSrc } from "./logos";
import { t, tp } from "./i18n";
import QuestionCard from "./questions/QuestionCard";
import PermExtra from "./sessions/PermExtra";
import LimitsRow from "./limits/LimitsRow";
import DiffPreview from "./sessions/DiffPreview";
import PlanReview from "./sessions/PlanReview";
import GitPanel from "./sessions/GitPanel";
import { addonsMissing, planTitle } from "./sessions/plan";
import CostsSection from "./costs/CostsSection";
import AgentActions, { BgPill } from "./sessions/AgentActions";
import { statusPill } from "./sessions/agentview";
import { sectionOn } from "./cards/sections";
import "./sessions.css";
import { gotoSettingsPanel } from "./palette/goto";
import ShowMore, { useCap } from "./layout/ShowMore";
import { PartSlot, PartsBox } from "./layout/PartSlot";
import { hiddenSummary, needsYou, orderSessions } from "./sessions/order";

/** stav → barva pilulky a popisek (funkce, ať se popisek přeloží až při vykreslení) */
const STATUS: Record<SessionView["status"], [string, () => string]> = {
  waiting: ["a", () => t("sessions.status.waiting", "čeká na tebe")],
  working: ["v", () => t("sessions.status.working", "pracuje")],
  idle: ["n", () => t("sessions.status.idle", "nečinná")],
  ended: ["r", () => t("sessions.status.ended", "skončila")],
};

const lastSeg = (p: string) => p.split(/[\\/]/).filter(Boolean).pop() ?? p;
/** Délka běhu subagenta; neznámá (start přišel před zapojením hooků) = nic, ne „?“. */
const dur = (ms: number | null) => {
  if (ms == null) return "";
  const m = Math.round(ms / 60_000);
  return ms < 60_000 ? `${Math.max(1, Math.round(ms / 1000))} s` : m < 60 ? `${m} min` : `${Math.floor(m / 60)} h ${m % 60} min`;
};

export default function ClaudeSessions(p: { s: Snapshot; now: number; onSettings: () => void; menu?: JSX.Element; page?: string; limit?: number }) {
  const live = () => p.s.sessions.filter((x) => x.status !== "ended").length;
  const asked = (sid: string) => p.s.pending.some((x) => x.sessionId === sid) || p.s.questions.some((x) => x.sessionId === sid);
  const cap = useCap(() => ({ page: p.page, card: "claude", limit: p.limit }), () => orderSessions(p.s.sessions, asked), (x) => needsYou(x, asked));
  const summary = () => {
    const h = hiddenSummary(cap.hidden());
    return [
      h.working ? tp("sessions.cap.working", h.working, ["+{n} pracující", "+{n} pracující", "+{n} pracujících"]) : "",
      h.idle ? tp("sessions.cap.idle", h.idle, ["+{n} nečinná", "+{n} nečinné", "+{n} nečinných"]) : "",
      h.ended ? tp("sessions.cap.ended", h.ended, ["+{n} skončená", "+{n} skončené", "+{n} skončených"]) : "",
    ].filter(Boolean).join(", ");
  };
  const pendingFor = (sid: string) => p.s.pending.filter((x) => x.sessionId === sid);
  const orphans = () => p.s.pending.filter((x) => !p.s.sessions.some((ss) => ss.id === x.sessionId));
  const asksFor = (sid: string) => p.s.questions.filter((x) => x.sessionId === sid);
  const orphanAsks = () => p.s.questions.filter((x) => !p.s.sessions.some((ss) => ss.id === x.sessionId));
  const needHooks = () => p.s.hooks.state === "none" || p.s.hooks.state === "partial";
  /** hooky z dřívější verze bez otázek Clauda / review plánu → doplnit jedním klikem (ostatní už šéf zapojil) */
  const onlyAddons = () => addonsMissing(p.s.hooks);
  async function addQuestions() {
    const q = p.s.hooks.missing.includes("PreToolUse");
    try {
      await api.hooksInstall();
      say(q ? t("questions.hooksAdded", "Otázky Clauda zapojené — platí pro nově spuštěné session") : t("sessions.plan.hookAdded", "Review plánu zapojené — platí pro nově spuštěné session"));
    } catch (e) {
      say(String(e), true);
    }
  }
  const row = (ss: SessionView) => <SessionRow ss={ss} s={p.s} now={p.now} pending={pendingFor(ss.id)} asks={asksFor(ss.id)} />;
  return (
    <div class="card" data-anim>
      <h4><span class="ic" style="background:var(--violet)" />Claude <span class="n">{live() ? tp("sessions.count", live(), "{n} session") : ""}</span>{p.menu}</h4>
      {/* části karty jdou skrýt (`drawer.sections.claude*`) i přeskládat (`layout…parts`, `layout/PartSlot.tsx`);
          skrytá útrata nespouští přepočet (`costs_refresh`), skryté části se nekreslí vůbec */}
      <PartsBox card="claude">
      <PartSlot card="claude" k="claudeLimits" hidden={!sectionOn(p.s.sections, "claudeLimits")} titled empty={!p.s.limits}><LimitsRow s={p.s} now={p.now} onSettings={p.onSettings} /></PartSlot>
      <PartSlot card="claude" k="body">
      <For each={orphans()}>{(pd) => <PendingRow pd={pd} />}</For>
      <For each={orphanAsks()}>{(a) => <QuestionCard ask={a} s={p.s} />}</For>
      <For each={cap.shown()}>{row}</For>
      <Show when={cap.toggles()}>
        <ShowMore open={cap.open()} hidden={cap.hidden().length} summary={summary()} onToggle={cap.toggle} />
      </Show>
      <Show when={p.s.sessions.length === 0 && p.s.pending.length === 0 && p.s.questions.length === 0}>
        <div class="empty">{t("sessions.empty", "Žádná běžící session Claude Code.")}</div>
      </Show>
      <Show when={!p.s.claudeEnabled} fallback={
        <Show when={needHooks()}>
          <Show when={onlyAddons()} fallback={
            <button class="hint-link" onClick={() => { p.onSettings(); gotoSettingsPanel("f-claude.hooks"); }} title={p.s.hooks.missing.length ? t("sessions.hooksMissing", "Chybí: {list}", { list: p.s.hooks.missing.join(", ") }) : undefined}>{t("sessions.connectHooks", "Zapojit hooky pro schvalování a subagenty")}</button>
          }>
            <button class="hint-link" onClick={addQuestions} title={p.s.hooks.hint ?? undefined}>
              {p.s.hooks.missing.includes("PreToolUse") ? t("questions.addHook", "Doplnit otázky Clauda (odpovídat na ně v Ledge)") : t("sessions.plan.addHook", "Doplnit review plánu (schvalovat plány v Ledge)")}
            </button>
          </Show>
        </Show>
      }>
        <div class="empty">{t("sessions.bridgeOff", "Most pro Claude je vypnutý v Nastavení — bez schvalování a subagentů.")}</div>
      </Show>
      </PartSlot>
      <PartSlot card="claude" k="claudeCosts" hidden={!sectionOn(p.s.sections, "claudeCosts")}><CostsSection s={p.s} now={p.now} /></PartSlot>
      </PartsBox>
    </div>
  );
}

function SessionRow(p: { ss: SessionView; s: Snapshot; now: number; pending: Pending[]; asks: Ask[] }) {
  const [open, setOpen] = createSignal(false);
  const [git, setGit] = createSignal(false);
  const proj = () => p.s.projects.find((x) => x.id === p.ss.projectId);
  const logo = () => { const pr = proj(); return pr ? logoSrc(pr) : undefined; };
  // Agent View ví víc (na co čeká, selhala / zastavena) → přesnější pilulka, jinak obecná
  const st = (): [string, () => string] => { const o = statusPill(p.ss); return o ? [o[0], () => o[1]] : STATUS[p.ss.status]; };
  const when = () => {
    if (p.ss.status === "ended") return t("sessions.ended", "ukončena");
    if (p.ss.status === "idle") return ago(p.ss.since ?? p.ss.updatedAt, p.now);
    return p.ss.since ? uptime(p.ss.since, p.now) : "";
  };
  const sub = () => [proj()?.name ?? lastSeg(p.ss.cwd), p.ss.title].filter(Boolean).join(" · ");
  const running = () => p.ss.subagents.running;
  const done = () => p.ss.subagents.done;
  const agentLine = () => {
    const r = running();
    if (r.length) return tp("sessions.agents", r.length, ["{n} agent: {types}", "{n} agenti: {types}", "{n} agentů: {types}"], { types: [...new Set(r.map((x) => x.agentType))].join(", ") });
    const d = done()[0];
    return d ? t("sessions.lastAgent", "naposledy {type}", { type: d.agentType }) + (d.durationMs != null ? ` (${dur(d.durationMs)})` : "") : "";
  };
  async function go() {
    if (!p.ss.canOpen) return say(p.ss.openHint, true);
    try {
      await api.sessionOpen(p.ss.id);
    } catch (e) {
      say(String(e), true);
    }
  }
  return (
    <>
      <div class="row sess" classList={{ off: !p.ss.canOpen }}>
        <button class="sess-hit" onClick={go} title={p.ss.openHint}>
          <span class="sess-logo" classList={{ "has-img": !!logo() }} style={{ background: proj() && !logo() ? proj()!.accent : undefined }}>
            <Show when={logo()} fallback={proj()?.monogram ?? "›"}>{(src) => <img src={src()} alt="" draggable={false} />}</Show>
          </span>
          <span class="grow">
            <span class="name">{p.ss.name}</span>
            <span class="sub">{sub()}</span>
          </span>
        </button>
        <Show when={p.ss.uncommitted.length > 0}>
          <span class="pill a" title={t("sessions.uncommitted", "Nezacommitováno:\n{list}", { list: p.ss.uncommitted.join("\n") })}>●{p.ss.uncommitted.length}</span>
        </Show>
        <Show when={p.ss.unpushed > 0}>
          <span class="pill a" title={t("sessions.unpushed", "Commity session, které nejsou na žádném remote")}>↑{p.ss.unpushed}</span>
        </Show>
        <BgPill ss={p.ss} />
        <span class={`pill ${st()[0]}`}>{st()[1]()}</span>
        <span class="meta">{when()}</span>
      </div>
      <Show when={sectionOn(p.s.sections, "claudeSubagents") && agentLine()}>
        <button class="subag" onClick={() => setOpen(!open())} aria-expanded={open()}>{open() ? "▾" : "▸"} {agentLine()}</button>
        <Show when={open()}>
          <div class="subag-list">
            <For each={running()}>{(r) => <div class="sub">● {r.agentType} · {t("sessions.agentRunning", "běží {time}", { time: uptime(r.since, p.now) })}</div>}</For>
            <For each={done()}>{(d: SubagentDone) => <div class="sub" title={d.message ?? ""}>✓ {d.agentType}{d.durationMs != null ? ` · ${dur(d.durationMs)}` : ""}{d.message ? ` · ${d.message}` : ""}</div>}</For>
          </div>
        </Show>
      </Show>
      <AgentActions ss={p.ss} />
      <Show when={sectionOn(p.s.sections, "claudeGit") && p.ss.cwd}>
        <button class="subag" onClick={() => setGit(!git())} aria-expanded={git()} title={p.ss.cwd}>{git() ? "▾" : "▸"} {t("sessions.git.toggle", "Git")}</button>
        <Show when={git()}><GitPanel id={p.ss.id} status={p.ss.status} /></Show>
      </Show>
      <For each={p.pending}>{(pd) => <PendingRow pd={pd} />}</For>
      <For each={p.asks}>{(a) => <QuestionCard ask={a} s={p.s} inSession />}</For>
    </>
  );
}

/** Čekající schválení a otázky Clauda, když karta Claude není vidět (záložka Osobní nebo karta vypnutá
 *  v Nastavení) — Ledge je drží a Claude čeká, takže nesmí zůstat neviditelné. */
export function ClaudeInbox(p: { s: Snapshot }) {
  return (
    <Show when={p.s.pending.length + p.s.questions.length > 0}>
      <div class="card" data-anim>
        <h4><span class="ic" style="background:var(--amber)" />{t("sessions.inbox", "Claude čeká na tebe")} <span class="n">{p.s.pending.length + p.s.questions.length}</span></h4>
        <For each={p.s.pending}>{(pd) => <PendingRow pd={pd} />}</For>
        <For each={p.s.questions}>{(a) => <QuestionCard ask={a} s={p.s} />}</For>
      </div>
    </Show>
  );
}

export function PendingRow(p: { pd: Pending }) {
  return (
    <>
      <div class="row">
        <span class="pill a">{p.pd.plan ? t("sessions.plan.pill", "Plán") : p.pd.tool}</span>
        <div class="grow"><div class="code" title={p.pd.summary}>{p.pd.plan ? planTitle(p.pd) : p.pd.summary}</div></div>
        <span class="r">
          <button class="mini" onClick={() => api.decide(p.pd.id, false)}>{t("sessions.deny", "Zamítnout")}</button>
          <button class="mini ok" onClick={() => api.decide(p.pd.id, true)}>{p.pd.plan ? t("sessions.plan.approve", "Schválit") : t("sessions.allow", "Povolit")}</button>
        </span>
      </div>
      <Show when={p.pd.preview}>{(pv) => <DiffPreview pv={pv()} />}</Show>
      <Show when={p.pd.plan}>{(pl) => <PlanReview pd={p.pd} plan={pl()} />}</Show>
      <PermExtra pd={p.pd} />
    </>
  );
}
