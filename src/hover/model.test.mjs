// Tabulkové testy náhledu po najetí (`model.ts`, `modes.ts`): priorita dění ostrůvku, co ukáže ostrůvek a dok
// kdy se náhled sám rozbalí, režimy pro Nastavení. Bez DOM.
//   node --experimental-strip-types --test src/hover/model.test.mjs
import { test } from "node:test";
import assert from "node:assert/strict";
import { arrowPlan, dockOfView, dockPinAfter, dockSlots, EXPAND_IGNORE, volumeStep, wheelToStep, ackDone, doneKey, doneSession, expandAfter, freshWeather, isHoverView, islandEvent, smallWidth, timerActive, DONE_FRESH_MS, WX_MAX_AGE_MS } from "./model.ts";
import { HOVER_MODES } from "./modes.ts";
import { tIn } from "../i18n.ts";

const NOW = 1_792_250_979_530;
const MIN = 60_000;

const sess = (id, status, since = null, ended = false) => ({ id, name: id, status, since, ended, updatedAt: NOW, canOpen: true });
const media = (status) => ({ title: "Midnight City", artist: "M83", app: "Spotify", status, canToggle: true, canPrev: true, canNext: true, coverKey: null });
const data = (o = {}) => ({ pending: [], questions: [], sessions: [], personal: { media: null }, ...o });

test("ostrůvek: priorita dění", () => {
  const cd = (state) => ({ presets: [], stopwatch: null, countdown: state === "done" ? { state, at: NOW - MIN, total: 5 * MIN } : { state, end: NOW + MIN, left: MIN, total: 5 * MIN } });
  const cases = [
    ["klid", data(), "idle"],
    ["schválení přebije všechno", data({ pending: [{ id: 7 }], questions: [{ id: 3 }], sessions: [sess("a", "working", NOW - MIN)], personal: { media: media("playing") } }), "approval"],
    ["otázka", data({ questions: [{ id: 3 }], sessions: [sess("a", "idle", NOW - MIN)] }), "question"],
    ["hotovo před časovačem", data({ sessions: [sess("a", "idle", NOW - MIN)], timer: cd("running") }), "done"],
    ["časovač před hudbou", data({ timer: cd("paused"), personal: { media: media("playing") } }), "timer"],
    ["doběhlý časovač", data({ timer: cd("done") }), "timer"],
    ["stopky", data({ timer: { presets: [], countdown: null, stopwatch: { state: "running", start: NOW - MIN } } }), "timer"],
    ["pozastavené stopky = nic", data({ timer: { presets: [], countdown: null, stopwatch: { state: "paused", elapsed: 5 } } }), "idle"],
    ["hraje hudba před pracujícím Claudem", data({ sessions: [sess("a", "working", NOW - MIN)], personal: { media: media("playing") } }), "music"],
    ["Claude pracuje", data({ sessions: [sess("a", "working", NOW - MIN)], personal: { media: media("paused") } }), "working"],
    ["pozastavená hudba až po práci", data({ personal: { media: media("paused") } }), "music"],
    ["zastavená hudba = klid", data({ personal: { media: media("stopped") } }), "idle"],
    ["schůzka za 3 min přebije hudbu i hotovo", data({ personal: { media: media("playing"), calendar: { days: [{ kind: "today", items: [{ title: "Daily", start: NOW + 3 * MIN, end: NOW + 30 * MIN, allDay: false }] }] } }, sessions: [sess("a", "idle", NOW - MIN)] }), "meeting"],
    ["schůzka za 30 min nic nemění", data({ personal: { media: media("playing"), calendar: { days: [{ kind: "today", items: [{ title: "Daily", start: NOW + 30 * MIN, end: NOW + 60 * MIN, allDay: false }] }] } } }), "music"],
    ["schválení přebije schůzku", data({ pending: [{ id: 1 }], personal: { media: null, calendar: { days: [{ kind: "today", items: [{ title: "Daily", start: NOW + 0, end: NOW + MIN, allDay: false }] }] } } }), "approval"],
    ["limit 5h ≥ 90 % před hotovo", data({ sessions: [sess("a", "idle", NOW - MIN)], limits: { at: NOW - MIN, staleMs: 10 * MIN, fiveHour: { pct: 93, resetsAt: NOW + 60 * MIN }, history5h: [] } }), "limit"],
    ["limit pod prahem = klid", data({ limits: { at: NOW - MIN, staleMs: 10 * MIN, fiveHour: { pct: 50, resetsAt: NOW + 60 * MIN }, history5h: [] } }), "idle"],
    ["schůzka přebije limit", data({ limits: { at: NOW - MIN, staleMs: 10 * MIN, fiveHour: { pct: 99, resetsAt: NOW + 60 * MIN }, history5h: [] }, personal: { media: null, calendar: { days: [{ kind: "today", items: [{ title: "Daily", start: NOW + MIN, end: NOW + 30 * MIN, allDay: false }] }] } } }), "meeting"],
    ["stará nečinná session = klid", data({ sessions: [sess("a", "idle", NOW - DONE_FRESH_MS - 1)] }), "idle"],
    ["skončená session nic nehlásí", data({ sessions: [sess("a", "idle", NOW - MIN, true), sess("b", "working", NOW, true)] }), "idle"],
  ];
  for (const [name, d, want] of cases) assert.equal(islandEvent(d, NOW).kind, want, name);
  const w = islandEvent(data({ sessions: [sess("a", "waiting"), sess("b", "working", NOW), sess("c", "working", NOW)] }), NOW);
  assert.deepEqual([w.kind, w.session.id, w.count], ["working", "b", 2]);
});

