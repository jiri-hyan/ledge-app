import test from "node:test";
import assert from "node:assert/strict";
import { addTarget, draftIds, draftKnown, findDuplicate, idChanged, linkProblem, remoteKey } from "./projectAdd.ts";

test("remote ve srovnatelném tvaru", () => {
  const rows = [
    ["https://example.com/uzivatel/app.git", "uzivatel/app"],
    ["git@example.com:uzivatel/app", "uzivatel/app"],
    ["Uzivatel/App", "uzivatel/app"],
    ["ssh://git@example.com/uzivatel/app.git/", "uzivatel/app"],
    ["", ""],
    [null, ""],
  ];
  for (const [v, want] of rows) assert.equal(remoteKey(v), want, String(v));
});

test("duplicita podle složky i remote, prázdné se neshodují", () => {
  const list = [
    { id: "a", path: "C:/Work/App//", remote: null },
    { id: "b", path: null, remote: "https://example.com/uzivatel/app-v2.git" },
    { id: "c", path: null, remote: null },
  ];
  const rows = [
    [{ path: "c:/work/app" }, 0],
    [{ path: "D:/jina" }, -1],
    [{ path: "D:/x", remote: "git@example.com:uzivatel/app-v2" }, 1],
    [{ path: null, remote: null }, -1],
    [{}, -1],
  ];
  for (const [c, want] of rows) assert.equal(findDuplicate(list, c), want, JSON.stringify(c));
});

test("rozpracovaný návrh: id, složky, remoty", () => {
  const list = [{ id: "app", path: "C:/a", remote: "o/app" }, { id: "", path: null }, {}];
  assert.deepEqual(draftIds(list), ["app"]);
  assert.deepEqual(draftKnown(list), { paths: ["C:/a"], remotes: ["o/app"] });
});

test("odkaz projektu: stejná pravidla jako jádro", () => {
  const rows = [
    [{ label: "", url: "https://example.com" }, "label"],
    [{ label: "  ", url: "https://example.com" }, "label"],
    [{ label: "Doc", url: "" }, "url"],
    [{ label: "Doc", url: "ftp://example.com" }, "url"],
    [{ label: "Doc", url: "example.com" }, "url"],
    [{ label: "Doc", url: "http://example.com/a" }, null],
    [{ label: "Doc", url: "HTTPS://example.com" }, null],
  ];
  for (const [l, want] of rows) assert.equal(linkProblem(l)?.field ?? null, want, JSON.stringify(l));
});

test("změna id uloženého projektu", () => {
  const saved = [{ id: "app" }, { id: "app-v2" }];
  assert.equal(idChanged(saved, "app", "app"), false);
  assert.equal(idChanged(saved, "app", "app-3"), true);
  assert.equal(idChanged(saved, "novy", "novy-2"), false, "neuložený projekt nic neosiří");
});

test("vložený text: složka = projekt, ostatní akce podle typu", () => {
  const rows = [
    ["", null],
    ["https://example.com/a", { as: "action", kind: "url" }],
    ["C:/Programy/app.exe", { as: "action", kind: "app" }],
    ["C:/Doc/zprava.pdf", { as: "action", kind: "file" }],
    ["C:/Work/app", { as: "project", kind: "folder" }],
  ];
  for (const [v, want] of rows) assert.deepEqual(addTarget(v), want, v);
});

import { kindForTarget } from "../actions/model.ts";
test("editor akce: typ se přepne podle toho, co je v cíli", () => {
  const rows = [
    ["url", "C:/Work/app", "folder"],
    ["web", "C:/Doc/zprava.pdf", "file"],
    ["url", "C:/Programy/app.exe", "app"],
    ["url", "https://example.com", null],
    ["url", "example.com", null],
    ["folder", "https://example.com/a", "url"],
    ["command", "https://example.com", null],
    ["command", "C:/x", null],
    ["url", "", null],
  ];
  for (const [k, t, want] of rows) assert.equal(kindForTarget(k, t), want, `${k} ${t}`);
});
