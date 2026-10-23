// Guard dokumentace configu: čisté pravidla (tabulkově, vč. negativních případů) + test nad skutečným stromem.
//   node --experimental-strip-types --test scripts/config-keys.test.mjs
import { test } from "node:test";
import assert from "node:assert/strict";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { ALLOW, checkConfigDocs, jsonKey, keyPaths, parseStructs, readSources, undocumented } from "./config-keys.mjs";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");

const RS = `
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(default, rename_all = "camelCase")]
pub struct Config {
    /// komentář
    #[serde(rename = "$schema", skip_serializing_if = "Option::is_none")]
    pub schema: Option<String>,
    pub drawer: DrawerConfig,
    pub projects: Vec<Project>,
    #[serde(skip)]
    pub runtime_only: u32,
    pub plain: crate::other::Plain,
}

#[serde(default, rename_all = "camelCase")]
pub struct DrawerConfig {
    pub keep_warm_s: u64,
    pub bar_windows: BarWindowsConfig,
}

#[serde(rename_all = "camelCase")]
pub struct BarWindowsConfig {
    pub push_ms: u64,
}

pub struct Project {
    pub id: String,
    #[serde(default)]
    pub open_browser: bool,
}

pub struct Plain {
    pub snake_case: bool,
}

pub struct Unrelated {
    pub nikdy: bool,
}
`;

test("parseStructs: struktury, pole, atributy; komentáře a prázdné řádky nerozbijí atributy", () => {
  const s = parseStructs(RS);
  assert.deepEqual(s.map((x) => x.name), ["Config", "DrawerConfig", "BarWindowsConfig", "Project", "Plain", "Unrelated"]);
  assert.equal(s[0].rename, "camelCase");
  assert.equal(s[3].rename, undefined);
  assert.deepEqual(s[1].fields.map((f) => f.name), ["keep_warm_s", "bar_windows"]);
  assert.match(s[0].fields[0].attrs.join(" "), /rename = "\$schema"/);
});

test("jsonKey: rename, camelCase, skip", () => {
  const [config, , , project, plain] = parseStructs(RS);
  assert.equal(jsonKey(config, config.fields[0]), "$schema");
  assert.equal(jsonKey(config, config.fields[3]), null); // #[serde(skip)]
  assert.equal(jsonKey(project, project.fields[1]), "open_browser"); // bez rename_all zůstává snake_case
  assert.equal(jsonKey(plain, plain.fields[0]), "snake_case");
  const [, drawer] = parseStructs(RS);
  assert.equal(jsonKey(drawer, drawer.fields[0]), "keepWarmS");
});

test("keyPaths: rozvinutí vnořených struktur i Vec<Struct>; nesouvisející struktura mimo", () => {
  const paths = keyPaths(parseStructs(RS));
  assert.deepEqual(paths, [
    "$schema", "drawer", "drawer.keepWarmS", "drawer.barWindows", "drawer.barWindows.pushMs",
    "projects", "projects.id", "projects.open_browser", "plain", "plain.snake_case",
  ]);
});

test("keyPaths: rekurzivní struktura se nezacyklí", () => {
  const src = "#[serde(rename_all = \"camelCase\")]\npub struct Config {\n    pub kid: Kid,\n}\npub struct Kid {\n    pub parent: Option<Config>,\n    pub x: u8,\n}\n";
  assert.deepEqual(keyPaths(parseStructs(src)), ["kid", "kid.parent", "kid.x"]);
});

test("undocumented: klíč bez zmínky se ohlásí, slovo musí být celé, výjimka a $schema", () => {
  const docs = "`drawer.keepWarmS` a taky pushMsExtra, `open_browser`";
  const paths = ["drawer.keepWarmS", "drawer.barWindows.pushMs", "projects.open_browser", "$schema"];
  assert.deepEqual(undocumented(paths, docs, {}), ["drawer.barWindows.pushMs", "$schema"]); // pushMsExtra není pushMs
  assert.deepEqual(undocumented(paths, docs, { $schema: "editor", "drawer.barWindows.pushMs": "test" }), []);
  assert.deepEqual(undocumented(["a.b"], "", {}), ["a.b"]);
});

test("skutečný strom: struktury se načetly a známé klíče v nich jsou (pojistka proti tichému rozbití parseru)", () => {
  const { structs } = readSources(root);
  const paths = new Set(keyPaths(structs));
  assert.ok(paths.size > 200, `parser našel jen ${paths.size} klíčů`);
  for (const k of ["drawer.keepWarmS", "drawer.barWindows.pushMs", "claude.enforceCommit", "personal.weather.lat", "layout", "appearance", "projects.muteDirty", "gitAlerts", "update.intervalH"]) {
    assert.ok(paths.has(k), `chybí klíč ${k}`);
  }
});

test("skutečný strom: každý klíč config.json je zmíněn v docs/agents/*.md", () => {
  const { missing } = checkConfigDocs(root);
  assert.deepEqual(missing, [], "klíč bez zmínky — doplň ho do docs/agents/config*.md (nebo výjimka s důvodem v ALLOW)");
});

test("ALLOW: výjimky mají důvod a nejsou zbytečné", () => {
  const { structs, docs } = readSources(root);
  const paths = new Set(keyPaths(structs));
  for (const [k, why] of Object.entries(ALLOW)) {
    assert.ok(why.length > 10, `výjimka ${k} bez důvodu`);
    assert.ok(paths.has(k), `výjimka ${k} už není klíč configu`);
    assert.equal(undocumented([k], docs, {}).length, 1, `výjimka ${k} je zbytečná — klíč už je v docs`);
  }
});
