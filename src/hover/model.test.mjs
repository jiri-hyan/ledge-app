// Tabulkové testy náhledu po najetí (`model.ts`, `modes.ts`): priorita dění ostrůvku, co ukáže kompaktní
// náhled, kdy se náhled sám rozbalí, režimy pro Nastavení. Bez DOM.
//   node --experimental-strip-types --test src/hover/model.test.mjs
import { test } from "node:test";
import assert from "node:assert/strict";
import { compactPlan, compactSessions, doneSession, expandAfter, freshWeather, isHoverView, islandEvent, smallWidth, timerActive, DONE_FRESH_MS, SESSION_ROWS, WX_MAX_AGE_MS } from "./model.ts";
import { HOVER_MODES, hoverModeOf } from "./modes.ts";
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

test("časovač: doběhlý jen 10 min", () => {
  const done = (ago) => ({ presets: [], stopwatch: null, countdown: { state: "done", at: NOW - ago, total: MIN } });
  assert.equal(timerActive(done(9 * MIN), NOW), "countdown");
  assert.equal(timerActive(done(10 * MIN), NOW), null);
  assert.equal(timerActive(undefined, NOW), null);
});

test("kompaktní náhled: co ukáže", () => {
  const many = ["a", "b", "c", "d", "e"].map((id) => sess(id, "working", NOW));
  assert.equal(compactSessions(many).length, SESSION_ROWS);
  assert.deepEqual(compactSessions([sess("x", "ended"), sess("y", "idle", NOW, true), sess("z", "idle", NOW)]).map((s) => s.id), ["z"]);
  const lim = (at, five) => ({ at, fiveHour: five, sevenDay: null, staleMs: 1, statusLine: "full", source: "statusLine", notice: null });
  const cases = [
    ["prázdno", data(), { approval: false, sessions: 0, media: false, limit: false }],
    ["schválení + hudba + limit", data({ pending: [{ id: 1 }], personal: { media: media("paused") }, limits: lim(NOW, { pct: 42, resetsAt: null }) }), { approval: true, sessions: 0, media: true, limit: true }],
    ["otázka", data({ questions: [{ id: 1 }] }), { approval: true, sessions: 0, media: false, limit: false }],
    ["zastavená hudba ne, limit bez vzorku ne", data({ personal: { media: media("stopped") }, limits: lim(null, { pct: 1, resetsAt: null }) }), { approval: false, sessions: 0, media: false, limit: false }],
    ["limit bez 5h okna ne", data({ limits: lim(NOW, null) }), { approval: false, sessions: 0, media: false, limit: false }],
  ];
  for (const [name, d, want] of cases) {
    const p = compactPlan(d);
    assert.deepEqual({ approval: p.approval, sessions: p.sessions.length, media: !!p.media, limit: p.limit }, want, name);
  }
});

test("kompaktní náhled: servery z Ledge se ■ Stop (běží, startují, selhaly; skončené ne)", () => {
  const pc = (id, phase) => ({ id, phase, pid: 1, port: 5210, url: null, startedAt: NOW, readyAt: null, exitCode: null, message: null, lastLine: null, errors: 0 });
  assert.deepEqual(compactPlan(data()).servers, [], "bez procs (starší volající) = nic");
  const procs = [pc("a", "ready"), pc("b", "exited"), pc("c", "failed"), pc("d", "starting"), pc("e", "running")];
  assert.deepEqual(compactPlan(data({ procs })).servers.map((x) => x.id), ["a", "c", "d"], "skončený vynechán, strop 3");
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
  const cases = [["compact", true, 420], ["island", true, 480], ["peek", false, 680], ["full", false, null], ["hidden", false, null]];
  for (const [v, hover, w] of cases) {
    assert.equal(isHoverView(v), hover, v);
    assert.equal(smallWidth(v), w, v);
  }
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
  assert.deepEqual(HOVER_MODES.map((m) => m.id), ["full", "compact", "island"]);
  for (const m of HOVER_MODES) {
    assert.ok(m.cs && m.en && m.key === `cards.hover.mode.${m.id}`, m.id);
    assert.ok(m.desc.length > 10, `${m.id}: popis`);
  }
  assert.equal(hoverModeOf("island"), "island");
  assert.equal(hoverModeOf("expand"), "full");
  assert.equal(hoverModeOf(undefined), "full");
});

test("anglický název režimu = slovník (ať Nastavení i t() říkají totéž)", async () => {
  await import("../i18n/en-hover.ts");
  for (const m of HOVER_MODES) {
    assert.equal(tIn("en", m.key, m.cs), m.en, m.id);
    assert.ok(tIn("en", `${m.key}.desc`, "").length > 10, `${m.id}: anglický popis`);
  }
});
