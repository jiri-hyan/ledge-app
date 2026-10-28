// Tři ukázkové sady dat pro `index.html?demo=1` (snímky a GIFy, kolo 24). Všechno smyšlené: projekty, session,
// poznámky i skladba; adresy jen `example.com`. Čas je relativní k `now`, aby „před 3 min“ vždy sedělo.
// Čistá funkce `buildSet(id, now)` → { layout, snapshot část } — žádné DOM, žádné IPC (test `sets.test.mjs`).
import type { Layout, LayoutCard } from "../layout/model";
import type { Snapshot } from "../types";

export const SET_IDS = ["studio", "creator", "minimal"] as const;
export type SetId = (typeof SET_IDS)[number];

const MIN = 60_000;
const HOUR = 3_600_000;
const card = (id: string, size: LayoutCard["size"], extra: Partial<LayoutCard> = {}): LayoutCard => ({ id: id as any, size, ...extra });

const proj = (id: string, name: string, accent: string, kind: "dev" | "app", git: any, extra: any = {}) => ({
  id, name, kind, accent, monogram: name.slice(0, 2), logoKey: null, path: `C:/Users/uzivatel/dev/${id}`, pathOk: true,
  clonable: false, cloning: false, github: `uzivatel/${id}`, links: [], git, run: kind === "dev" ? "npm run dev" : null,
  port: null, place: { kind: "ok" }, ...extra,
});
const clean = { branch: "main", dirty: 0, ahead: 0, behind: 0 };

const note = (i: number, title: string, now: number, extra: any = {}) => ({
  path: `C:/Users/uzivatel/Ledge/notes/${i}.md`, project: null, created: new Date(now - i * HOUR).toISOString(), createdMs: now - i * HOUR,
  remind: null, remindMs: null, done: false, title, images: 0, kind: "info", updated: null, tasksDone: 0, tasksTotal: 0, ...extra,
});

const session = (id: string, name: string, project: string, projectId: string, status: "working" | "waiting" | "idle", now: number, extra: any = {}) => ({
  id, name, title: null, cwd: `C:/Users/uzivatel/dev/${projectId}`, project, projectId, status, state: status,
  since: now - 4 * MIN, updatedAt: now - 20_000, entrypoint: "cli", pid: 4100, alive: true, hooked: true, uncommitted: [], unpushed: 0,
  ended: false, subagents: { running: [], done: [] }, canOpen: true, openHint: "", background: false, job: null, waitingFor: null, ...extra,
});

/** Svg „obal“ skladby — vlastní abstraktní obrázek (žádné skutečné album). */
export const COVER =
  "data:image/svg+xml;utf8," +
  encodeURIComponent(
    "<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 120 120'><defs><linearGradient id='g' x1='0' y1='0' x2='1' y2='1'><stop offset='0' stop-color='#7c6cff'/><stop offset='1' stop-color='#ff7ab0'/></linearGradient></defs><rect width='120' height='120' fill='url(#g)'/><circle cx='84' cy='38' r='22' fill='#ffd36e' opacity='.85'/><path d='M0 92 Q30 62 60 86 T120 70 V120 H0Z' fill='#1b1740' opacity='.7'/></svg>",
  );

const weather = (label: string, temp: number, desc: string, icon: string, now: number) => ({
  status: "ok", label, fetchedMs: now, error: null, loading: false,
  data: { temp, code: 2, desc, icon, isDay: true, min: temp - 6, max: temp + 3, precipSum: 0.4, precipProb: 20, observed: new Date(now - 15 * MIN - new Date(now).getTimezoneOffset() * MIN).toISOString().slice(0, 16) },
});

const links = [
  { name: "Docs", target: "https://example.com/docs", kind: "web", monogram: "Do", problem: null },
  { name: "Board", target: "https://example.com/board", kind: "web", monogram: "Bo", problem: null },
  { name: "Mail", target: "https://example.com/mail", kind: "web", monogram: "Ma", problem: null },
  { name: "Návrhy", target: "C:/Users/uzivatel/Design", kind: "folder", monogram: "Ná", problem: null },
];

