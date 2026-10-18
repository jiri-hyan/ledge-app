// Tabulkové testy karty Kalendář (`model.ts`): barvy, úprava odkazu (shoda s Rust `normalize_url`),
// maskování adresy, čas řádku a změna jednoho kalendáře v configu. Časy přes místní `Date` (nezávisí na pásmu stroje).
//   node --experimental-strip-types --test src/calendar/model.test.mjs
import { test } from "node:test";
import assert from "node:assert/strict";
import { CAL_COLORS, calColor, domainOf, joinable, nextColor, normalizeUrl, outlookFeed, placeLabel, provider, providerName, remindOf, remindOptions, sourceOf, startsIn, timeLabel, withCalendar } from "./model.ts";

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

test("zdroj: starý config = ICS, Outlook v PC a účty poznané", () => {
  assert.equal(sourceOf({}), "ics");
  assert.equal(sourceOf(null), "ics");
  assert.equal(sourceOf({ source: "outlook-local" }), "outlook-local");
  assert.equal(sourceOf({ source: "google" }), "google");
  assert.equal(sourceOf({ source: "nesmysl" }), "ics");
  const o = outlookFeed("#60a5fa");
  assert.deepEqual([o.source, o.url, o.remindMin, o.remindToast, o.enabled], ["outlook-local", "", 5, false, true]);
});

test("Připojit: jen s odkazem a do konce události; místo = odkaz se neukazuje", () => {
  const join = { provider: "zoom", url: "https://zoom.us/j/1" };
  const cases = [
    [{ start: 0, end: 1000, join }, 500, true],
    [{ start: 0, end: 120_000, join }, 120_000, false],
    [{ start: 0, end: 0, join }, 59_999, true],
    [{ start: 0, end: 1000, join: null }, 500, false],
    [{ start: 0, end: 1000 }, 500, false],
  ];
  for (const [it, now, want] of cases) assert.equal(joinable(it, now), want, JSON.stringify([it, now]));
  assert.equal(placeLabel({ location: "https://zoom.us/j/1", join }), "");
  assert.equal(placeLabel({ location: "https://zoom.us/j/1", join: null }), "https://zoom.us/j/1");
  assert.equal(placeLabel({ location: " Zasedačka ", join }), "Zasedačka");
  assert.equal(providerName("teams"), "Teams");
  assert.equal(providerName("x"), "");
});

test("začíná za: do 15 min nahoru zaokrouhleno, celodenní a probíhající ne", () => {
  const m = 60_000;
  const cases = [
    [{ start: 5 * m, allDay: false }, 0, 15, 5],
    [{ start: 5 * m, allDay: false }, 1, 15, 5],
    [{ start: 5 * m, allDay: false }, 4 * m + 1, 15, 1],
    [{ start: 16 * m, allDay: false }, 0, 15, null],
    [{ start: 0, allDay: false }, 0, 15, null],
    [{ start: 5 * m, allDay: true }, 0, 15, null],
  ];
  for (const [it, now, within, want] of cases) assert.equal(startsIn(it, now, within), want, JSON.stringify([it, now]));
});

test("připomínka: volby, vlastní hodnota z configu, výchozí 5", () => {
  assert.deepEqual(remindOptions(5), [0, 1, 5, 10, 15, 30]);
  assert.deepEqual(remindOptions(7), [0, 1, 5, 7, 10, 15, 30]);
  assert.deepEqual(remindOptions(500), [0, 1, 5, 10, 15, 30]);
  assert.deepEqual(remindOptions(undefined), [0, 1, 5, 10, 15, 30]);
  assert.equal(remindOf({}), 5);
  assert.equal(remindOf({ remindMin: 0 }), 0);
  assert.equal(remindOf(null), 5);
});
