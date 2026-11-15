// Záložka Úkoly: skládání sekcí, hranice půlnoci, celodenní, „+N“, forwarded, odložení, řádek v editoru.
import { test } from "node:test";
import assert from "node:assert/strict";
import { buildAgenda, dayDiff, isOverdue, offsetOfLine, snoozeIso, UNDATED_MAX } from "./agenda.ts";

const at = (d, h = 0, m = 0) => new Date(2026, 10, d, h, m).getTime(); // listopad 2026 (13. = pátek)
const NOW = at(13, 10, 0);
const row = (o) => ({ path: "a.md", line: 0, text: "x", state: "open", dueMs: null, allDay: false, prio: 0, project: null, noteTitle: "A", kind: "", forwarded: false, ...o });
const cal = (items, status = "ok") => ({ status, days: [{ kind: "today", date: "2026-11-13", items }], more: 0, calendars: [], loading: false });
const meet = (h, endH, o = {}) => ({ start: at(13, h), end: at(13, endH), allDay: false, title: `m${h}`, location: "", cal: 0, ongoing: false, tzGuess: false, ...o });

test("sekce: po termínu / dnes / čeká / brzy / bez termínu", () => {
  const a = buildAgenda(
    [
      row({ line: 1, dueMs: at(13, 9) }), // dnes, ale už po termínu
      row({ line: 2, dueMs: at(13, 15) }),
      row({ line: 3, dueMs: at(14, 8) }),
      row({ line: 4, state: "proposed" }),
      row({ line: 5, state: "proposed", dueMs: at(12, 8) }), // čeká — nezdvojuje se v Po termínu
      row({ line: 6 }),
      row({ line: 7, dueMs: at(1, 8) }),
    ],
    null,
    NOW,
  );
  assert.deepEqual(a.overdue.map((r) => r.line), [7, 1]);
  assert.deepEqual(a.today.map((i) => i.row.line), [2]);
  assert.deepEqual(a.waiting.map((r) => r.line), [4, 5]);
  assert.deepEqual(a.soon.map((r) => r.line), [3]);
  assert.deepEqual(a.undated.map((r) => r.line), [6]);
  assert.equal(a.empty, false);
});

test("celodenní: po termínu až od půlnoci, dnes nahoře", () => {
  const today9 = row({ line: 1, dueMs: at(13, 9), allDay: true }); // Rust: 9:00 toho dne
  const yest9 = row({ line: 2, dueMs: at(12, 9), allDay: true });
  assert.equal(isOverdue(today9, at(13, 23, 59)), false);
  assert.equal(isOverdue(today9, at(14, 0, 0)), true);
  assert.equal(isOverdue(yest9, at(13, 0, 0)), true);
  assert.equal(isOverdue(yest9, at(12, 23, 0)), false);
  const a = buildAgenda([row({ line: 3, dueMs: at(13, 8) }), today9], null, at(13, 7));
  assert.deepEqual(a.today.map((i) => i.row.line), [1, 3]);
});

test("Dnes: schůzky a body na jedné ose, celodenní nahoře, skončená schůzka pryč", () => {
  const c = cal([meet(14, 15, { join: { url: "https://x", provider: "teams" } }), meet(8, 9), meet(11, 12), { ...meet(0, 0), title: "svátek", allDay: true, start: at(13), end: at(14) }]);
  const a = buildAgenda([row({ line: 1, dueMs: at(13, 11, 30) }), row({ line: 2, dueMs: at(13, 9), allDay: true })], c, NOW);
  assert.deepEqual(a.today.map((i) => (i.kind === "meeting" ? i.title : `t${i.row.line}`)), ["svátek", "t2", "m11", "t1", "m14"]);
  const m14 = a.today.find((i) => i.kind === "meeting" && i.title === "m14");
  assert.ok(m14.join);
  assert.equal(m14.ongoing, false);
  assert.equal(a.today.find((i) => i.title === "m8"), undefined);
});

test("schůzky jen při dostupném kalendáři; probíhající se ukáže", () => {
  for (const st of ["off", "unset", "loading", "unavailable", "idle"]) assert.equal(buildAgenda([], cal([meet(11, 12)], st), NOW).today.length, 0, st);
  const a = buildAgenda([], cal([meet(9, 11)], "stale"), NOW);
  assert.equal(a.today[0].ongoing, true);
  assert.equal(buildAgenda([], cal([meet(9, 11)]), NOW).empty, false);
  assert.equal(buildAgenda([], null, NOW).empty, true);
});

test("forwarded se nepočítá ani nekreslí", () => {
  const a = buildAgenda([row({ line: 1, dueMs: at(13, 9), forwarded: true }), row({ line: 2, forwarded: true })], null, NOW);
  assert.equal(a.empty, true);
});

test("Bez termínu: strop a +N; Brzy řazeno podle času", () => {
  const many = Array.from({ length: UNDATED_MAX + 5 }, (_, i) => row({ line: i }));
  const a = buildAgenda([...many, row({ line: 100, dueMs: at(16, 8) }), row({ line: 101, dueMs: at(15, 8) })], null, NOW);
  assert.equal(a.undated.length, UNDATED_MAX);
  assert.equal(a.undatedMore, 5);
  assert.deepEqual(a.soon.map((r) => r.line), [101, 100]);
});

test("půlnoc: 23:59:59 ještě dnes, 00:00 už zítra", () => {
  const r = row({ line: 1, dueMs: at(14, 0, 0) });
  assert.equal(buildAgenda([r], null, at(13, 23, 59)).soon.length, 1);
  assert.equal(buildAgenda([r], null, at(14, 0, 0)).today.length, 1);
  assert.equal(dayDiff(at(14, 0, 1), at(13, 23, 59)), 1);
  assert.equal(dayDiff(at(12, 23, 59), at(13, 0, 0)), -1);
});

test("snoozeIso: zítra / pondělí, čas zůstane, celodenní bez času", () => {
  const timed = row({ dueMs: at(13, 9, 5) });
  const allDay = row({ dueMs: at(13, 9), allDay: true });
  assert.equal(snoozeIso(timed, "tomorrow", NOW), "2026-11-14 09:05");
  assert.equal(snoozeIso(allDay, "tomorrow", NOW), "2026-11-14");
  assert.equal(snoozeIso(timed, "monday", NOW), "2026-11-16 09:05"); // pátek → pondělí
  assert.equal(snoozeIso(timed, "monday", at(16, 8)), "2026-11-23 09:05"); // v pondělí → další pondělí
  assert.equal(snoozeIso(timed, "monday", at(15, 8)), "2026-11-16 09:05"); // v neděli → zítra
  assert.equal(snoozeIso(row(), "tomorrow", NOW), "2026-11-14"); // bez termínu
  assert.equal(snoozeIso(timed, "tomorrow", at(30, 8)), "2026-12-01 09:05"); // přelom měsíce
});

test("offsetOfLine: LF, CRLF, za koncem", () => {
  assert.equal(offsetOfLine("a\nbb\nc", 0), 0);
  assert.equal(offsetOfLine("a\nbb\nc", 2), 5);
  assert.equal(offsetOfLine("a\r\nbb", 1), 3);
  assert.equal(offsetOfLine("a\nb", 9), 3);
});
