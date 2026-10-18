// Stav kroku „Nastavení s Claudem“: stav MCP z jádra, zvolená cesta a kopírování do schránky.
// Žije zvlášť od `store.ts` (průvodce) — když `mcp_state` selže (starší jádro), krok ukáže jen ruční cestu.
import { createStore } from "solid-js/store";
import { api } from "../api";
import { mcpSetEnabled, mcpState, type McpState } from "../mcp/api";
import { t } from "../i18n";
import { say, sayError } from "../ui/toast";
import { buildSetupPrompt, type AssistMode, type McpLoad } from "./claudeSetup";

export const [assist, setAssist] = createStore<{ load: McpLoad; mode: AssistMode; mcp: McpState | null; busy: boolean }>({ load: "idle", mode: "manual", mcp: null, busy: false });

/** Načte stav MCP (1× za krok). Jakákoli chyba = jádro to neumí → ruční cesta, žádný pád. */
export async function loadMcp(): Promise<void> {
  if (assist.load === "loading" || assist.load === "ready") return;
  setAssist("load", "loading");
  try {
    setAssist({ mcp: await mcpState(), load: "ready" });
  } catch {
    setAssist({ mcp: null, load: "error" });
  }
}

/** „Nastavit s Claudem“: jedním klikem zapne MCP (první zapnutí vytvoří klíč ve Správci pověření). */
export async function chooseClaude(): Promise<void> {
  if (assist.busy) return;
  setAssist("mode", "claude");
  if (assist.mcp?.enabled) return;
  setAssist("busy", true);
  try {
    setAssist("mcp", await mcpSetEnabled(true));
  } catch (e) {
    setAssist({ mode: "manual", load: "error" });
    sayError(e);
  } finally {
    setAssist("busy", false);
  }
}

export const chooseManual = () => setAssist("mode", "manual");

/** Vypnout připojení (kdykoli jde i v Nastavení → Claude (MCP)). */
export async function disableMcp(): Promise<void> {
  if (assist.busy) return;
  setAssist("busy", true);
  try {
    setAssist({ mcp: await mcpSetEnabled(false), mode: "manual" });
    say(t("onboarding.assist.off", "Připojení Clauda je vypnuté"));
  } catch (e) {
    sayError(e);
  } finally {
    setAssist("busy", false);
  }
}

/** Text do schránky. Šuplík otevřený najetím nemá klávesnici → nejdřív výslovný fokus (klik už proběhl). */
export async function copy(text: string, what: string): Promise<void> {
  try {
    await api.focusDrawer();
  } catch {
    /* fokus je jen pomoc */
  }
  let ok = false;
  try {
    await navigator.clipboard.writeText(text);
    ok = true;
  } catch {
    const ta = document.createElement("textarea");
    ta.value = text;
    ta.setAttribute("readonly", "");
    ta.style.cssText = "position:fixed;left:-9999px;opacity:0";
    document.body.appendChild(ta);
    ta.select();
    try {
      ok = document.execCommand("copy");
    } catch {
      ok = false;
    }
    ta.remove();
  }
  if (ok) say(t("onboarding.assist.copied", "Zkopírováno: {what}", { what }));
  else say(t("onboarding.assist.copyFail", "Kopírování se nepovedlo — označ text a zkopíruj ho ručně"), true);
}

/** Zadání pro Clauda v jazyce UI. */
export const promptText = () => buildSetupPrompt(t);
