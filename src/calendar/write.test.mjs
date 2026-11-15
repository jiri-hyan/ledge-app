// „Do kalendáře“: výběr události z poznámky (`write.ts`).
import { test } from "node:test";
import assert from "node:assert/strict";
import { firstDatedTask, pickCalEvent } from "./write.ts";

test("firstDatedTask: tabulka", () => {
  const cases = [
    ["- [ ] Doktor ⏰ 2026-11-14 09:00 !", { title: "Doktor", due: "2026-11-14 09:00", allDay: false }],
    ["- [ ] Bez termínu\n- [ ] Svátek ⏰ 2026-12-24", { title: "Svátek", due: "2026-12-24", allDay: true }],
    ["- [x] Hotovo ⏰ 2026-11-14 09:00\n- [ ] Další ⏰ 2026-11-15 10:00", { title: "Další", due: "2026-11-15 10:00", allDay: false }],
    ["```\n- [ ] V kódu ⏰ 2026-11-14 09:00\n```\n- [ ] Venku 📅 2026-11-20", { title: "Venku", due: "2026-11-20", allDay: true }],
    ["  * [ ] Odsazený ⏰ 2026-11-14 08:30\r\n", { title: "Odsazený", due: "2026-11-14 08:30", allDay: false }],
    ["- [ ] ⏰ 2026-11-14 09:00", null],
    ["- [ ] Bez termínu", null],
    ["prostý text ⏰ 2026-11-14", null],
    ["", null],
  ];
  for (const [body, want] of cases) assert.deepEqual(firstDatedTask(body), want, body);
});

test("pickCalEvent: remind má přednost, jinak bod; uid je stabilní", () => {
  const r = pickCalEvent("a.md", " Schůzka ", "- [ ] X ⏰ 2026-11-20 10:00", "2026-11-14T09:00");
  assert.deepEqual(r, { title: "Schůzka", startIso: "2026-11-14T09:00", allDay: false, uid: "a.md|remind", source: "remind" });
  const t = pickCalEvent("a.md", "Schůzka", "- [ ] X ⏰ 2026-11-20 10:00", "");
  assert.deepEqual(t, { title: "X", startIso: "2026-11-20T10:00", allDay: false, uid: "a.md|task|X", source: "task" });
  assert.equal(pickCalEvent("a.md", "Schůzka", "- [ ] X ⏰ 2026-11-20 10:00", "").uid, t.uid);
  const d = pickCalEvent("a.md", "", "- [ ] Svátek ⏰ 2026-12-24", "");
  assert.deepEqual([d.startIso, d.allDay], ["2026-12-24", true]);
});

test("pickCalEvent: odložení ⏰ nebo remind nezmění uid (P9: jinak druhá událost)", () => {
  const a = pickCalEvent("a.md", "N", "- [ ] X ⏰ 2026-11-20 10:00", "");
  const b = pickCalEvent("a.md", "N", "- [ ] X ⏰ 2026-11-27 09:30", "");
  assert.notEqual(a.startIso, b.startIso);
  assert.equal(a.uid, b.uid);
  const r1 = pickCalEvent("a.md", "N", "", "2026-11-14T09:00");
  const r2 = pickCalEvent("a.md", "N", "", "2026-11-15T10:00");
  assert.equal(r1.uid, r2.uid);
  assert.notEqual(r1.uid, a.uid, "připomínka poznámky a bod jsou dvě různé události");
});

test("pickCalEvent: nic k přidání", () => {
  assert.equal(pickCalEvent("a.md", "N", "text", ""), null);
  assert.equal(pickCalEvent("a.md", "", "text", "2026-11-14T09:00"), null, "remind bez názvu a bez bodu");
  assert.equal(pickCalEvent("a.md", "N", "text", "zítra"), null, "neplatný tvar remind");
});

test("firstDatedTask: odeslaný (řádek „↳ [“ pod bodem, jak ho píše Rust) se přeskočí, číslovaný bod se nabídne", () => {
  const cases = [
    ["- [ ] Odeslaný ⏰ 2026-11-14 09:00\n  ↳ [Demo](https://example.test/x)\n- [ ] Další ⏰ 2026-11-15 10:00", { title: "Další", due: "2026-11-15 10:00", allDay: false }],
    ["- [ ] Odeslaný ⏰ 2026-11-14 09:00\n  ![s](./a.png)\n  ↳ [Demo] abc", null],
    ["1. [ ] Číslovaný ⏰ 2026-11-14 09:00", { title: "Číslovaný", due: "2026-11-14 09:00", allDay: false }],
    ["2) [ ] Číslovaný ⏰ 2026-11-14", { title: "Číslovaný", due: "2026-11-14", allDay: true }],
    ["- [~] Navržený ⏰ 2026-11-14 09:00", null],
  ];
  for (const [body, want] of cases) assert.deepEqual(firstDatedTask(body), want, body);
});
