// „Připojit: <schůzka>“ v paletě (`meet.ts`): jen dnešní, neskončené události s odkazem.
//   node --experimental-strip-types --test src/palette/meet.test.mjs
import { test } from "node:test";
import assert from "node:assert/strict";
import { meetItems } from "./meet.ts";

const NOW = new Date(2026, 10, 13, 10, 0).getTime();
const at = (h, m = 0) => new Date(2026, 10, 13, h, m).getTime();
const ev = (title, start, end, join = { provider: "teams", url: `https://teams.example/${title}` }) => ({ title, start, end, allDay: false, location: "", cal: 0, ongoing: false, tzGuess: false, join });
const snap = (today, tomorrow = []) => ({ personal: { calendar: { status: "ok", more: 0, calendars: [], loading: false, days: [{ kind: "today", date: "x", items: today }, { kind: "tomorrow", date: "y", items: tomorrow }] } } });
const opened = [];
const items = (s, now = NOW) => meetItems(s, now, (u) => opened.push(u));

test("bez kalendáře / bez odkazu / skončená nebo zítřejší schůzka = nic", () => {
  assert.deepEqual(items({ personal: undefined }), []);
  assert.deepEqual(items(snap([ev("Bez odkazu", at(11), at(12), null)])), []);
  assert.deepEqual(items(snap([ev("Ráno", at(8), at(9))])), [], "skončila");
  assert.deepEqual(items(snap([], [ev("Zítra", at(11), at(12))])), [], "zítřek není dnes");
});

test("dnešní s odkazem: řazeno podle času, probíhající i budoucí, Enter otevře odkaz", () => {
  const out = items(snap([ev("Pozdě", at(15), at(16)), ev("Teď", at(9, 30), at(10, 30), { provider: "zoom", url: "https://zoom.example/1" })]));
  assert.deepEqual(out.map((i) => i.title), ["Připojit: Teď", "Připojit: Pozdě"]);
  assert.equal(out[0].sub, "09:30 · Zoom");
  opened.length = 0;
  out[0].run();
  assert.deepEqual(opened, ["https://zoom.example/1"]);
});