test("hotovo: nejčerstvější nečinná, bez budoucnosti a bez neznámého času", () => {
  const ss = [sess("old", "idle", NOW - 4 * MIN), sess("new", "idle", NOW - MIN), sess("nil", "idle", null), sess("fut", "idle", NOW + MIN)];
  assert.equal(doneSession(ss, NOW)?.id, "new");
  assert.equal(doneSession([sess("x", "working", NOW - MIN)], NOW), null);
  assert.equal(doneSession([sess("x", "idle", NOW - DONE_FRESH_MS)], NOW)?.id, "x", "hranice platí");
});

test("hotovo: potvrzení skryje událost, nové dokončení téže session ji ukáže znovu (L146)", () => {
  const a = sess("a", "idle", NOW - MIN);
  const acked = ackDone(new Map(), a, NOW);
  assert.equal(islandEvent(data({ sessions: [a] }), NOW, acked).kind, "idle", "potvrzená nevisí");
  assert.equal(islandEvent(data({ sessions: [a] }), NOW).kind, "done", "bez potvrzení visí");
  const again = sess("a", "idle", NOW + 10 * MIN);
  assert.equal(islandEvent(data({ sessions: [again] }), NOW + 11 * MIN, acked).kind, "done", "jiný čas dokončení = nová událost");
  const other = sess("b", "idle", NOW - 2 * MIN);
  assert.equal(doneSession([a, other], NOW, acked)?.id, "b", "jiná session zůstává");
  assert.equal(islandEvent(data({ pending: [{ id: 1 }], sessions: [a] }), NOW, acked).kind, "approval");
  // ohraničení: staré klíče se zahazují, session bez času se neeviduje
  const later = NOW + DONE_FRESH_MS + MIN;
  const pruned = ackDone(acked, other, later);
  assert.equal(pruned.has(doneKey(a)), false);
  assert.equal(pruned.has(doneKey(other)), true);
  assert.equal(ackDone(new Map(), sess("n", "idle", null), NOW).size, 0);
});

test("časovač: doběhlý jen 10 min", () => {
  const done = (ago) => ({ presets: [], stopwatch: null, countdown: { state: "done", at: NOW - ago, total: MIN } });
  assert.equal(timerActive(done(9 * MIN), NOW), "countdown");
  assert.equal(timerActive(done(10 * MIN), NOW), null);
  assert.equal(timerActive(undefined, NOW), null);
});

