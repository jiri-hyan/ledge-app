// Gramatika řádku úkolu — zrcadlo Rustu `src-tauri/src/notes/taskline.rs` (tytéž tabulky:
// `src-tauri/src/notes/taskline_cases.json`, test `taskline.test.mjs`). Text za značkou `- [ ] ` je
// `text [⏰ YYYY-MM-DD[ HH:MM]] [! | !!] [ — důkaz: …]`; co gramatika nezná, zůstane v `text` bajt po bajtu.
// Čisté funkce bez DOM a bez api.

export interface TailLine {
  /** text bez ocasu (termín, priorita), důkazu a koncových mezer */
  text: string;
  /** text + tokeny (bez důkazu a koncových mezer) */
  head: string;
  /** `YYYY-MM-DD` nebo `YYYY-MM-DD HH:MM` (místní čas), `null` = bez termínu */
  due: string | null;
  allDay: boolean;
  /** termín přečtený z `📅` (Obsidian Tasks) */
  cal: boolean;
  /** 0 žádná, 1 = `!`, 2 = `!!` */
  prio: 0 | 1 | 2;
  /** ` — důkaz: …` včetně mezer před pomlčkou, jinak "" */
  evidenceRaw: string;
  trail: string;
}

export const DUE_MARK = "⏰";
export const CAL_MARK = "📅";

/** Začátek důkazu: [index po ořezu mezer před pomlčkou, index za dvojtečkou]; platí nejdřívější shoda. */
function evidenceAt(text: string): [number, number] | null {
  let best: [number, number] | null = null;
  for (const dash of [" — ", " – ", " - "]) {
    let from = 0;
    for (;;) {
      const i = text.indexOf(dash, from);
      if (i < 0) break;
      from = i + 1;
      const rest = text.slice(i + dash.length);
      for (const kw of ["důkaz", "dukaz", "evidence"]) {
        if (rest.slice(0, kw.length).toLowerCase() === kw && rest[kw.length] === ":" && (best === null || i < best[0])) {
          best = [i, i + dash.length + kw.length + 1];
        }
      }
    }
  }
  return best ? [text.slice(0, best[0]).trimEnd().length, best[1]] : null;
}

/** Text bodu rozdělený na text (včetně tokenů) a důkaz. */
export function splitEvidence(text: string): [string, string | null] {
  const e = evidenceAt(text);
  if (!e) return [text.trimEnd(), null];
  const ev = text.slice(e[1]).trim();
  return [text.slice(0, e[0]), ev || null];
}

function validDate(s: string): boolean {
  if (!/^\d{4}-\d{2}-\d{2}$/.test(s)) return false;
  const [y, m, d] = s.split("-").map(Number);
  const t = new Date(Date.UTC(y, m - 1, d));
  return t.getUTCFullYear() === y && t.getUTCMonth() === m - 1 && t.getUTCDate() === d;
}

const validTime = (s: string) => /^\d{2}:\d{2}$/.test(s) && Number(s.slice(0, 2)) < 24 && Number(s.slice(3)) < 60;

/** Poslední token oddělený jednou mezerou: [zbytek | null, token]. */
function lastToken(s: string): [string | null, string] {
  const i = s.lastIndexOf(" ");
  return i < 0 ? [null, s] : [s.slice(0, i), s.slice(i + 1)];
}

/** Zbytek před tokenem: prázdný jen když token stojí na začátku; nesmí končit mezerou. */
function cleanHead(h: string | null): string | null {
  if (h === null) return "";
  return h !== "" && !h.endsWith(" ") ? h : null;
}

function peelPrio(s: string): [string, 1 | 2] | null {
  const [h, t] = lastToken(s);
  const p = t === "!" ? 1 : t === "!!" ? 2 : 0;
  const c = p ? cleanHead(h) : null;
  return p && c !== null ? [c, p as 1 | 2] : null;
}

