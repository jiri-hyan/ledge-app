// Okno s webem v UI (bez DOM): kontrola rozepsané akce `kind: "web"` a úklid před uložením
// (zrcadlo `webpanel::check_url` / `check_size` v Rustu, který je autoritou).
import { test } from "node:test";
import assert from "node:assert/strict";
import { cleanAction, draftError, WEB_SIZE } from "./model.ts";

const web = { id: "yt", label: "YT Music", kind: "web", target: "https://music.youtube.com" };

test("okno s webem: draftError — tabulka", () => {
  const cases = [
    [web, null],
    [{ ...web, target: "http://localhost:5173" }, null],
    [{ ...web, target: "music.youtube.com" }, "target"],
    [{ ...web, target: "file:///C:/x.html" }, "target"],
    [{ ...web, target: "javascript:alert(1)" }, "target"],
    [{ ...web, target: "http://tauri.localhost/index.html" }, "target"],
    [{ ...web, target: "http://ipc.localhost:80/x" }, "target"],
    [{ ...web, target: "https://user:pw@evil.cz" }, "target"],
    [{ ...web, width: 420, height: 640 }, null],
    [{ ...web, width: WEB_SIZE.minW - 1 }, "size"],
    [{ ...web, height: WEB_SIZE.maxH + 1 }, "size"],
    [{ ...web, width: 400.5 }, "size"],
    [{ ...web, label: "" }, "label"],
  ];
  for (const [a, field] of cases) assert.equal(draftError(a)?.field ?? null, field, JSON.stringify(a));
});

test("okno s webem: cleanAction nechá rozměry jen u web a jen jiné než výchozí", () => {
  assert.deepEqual(cleanAction({ ...web, width: 420, height: 720, args: ["x"], cwd: "C:/repo" }), { ...web, width: 420 });
  assert.deepEqual(cleanAction({ ...web, width: WEB_SIZE.w, height: WEB_SIZE.h }), web);
  assert.deepEqual(cleanAction({ ...web, kind: "url", width: 420 }), { ...web, kind: "url" });
  assert.deepEqual(cleanAction({ ...web, width: undefined, height: 900 }), { ...web, height: 900 });
});
