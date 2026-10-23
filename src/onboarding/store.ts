// Stav průvodce (přežije zavření šuplíku — modul žije s WebView): krok, nalezená repa/aplikace, výběr,
// Claude, odkazy, motivy, ikony a souhrn toho, co se uložilo. Ukládá se po každém kroku (`applyStep`),
// takže zavřený šuplík nic neztratí. IPC v `api.ts`, čisté pomůcky v `logic.ts`.
import { createStore } from "solid-js/store";
import { api, type HooksStatus } from "../api";
import { setLang, t, type Lang } from "../i18n";
import { say, sayError } from "../ui/toast";
import type { ClaudeInfo, FoundApp, FoundRepo, OnboardingPatch, ThemeOption } from "../types";
import { ob } from "./api";
import { linkSuggestions, appProject, defaultRepoSelection, type StepId } from "./logic";
import { visibleSteps } from "./plan";

type Load = "idle" | "loading" | "ready" | "error";

export interface Wiz {
  step: number;
  /** směr posledního přechodu (animace) */
  dir: 1 | -1;
  busy: boolean;
  lang: Lang;
  trigger: string;
  repos: FoundRepo[];
  reposState: Load;
  /** výchozí výběr rep už proběhl (další skeny výběr uživatele nepřepíšou) */
  repoSelInit: boolean;
  extra: string[];
  repoSel: string[];
  apps: FoundApp[];
  appsState: Load;
  appSel: string[];
  query: string;
  claude: ClaudeInfo | null;
  hooks: HooksStatus | null;
  claudeState: Load;
  linkSel: string[];
  custom: { label: string; url: string }[];
  themes: ThemeOption[];
  themesState: Load;
  theme: string | null;
  icons: Record<string, string | null>;
  sum: { projects: string[]; skipped: string[]; links: number; linksTo: string | null; theme: string | null; themeSaved: boolean | null; hooks: boolean };
}

const fresh = (): Wiz => ({
  step: 0,
  dir: 1,
  busy: false,
  lang: "cs",
  trigger: "hover",
  repos: [],
  reposState: "idle",
  repoSelInit: false,
  extra: [],
  repoSel: [],
  apps: [],
  appsState: "idle",
  appSel: [],
  query: "",
  claude: null,
  hooks: null,
  claudeState: "idle",
  linkSel: [],
  custom: [],
  themes: [],
  themesState: "idle",
  theme: null,
  icons: {},
  sum: { projects: [], skipped: [], links: 0, linksTo: null, theme: null, themeSaved: null, hooks: false },
});

export const [wiz, setWiz] = createStore<Wiz>(fresh());

export function resetWizard() {
  setWiz(fresh());
}

/** Claude Code je na tomhle počítači (detekce doběhla a našla ho) — jen tak se ukáže krok Claude. */
export const claudeInstalled = () => wiz.claudeState === "ready" && wiz.claude?.installed === true;

/** Kroky, které se tomuhle uživateli ukážou (`plan.ts`) — jediný zdroj pro hlavičku, Vítej i Wizard. */
export const steps = () => visibleSteps({ themesReady: wiz.themesState === "ready", themeCount: wiz.themes.length, claudeInstalled: claudeInstalled() });

// ---------- načítání (běží na pozadí od otevření průvodce, ať kroky nečekají) ----------

export async function prefetch() {
  const cfg = await api.getConfig().catch(() => null);
  if (cfg) {
    setWiz({ trigger: cfg.drawer?.trigger ?? "hover", lang: cfg.ui?.lang === "en" ? "en" : "cs", theme: cfg.appearance?.theme ?? null });
  }
  void loadThemes();
  void loadClaude(); // hned: podle něj se ukáže krok Claude a karta na Práci
  await loadRepos();
  await loadApps();
}

export async function loadRepos() {
  setWiz("reposState", "loading");
  try {
    const repos = (await ob.repos([...wiz.extra])) ?? [];
    setWiz({ repos, reposState: "ready", ...(wiz.repoSelInit ? {} : { repoSel: defaultRepoSelection(repos), repoSelInit: true }) });
    void loadIcons(repos.slice(0, 48).map((r) => `repo:${r.path}`));
  } catch (e) {
    setWiz("reposState", "error");
    sayError(e);
  }
}

/** „Přidat složku…“: dialog Windows (vlastněný šuplíkem), pak nový sken včetně ní. */
export async function addFolder() {
  try {
    await api.focusDrawer(); // dialog patří šuplíku: odjetí myší ani blur ho pak nezavřou
    const dir = await ob.pickFolder();
    if (!dir) return;
    if (!wiz.extra.includes(dir)) setWiz("extra", (x) => [...x, dir]);
    const before = new Set(wiz.repos.map((r) => r.path));
    await loadRepos();
    const added = wiz.repos.filter((r) => !before.has(r.path) && r.project && !r.already).map((r) => r.path);
    setWiz("repoSel", (s) => [...new Set([...s, ...added])]);
    say(added.length ? t("onboarding.folder.found", "Ve složce: {n} nových rep", { n: added.length }) : t("onboarding.folder.none", "Ve složce jsem žádné nové repo nenašel"));
  } catch (e) {
    sayError(e);
  }
}

