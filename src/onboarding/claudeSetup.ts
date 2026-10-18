// Krok průvodce „Nastavení s Claudem“ — čistá logika bez stavu, IPC a DOM: sestavení zadání pro Clauda
// a volba, co krok ukáže (ruční cesta / volba / připojení). Texty dostává přes `t` (volající předá
// překladač), takže modul jde testovat bez i18n: `claudeSetup.test.mjs`.
import type { McpState } from "../mcp/api";

/** Návod psaný pro Clauda — asset každého vydání (`scripts/publish.ps1`, zdroj `docs/pro-claude.md`). */
export const PRO_CLAUDE_URL = "https://github.com/jiri-hyan/ledge-app/releases/latest/download/pro-claude.md";

export type Tr = (key: string, cs: string, vars?: Record<string, string | number>) => string;

/** Zadání, které si člověk vloží do Clauda: připoj se k Ledge přes MCP, vyzpovídej mě, navrhni, po schválení nastav. */
export function buildSetupPrompt(t: Tr, guideUrl: string = PRO_CLAUDE_URL): string {
  return [
    t(
      "onboarding.assist.prompt.p1",
      "Mám nainstalovaný Ledge (šuplík u horního okraje obrazovky) a zapnuté připojení přes MCP. Připoj se k němu podle návodu {url} a nastav mi ho na míru.",
      { url: guideUrl },
    ),
    t(
      "onboarding.assist.prompt.p2",
      "Nejdřív se mě stručně zeptej, jaké aplikace, weby a práci používám. Podle odpovědí navrhni 2–3 rozložení a popiš je slovy. Změň něco, až když si jedno vyberu a schválím to.",
    ),
    t(
      "onboarding.assist.prompt.p3",
      "Nic nespouštěj, nic neposílej ven a nevkládej tajemství. Po nastavení mi otevři šuplík, ať výsledek vidím, a nabídni úpravy.",
    ),
  ].join("\n\n");
}

export type McpLoad = "idle" | "loading" | "ready" | "error";
export type AssistMode = "manual" | "claude";
/** `manualOnly` = jádro MCP neumí (starší verze) → jen ruční cesta, `loading` = čekám na stav,
 *  `choose` = dvě rovnocenné cesty, `connect` = vybráno „s Claudem“ a MCP zapnuté → návod k připojení. */
export type AssistView = "manualOnly" | "loading" | "choose" | "connect";

export function setupView(load: McpLoad, mode: AssistMode, state: McpState | null): AssistView {
  if (load === "error") return "manualOnly";
  if (load !== "ready") return "loading";
  return mode === "claude" && state?.enabled && hasConnectionKit(state) ? "connect" : "choose";
}

/** Stav MCP z jádra je použitelný k zobrazení připojení (oba úryvky neprázdné). */
export const hasConnectionKit = (s: Pick<McpState, "cliCommand" | "desktopConfig"> | null | undefined): boolean =>
  !!s && s.cliCommand.trim() !== "" && s.desktopConfig.trim() !== "";