test("samo-rozbalení: kdy odpočítávat", () => {
  // (ms, čeká schválení, myš nad tlačítkem) → ms | null
  const cases = [
    [1200, false, false, 1200],
    [0, false, false, null],
    [undefined, false, false, null],
    [1200, true, false, null],
    [1200, false, true, null],
    [-5, false, false, null],
  ];
  for (const [ms, waiting, over, want] of cases) assert.equal(expandAfter(ms, waiting, over), want, JSON.stringify([ms, waiting, over]));
});

test("pohledy po najetí a jejich šířka", () => {
  const cases = [["compact", false, null], ["island", true, 480], ["peek", false, 680], ["full", false, null], ["hidden", false, null], ["mid", false, 640]];
  for (const [v, hover, w] of cases) {
    assert.equal(isHoverView(v), hover, v);
    assert.equal(smallWidth(v), w, v);
  }
  assert.equal(smallWidth("island", true), 640, "dok je širší než ostrůvek (L148)");
  assert.equal(smallWidth("island", false), 480);
  assert.equal(smallWidth("compact", true), null, "zrušený kompaktní náhled (kolo 34) nemá šířku");
});

test("počasí v klidu: jen čerstvá data", () => {
  const w = (status, fetchedMs, data = { temp: 12, desc: "zataženo", icon: "☁" }) => ({ status, label: "", data, fetchedMs, error: null, loading: false });
  const cases = [
    ["čerstvé", w("ok", NOW - MIN), true],
    ["staré podle času", w("ok", NOW - WX_MAX_AGE_MS), false],
    ["stale", w("stale", NOW - MIN), false],
    ["bez dat", w("ok", NOW, null), false],
    ["bez času", w("ok", null), false],
    ["chybí úplně", undefined, false],
  ];
  for (const [name, x, want] of cases) assert.equal(freshWeather(x, NOW) !== null, want, name);
});

test("režimy pro Nastavení: id jsou kontrakt, názvy v obou jazycích", () => {
  assert.deepEqual(HOVER_MODES.map((m) => m.id), ["full", "island", "dock"]);
  for (const m of HOVER_MODES) {
    assert.ok(m.cs && m.en && m.key === `cards.hover.mode.${m.id}`, m.id);
  }
});

test("anglický název režimu = slovník (ať Nastavení i t() říkají totéž)", async () => {
  await import("../i18n/en-hover.ts");
  for (const m of HOVER_MODES) {
    assert.equal(tIn("en", m.key, m.cs), m.en, m.id);
  }
});

test("dok: sloty z dat (hudba jen s daty, časovač jen když je vidět, hlasitost jen když ji jádro zná)", () => {
  const vol = { level: 40, muted: false, device: "" };
  const cd = { presets: [], stopwatch: null, countdown: { state: "running", end: NOW + MIN, left: MIN, total: 5 * MIN } };
  const cases = [
    ["nic", data(), null, null],
    ["hraje + hlasitost", data({ personal: { media: media("playing"), volume: vol } }), "playing", "vol"],
    ["pauza bez hlasitosti", data({ personal: { media: media("paused") } }), "paused", null],
    ["zastavená hudba = bez slotu", data({ personal: { media: media("stopped"), volume: vol } }), null, null],
  ];
  for (const [name, d, st, v] of cases) {
    const x = dockSlots(d, NOW);
    assert.equal(x.music?.media.status ?? null, st, name);
    assert.equal(x.music?.volume ? "vol" : null, v, name);
    assert.equal(x.timer, null, name);
  }
  assert.equal(dockSlots(data({ timer: cd }), NOW).timer, "countdown");
  const dn = sess("a", "idle", NOW - MIN);
  assert.equal(dockSlots(data({ sessions: [dn] }), NOW).done?.id, "a", "hotovo je slot, ne převzetí okna");
  assert.equal(dockSlots(data({ sessions: [dn] }), NOW, ackDone(new Map(), dn, NOW)).done, null, "potvrzené = bez slotu");
  assert.equal(dockSlots(data(), NOW).done, null);
  assert.equal(dockSlots(data({ timer: { presets: [], countdown: null, stopwatch: { state: "paused", elapsed: 5 } } }), NOW).timer, null);
});