export async function loadApps() {
  if (wiz.appsState === "loading" || wiz.appsState === "ready") return;
  setWiz("appsState", "loading");
  try {
    setWiz({ apps: (await ob.apps()) ?? [], appsState: "ready" });
  } catch (e) {
    setWiz("appsState", "error");
    sayError(e);
  }
}

export async function loadClaude() {
  if (wiz.claudeState === "loading" || wiz.claudeState === "ready") return;
  setWiz("claudeState", "loading");
  try {
    const [claude, hooks] = await Promise.all([ob.claude(), api.hooksStatus()]);
    setWiz({ claude: claude ?? null, hooks: hooks ?? null, claudeState: "ready" });
  } catch {
    setWiz("claudeState", "error");
  }
}

async function loadThemes() {
  setWiz("themesState", "loading");
  const themes = (await ob.themes().catch(() => [])) ?? [];
  setWiz({ themes, themesState: "ready" });
}

/** Ikony pro klíče, které ještě nemáme (dávka ≤ 48). */
export async function loadIcons(keys: string[]) {
  const want = keys.filter((k) => !(k in wiz.icons)).slice(0, 48);
  if (!want.length) return;
  setWiz("icons", Object.fromEntries(want.map((k) => [k, null])));
  const urls = (await ob.icons(want).catch(() => null)) ?? [];
  setWiz("icons", Object.fromEntries(want.map((k, i) => [k, urls[i] ?? null])));
}

export async function installHooks() {
  try {
    const h = await api.hooksInstall();
    setWiz({ hooks: h ?? null, sum: { ...wiz.sum, hooks: true } });
    say(t("onboarding.claude.hooked", "Hooky zapojené — platí pro nově spuštěné session"));
  } catch (e) {
    sayError(e);
  }
}

export function chooseLang(l: Lang) {
  setLang(l);
  setWiz("lang", l);
}

// ---------- ukládání po krocích ----------

/** Krok má co uložit (tlačítko „Uložit a pokračovat“ místo „Další“). */
export const pending = (id: StepId) => patchFor(id) !== null && id !== "welcome";

function patchFor(id: StepId): OnboardingPatch | null {
  switch (id) {
    case "welcome":
      return { lang: wiz.lang };
    case "projects": {
      const projects = wiz.repos.filter((r) => r.project && !r.already && wiz.repoSel.includes(r.path)).map((r) => r.project!);
      return projects.length ? { projects } : null;
    }
    case "apps": {
      const projects = wiz.apps.filter((a) => !a.already && wiz.appSel.includes(a.exe)).map(appProject);
      return projects.length ? { projects } : null;
    }
    case "links": {
      const links = [...linkSuggestions().filter((l) => wiz.linkSel.includes(l.id)).map(({ label, url }) => ({ label, url })), ...wiz.custom];
      return links.length ? { links } : null;
    }
    case "look":
      return wiz.theme ? { theme: wiz.theme } : null;
    default:
      return null;
  }
}

/** Uloží výběr kroku. `false` = chyba (zůstat na kroku, hláška už je venku). */
export async function applyStep(id: StepId): Promise<boolean> {
  const patch = patchFor(id);
  if (!patch) return true;
  setWiz("busy", true);
  try {
    const r = await ob.apply(patch);
    if (r) {
      const s = { ...wiz.sum };
      s.projects = [...s.projects, ...r.added];
      s.skipped = [...s.skipped, ...r.skipped];
      if (patch.links) {
        s.links += r.linksAdded;
        s.linksTo = r.linksTo;
      }
      if (patch.theme) {
        s.theme = patch.theme;
        s.themeSaved = r.themeSaved;
      }
      setWiz("sum", s);
      if (r.added.length) say(t("onboarding.saved.projects", "Přidáno do Ledge: {n}", { n: r.added.length }));
      else if (patch.links && r.linksAdded) say(t("onboarding.saved.links", "Uloženo odkazů: {n}", { n: r.linksAdded }));
    }
    return true;
  } catch (e) {
    sayError(e);
    return false;
  } finally {
    setWiz("busy", false);
  }
}

/** Konec (Hotovo i Přeskočit průvodce): `onboarding.done = true`, ať se průvodce sám znovu neukáže. Znovu ho
 *  spustí `startWizard()` (`state.ts`, Nastavení → Začínáme). */
export async function finish(): Promise<boolean> {
  try {
    // `claude`: průvodce našel Claude Code → Rust přidá na Práci kartu Claude (jen při prvním dokončení)
    await ob.apply({ done: true, claude: claudeInstalled() });
    return true;
  } catch (e) {
    sayError(e);
    return false;
  }
}
