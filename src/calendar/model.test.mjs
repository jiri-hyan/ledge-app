// Tabulkové testy karty Kalendář (`model.ts`): barvy, úprava odkazu (shoda s Rust `normalize_url`),
// maskování adresy, čas řádku a změna jednoho kalendáře v configu. Časy přes místní `Date` (nezávisí na pásmu stroje).
//   node --experimental-strip-types --test src/calendar/model.test.mjs
import { test } from "node:test";
import assert from "node:assert/strict";
import { CAL_COLORS, calColor, domainOf, nextColor, normalizeUrl, provider, timeLabel, withCalendar } from "./model.ts";

const SECRET = "https://calendar.google.com/calendar/ical/jana%40example.com/private-0123456789abcdef/basic.ics";

test("barvy: z configu, jinak podle pořadí; nový kalendář dostane volnou", () => {
  assert.equal(calColor("#123abc", 0), "#123abc");
  assert.equal(calColor(null, 1), CAL_COLORS[1]);
  assert.equal(calColor("modrá", 0), CAL_COLORS[0]);
  assert.equal(calColor(undefined, CAL_COLORS.length + 2), CAL_COLORS[2]);
  assert.equal(nextColor([]), CAL_COLORS[0]);
  assert.equal(nextColor([CAL_COLORS[0].toUpperCase(), null]), CAL_COLORS[1]);
  assert.equal(nextColor([...CAL_COLORS]), CAL_COLORS[0]);
});

test("odkaz: webcal → https, jen https, stejně jako Rust", () => {
  const cases = [
    [SECRET, { url: SECRET }],
    ["  webcal://outlook.office365.com/owa/calendar/a/b/calendar.ics ", { url: "https://outlook.office365.com/owa/calendar/a/b/calendar.ics" }],
    ["WEBCAL://x.cz/a.ics", { url: "https://x.cz/a.ics" }],
    ["HTTPS://x.cz/a.ics", { url: "https://x.cz/a.ics" }],
    ["http://x.cz/a.ics", { problem: "http" }],
    ["ftp://x.cz/a.ics", { problem: "scheme" }],
    ["x.cz/a.ics", { problem: "scheme" }],
    ["https://", { problem: "invalid" }],
    ["https://x.cz/a b.ics", { problem: "invalid" }],
    ["https://u:p@x.cz/a.ics", { problem: "invalid" }],
    ["", { problem: "empty" }],
    ["   ", { problem: "empty" }],
  ];
  for (const [raw, want] of cases) assert.deepEqual(normalizeUrl(raw), want, raw);
});

test("maskování: jen doména, poskytovatel podle domény", () => {
  const cases = [
    [SECRET, "calendar.google.com", "google"],
    ["webcal://Outlook.Office365.com:443/owa/x?y=1", "outlook.office365.com", "outlook"],
    ["https://outlook.live.com/owa/calendar/x/calendar.ics", "outlook.live.com", "outlook"],
    ["https://u:p@x.cz/a", "x.cz", "other"],
    ["nesmysl", "nesmysl", "other"],
  ];
  for (const [raw, dom, prov] of cases) {
    assert.equal(domainOf(raw), dom, raw);
    assert.equal(provider(raw), prov, raw);
  }
  assert.ok(!domainOf(SECRET).includes("private"));
});

test("čas řádku v rámci dne", () => {
  const at = (d, h, m = 0) => new Date(2026, 9, d, h, m).getTime();
  const day = "2026-10-18";
  const cases = [
    [{ start: at(18, 9, 15), end: at(18, 9, 30), allDay: false }, "09:15–09:30"],
    [{ start: at(18, 14), end: at(18, 14), allDay: false }, "14:00"],
    [{ start: at(18, 22), end: at(19, 1), allDay: false }, "22:00 →"],
    [{ start: at(17, 22), end: at(18, 1), allDay: false }, "→ 01:00"],
    [{ start: at(17, 22), end: at(19, 1), allDay: false }, "↔"],
    // konec přesně o půlnoci patří ještě dnešku
    [{ start: at(18, 22), end: at(19, 0), allDay: false }, "22:00–00:00"],
    [{ start: Date.UTC(2026, 9, 18), end: Date.UTC(2026, 9, 19), allDay: true }, null],
  ];
  for (const [it, want] of cases) assert.equal(timeLabel(it, day), want, JSON.stringify(it));
});

test("withCalendar mění jedno pole jednoho kalendáře a nesahá na vstup", () => {
  const cfg = { personal: { weather: { lat: 1 }, calendars: [{ name: "A", url: "u1", enabled: true }, { name: "B", url: "u2", enabled: true }] }, ui: {} };
  const before = JSON.stringify(cfg);
  const out = withCalendar(cfg, 1, { enabled: false });
  assert.equal(JSON.stringify(cfg), before);
  assert.deepEqual(out.personal.calendars, [{ name: "A", url: "u1", enabled: true }, { name: "B", url: "u2", enabled: false }]);
  assert.equal(out.personal.weather, cfg.personal.weather);
  assert.equal(withCalendar(cfg, 5, { enabled: false }), cfg);
  assert.equal(withCalendar({}, 0, { enabled: false }).personal, undefined);
});