function calendar(now: number) {
  const d = new Date(now);
  const at = (h: number, m = 0) => new Date(d.getFullYear(), d.getMonth(), d.getDate(), h, m).getTime();
  const it = (h: number, m: number, mins: number, title: string, location = "", cal = 0, ongoing = false) => ({
    start: at(h, m), end: at(h, m) + mins * MIN, allDay: false, title, location, cal, ongoing, tzGuess: false,
  });
  const ymd = (t: number) => new Date(t).toISOString().slice(0, 10);
  return {
    status: "ok", more: 0, loading: false,
    calendars: [{ name: "Práce", color: "#6ea8ff", domain: "example.com", source: "ics", enabled: true, status: "ok", error: null, fetchedMs: now, events: 5 }],
    days: [
      { kind: "today", date: ymd(now), items: [it(9, 30, 30, "Daily sync"), it(13, 0, 60, "Review návrhu"), it(16, 30, 45, "Plán sprintu", "Zasedačka 2")] },
      { kind: "tomorrow", date: ymd(now + 24 * HOUR), items: [{ ...it(10, 0, 60, "Demo pro tým"), start: at(10, 0) + 24 * HOUR, end: at(11, 0) + 24 * HOUR }] },
    ],
  };
}

export type SetData = { layout: Layout; snap: Partial<Snapshot>; appearance: Record<string, unknown>; mediaVariant: string };

