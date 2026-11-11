// Zadání pro Claude Code v poznámce — čisté funkce (bez DOM a bez api): body `[ ]` / `[~]` / `[x]` se snímky,
// sekce Otázky a Log, stav → akce tlačítka, náklad, `@cesta` odkazy a token pod kurzorem, kotva snímku.
// Řádky se počítají od 0 v těle bez frontmatteru (stejně jako `note_toggle_task`).

import type { ImageIn } from "../types";
export type TaskState = "open" | "review" | "done";
export type BriefStatus = "draft" | "ready" | "running" | "waiting" | "review" | "done";

export interface TaskImage { line: number; caption: string; src: string }
export interface BriefTask { line: number; indent: number; state: TaskState; text: string; evidence: string | null; images: TaskImage[] }

const TASK = /^(\s*)(?:[-*+]|\d+[.)]) \[([ xX~])\](?: (.*))?$/;
const IMAGE = /^(\s+)!\[([^\]]*)\]\(([^)\s]+)\)\s*$/;
const EVIDENCE = /\s+[—–-]\s*důkaz:\s*(.*)$/i;

/** Body se stavem, důkazem (`— důkaz: X`) a snímky odsazenými pod bodem. */
export function parseTasks(body: string): BriefTask[] {
  const lines = body.split("\n");
  const out: BriefTask[] = [];
  for (let i = 0; i < lines.length; i++) {
    const m = TASK.exec(lines[i]);
    if (!m) continue;
    const raw = m[3] ?? "";
    const ev = EVIDENCE.exec(raw);
    const task: BriefTask = {
      line: i,
      indent: m[1].length,
      state: m[2] === "~" ? "review" : m[2] === " " ? "open" : "done",
      text: ev ? raw.slice(0, ev.index) : raw,
      evidence: ev ? ev[1].trim() || null : null,
      images: [],
    };
    for (let j = i + 1; j < lines.length; j++) {
      const im = IMAGE.exec(lines[j]);
      if (!im || im[1].length <= task.indent) break;
      task.images.push({ line: j, caption: im[2], src: im[3] });
    }
    out.push(task);
  }
  return out;
}

/** Nový řádek bodu po akci Potvrdit / Vrátit (text a důkaz zůstanou; `[x]` důkaz zachová, `[ ]` ho zahodí). */
export function taskLine(t: BriefTask, state: TaskState): string {
  const mark = state === "done" ? "x" : state === "review" ? "~" : " ";
  const ev = state === "open" || !t.evidence ? "" : ` — důkaz: ${t.evidence}`;
  const sp = t.text ? ` ${t.text}` : "";
  return `${" ".repeat(t.indent)}- [${mark}]${sp}${ev}`;
}

export interface Question { index: number; line: number; text: string; answer: string | null }
const Q = /^\s*[-*+] ❓\s*(.*)$/;
const A = /^\s+[-*+] ↳\s*(.*)$/;
const H2 = /^##\s+(.*?)\s*$/;

/** Řádky sekce `## <název>` (bez nadpisu) s jejich čísly v těle. */
export function section(body: string, name: string): { line: number; text: string }[] {
  const lines = body.split("\n");
  const out: { line: number; text: string }[] = [];
  let inside = false;
  lines.forEach((l, i) => {
    const h = H2.exec(l);
    if (h) inside = h[1].toLowerCase() === name.toLowerCase();
    else if (inside) out.push({ line: i, text: l });
  });
  return out;
}

/** Otázky ze sekce `## Otázky`; `index` = pořadí otázky (od 0) pro `note_answer`. */
export function parseQuestions(body: string): Question[] {
  const rows = section(body, "Otázky");
  const out: Question[] = [];
  for (let k = 0; k < rows.length; k++) {
    const q = Q.exec(rows[k].text);
    if (!q) continue;
    const a = k + 1 < rows.length ? A.exec(rows[k + 1].text) : null;
    out.push({ index: out.length, line: rows[k].line, text: q[1], answer: a && a[1].trim() ? a[1].trim() : null });
  }
  return out;
}

export const allAnswered = (qs: Question[]) => qs.length > 0 && qs.every((q) => q.answer !== null);

/** Obsah sekce `## Log` (jen k zobrazení), bez prázdných okrajů. */
export function logLines(body: string): string[] {
  return section(body, "Log").map((r) => r.text).join("\n").trim().split("\n").filter((l, i, a) => l !== "" || (i > 0 && i < a.length - 1));
}

export type SendBlock = "project" | "key" | "goal" | "state";
/** Co smí tlačítko: poslat (draft/ready/nic), pokračovat (waiting + vše zodpovězeno), zastavit (running). */
export type BriefAction = { kind: "send"; block: SendBlock | null } | { kind: "resume"; block: "answers" | null } | { kind: "stop" } | { kind: "none" };

export function briefAction(n: { status?: BriefStatus | null; project: string | null; body: string }, hasKey: boolean | undefined): BriefAction {
  const s = n.status;
  if (s === "running") return { kind: "stop" };
  if (s === "waiting") return { kind: "resume", block: allAnswered(parseQuestions(n.body)) ? null : "answers" };
  if (s === "review" || s === "done") return { kind: "none" };
  const goal = n.body.split("\n").find((l) => l.trim() !== "" && !H2.test(l));
  const block: SendBlock | null = !n.project ? "project" : !goal ? "goal" : hasKey === false ? "key" : null;
  return { kind: "send", block };
}

