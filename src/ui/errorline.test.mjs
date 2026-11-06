import { test } from "node:test"
import assert from "node:assert/strict"
import { errorLine, fresh, quietOn } from "./errorline.ts"

test("errorLine: tvary chyb", () => {
  const cases = [
    [new TypeError("x je undefined"), undefined, "TypeError: x je undefined"],
    [new Error("a\nb\n   at f (x.js:1:2)"), "app.js:3", "Error: a (app.js:3)"],
    ["Složka nenalezena.", undefined, "Složka nenalezena."],
    [{ code: 5 }, undefined, '{"code":5}'],
    [undefined, undefined, "undefined"],
    ["", undefined, ""],
    ["\n  \n", undefined, ""],
  ]
  for (const [e, where, want] of cases) assert.equal(errorLine(e, where), want, String(e))
})

test("fresh: jednou, a ne víc než strop různých", () => {
  const set = new Set()
  assert.equal(fresh("a", set), true)
  assert.equal(fresh("a", set), false)
  assert.equal(fresh("", set), false)
  for (let i = 0; i < 100; i++) fresh(`e${i}`, set)
  assert.equal(set.size, 30)
  assert.equal(fresh("nova", set), false)
})

test("quietOn: výsledek projde, chyba se zaloguje a vrátí se záloha", async () => {
  const logged = []
  const log = (e, where) => logged.push([String(e.message), where])
  assert.equal(await quietOn(log, Promise.resolve(7), 0, "ok"), 7)
  assert.deepEqual(logged, [])
  assert.deepEqual(await quietOn(log, Promise.reject(new Error("pád")), [], "seznam"), [])
  assert.equal(await quietOn(log, Promise.reject(new Error("pád2")), undefined, "x"), undefined)
  assert.deepEqual(logged, [["pád", "seznam"], ["pád2", "x"]])
})
