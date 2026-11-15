// Složí scénu tapety: src/ext/wall/scene.html (CSS + značky + lepidlo) + src/ext/wall/model.ts (čistá logika,
// bez typů a `export`) → src-tauri/src/ext/wall/wall.html (jeden soubor, offline; Rust ho vkládá `include_str!`).
// Použití: `node scripts/wall-build.mjs` (zapíše), `--check` (jen ověří, že je aktuální; exit 1). Aktuálnost hlídá
// i src/ext/wall/model.test.mjs, takže zapomenutý build spadne v `test:ui`.
import { readFileSync, writeFileSync } from "node:fs";
import { stripTypeScriptTypes } from "node:module";
import { fileURLToPath } from "node:url";

const root = new URL("../", import.meta.url);
const SCENE = new URL("src/ext/wall/scene.html", root);
const MODEL = new URL("src/ext/wall/model.ts", root);
export const OUT = new URL("src-tauri/src/ext/wall/wall.html", root);
const MARK = "/*@MODEL@*/";

const lf = (s) => s.replace(/\r\n/g, "\n");

/** model.ts → prosté JS do `<script>`: bez typů, bez `export`, bez zbytečných prázdných řádků. */
export function modelJs(ts) {
  const js = stripTypeScriptTypes(lf(ts), { mode: "strip" })
    .replace(/^export\s+/gm, "")
    .replace(/^\s*\/\/.*$/gm, "")
    .replace(/[ \t]+$/gm, "")
    .replace(/\n{2,}/g, "\n");
  if (/\b(import|export)\b/.test(js.replace(/\/\*[\s\S]*?\*\//g, ""))) throw new Error("model.ts nesmí mít import/export za běhu");
  return js.trim();
}

export function buildWall() {
  const scene = lf(readFileSync(SCENE, "utf8"));
  if (!scene.includes(MARK)) throw new Error(`scene.html chybí ${MARK}`);
  return scene.replace(MARK, () => modelJs(readFileSync(MODEL, "utf8")));
}

if (process.argv[1] === fileURLToPath(import.meta.url)) {
  const html = buildWall();
  if (process.argv.includes("--check")) {
    const cur = lf(readFileSync(OUT, "utf8"));
    if (cur !== html) {
      console.error("wall.html je zastaralý: node scripts/wall-build.mjs");
      process.exit(1);
    }
    console.log("wall.html aktuální");
  } else {
    writeFileSync(OUT, html);
    console.log(`wall.html: ${Buffer.byteLength(html)} B`);
  }
}
