// QA před vydáním (L295, L296): čisté funkce bez I/O — detektor syrových překladových klíčů, fixture config sandboxu,
// kroky a verdikt PASS/FAIL/SKIP, skládání zprávy. Test: scripts/qa.test.mjs; běh: scripts/qa.mjs.

export const PASS = "PASS";
export const FAIL = "FAIL";
export const SKIP = "SKIP";

/** Koncovky, které z `a.b.c` dělají hostitele / soubor, ne překladový klíč (`api.anthropic.com`, `config.v2.json`). */
const NOT_KEY_TAIL = new Set(["com", "cz", "io", "dev", "net", "org", "app", "exe", "json", "md", "txt", "png", "svg", "ts", "tsx", "rs", "mjs", "js", "css", "html", "log", "toml", "yml", "yaml", "lnk", "dll", "pdf", "zip", "local"]);

/** Syrové překladové klíče v textu: `slovo.slovo.slovo` (≥ 3 segmenty, malé písmeno na začátku, bez mezer). */
export function rawKeys(text) {
  const out = new Set();
  for (const m of String(text ?? "").matchAll(/(?<![\w./@:-])[a-z][a-z0-9]+(?:\.[a-z][A-Za-z0-9]+){2,}(?![\w@-]|\.[\w])/g)) {
    const tail = m[0].slice(m[0].lastIndexOf(".") + 1);
    if (!NOT_KEY_TAIL.has(tail.toLowerCase())) out.add(m[0]);
  }
  return [...out];
}

/** Hodnota podle cesty `a.b.c` — `undefined`, když kterýkoli článek chybí (pole z /status nemusí existovat). */
export function field(obj, path) {
  let cur = obj;
  for (const k of path.split(".")) {
    if (cur === null || typeof cur !== "object" || !(k in cur)) return undefined;
    cur = cur[k];
  }
  return cur;
}

/** Je v `qa.hotkeys` akce (nebo kombinace)? `undefined`, když seznam chybí. */
export function hasHotkey(list, action, key) {
  if (!Array.isArray(list)) return undefined;
  const norm = (s) => String(s ?? "").toLowerCase().replace(/\s+/g, "");
  return list.some((h) => h && (h.action === action || (key && norm(h.key) === norm(key))));
}

/** Pohled z /status (`drawer.view`) nezávisle na velikosti písmen. */
export const viewIs = (status, want) => String(field(status, "drawer.view") ?? "").toLowerCase() === want;

/** console.error, které nejsou chyba aplikace (zatím jen favicon). */
const NOISE = [/favicon/i];
export const isNoise = (text) => NOISE.some((re) => re.test(text));

// ---------- kroky a verdikt ----------

export const step = (scenario, name, status, evidence = "") => ({ scenario, name, status, evidence: String(evidence) });

/** Tvrzení nad polem z /status: pole chybí → SKIP s důvodem (nikdy tiché PASS), jinak PASS/FAIL podle `ok`. */
export function checkField(scenario, name, status, path, ok, expect) {
  const v = field(status, path);
  if (v === undefined) return step(scenario, name, SKIP, `pole ${path} chybí v /status`);
  return step(scenario, name, ok(v) ? PASS : FAIL, `${path} = ${JSON.stringify(v)}${ok(v) ? "" : ` (čekáno ${expect})`}`);
}

export function summarize(steps) {
  const n = { PASS: 0, FAIL: 0, SKIP: 0 };
  for (const s of steps) n[s.status] = (n[s.status] ?? 0) + 1;
  return n;
}

/** Kód procesu: 0 = bez FAIL, 1 = FAIL. (2 = nespustitelné řeší qa.mjs.) */
export const exitCode = (steps) => (steps.some((s) => s.status === FAIL) ? 1 : 0);

/** Scénář bez jediného PASS/FAIL (jen SKIP) se nesmí tvářit jako úspěch — vrátí jeho jména. */
export function hollowScenarios(steps) {
  const by = new Map();
  for (const s of steps) by.set(s.scenario, [...(by.get(s.scenario) ?? []), s.status]);
  return [...by].filter(([, st]) => !st.includes(PASS) && !st.includes(FAIL)).map(([k]) => k);
}

// ---------- zpráva ----------

