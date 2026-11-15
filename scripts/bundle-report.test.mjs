import { test } from "node:test";
import assert from "node:assert/strict";
import { startupFiles } from "./bundle-report.mjs";

test("startupFiles: script, modulepreload i stylesheet, bez duplicit", () => {
  const html = `<script type="module" crossorigin src="./assets/index-A.js"></script>
<link rel="modulepreload" crossorigin href="./assets/x-B.js">
<link rel="stylesheet" crossorigin href="./assets/index-C.css">
<link rel="modulepreload" crossorigin href="./assets/x-B.js">
<link rel="icon" href="./favicon.ico">`;
  assert.deepEqual(startupFiles(html), ["assets/index-A.js", "assets/x-B.js", "assets/index-C.css"]);
});