test("dok: kolečko a hlasitost (±5, strop 0–100)", () => {
  const wheel = [[-100, 1], [-1, 1], [120, -1], [0, 0], [NaN, 0], [Infinity, 0]];
  for (const [dy, want] of wheel) assert.equal(wheelToStep(dy), want, String(dy));
  const vol = [[40, 1, 45], [40, -1, 35], [98, 1, 100], [100, 1, 100], [3, -1, 0], [0, -1, 0], [40, 3, 55], [40.4, 1, 45], [NaN, 1, 5], [40, 0, 40]];
  for (const [lv, st, want] of vol) assert.equal(volumeStep(lv, st), want, `${lv} ${st}`);
});

test("dok: šipky — ↑↓ hlasitost, ←→ seek ±10 s jen když to jde", () => {
  const tl = (o = {}) => ({ positionMs: 30_000, durationMs: 200_000, updatedMs: NOW, rate: 1, canSeek: true, ...o });
  const cases = [
    ["nahoru", "ArrowUp", tl(), { kind: "volume", steps: 1 }],
    ["dolů", "ArrowDown", null, { kind: "volume", steps: -1 }],
    ["vpřed", "ArrowRight", tl(), { kind: "seek", ms: 40_000 }],
    ["zpět", "ArrowLeft", tl(), { kind: "seek", ms: 20_000 }],
    ["zpět ořez na 0", "ArrowLeft", tl({ positionMs: 4_000 }), { kind: "seek", ms: 0 }],
    ["vpřed ořez na délku", "ArrowRight", tl({ positionMs: 195_000 }), { kind: "seek", ms: 200_000 }],
    ["zdroj nemá seek", "ArrowRight", tl({ canSeek: false }), null],
    ["bez timeline", "ArrowLeft", null, null],
    ["jiná klávesa", "a", tl(), null],
  ];
  for (const [name, key, line, want] of cases) assert.deepEqual(arrowPlan(key, line, false, NOW), want, name);
  assert.deepEqual(arrowPlan("ArrowRight", tl(), true, NOW + 5_000), { kind: "seek", ms: 45_000 }, "hraje: pozice tiká");
});

test("dok: odpočet rozbalení nad sloty a při upevnění stojí", () => {
  // [ms, waiting, overControl, pinned, want]
  const cases = [
    [3000, false, false, false, 3000],
    [3000, false, true, false, null], // myš nad slotem / tlačítkem
    [3000, true, false, false, null], // píše se poznámka
    [3000, false, false, true, null], // 📌
    [0, false, false, false, null],
  ];
  for (const [ms, w, o, pin, want] of cases) assert.equal(expandAfter(ms, w, o, pin), want, JSON.stringify([ms, w, o, pin]));
  assert.ok(EXPAND_IGNORE.includes("[data-noexpand]"), "sloty docku jsou data-noexpand");
});

test("dok: ViewMsg.dock jen u island; stav 📌 (L137): set ho nastaví, view ruší", () => {
  for (const [m, want] of [[{ view: "island", dock: true }, true], [{ view: "island", dock: false }, false], [{ view: "full", dock: true }, null], [{ view: "hidden", dock: false }, null]]) assert.equal(dockOfView(m), want, JSON.stringify(m));
  assert.equal(dockPinAfter({ type: "set", on: true }), true);
  assert.equal(dockPinAfter({ type: "set", on: false }), false);
  assert.equal(dockPinAfter({ type: "view" }), false, "nové show / skrytí upevnění ruší");
  assert.equal(dockPinAfter({ type: "view", pinned: false }), false);
  assert.equal(dockPinAfter({ type: "view", pinned: true }), true, "rozbalení docku drží upevnění, když ho jádro ohlásí");
});

test("dok: anglické texty existují", async () => {
  await import("../i18n/en-hover.ts");
  for (const k of ["note", "noteHint", "saved", "pin", "unpin", "mute", "unmute", "wheelHint"]) assert.ok(tIn("en", `cards.hover.dock.${k}`, "").length > 2, k);
});