const cell = (s) => String(s).replace(/\|/g, "\\|").replace(/\r?\n/g, " ").slice(0, 700);

export function renderReport({ steps, binary, started, durationMs, notes = [], out }) {
  const sum = summarize(steps);
  const verdict = sum.FAIL ? "FAIL" : "PASS";
  const lines = [
    `# Ledge QA — ${verdict}`,
    "",
    `- binárka: ${binary?.path ?? "?"} (${binary?.version ?? "verze neznámá"}, ${binary?.sha ?? "bez hashe"})`,
    `- začátek: ${started}, doba běhu: ${(durationMs / 1000).toFixed(1)} s`,
    `- souhrn: ${sum.PASS} PASS · ${sum.FAIL} FAIL · ${sum.SKIP} SKIP`,
    ...(out ? [`- snímky a log: ${out}`] : []),
    "",
    "| scénář | krok | výsledek | důkaz |",
    "|---|---|---|---|",
    ...steps.map((s) => `| ${cell(s.scenario)} | ${cell(s.name)} | ${s.status} | ${cell(s.evidence)} |`),
  ];
  const hollow = hollowScenarios(steps);
  if (hollow.length) lines.push("", `Bez jediného ověření (jen SKIP): ${hollow.join(", ")}`);
  if (notes.length) lines.push("", "Po cestě:", ...notes.map((n) => `- ${n}`));
  return lines.join("\n") + "\n";
}

/** Zpráva na stdout, nejvýš `max` řádků: souhrn + FAIL/SKIP řádky. */
export function stdoutSummary(steps, reportPath, max = 25) {
  const sum = summarize(steps);
  const head = `QA ${sum.FAIL ? "FAIL" : "PASS"}: ${sum.PASS} PASS, ${sum.FAIL} FAIL, ${sum.SKIP} SKIP`;
  const rows = [
    ...steps.filter((s) => s.status === FAIL).map((s) => `FAIL ${s.scenario}/${s.name}: ${s.evidence}`),
    ...steps.filter((s) => s.status === SKIP).map((s) => `SKIP ${s.scenario}/${s.name}: ${s.evidence}`),
  ];
  const room = max - 2;
  const shown = rows.slice(0, room).map((r) => r.slice(0, 200));
  if (rows.length > room) shown[room - 1] = `… a dalších ${rows.length - room + 1} řádků ve zprávě`;
  return [head, ...shown, `zpráva: ${reportPath}`].join("\n");
}

// ---------- argumenty ----------

export function parseArgs(argv) {
  const o = { exe: null, oldExe: null, out: null, only: null, keep: false, list: false, apiPort: 47811, cdpPort: 47812 };
  for (let i = 0; i < argv.length; i++) {
    const a = argv[i];
    if (a === "--keep") o.keep = true;
    else if (a === "--list") o.list = true;
    else if (a === "--exe") o.exe = argv[++i] ?? null;
    else if (a === "--old-exe") o.oldExe = argv[++i] ?? null;
    else if (a === "--out") o.out = argv[++i] ?? null;
    else if (a === "--only") o.only = (argv[++i] ?? "").split(",").map((x) => x.trim()).filter(Boolean);
    else if (a === "--api-port") o.apiPort = Number(argv[++i]);
    else if (a === "--cdp-port") o.cdpPort = Number(argv[++i]);
    else throw new Error(`neznámý argument ${a}`);
  }
  if (!Number.isInteger(o.apiPort) || !Number.isInteger(o.cdpPort) || o.apiPort === 47800 || o.cdpPort === 47800 || o.apiPort === o.cdpPort) {
    throw new Error("porty musí být celá čísla, různá, a nikdy 47800 (instance šéfa)");
  }
  return o;
}

// ---------- fixture sandboxu ----------

export const PASSTHROUGH_KEY = "Ctrl+Alt+Shift+F11";

/** config.json izolované instance: jen klíče, které nejsou výchozí (zbytek doplní serde default). */
export function fixtureConfig({ apiPort, hotkeys = false, notesRoot }) {
  return {
    autostart: false,
    claude: { enabled: true, port: apiPort },
    notes: { enabled: true, root: notesRoot },
    drawer: { hover: "island" },
    strip: { enabled: false, items: [] },
    hotkeys: { enabled: hotkeys, toggle: "", approve: "", deny: "", note: "", gameMode: "", notePassthrough: PASSTHROUGH_KEY, notePin: "" },
  };
}