/** Počet bodů čekajících na potvrzení. */
export const reviewCount = (ts: BriefTask[]) => ts.filter((t) => t.state === "review").length;

/** `$1.23`, `$1.23 / $5.00`, jen limit `limit $5.00`; nic z reálných polí = null. */
export function costText(cost?: number | null, budget?: number | null): string | null {
  const ok = (v: unknown): v is number => typeof v === "number" && Number.isFinite(v) && v >= 0;
  if (ok(cost) && ok(budget)) return `$${cost.toFixed(2)} / $${budget.toFixed(2)}`;
  if (ok(cost)) return `$${cost.toFixed(2)}`;
  if (ok(budget)) return `limit $${budget.toFixed(2)}`;
  return null;
}

// ---------- @ odkazy na soubory projektu ----------

export interface AtRef { path: string; line: number | null }
export type AtPart = { text: string; ref?: AtRef };
const AT = /(^|[\s(])@([^\s@:,;()<>"'`]+)(?::(\d+))?/g;

/** Rozseká text na prosté kusy a `@rel/cesta[:řádek]` odkazy (cesta musí mít `/` nebo `.`, ať `@jmeno` není soubor). */
export function splitAtRefs(text: string): AtPart[] {
  const out: AtPart[] = [];
  let last = 0;
  for (const m of text.matchAll(AT)) {
    let path = m[2].replace(/[.!?]+$/, "");
    if (!/[/.]/.test(path) || !path) continue;
    const start = (m.index ?? 0) + m[1].length;
    const trimmed = m[2].length - path.length;
    const end = start + 1 + path.length + (m[3] && !trimmed ? 1 + m[3].length : 0);
    if (start > last) out.push({ text: text.slice(last, start) });
    out.push({ text: text.slice(start, end), ref: { path: path.replace(/\\/g, "/"), line: m[3] && !trimmed ? Number(m[3]) : null } });
    last = end;
  }
  if (last < text.length) out.push({ text: text.slice(last) });
  return out;
}

export interface AtToken { start: number; query: string }
/** `@token` těsně před kurzorem (po mezeře / na začátku řádku, bez mezer uvnitř); jinak null. */
export function atToken(text: string, caret: number): AtToken | null {
  for (let i = caret - 1; i >= 0; i--) {
    const c = text[i];
    if (c === "@") return i === 0 || /[\s(]/.test(text[i - 1]) ? { start: i, query: text.slice(i + 1, caret) } : null;
    if (/\s/.test(c)) return null;
  }
  return null;
}

/** Nahradí token vybranou cestou (+ mezera za ním); vrací nový text a kurzor. */
export function applyAt(text: string, tok: AtToken, caret: number, path: string): { text: string; caret: number } {
  const ins = `@${path} `;
  const rest = text.slice(caret);
  return { text: text.slice(0, tok.start) + ins + (rest.startsWith(" ") ? rest.slice(1) : rest), caret: tok.start + ins.length };
}

// ---------- snímek k bodu ----------

export interface Anchor { line: number; text: string }
/** Řádek, ke kterému se snímek naváže: řádek s kurzorem; na prázdném / snímkovém řádku nejbližší nad ním. */
export function anchorAt(body: string, caret: number): Anchor | null {
  const lines = body.split("\n");
  let pos = 0;
  let idx = 0;
  for (; idx < lines.length; idx++) {
    if (caret <= pos + lines[idx].length) break;
    pos += lines[idx].length + 1;
  }
  idx = Math.min(idx, lines.length - 1);
  for (let i = idx; i >= 0; i--) if (lines[i].trim() !== "" && !IMAGE.test(lines[i])) return { line: i, text: lines[i] };
  return null;
}

/** Kotva při uložení: text se mezitím mohl posunout — řádek platí, jen když tam ještě je, jinak první shoda; jinak žádná. */
export function resolveAnchor(body: string, a: Anchor | null | undefined): number | undefined {
  if (!a) return undefined;
  const lines = body.split("\n");
  if (lines[a.line] === a.text) return a.line;
  const i = lines.indexOf(a.text);
  return i >= 0 ? i : undefined;
}

/** Popisek snímku pro `![popisek](…)`: bez `]` a zalomení. */
export const cleanCaption = (s: string) => s.replace(/[\r\n]+/g, " ").replace(/[\][]/g, "").trim();

/** Chyba příkazu zadání lidsky: starší jádro příkaz nezná → vysvětlení místo cizího textu Tauri. */
export function briefError(e: unknown, unknown: string): string {
  const s = String(e);
  return /not found|nenalezen|unknown command/i.test(s) ? unknown : s;
}

/** Snímek pro `note_create` / `note_save` (Rust `ImageIn`): bez popisku i kotvy holé PNG, jinak `{ png, caption?, anchorLine? }`. */
export function imageIn(png: string, caption?: string, anchorLine?: number): ImageIn {
  return caption || anchorLine !== undefined ? { png, ...(caption ? { caption } : {}), ...(anchorLine !== undefined ? { anchorLine } : {}) } : png;
}
