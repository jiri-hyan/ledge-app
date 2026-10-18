// Krok „Nastavení s Claudem“ — rovnocenná volba vedle ruční cesty. Jedním klikem zapne MCP, ukáže dvě
// možnosti připojení (Claude Code / Claude Desktop) a zadání pro Clauda ke zkopírování. Když jádro MCP
// nezná, ukáže jen ruční cestu. Logika výběru zobrazení: `../claudeSetup.ts`.
import { onMount, Show } from "solid-js";
import { t } from "../../i18n";
import { Skeleton } from "./parts";
import { assist, chooseClaude, chooseManual, copy, disableMcp, loadMcp, promptText } from "../assist";
import { setupView } from "../claudeSetup";

export function AssistStep() {
  onMount(() => void loadMcp());
  const view = () => setupView(assist.load, assist.mode, assist.mcp);
  return (
    <div class="ob-center">
      <div class="ob-lead">
        <h3>{t("onboarding.assist.title", "Jak chceš Ledge nastavit?")}</h3>
        <p>{t("onboarding.assist.lead", "Projdi si to sám krok po kroku, nebo to nech na Claudovi: zeptá se tě, co používáš, navrhne rozložení a po tvém schválení ho nastaví.")}</p>
      </div>
      <Show when={view() !== "loading"} fallback={<Skeleton rows={2} label={t("onboarding.assist.loading", "Zjišťuji, jestli to tahle verze umí…")} />}>
        <Show
          when={view() !== "manualOnly"}
          fallback={<div class="ob-card ob-quiet">{t("onboarding.assist.manualOnly", "Nastavení s Claudem tahle verze Ledge ještě neumí. Pokračuj ručně — stačí kliknout na Další.")}</div>}
        >
          <div class="ob-ways" role="radiogroup" aria-label={t("onboarding.assist.title", "Jak chceš Ledge nastavit?")}>
            <button type="button" role="radio" class="ob-lang" classList={{ on: assist.mode === "manual" }} aria-checked={assist.mode === "manual"} onClick={chooseManual}>
              <b>{t("onboarding.assist.manual", "Projdu to sám")}</b>
              <small>{t("onboarding.assist.manualHint", "Pár kroků, asi 2 minuty: projekty, aplikace, odkazy a vzhled.")}</small>
            </button>
            <button type="button" role="radio" class="ob-lang" classList={{ on: assist.mode === "claude" }} aria-checked={assist.mode === "claude"} disabled={assist.busy} onClick={() => void chooseClaude()}>
              <b>{t("onboarding.assist.withClaude", "Nastavit s Claudem")}</b>
              <small>{assist.busy ? t("onboarding.assist.enabling", "Zapínám připojení…") : t("onboarding.assist.withClaudeHint", "Claude se tě zeptá a Ledge nastaví za tebe.")}</small>
            </button>
          </div>
          <Show when={view() === "connect"}>
            <Connect />
          </Show>
          <Show when={view() === "choose" && assist.mode === "manual" && assist.mcp?.enabled}>
            <div class="ob-card ob-quiet">
              <span>{t("onboarding.assist.stillOn", "Připojení Clauda je zapnuté. Vypnout ho můžeš tady, nebo kdykoli v Nastavení → Claude (MCP).")}</span>
              <div class="ob-actions">
                <button class="btn" disabled={assist.busy} onClick={() => void disableMcp()}>
                  {t("onboarding.assist.turnOff", "Vypnout připojení")}
                </button>
              </div>
            </div>
          </Show>
        </Show>
      </Show>
    </div>
  );
}

function Connect() {
  const m = () => assist.mcp!;
  return (
    <div class="ob-card">
      <p class="ob-quiet">
        {t("onboarding.assist.allowed", "Claude smí číst a měnit nastavení Ledge (zástupce, akce, rozložení, vzhled). Nic nespouští a nic neposílá ven. Připojení jde kdykoli vypnout v Nastavení → Claude (MCP).")}
      </p>
      <div class="ob-conn">
        <h5>{t("onboarding.assist.cliTitle", "Claude Code")}</h5>
        <p class="ob-quiet">{t("onboarding.assist.cliLead", "Vlož tenhle příkaz do terminálu:")}</p>
        <pre class="ob-code">{m().cliCommand}</pre>
        <div class="ob-actions">
          <button class="btn" onClick={() => void copy(m().cliCommand, t("onboarding.assist.cliTitle", "Claude Code"))}>
            {t("onboarding.assist.copy", "Kopírovat")}
          </button>
        </div>
      </div>
      <div class="ob-conn">
        <h5>{t("onboarding.assist.deskTitle", "Claude Desktop")}</h5>
        <p class="ob-quiet">{t("onboarding.assist.deskLead", "Přidej do souboru claude_desktop_config.json (Claude Desktop → Nastavení → Vývojář → Upravit konfiguraci) a Claude Desktop restartuj:")}</p>
        <pre class="ob-code">{m().desktopConfig}</pre>
        <div class="ob-actions">
          <button class="btn" onClick={() => void copy(m().desktopConfig, t("onboarding.assist.deskTitle", "Claude Desktop"))}>
            {t("onboarding.assist.copy", "Kopírovat")}
          </button>
        </div>
      </div>
      <div class="ob-conn">
        <h5>{t("onboarding.assist.promptTitle", "Zadání pro Clauda")}</h5>
        <p class="ob-quiet">{t("onboarding.assist.promptLead", "Až se Claude připojí, vlož mu tohle zadání. Zeptá se tě, co používáš, a po tvém schválení vše nastaví.")}</p>
        <div class="ob-actions">
          <button class="btn pri" onClick={() => void copy(promptText(), t("onboarding.assist.promptTitle", "Zadání pro Clauda"))}>
            {t("onboarding.assist.copyPrompt", "Zkopírovat zadání pro Clauda")}
          </button>
          <button class="mini" disabled={assist.busy} onClick={() => void disableMcp()}>
            {t("onboarding.assist.turnOff", "Vypnout připojení")}
          </button>
        </div>
      </div>
    </div>
  );
}
