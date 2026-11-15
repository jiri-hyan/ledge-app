// @vitest-environment node
// Jedna priorita dění (`priority.ts`): tabulka pořadí, schůzka / limit z dat snapshotu a zrcadlo se dvěma dalšími
// „mozky“ — pruh návrhů (`suggest/model.ts` `pick`) a proužek (Rust `attention.rs` `Event::rank`).
//   node --experimental-strip-types --test src/hover/priority.test.mjs
import { test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { ETA_MIN_POINTS, KIND_PRIO, LIMIT_ETA_MS, LIMIT_PCT, MEETING_LEAD_MS, PRIORITY, etaAtOf, limitNear, meetingNow, rankOf, topOf } from "./priority.ts";
import { pick, pickAll } from "../suggest/model.ts";

const NOW = 1_800_000_000_000;
const MIN = 60_000;

test("tabulka: pořadí druhů dění, bez duplicit", () => {
  assert.deepEqual([...PRIORITY], ["approval", "question", "brief", "meeting", "limit", "reminder", "today", "meetingNote", "since", "morning", "ci", "review", "dev", "clip", "done", "timer", "music", "working", "musicPaused", "idle"]);
  assert.equal(new Set(PRIORITY).size, PRIORITY.length);
  assert.ok(rankOf("question") < rankOf("brief") && rankOf("brief") < rankOf("meeting") && rankOf("limit") < rankOf("since") && rankOf("since") < rankOf("ci"));
  assert.ok(rankOf("approval") < rankOf("question") && rankOf("meeting") < rankOf("limit") && rankOf("limit") < rankOf("done") && rankOf("working") < rankOf("musicPaused"));
});

test("topOf: nejdůležitější vyhrává, při shodě první, prázdné = null", () => {
  assert.equal(topOf([]), null);
  assert.equal(topOf([{ prio: "music", n: 1 }, { prio: "meeting", n: 2 }, { prio: "done", n: 3 }]).n, 2);
  assert.equal(topOf([{ prio: "done", n: 1 }, { prio: "done", n: 2 }]).n, 1);
});

const pr = (n) => ({ repo: "a/b", number: n, title: `PR ${n}`, url: `https://github.com/a/b/pull/${n}` });
// ranní okno: místní 8:00 (isMorningOpen čte místní hodinu), šuplík schovaný včera večer
const AM = new Date(2027, 0, 20, 8, 0).getTime();
const everything = () => ({
  on: { clipboard: true, devServer: true, github: true, reminders: true, digest: true, githubPulse: true },
  reminders: [{ path: "C:\\n\\r.md", title: "R", remindMs: AM - 5 }],
  briefs: { waiting: 1, review: 0, first: { path: "C:\\n\\z.md", title: "Z", status: "waiting" } },
  claude: { hiddenAt: AM - 12 * 60 * MIN, sessions: [{ id: "w", name: "W", status: "waiting", ended: false, updatedAt: AM, canOpen: true }] },
  tasks: { overdue: 1, today: 2 },
  notes: true,
  meetings: [
    { title: "Daily", start: AM + 5 * MIN, end: AM + 35 * MIN, join: { url: "https://meet.example/x", provider: "meet" } },
    { title: "Retro", start: AM - 40 * MIN, end: AM - 10 * MIN, join: { url: "https://meet.example/y", provider: "meet" } },
  ],
  github: { review: [pr(1)], failing: [pr(2)], fetchedMs: AM - 1000 },
  procs: [{ id: "web", phase: "ready", port: 5173, url: null, readyAt: AM - 1000, errors: 0 }],
  projects: [{ id: "web", name: "Web" }],
  clip: { url: "https://example.com/a", atMs: AM - 1000 },
  known: [],
  morning: { hiddenAt: AM - 12 * 60 * MIN, agenda: [{ start: AM + 5 * MIN, end: AM + 35 * MIN }], limits: null },
});

test("každý druh chipu pruhu má řádek v tabulce (L175)", () => {
  const src = readFileSync(new URL("../suggest/model.ts", import.meta.url), "utf8");
  const union = /export type SuggestKind = ([^;]+);/.exec(src)[1].match(/"(\w+)"/g).map((x) => x.slice(1, -1));
  assert.ok(union.length >= 11, "našel jsem SuggestKind");
  assert.deepEqual(Object.keys(KIND_PRIO).sort(), [...union].sort(), "nový druh chipu? doplň KIND_PRIO a PRIORITY");
  for (const k of union) assert.ok(rankOf(KIND_PRIO[k]) >= 0, `druh ${k} má řádek v PRIORITY`);
});

test("zrcadlo: pořadí všech chipů pruhu odpovídá tabulce, strop 3 bere nejdůležitější", () => {
  const all = pickAll(everything(), new Map(), AM);
  const kinds = all.map((c) => c.kind);
  // „Dnes“ ustoupí rannímu shrnutí (obsahuje úkoly), proto tu není
  assert.deepEqual(kinds, ["briefWaiting", "meeting", "reminder", "meetingNote", "claudeSince", "morning", "ghCi", "ghReview", "dev", "clip"]);
  const ranks = kinds.map((k) => rankOf(KIND_PRIO[k]));
  assert.deepEqual(ranks, [...ranks].sort((a, b) => a - b), `pořadí pickAll odpovídá tabulce: ${kinds}`);
  assert.deepEqual(pick(everything(), new Map(), AM).map((c) => c.kind), ["briefWaiting", "meeting", "reminder"], "strop 3 = tři nejdůležitější");
  // bez ranního shrnutí se „Dnes“ vrátí na své místo (za připomínku, před poznámku ze schůzky)
  const noMorning = pickAll({ ...everything(), morning: undefined }, new Map(), AM).map((c) => c.kind);
  assert.deepEqual(noMorning, ["briefWaiting", "meeting", "reminder", "today", "meetingNote", "claudeSince", "ghCi", "ghReview", "dev", "clip"]);
  const r = noMorning.map((k) => rankOf(KIND_PRIO[k]));
  assert.deepEqual(r, [...r].sort((a, b) => a - b));
  // L175: s briefem + připomínkou + dnes se schůzka už do tří chipů vejde
  assert.ok(noMorning.slice(0, 3).includes("meeting"));
});

test("zrcadlo: Rust Event::rank odpovídá tabulce", () => {
  const rs = readFileSync(new URL("../../src-tauri/src/attention.rs", import.meta.url), "utf8");
  const body = rs.slice(rs.indexOf("fn rank(self)"));
  const arms = [...body.slice(0, body.indexOf("\n    }\n")).matchAll(/Event::(\w+)\s*=>\s*(\d+)/g)].map((m) => [m[1], Number(m[2])]);
  assert.ok(arms.length >= 5, "našel jsem větve rank");
  const MAP = { NeedsInput: "approval", Meeting: "meeting", GithubCi: "ci", GithubReview: "review", Done: "done" };
  assert.deepEqual(arms.map(([e]) => e).sort(), Object.keys(MAP).sort(), "nový druh Event? doplň MAP a tabulku PRIORITY");
  const byRank = arms.sort((a, b) => a[1] - b[1]).map(([e]) => rankOf(MAP[e]));
  assert.deepEqual(byRank, [...byRank].sort((a, b) => a - b), "Event::rank je v souladu s PRIORITY");
});

const cal = (items) => ({ days: [{ kind: "tomorrow", items: [{ title: "zítra", start: NOW, end: NOW + MIN, allDay: false }] }, { kind: "today", items }] });
const it = (title, start, end, o = {}) => ({ title, start, end, allDay: false, ...o });

test("schůzka: ≤ 10 min před začátkem do konce, bez celodenních a bez jiných dnů", () => {
  const J = { url: "https://meet.example/x", provider: "meet" };
  const cases = [
    ["bez kalendáře", undefined, null],
    ["za 11 min", cal([it("A", NOW + 11 * MIN, NOW + 40 * MIN)]), null],
    ["za 10 min", cal([it("A", NOW + MEETING_LEAD_MS, NOW + 40 * MIN)]), { title: "A", ongoing: false, join: null }],
    ["probíhá", cal([it("A", NOW - 5 * MIN, NOW + 20 * MIN, { join: J })]), { title: "A", ongoing: true, join: J }],
    ["skončila", cal([it("A", NOW - 40 * MIN, NOW)]), null],
    ["celodenní", cal([it("A", NOW - 5 * MIN, NOW + 20 * MIN, { allDay: true })]), null],
    ["nejbližší první", cal([it("B", NOW + 8 * MIN, NOW + 30 * MIN), it("A", NOW + 2 * MIN, NOW + 30 * MIN)]), { title: "A", ongoing: false, join: null }],
  ];
  for (const [name, c, want] of cases) {
    const got = meetingNow(c, NOW);
    if (want === null) assert.equal(got, null, name);
    else for (const [k, v] of Object.entries(want)) assert.deepEqual(got?.[k], v, `${name}: ${k}`);
  }
});

const lim = (pct, resetsAt, hist = [], at = NOW - MIN) => ({ at, staleMs: 10 * MIN, fiveHour: { pct, resetsAt }, history5h: hist });
const hist = (pairs) => pairs.map(([m, p]) => [NOW - m * MIN, p]);

test("limit: od 90 %, nebo když tempo dojde brzy; bez čerstvých dat nic", () => {
  const R = NOW + 120 * MIN;
  const fast = hist([[30, 40], [20, 55], [10, 70], [0, 85]]); // 1,5 %/min → 100 % za 10 min
  const slow = hist([[30, 10], [20, 20], [10, 30], [0, 40]]); // 1 %/min → za 60 min
  const cases = [
    ["bez dat", undefined, false],
    ["pod prahem, bez historie", lim(60, R), false],
    ["90 %", lim(LIMIT_PCT, R), true],
    ["rychlé tempo → dojde za 10 min", lim(85, R, fast), true],
    ["pomalé tempo → dojde za 60 min (> 30)", lim(40, R, slow), false],
    ["staré vzorky", lim(95, R, [], NOW - 11 * MIN), false],
    ["okno už skončilo", lim(95, NOW - 1), false],
    ["bez 5h okna", { at: NOW, staleMs: MIN, fiveHour: null, history5h: [] }, false],
  ];
  for (const [name, l, want] of cases) assert.equal(!!limitNear(l, NOW), want, name);
  assert.equal(limitNear(lim(85, R, fast), NOW).etaAt, NOW + 10 * MIN);
});

test("odhad: aspoň 3 vzorky a rozpětí, jen „recent“, jen před obnovou", () => {
  const R = NOW + 120 * MIN;
  assert.equal(ETA_MIN_POINTS, 3);
  assert.equal(etaAtOf(hist([[20, 10], [0, 40]]), 40, R, NOW), null, "2 vzorky");
  assert.equal(etaAtOf(hist([[30, 10], [20, 20], [10, 30], [0, 40]]), 40, R, NOW), NOW + 60 * MIN);
  assert.equal(etaAtOf(hist([[30, 10], [20, 20], [10, 30], [0, 40]]), 40, NOW + 50 * MIN, NOW), null, "obnova dřív");
  assert.equal(etaAtOf(hist([[30, 10], [20, 20], [10, 30], [0, 40]]), 40, null, NOW), null, "bez resetu");
  assert.ok(LIMIT_ETA_MS > 0);
});
