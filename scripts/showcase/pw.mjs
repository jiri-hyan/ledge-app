// Sdílený loader playwright-core pro skripty (showcase, qa): z devDependency, jinak PLAYWRIGHT_CORE=<složka s node_modules>.
import { createRequire } from "node:module";
import { join } from "node:path";

export function loadPlaywright(root) {
  const require = createRequire(join(root, "package.json"));
  for (const dir of [process.env.PLAYWRIGHT_CORE, ""].filter((x) => x !== undefined)) {
    try { return require(dir ? join(dir, "playwright-core") : "playwright-core"); } catch { /* další */ }
  }
  throw new Error("playwright-core nenalezen: pnpm install, nebo PLAYWRIGHT_CORE=<složka s node_modules>");
}
