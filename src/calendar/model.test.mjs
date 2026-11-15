// Tabulkové testy karty Kalendář (`model.ts`): barvy, úprava odkazu (shoda s Rust `normalize_url`),
// maskování adresy, čas řádku a změna jednoho kalendáře v configu. Časy přes místní `Date` (nezávisí na pásmu stroje).
//   node --experimental-strip-types --test src/calendar/model.test.mjs
import { test } from "node:test";
import assert from "node:assert/strict";
import { CAL_COLORS, avatarColor, avatarRow, calColor, initials, meetingSpan, minLabel, domainOf, joinable, nextColor, normalizeUrl, outlookFeed, placeLabel, provider, providerName, remindOf, remindOptions, sourceOf, timeLabel, withCalendar } from "./model.ts";

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
  assert.deepEqual([o.source, o.url, o.remindMin, o.remindToast, o.enabled], ["outlook-local", "", 5, true, true]);
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

test("délka schůzky: před (do hodiny), během, hranice, celodenní, přes půlnoc", () => {
  const m = 60_000;
  const H = 60 * m;
  const it = (start, end, allDay = false) => ({ start, end, allDay });
  const cases = [
    ["za 8 min na 30 min", it(8 * m, 38 * m), 0, { phase: "before", startsIn: 8, total: 30 }],
    ["zaokrouhleno nahoru", it(5 * m, 35 * m), 1, { phase: "before", startsIn: 5, total: 30 }],
    ["přesně hodinu před", it(60 * m, 90 * m), 0, { phase: "before", startsIn: 60, total: 30 }],
    ["hodinu a minutu před = nic", it(61 * m, 90 * m), 0, null],
    ["začíná právě teď = běží", it(0, 30 * m), 0, { phase: "running", run: 0, left: 30, total: 30 }],
    ["běží 35 zbývá 12", it(0, 47 * m), 35 * m, { phase: "running", run: 35, left: 12, total: 47 }],
    ["zbývá zaokrouhleno nahoru, aspoň 1", it(0, 30 * m), 30 * m - 1, { phase: "running", run: 29, left: 1, total: 30 }],
    ["konec = skončila", it(0, 30 * m), 30 * m, null],
    ["nulová délka", it(5 * m, 5 * m), 0, null],
    ["celodenní", it(0, 24 * H, true), 0, null],
    ["přes půlnoc", it(23 * H, 25 * H), 24 * H + 30 * m, { phase: "running", run: 90, left: 30, total: 120 }],
  ];
  for (const [name, ev, now, want] of cases) assert.deepEqual(meetingSpan(ev, now), want, name);
  assert.equal(meetingSpan(it(120 * m, 150 * m), 0, 180)?.phase, "before", "vlastní okno");
  assert.deepEqual([0, 45, 60, 90, 130, 120].map(minLabel), ["0 min", "45 min", "1 h", "1 h 30 min", "2 h 10 min", "2 h"]);
});

test("iniciály: čeština, e-mail, závorky, Příjmení Jméno, nic", () => {
  const cases = [
    ["Tomáš Hora", "", "TH"],
    ["  tomáš   hora ", "", "TH"],
    ["Žofie Řezníčková", "", "ŽŘ"],
    ["Hora, Tomáš", "", "TH"],
    ["Jan Karel Novák", "", "JN"],
    ["Jan Novák (Externí)", "", "JN"],
    ["Madonna", "", "M"],
    ["", "eva.svoboda@firma.cz", "E"],
    ["", "Žaneta@firma.cz", "Ž"],
    ["", "", ""],
    ["(?)", "", ""],
    ["Tomáš Hora", "x@y.cz", "TH"],
  ];
  for (const [n, e, want] of cases) assert.equal(initials(n, e), want, JSON.stringify([n, e]));
});

test("barva avataru: stabilní, z palety, malá písmena", () => {
  assert.equal(avatarColor("Tomas@Firma.cz"), avatarColor("tomas@firma.cz"));
  const used = new Set(["a@x.cz", "b@x.cz", "c@x.cz", "d@x.cz", "e@x.cz", "f@x.cz", "g@x.cz", "h@x.cz", "i@x.cz", "j@x.cz"].map(avatarColor));
  for (const c of used) assert.ok(CAL_COLORS.includes(c));
  assert.ok(used.size > 2, "rozkládá se");
});

test("řada avatarů: strop 4 + zbytek, duplicity, zdroj bez účastníků", () => {
  const P = (name, email = "", resp = "accepted") => ({ name, email, resp });
  const six = [P("Tomáš Hora", "j@x.cz"), P("Eva Černá"), P("Petr", "p@x.cz", "tentative"), P("Žofie Řezníčková"), P("Karel Nový"), P("Lenka")];
  const r = avatarRow({ list: six, more: 2 });
  assert.deepEqual(r.shown.map((a) => a.initials), ["TH", "EČ", "P", "ŽŘ"]);
  assert.equal(r.more, 4, "2 nevešli + 2 z Rustu");
  assert.equal(r.shown[2].tentative, true);
  assert.equal(r.shown[0].label, "Tomáš Hora <j@x.cz>");
  assert.equal(r.names.split("\n").length, 6);
  // duplicita (stejný e-mail jinou velikostí) a prázdný záznam se vynechají
  const d = avatarRow({ list: [P("A B", "a@x.cz"), P("a b", "A@x.cz"), P("", ""), P("(?)", "")], more: 0 });
  assert.equal(d.shown.length, 1);
  assert.equal(d.more, 0);
  // žádní účastníci / chybí pole (starý snapshot, ICS bez ATTENDEE) = nic
  for (const none of [undefined, null, {}, { list: [], more: 0 }]) assert.deepEqual(avatarRow(none).shown, []);
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

test("oldestFetchedMs: nejstarší stažení ze zapnutých kalendářů", async () => {
  const { oldestFetchedMs } = await import("./model.ts");
  const c = (enabled, fetchedMs) => ({ enabled, fetchedMs });
  const cases = [
    [undefined, null],
    [[], null],
    [[c(true, null)], null],
    [[c(true, 90), c(true, 30), c(true, 60)], 30],
    [[c(false, 1), c(true, 40)], 40],
  ];
  for (const [x, want] of cases) assert.equal(oldestFetchedMs(x), want, JSON.stringify(x));
});