export function buildSet(id: SetId, now: number): SetData {
  const pending = (cwd: string, sessionId: string, tool: string, summary: string, file: string, lines: any[]) => ({
    id: 1, sessionId, cwd, tool, summary, createdAt: now - 12_000, expiresAt: now + 110_000, always: { rules: [{ toolName: tool }], label: `${tool} ve složce`, fromClaude: false },
    plan: null, via: "permission",
    preview: { file, path: `${cwd}/${file}`, shown: "diff", added: lines.filter((l) => l.kind === "add").length, removed: lines.filter((l) => l.kind === "del").length, lines, cut: 0, numbered: true, replaceAll: false, occurrences: 1, rough: false, total: 0 },
  });
  const diff = [
    { kind: "ctx", old: 41, new: 41, text: "export function retry(fn, tries = 3) {" },
    { kind: "del", old: 42, new: null, text: "  for (let i = 0; i < tries; i++) await fn();" },
    { kind: "add", old: null, new: 42, text: "  for (let i = 0; i < tries; i++) {" },
    { kind: "add", old: null, new: 43, text: "    try { return await fn(); } catch (e) { if (i === tries - 1) throw e; }" },
    { kind: "add", old: null, new: 44, text: "  }" },
    { kind: "ctx", old: 43, new: 45, text: "}" },
  ];
  const limits = (five: number, week: number) => ({
    at: now, source: "statusLine", staleMs: 600_000, statusLine: "full", notice: null, pace: true,
    fiveHour: { pct: five, resetsAt: now + 100 * MIN }, sevenDay: { pct: week, resetsAt: now + 3 * 24 * HOUR },
    history5h: Array.from({ length: 12 }, (_, i) => [now - (12 - i) * 20 * MIN, Math.round((five * (i + 1)) / 12)] as [number, number]),
  });
  const kinds = [
    { id: "napad", label: "Nápad", color: "#fbbf24", icon: "bulb" },
    { id: "ukol", label: "Úkol", color: "#34d399", icon: "check" },
    { id: "info", label: "Info", color: "#60a5fa", icon: "info" },
  ];
  const media = () => ({
    title: "Neon Harbor", artist: "Kestrel Lane", app: "Přehrávač", status: "playing", canToggle: true, canPrev: true, canNext: true,
    coverKey: "demo", timeline: { positionMs: 84_000, durationMs: 212_000, updatedMs: now, rate: 0, canSeek: true }, shuffle: false, repeat: "none",
  });
  const noPersonal = { weather: weather("", 0, "", "", now), links: [], notes: [], notesMore: 0, media: null, volume: null, battery: null };

  if (id === "creator") {
    const projects = [
      proj("atelier-web", "Atelier Web", "#f472b6", "dev", { branch: "feat/hero", dirty: 3, ahead: 1, behind: 0 }, { port: 4321 }),
      proj("pocket-notes", "Pocket Notes", "#34d399", "app", clean, { run: null, app: "pocket-notes.exe" }),
      proj("blog-lantern", "Blog Lantern", "#fbbf24", "dev", { branch: "main", dirty: 0, ahead: 0, behind: 2 }, { port: 4000 }),
    ];
    return {
      mediaVariant: "cover", appearance: { style: "glass", theme: "aurora" },
      layout: {
        hiddenParts: [],
        pages: [
          { id: "prace", name: "Práce", bento: true, cards: [card("projects", "l", { h: 1 }), card("calendar", "s", { h: 2 }), card("notes", "l", { h: 2 }), card("weather", "s", { h: 1 }), card("clock", "s", { h: 1 })] },
          { id: "hudba", name: "Hudba", icon: "🎵", bento: true, cards: [card("media", "m", { h: 2 }), card("clock", "s", { h: 1 }), card("weather", "s", { h: 1 }), card("links", "m", { h: 1 }), card("calendar", "m", { h: 1 })] },
          { id: "plan", name: "Plán", icon: "📚", cards: [card("calendar", "xl"), card("notes", "l")] },
        ],
      },
      snap: {
        projects: projects as any,
        procs: [{ id: "atelier-web", phase: "ready", pid: 5120, port: 4321, url: "http://localhost:4321", startedAt: now - 25 * MIN, readyAt: now - 25 * MIN, exitCode: null, message: null, lastLine: null, errors: 0 }] as any,
        claudeEnabled: false, sessions: [], pending: [], questions: [],
        noteKinds: kinds, notesKindCounts: { ukol: 3, napad: 2 },
        notes: [
          note(1, "Hero sekce: nový claim + tři varianty tlačítka", now, { kind: "napad", tags: ["web"] }),
          note(2, "Podcast #14 – osnova a hosté", now, { kind: "ukol", tasksDone: 2, tasksTotal: 5, tags: ["blog"] }),
          note(3, "Fotky pro článek o Lisabonu", now, { kind: "ukol", tasksDone: 1, tasksTotal: 4, remind: new Date(now + 3 * HOUR).toISOString(), remindMs: now + 3 * HOUR }),
          note(4, "Barvy pro jarní kolekci", now, { kind: "napad" }),
        ] as any,
        personal: { ...noPersonal, weather: weather("Praha", 17, "Polojasno", "⛅", now), links, media: media(), volume: { level: 46, muted: false, device: "Sluchátka" }, battery: { percent: 82, charging: false, plugged: false, remainingS: 14_400 }, calendar: calendar(now) } as any,
      },
    };
  }

  if (id === "minimal") {
    const projects = [
      proj("app", "app", "#6ea8ff", "dev", { branch: "main", dirty: 2, ahead: 0, behind: 0 }, { port: 5173 }),
      proj("app-v2", "app-v2", "#a78bfa", "dev", clean, { port: 5174 }),
    ];
    return {
      mediaVariant: "compact", appearance: { style: "classic", theme: "dark", density: "compact", width: "narrow" },
      layout: { hiddenParts: [], pages: [{ id: "prace", name: "Práce", cards: [card("projects", "s"), card("claude", "s"), card("notes", "s")] }] },
      snap: {
        projects: projects as any, procs: [],
        sessions: [session("s-app", "Oprava přihlášení", "app", "app", "working", now, { activity: { phase: "editing", tool: "Edit", file: "src/auth.ts", message: null, at: now - 5000 } })] as any,
        pending: [], questions: [], noteKinds: kinds, notesKindCounts: { ukol: 1 },
        notes: [note(1, "Zkontrolovat verzi balíčků", now, { kind: "ukol" })] as any,
        limits: limits(31, 12) as any,
        personal: noPersonal as any,
      },
    };
  }

  const projects = [
    proj("atlas-web", "Atlas Web", "#6ea8ff", "dev", { branch: "feat/search", dirty: 4, ahead: 2, behind: 0 }, { port: 5173 }),
    proj("harbor-api", "Harbor API", "#34d399", "dev", { branch: "main", dirty: 0, ahead: 0, behind: 0 }, { port: 8080 }),
    proj("orbit", "Orbit", "#a78bfa", "dev", { branch: "fix/sync", dirty: 1, ahead: 0, behind: 3 }, { port: 3000 }),
    proj("pixel-docs", "Pixel Docs", "#f472b6", "dev", clean, { port: 4000 }),
    proj("lumen", "Lumen", "#fbbf24", "app", clean, { run: null, app: "lumen.exe" }),
    proj("tide-cli", "Tide CLI", "#22d3ee", "dev", { branch: "main", dirty: 0, ahead: 1, behind: 0 }, { run: "cargo run" }),
  ];
  return {
    mediaVariant: "card", appearance: { style: "classic", theme: "dark" },
    layout: {
      hiddenParts: [],
      pages: [
        { id: "prace", name: "Práce", cards: [card("claude", "l"), card("projects", "l"), card("repos", "m"), card("notes", "m")] },
        { id: "osobni", name: "Osobní", cards: [card("clock", "m"), card("weather", "m"), card("media", "m"), card("calendar", "l"), card("links", "m"), card("notes", "m")] },
      ],
    },
    snap: {
      projects: projects as any,
      procs: [
        { id: "atlas-web", phase: "ready", pid: 5120, port: 5173, url: "http://localhost:5173", startedAt: now - 42 * MIN, readyAt: now - 42 * MIN, exitCode: null, message: null, lastLine: null, errors: 0 },
        { id: "harbor-api", phase: "ready", pid: 5188, port: 8080, url: "http://localhost:8080", startedAt: now - 18 * MIN, readyAt: now - 18 * MIN, exitCode: null, message: null, lastLine: null, errors: 0 },
      ] as any,
      sessions: [
        session("s-atlas", "Hledání v Atlas Web", "Atlas Web", "atlas-web", "working", now, { activity: { phase: "editing", tool: "Edit", file: "src/search/index.ts", message: null, at: now - 4000 } }),
        session("s-harbor", "Migrace endpointů", "Harbor API", "harbor-api", "waiting", now, { waitingFor: "permission", since: now - 40_000 }),
      ] as any,
      pending: [pending("C:/Users/uzivatel/dev/harbor-api", "s-harbor", "Edit", "Edit src/retry.ts", "src/retry.ts", diff)] as any,
      questions: [],
      limits: limits(62, 38) as any,
      noteKinds: kinds, notesKindCounts: { ukol: 2, napad: 1 },
      notes: [
        note(1, "Zkontrolovat benchmark vyhledávání", now, { kind: "ukol", project: "atlas-web", tasksDone: 1, tasksTotal: 3, remind: new Date(now + 2 * HOUR).toISOString(), remindMs: now + 2 * HOUR }),
        note(2, "Nápad: sdílené filtry mezi zobrazeními", now, { kind: "napad", project: "atlas-web", tags: ["ux"] }),
        note(3, "Release notes pro Orbit 1.4", now, { kind: "ukol", project: "orbit" }),
        note(5, "Rezervovat vstupenky na koncert", now, { kind: "info" }),
      ] as any,
      personal: { ...noPersonal, weather: weather("Brno", 19, "Polojasno", "⛅", now), links, media: media(), volume: { level: 38, muted: false, device: "Reproduktory" }, battery: { percent: 74, charging: true, plugged: true, remainingS: null }, calendar: calendar(now) } as any,
    },
  };
}

