// Tabulkové testy karty Dnes (`today.ts`): co se na kartě ukáže a že řádek bez dat nevznikne.
//   node --experimental-strip-types --test src/cards/today.test.mjs
import { test } from "node:test";
import assert from "node:assert/strict";
import { dueReminders, endOfDay, limitRow, nextMeeting, reviewRow, sameRows, todayRows } from "./today.ts";

const MIN = 60_000;
// 15. 10. 2026 10:00 místního času
const NOW = new Date(2026, 9, 15, 10, 0, 0).getTime();
const at = (h, m = 0) => new Date(2026, 9, 15, h, m, 0).getTime();
const JOIN = { provider: "teams", url: "https://teams.microsoft.com/l/meetup-join/x" };
const item = (title, start, end, over = {}) => ({ title, start, end, allDay: false, location: "", cal: 0, ongoing: false, tzGuess: false, join: null, ...over });
const cal = (items) => ({ status: "ok", days: [{ kind: "today", date: "2026-10-15", items }], more: 0, calendars: [], loading: false });
const lim = (pct, resetsAt, over = {}) => ({ at: NOW - MIN, source: "statusLine", fiveHour: { pct, resetsAt }, sevenDay: null, staleMs: 15 * MIN, statusLine: "full", notice: null, pace: false, history5h: [], ...over });
const note = (path, remindMs, over = {}) => ({ path, title: path, remindMs, done: false, ...over });
const pr = (n) => ({ repo: "a/b", number: n, title: `PR ${n}`, url: `https://github.com/a/b/pull/${n}` });
const empty = { calendar: undefined, notes: [], reminders: [], github: undefined, limits: undefined };

test("prázdná data = žádný řádek (karta se nekreslí)", () => {
  assert.deepEqual(todayRows(empty, NOW), []);
  assert.deepEqual(todayRows({ ...empty, calendar: cal([]), github: { review: [], fetchedMs: NOW }, limits: lim(10, NOW + 60 * MIN) }, NOW), []);
});

test("další schůzka: první neskončená, bez celodenních; Připojit jen s ověřeným odkazem", () => {
  const cases = [
    ["budoucí", [item("Standup", at(11), at(11, 30))], "Standup", false, null],
    ["první podle začátku", [item("Pozdě", at(15), at(16)), item("Dřív", at(12), at(13))], "Dřív", false, null],
    ["skončená se přeskočí", [item("Ráno", at(8), at(9)), item("Odpoledne", at(14), at(15))], "Odpoledne", false, null],
    ["celodenní ne", [item("Dovolená", at(0), at(24), { allDay: true }), item("Call", at(13), at(14))], "Call", false, null],
    ["probíhající se počítá", [item("Teď", at(9, 30), at(10, 30), { ongoing: true, join: JOIN })], "Teď", true, JOIN],
    ["odkaz u budoucí", [item("Teams", at(10, 20), at(11), { join: JOIN })], "Teams", false, JOIN],
  ];
  for (const [name, items, title, ongoing, join] of cases) {
    const r = nextMeeting(cal(items), NOW);
    assert.equal(r?.title, title, name);
    assert.equal(r?.ongoing, ongoing, name);
    assert.deepEqual(r?.join, join, name);
  }
  assert.equal(nextMeeting(cal([item("Ráno", at(8), at(9))]), NOW), null, "vše skončilo");
  assert.equal(nextMeeting(undefined, NOW), null);
  assert.equal(nextMeeting({ ...cal([item("X", at(11), at(12))]), days: [{ kind: "tomorrow", date: "x", items: [item("Zítra", at(11), at(12))] }] }, NOW), null, "zítřek není dnes");
});

test("připomínky: do konce dneška, po termínu s příznakem, bez hotových a duplicit, strop 3 + zbytek", () => {
  const end = endOfDay(NOW);
  assert.equal(end, new Date(2026, 9, 16).getTime());
  const rows = (input) => dueReminders({ notes: [], reminders: [], ...input }, NOW);
  assert.deepEqual(rows({}), []);
  assert.deepEqual(rows({ notes: [note("zítra", end), note("bez", null), note("hotová", at(9), { done: true })] }), []);
  const r = rows({ reminders: [note("a", at(8))], notes: [note("a", at(8)), note("b", at(15)), note("c", at(9))] });
  assert.deepEqual(r.map((x) => [x.path, x.overdue]), [["a", true], ["c", true], ["b", false]]);
  const many = rows({ notes: [note("1", at(11)), note("2", at(12)), note("3", at(13)), note("4", at(14)), note("5", at(15))] });
  assert.deepEqual(many.map((x) => x.kind), ["reminder", "reminder", "reminder", "reminderMore"]);
  assert.equal(many[3].n, 2);
});

test("PR k revizi: jen čerstvá data a aspoň jedno PR", () => {
  const cases = [
    ["bez dat", undefined, null],
    ["nikdy načteno", { review: [pr(1)], fetchedMs: null }, null],
    ["staré", { review: [pr(1)], fetchedMs: NOW - 61 * MIN }, null],
    ["prázdné", { review: [], fetchedMs: NOW }, null],
    ["jedno", { review: [pr(1)], fetchedMs: NOW - 5 * MIN }, [1, 0]],
    ["víc", { review: [pr(7), pr(8), pr(9)], fetchedMs: NOW }, [7, 2]],
  ];
  for (const [name, gh, want] of cases) {
    const r = reviewRow(gh, NOW);
    assert.deepEqual(r && [r.number, r.more], want, name);
  }
});

test("limit 5h: od 50 %, čerstvý vzorek, reset v budoucnu", () => {
  const cases = [
    ["49 %", lim(49, NOW + 60 * MIN), null],
    ["50 %", lim(50, NOW + 60 * MIN), 50],
    ["87,4 %", lim(87.4, NOW + 5 * MIN), 87.4],
    ["po resetu", lim(80, NOW - 1), null],
    ["bez času resetu", lim(80, null), null],
    ["staré", lim(80, NOW + 60 * MIN, { at: NOW - 20 * MIN }), null],
    ["bez vzorku", lim(80, NOW + 60 * MIN, { at: null }), null],
    ["bez okna", lim(80, NOW + 60 * MIN, { fiveHour: null }), null],
    ["bez limitů", undefined, null],
  ];
  for (const [name, lv, want] of cases) assert.equal(limitRow(lv, NOW)?.pct ?? null, want, name);
});

test("pořadí řádků a stabilita (tik času nepřekreslí)", () => {
  const input = {
    calendar: cal([item("Call", at(11), at(12), { join: JOIN })]),
    notes: [note("n", at(9))],
    reminders: [],
    github: { review: [pr(3)], fetchedMs: NOW },
    limits: lim(70, NOW + 90 * MIN),
  };
  const rows = todayRows(input, NOW);
  assert.deepEqual(rows.map((r) => r.kind), ["meeting", "reminder", "review", "limit"]);
  assert.ok(sameRows(rows, todayRows(input, NOW + 1000)), "o sekundu později totéž");
  assert.ok(!sameRows(rows, todayRows(input, at(11, 30))), "po začátku schůzky se řádky liší");
});
