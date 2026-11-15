// @vitest-environment node
// Tabulkové testy „Claude od posledně“ (`since.ts`): co čeká / doběhlo / selhalo / skončilo po schování šuplíku.
//   node --experimental-strip-types --test src/suggest/since.test.mjs
import { test } from "node:test";
import assert from "node:assert/strict";
import { claudeSince, sinceSig, sinceTarget } from "./since.ts";

const H = 1_800_000_000_000; // kdy se šuplík schoval
const s = (id, o = {}) => ({ id, name: id.toUpperCase(), status: "idle", ended: false, endReason: null, updatedAt: H, canOpen: true, activity: null, ...o });
const act = (phase, at) => ({ phase, at });
const view = (list, hidden = H) => (claudeSince(list, hidden) ?? []).map((i) => `${i.kind}:${i.id}`);

test("od posledního schování: doběhlo, chyba, čeká, skončila — v pořadí čeká · chyba · doběhlo · skončila", () => {
  const list = [
    s("a", { activity: act("done", H + 5) }),
    s("b", { status: "waiting", activity: act("working", H - 100) }),
    s("c", { activity: act("error", H + 1) }),
    s("d", { ended: true, status: "ended", updatedAt: H + 9, endReason: "logout" }),
    s("e", { activity: act("done", H + 7) }),
  ];
  assert.deepEqual(view(list), ["waiting:b", "failed:c", "done:a", "done:e", "ended:d"]);
});

test("co bylo před schováním (už viděné) se nepočítá; čekání platí vždy", () => {
  assert.deepEqual(view([s("a", { activity: act("done", H - 1) }), s("c", { activity: act("error", H) }), s("d", { ended: true, status: "ended", updatedAt: H })]), []);
  assert.equal(claudeSince([s("a", { activity: act("done", H - 1) })], H), null);
  assert.deepEqual(view([s("w", { status: "waiting", activity: act("done", H - 5) })]), ["waiting:w"]);
});

test("bez znalosti schování (null / undefined) jen čekající, nic „od posledně“", () => {
  const list = [s("a", { activity: act("done", H + 5) }), s("w", { status: "waiting" }), s("d", { ended: true, status: "ended", updatedAt: H + 9 })];
  for (const h of [null, undefined]) assert.deepEqual((claudeSince(list, h) ?? []).map((i) => `${i.kind}:${i.id}`), ["waiting:w"]);
});

test("fáze, které nejsou konec / chyba, a session bez aktivity nic neukážou; rozpracovaná po done už není done", () => {
  const list = [s("r", { activity: act("reading", H + 1) }), s("n"), s("w2", { status: "working", activity: act("working", H + 3) })];
  assert.equal(claudeSince(list, H), null);
});

test("endReason: nese se jen u skončené, jinak null", () => {
  const got = claudeSince([s("d", { ended: true, status: "ended", updatedAt: H + 1, endReason: "prompt_input_exit" }), s("x", { ended: true, status: "ended", updatedAt: H + 1 })], H);
  assert.deepEqual(got.map((i) => [i.id, i.reason]), [["d", "prompt_input_exit"], ["x", null]]);
  assert.equal(claudeSince([s("a", { activity: act("done", H + 1), endReason: "logout" })], H)[0].reason, null);
});

test("sinceTarget: první otevíratelná neskončená; skončené a neotevíratelné ne", () => {
  const items = claudeSince([s("n", { status: "waiting", canOpen: false }), s("a", { activity: act("done", H + 1) }), s("d", { ended: true, status: "ended", updatedAt: H + 1 })], H);
  assert.equal(sinceTarget(items).id, "a");
  assert.equal(sinceTarget(claudeSince([s("d", { ended: true, status: "ended", updatedAt: H + 1 })], H)), null);
  assert.equal(sinceTarget([]), null);
});

test("sinceSig: stejný obsah = stejný podpis, změna stavu / důvodu ho mění", () => {
  const a = claudeSince([s("a", { activity: act("done", H + 1) })], H);
  const b = claudeSince([s("a", { activity: act("done", H + 99) })], H);
  assert.equal(sinceSig(a), sinceSig(b));
  assert.notEqual(sinceSig(a), sinceSig(claudeSince([s("a", { status: "waiting" })], H)));
  assert.equal(sinceSig(undefined), "");
});

test("L233: chyba nástroje u pracující session není „selhalo“; po ukončení tahu (idle) ano", () => {
  assert.deepEqual(view([s("w", { status: "working", activity: act("error", H + 1) })]), []);
  assert.deepEqual(view([s("i", { status: "idle", activity: act("error", H + 1) })]), ["failed:i"]);
});
