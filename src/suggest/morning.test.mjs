// Tabulkové testy ranního shrnutí (`morning.ts`): kdy je „první otevření po probuzení“ a co se do shrnutí složí.
//   node --experimental-strip-types --test src/suggest/morning.test.mjs
import { test } from "node:test";
import assert from "node:assert/strict";
import { MORNING_GAP_MS, MORNING_TO_H, MORNING_WEEK_PCT, isMorningOpen, morningKey, morningParts, morningSig, morningWindowEnd } from "./morning.ts";
import { pickAll } from "./model.ts";

const MIN = 60_000;
const H = 3600_000;
// místní čas (funkce čtou místní hodinu)
const at = (h, m = 0, d = 20) => new Date(2027, 0, d, h, m).getTime();
const NOW = at(8);
const none = { tasks: { overdue: 0, today: 0 }, reminders: 0, briefs: 0, review: 0, failing: 0 };
const mi = (o = {}) => ({ hiddenAt: NOW - 10 * H, agenda: [], limits: null, ...o });

test("kdy je první otevření: ranní okno a nečinnost ≥ 6 h (nebo od startu nikdy)", () => {
  const cases = [
    ["8:00, schováno před 10 h", NOW - 10 * H, NOW, true],
    ["8:00, schováno před 6 h přesně", NOW - MORNING_GAP_MS, NOW, true],
    ["8:00, schováno před 5 h 59 min", NOW - MORNING_GAP_MS + MIN, NOW, false],
    ["8:00, od startu nikdy", null, NOW, true],
    ["4:59, dlouhá nečinnost", at(4, 59) - 10 * H, at(4, 59), false],
    ["5:00", at(5) - 10 * H, at(5), true],
    ["12:59", at(12, 59) - 10 * H, at(12, 59), true],
    ["13:00 už není ráno", at(13) - 10 * H, at(13), false],
    ["odpoledne, od startu nikdy", null, at(15), false],
  ];
  for (const [name, hidden, now, want] of cases) assert.equal(isMorningOpen(hidden, now), want, name);
});

test("části shrnutí: jen neprázdné, v pořadí; nic = prázdné", () => {
  const meet = (s, e) => ({ start: NOW + s * MIN, end: NOW + e * MIN });
  const lim = (pct, o = {}) => ({ at: NOW - MIN, staleMs: 10 * MIN, sevenDay: { pct, resetsAt: NOW + 24 * H }, ...o });
  const cases = [
    ["nic", mi(), none, ""],
    ["schůzky: počet a první začátek", mi({ agenda: [meet(90, 120), meet(30, 60)] }), none, "meetings:2@" + (NOW + 30 * MIN)],
    ["skončená schůzka se nepočítá", mi({ agenda: [meet(-60, -30), meet(30, 60)] }), none, "meetings:1@" + (NOW + 30 * MIN)],
    ["jen skončené schůzky = nic", mi({ agenda: [meet(-60, -30)] }), none, ""],
    ["vše najednou ve správném pořadí", mi({ agenda: [meet(30, 60)], limits: lim(72.4) }), { tasks: { overdue: 2, today: 3 }, reminders: 1, briefs: 1, review: 4, failing: 1 }, `meetings:1@${NOW + 30 * MIN}|overdue:2|tasks:3|reminders:1|briefs:1|review:4|ci:1|week:72`],
    ["týdenní limit pod prahem se nehlásí", mi({ limits: lim(MORNING_WEEK_PCT - 1) }), none, ""],
    ["týdenní limit přesně na prahu", mi({ limits: lim(MORNING_WEEK_PCT) }), none, `week:${MORNING_WEEK_PCT}`],
    ["staré vzorky limitu se nehlásí (divadlo)", mi({ limits: lim(80, { at: NOW - 11 * MIN }) }), none, ""],
    ["po resetu týdne se nehlásí", mi({ limits: lim(80, { sevenDay: { pct: 80, resetsAt: NOW - 1 } }) }), none, ""],
    ["bez dat o limitu", mi({ limits: { at: null, staleMs: MIN, sevenDay: null } }), none, ""],
    ["mimo ranní okno nic", mi({ hiddenAt: at(15) - 10 * H, agenda: [meet(30, 60)] }), { ...none, tasks: { overdue: 1, today: 0 } }, ""],
  ];
  for (const [name, input, counts, sig] of cases) {
    const now = name.startsWith("mimo") ? at(15) : NOW;
    assert.equal(morningSig(morningParts(input, counts, now)), sig, name);
  }
});

test("klíč zavření je jednou denně, okno končí ve 13:00", () => {
  assert.equal(morningKey(at(8)), "morning:2027-01-20");
  assert.equal(morningKey(at(12, 30)), morningKey(at(6)));
  assert.notEqual(morningKey(at(8, 0, 21)), morningKey(at(8)));
  assert.equal(morningWindowEnd(at(8)), at(MORNING_TO_H));
  assert.equal(morningWindowEnd(at(14)), at(MORNING_TO_H, 0, 21), "po okně příští den");
});

test("pick: ráno nahradí „Dnes“, zavřené se nevrací, vypnuté připomínky ho vypnou, bez dat žádný chip", () => {
  const ON = { clipboard: false, devServer: false, github: true, reminders: true, digest: true, githubPulse: true };
  const input = {
    on: ON, reminders: [], tasks: { overdue: 1, today: 2 }, notes: true, meetings: [], github: { review: [], failing: [], fetchedMs: null },
    procs: [], projects: [], clip: null, known: [], morning: mi(),
  };
  const kinds = (i, d = new Map()) => pickAll(i, d, NOW).map((c) => c.kind);
  assert.deepEqual(kinds(input), ["morning"], "ráno místo Dnes");
  assert.deepEqual(pickAll(input, new Map(), NOW)[0].morning.map((p) => p.k), ["overdue", "tasks"]);
  assert.deepEqual(kinds({ ...input, morning: mi({ hiddenAt: NOW - H }) }), ["today"], "krátká nečinnost: zůstane Dnes");
  assert.deepEqual(kinds(input, new Map([[morningKey(NOW), NOW + H]])), ["today"], "zavřené: Dnes se vrátí");
  assert.deepEqual(kinds({ ...input, on: { ...ON, digest: false } }), []);
  assert.deepEqual(kinds({ ...input, tasks: { overdue: 0, today: 0 } }), [], "žádná data = žádný chip");
  assert.deepEqual(kinds({ ...input, morning: undefined, tasks: { overdue: 0, today: 0 } }), []);
  // GitHub se počítá jen čerstvý a zapnutý
  const gh = { ...input, tasks: { overdue: 0, today: 0 }, github: { review: [{ repo: "a/b", number: 1, title: "t", url: "u" }], failing: [], fetchedMs: NOW - 1000 } };
  assert.deepEqual(pickAll(gh, new Map(), NOW).find((c) => c.kind === "morning").morning, [{ k: "review", n: 1 }]);
  assert.ok(!pickAll({ ...gh, on: { ...ON, github: false } }, new Map(), NOW).some((c) => c.kind === "morning"));
  assert.ok(!pickAll({ ...gh, github: { ...gh.github, fetchedMs: NOW - 2 * H } }, new Map(), NOW).some((c) => c.kind === "morning"), "stará data GitHubu");
});
