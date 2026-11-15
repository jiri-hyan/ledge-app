// Velikost JS/CSS na kritické cestě startu: sečte soubory z `dist/index.html` (entry script + modulepreload + stylesheet).
// `node scripts/bundle-report.mjs [dist] [--json]` — před/po změně bundlu (po `corepack pnpm build`). Raw bajty, ne gzip.
import { readFileSync, statSync, existsSync } from "node:fs";
import { join } from "node:path";

/** Z textu index.html vytáhne cesty startovních souborů (relativně k dist). */
export function startupFiles(html) {
  const out = new Set();
  for (const m of html.matchAll(/<(?:script|link)\b[^>]*?(?:src|href)="\.?\/?(assets\/[^"]+)"[^>]*>/g)) out.add(m[1]);
  return [...out];
}

export function report(dist) {
  const html = readFileSync(join(dist, "index.html"), "utf8");
  const files = startupFiles(html).map((f) => ({ f, bytes: statSync(join(dist, f)).size }));
  const sum = (re) => files.filter((x) => re.test(x.f)).reduce((a, x) => a + x.bytes, 0);
  return { files, js: sum(/\.js$/), css: sum(/\.css$/) };
}

if (process.argv[1]?.endsWith("bundle-report.mjs")) {
  const dist = process.argv.slice(2).find((a) => !a.startsWith("--")) ?? "dist";
  if (!existsSync(join(dist, "index.html"))) throw new Error(`chybí ${dist}/index.html (nejdřív build)`);
  const r = report(dist);
  if (process.argv.includes("--json")) console.log(JSON.stringify({ js: r.js, css: r.css, files: r.files.length }));
  else {
    for (const x of r.files.sort((a, b) => b.bytes - a.bytes)) console.log(`${(x.bytes / 1024).toFixed(1).padStart(8)} kB  ${x.f}`);
    console.log(`JS ${(r.js / 1024).toFixed(1)} kB, CSS ${(r.css / 1024).toFixed(1)} kB na startu (${r.files.length} souborů)`);
  }
}