const pad = (n) => String(n).padStart(2, "0");
const stamp = (d) => {
  const m = -d.getTimezoneOffset();
  const off = `${m < 0 ? "-" : "+"}${pad(Math.floor(Math.abs(m) / 60))}:${pad(Math.abs(m) % 60)}`;
  return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}T${pad(d.getHours())}:${pad(d.getMinutes())}:${pad(d.getSeconds())}${off}`;
};

/** Cesta lístku, který scénář připíná (relativně ke kořeni poznámek). */
export const PIN_NOTE = "_obecne/2026-01-01-100000-qa-listek.md";

/** Fixture poznámky (soubor → obsah); čas ISO s offsetem, jak chce formát. */
export function fixtureNotes(now = new Date()) {
  const remind = new Date(now.getTime() + 36 * 3600 * 1000);
  return {
    [PIN_NOTE]: `---\ncreated: ${stamp(now)}\nkind: info\ntitle: QA lístek\ndone: false\n---\n\nPoznámka pro QA lístek na ploše.\n`,
    "_obecne/2026-01-01-100100-qa-ukol.md": `---\ncreated: ${stamp(now)}\nkind: ukol\nremind: ${stamp(remind)}\ndone: false\n---\n\nQA úkol s termínem\n\n- [ ] první bod\n- [ ] druhý bod\n`,
  };
}

/** desk.json s jedním widgetem `limits` (vrstva jen na ploše, pod okny). */
export function fixtureDesk() {
  return { widgets: [{ id: "qa1", kind: "limits", x: 60, y: 200, w: 320, h: 160, layer: "desktop", locked: false, through: false, opacity: 100, hideInGame: true, compact: false }], profiles: [], active: "" };
}

/** ratelimits.json (vzorek stavového řádku): čerstvý, 5h 37 %, týden 12 %. */
export function fixtureLimits(nowMs) {
  return { at: nowMs, fiveHour: { pct: 37, resetsAt: nowMs + 2 * 3600 * 1000 }, sevenDay: { pct: 12, resetsAt: nowMs + 3 * 24 * 3600 * 1000 } };
}

/** Skupiny instancí: jedna spuštěná instance obslouží víc scénářů. */
export const GROUPS = [
  { id: "zaklad", opts: { hotkeys: false, limits: false, desk: true, oldProfile: true }, scenarios: ["supik", "listky-l260", "kit-bez-dat", "hudba", "nastaveni", "migrace-id"] },
  { id: "zkratky", opts: { hotkeys: true, limits: false, desk: false }, scenarios: ["listky-l260-zkratky"] },
  { id: "kitdata", opts: { hotkeys: false, limits: true, desk: true }, scenarios: ["kit-data"] },
  { id: "propojeni", opts: { hotkeys: false, limits: false, desk: false }, scenarios: ["propojeni"] },
];
export const GROUP_SCENARIOS = GROUPS.flatMap((g) => g.scenarios);
/** Scénáře mimo skupiny instancí: `upgrade` řídí dvě binárky po sobě nad jedním sandboxem (`qa/upgrade.mjs`, vyžaduje `--old-exe`). */
export const SPECIAL_SCENARIOS = ["upgrade"];
export const ALL_SCENARIOS = [...GROUP_SCENARIOS, ...SPECIAL_SCENARIOS];
/** Pustit `upgrade`? Výchozí (bez `--only`) ano; jinak jen když je v seznamu. */
export const wantsUpgrade = (only) => !only?.length || only.includes("upgrade");

/** Které skupiny a scénáře pustit podle `--only` (alias `kit` = oba kit-*, `listky-l260` bere i variantu se zkratkami). */
export function selectGroups(only) {
  const expand = (o) => (o === "kit" ? ["kit-bez-dat", "kit-data"] : o === "listky-l260" ? ["listky-l260", "listky-l260-zkratky"] : [o]);
  const want = only?.length ? new Set(only.flatMap(expand)) : null;
  const unknown = want ? [...want].filter((w) => !ALL_SCENARIOS.includes(w)) : [];
  if (unknown.length) throw new Error(`neznámý scénář: ${unknown.join(", ")} (--list)`);
  return GROUPS.map((g) => ({ ...g, scenarios: want ? g.scenarios.filter((s) => want.has(s)) : g.scenarios })).filter((g) => g.scenarios.length);
}

// ---------- animace před snímkem (L296) ----------

/** Krok „animace doběhly“ z výsledku `settle` (`{total, finite, running, forced, visibility, ms}`); `undefined` = snímek nebyl. */
export function settleStep(scenario, r) {
  if (!r) return step(scenario, "animace doběhly", SKIP, "snímek nebyl pořízen");
  const info = `animací ${r.total} (konečných ${r.finite}), viditelnost okna ${r.visibility}, čekáno ${r.ms} ms`;
  if (r.running === 0) return step(scenario, "animace doběhly", PASS, info);
  return step(scenario, "animace doběhly", FAIL, `${r.running} konečných animací nedoběhlo do limitu a byly dokončeny násilně (${info}) — zaparkované okno animace nepustí, snímek je bez nich`);
}

// ---------- pojistka dat WebView2 (L299) ----------

const normPath = (p) => String(p ?? "").replace(/^["']|["']$/g, "").replace(/\//g, "\\").replace(/[\\]+$/, "").toLowerCase();
export const insideDir = (path, dir) => { const p = normPath(path), d = normPath(dir); return !!p && !!d && (p === d || p.startsWith(d + "\\")); };

/**
 * Rozpozná složku dat WebView2 NAINSTALOVANÉ kopie podle tvaru cesty: `new` = `...\app.ledge.desktop\EBWebView`,
 * `old` = starý identifikátor, nalezený maskou `cz.*.ledge\EBWebView` (jméno se nikde nepíše); jinak `null`.
 */
export function installedProfileKind(path) {
  const p = normPath(path).replace(/\\/g, "/");
  if (/(^|\/)app\.ledge\.desktop\/ebwebview(\/|$)/.test(p)) return "new";
  if (/(^|\/)cz\.[^/]+\.ledge\/ebwebview(\/|$)/.test(p)) return "old";
  return null;
}

/** Potomci `rootPid` (vč. kořene) z plochého seznamu `{pid, ppid, name, cmd}`. */
export function descendants(procs, rootPid) {
  const out = new Map();
  const walk = (pid) => { for (const p of procs) if (p.ppid === pid && !out.has(p.pid)) { out.set(p.pid, p); walk(p.pid); } };
  walk(rootPid);
  return [...out.values()];
}

/** `--user-data-dir=…` z příkazové řádky (s uvozovkami i bez); `null`, když chybí. */
export function userDataDir(cmd) {
  const m = /--user-data-dir=(?:"([^"]*)"|(\S+))/i.exec(String(cmd ?? ""));
  return m ? (m[1] ?? m[2]) : null;
}

/**
 * Rozhodnutí pojistky: každý potomek msedgewebview2.exe s `--user-data-dir` ho musí mít UVNITŘ sandboxu; hlavní (browser) proces bez
 * `--type=` musí příznak mít vždy (jinak by běžel nad výchozí složkou = skutečná data); bez jediného procesu nelze ověřit.
 * Vrací `{ok, found, bad:[řádek…]}`.
 */
export function dataDirVerdict(procs, rootPid, sandboxDir) {
  const wv = descendants(procs, rootPid).filter((p) => /^msedgewebview2(\.exe)?$/i.test(p.name ?? ""));
  const bad = [];
  let browsers = 0;
  for (const p of wv) {
    const dir = userDataDir(p.cmd);
    const isBrowser = !/--type=/.test(p.cmd ?? "");
    if (isBrowser) browsers++;
    if (dir === null ? isBrowser : !insideDir(dir, sandboxDir)) bad.push(`${p.pid}: ${dir === null ? "bez --user-data-dir (výchozí složka)" : dir}${installedProfileKind(dir) ? ` — SKUTEČNÝ PROFIL nainstalované kopie (${installedProfileKind(dir) === "new" ? "nový" : "starý"} identifikátor)` : ""}`);
  }
  if (!wv.length || !browsers) return { ok: false, found: wv.length, bad: [...bad, "žádný hlavní proces msedgewebview2.exe potomkem instance — nelze ověřit"] };
  return { ok: bad.length === 0, found: wv.length, bad };
}
