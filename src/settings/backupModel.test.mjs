import test from "node:test";
import assert from "node:assert/strict";
import { keyPaths, readSources } from "../../scripts/config-keys.mjs";
import { SECTION_NAMES } from "./backupModel.ts";

// Sekce, které se v zálohách nenabízejí pod vlastním jménem (metadata schématu).
const SKIP = new Set(["$schema"]);

test("Záloha zná všechny sekce configu a žádnou navíc", () => {
  const { structs } = readSources(new URL("../..", import.meta.url).pathname.replace(/^\/([A-Za-z]:)/, "$1"));
  const top = keyPaths(structs).filter((p) => !p.includes(".") && !SKIP.has(p));
  const missing = top.filter((k) => !SECTION_NAMES[k]);
  const ghosts = Object.keys(SECTION_NAMES).filter((k) => !top.includes(k));
  assert.deepEqual(missing, [], "sekce bez názvu v Záloze");
  assert.deepEqual(ghosts, [], "názvy sekcí, které config nemá");
});