/** Otázka Clauda do scény „otázka“ (ke sloučení do snapshotu přes `?ask=1`). */
export function askFor(now: number) {
  return {
    id: 7, sessionId: "s-harbor", cwd: "C:/Users/uzivatel/dev/harbor-api", createdAt: now - 8000, expiresAt: now + 290_000, answerable: true,
    questions: [{
      question: "Jak mám řešit zastaralé endpointy v Harbor API?", header: "Migrace", multiSelect: false,
      options: [
        { label: "Označit jako deprecated", description: "Ponechá je a přidá hlavičku Sunset.", preview: null },
        { label: "Přesměrovat na v2", description: "Vrací 308 na nové cesty.", preview: null },
        { label: "Smazat hned", description: "Odstraní handlery i testy.", preview: null },
      ],
    }],
  };
}

const WIDTHS: Record<string, number> = { narrow: 820, medium: 1080, wide: 1440, full: 1600 };
const NOTCH_PX = 720;

/** Šířka šuplíku jako ji spočítá Rust (`geom.rs` `layout_width` / `sized_width`) — pro scénu (iframe). */
export function drawerWidth(a: { layout?: string; width?: string; size?: { width?: number } }, custom = 1080): number {
  if (a.layout === "notch") return NOTCH_PX;
  if (a.size?.width) return a.size.width;
  return WIDTHS[a.width ?? ""] ?? custom;
}