function peelDue(s: string): { head: string; due: string; allDay: boolean; cal: boolean } | null {
  const [h1, t1] = lastToken(s);
  let time: string | null = null;
  let dateTok = t1;
  let h = h1;
  if (validTime(t1)) {
    if (h1 === null) return null;
    time = t1;
    [h, dateTok] = lastToken(h1);
  }
  if (!validDate(dateTok) || h === null) return null;
  const [h3, mark] = lastToken(h);
  const cal = mark === CAL_MARK && time === null;
  if (mark !== DUE_MARK && !cal) return null;
  const head = cleanHead(h3);
  return head === null ? null : { head, due: time ? `${dateTok} ${time}` : dateTok, allDay: time === null, cal };
}

/** Rozebere text za značkou úkolu. */
export function parseTail(s: string): TailLine {
  const e = evidenceAt(s);
  const pre = e ? s.slice(0, e[0]) : s;
  const evidenceRaw = e ? s.slice(e[0]) : "";
  const head = pre.trimEnd();
  const trail = pre.slice(head.length);
  let text = head;
  let prio: 0 | 1 | 2 = 0;
  let due: string | null = null;
  let allDay = false;
  let cal = false;
  const p = peelPrio(text);
  if (p) [text, prio] = p;
  const d = peelDue(text);
  if (d) {
    ({ head: text, due, allDay, cal } = d);
  }
  return { text, head, due, allDay, cal, prio, evidenceRaw, trail };
}

/** Opak `parseTail`: `renderTail(parseTail(x)) === x`. */
export function renderTail(t: TailLine): string {
  let o = t.text;
  const sep = () => { if (o) o += " "; };
  if (t.due) {
    sep();
    o += `${t.cal && t.allDay ? CAL_MARK : DUE_MARK} ${t.due}`;
  }
  if (t.prio > 0) {
    sep();
    o += t.prio >= 2 ? "!!" : "!";
  }
  return o + t.evidenceRaw + t.trail;
}

// ---------- řádek úkolu (zrcadlo Rust `md::list_item` + `md::task_state`) ----------

export type TaskMark = "open" | "proposed" | "done";

export interface TaskLineParts {
  /** počet mezer / tabulátorů před značkou seznamu */
  indent: number;
  /** `-` `*` `+` nebo `1.` / `1)` */
  marker: string;
  state: TaskMark;
  /** text za `[ ]` bez úvodních mezer (včetně ocasu, který rozebere `parseTail`) */
  rest: string;
}

/** Jediná TS gramatika řádku úkolu: odsazení, značka (`-`/`*`/`+` nebo 1–9 číslic + `.`/`)`), JEDNA mezera,
 *  `[ ]`/`[~]`/`[x]`/`[X]`, pak konec řádku nebo mezera — přesně jako Rust. Řádek bez CR; bloky kódu řeší volající.
 *  Tabulka `taskitem_cases.json` je sdílená s Rustem. */
export function taskLineOf(line: string): TaskLineParts | null {
  const m = /^([ \t]*)(?:([-*+])|(\d{1,9}[.)])) \[([ ~xX])\](?: ([^]*))?$/.exec(line.replace(/\r$/, ""));
  if (!m) return null;
  return { indent: m[1].length, marker: m[2] ?? m[3], state: m[4] === " " ? "open" : m[4] === "~" ? "proposed" : "done", rest: (m[5] ?? "").trimStart() };
}

/** Značka odeslání (Rust `taskline::SENT_MARK`): řádek pod bodem `↳ [{název}](url)`, `↳ [Úkol](url)` nebo `↳ [{název}] id`; rozpozná starý i nový tvar. */
export const SENT_MARK = "↳ [";

/** Je bod na řádku `at` odeslaný (propojené aplikaci)? Jako Rust `taskindex::forwarded`: první řádek za bodem, který není
 *  obrázek `![…](…)`, začíná `SENT_MARK` (prázdný řádek řetěz přeruší). */
export function forwardedAt(lines: readonly string[], at: number): boolean {
  for (let i = at + 1; i < lines.length; i++) {
    const l = lines[i].trim();
    if (l.startsWith("![") && l.endsWith(")")) continue;
    return l.startsWith(SENT_MARK);
  }
  return false;
}
