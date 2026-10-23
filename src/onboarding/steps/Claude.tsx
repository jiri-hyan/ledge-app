// Krok „Claude“ (jen s nainstalovaným Claude Code, poslední před Hotovo): lidská věta, dvě volby jedním
// klikem — ukázat, když Claude čeká (hooky), a nechat Clauda Ledge nastavit (MCP). Příkazy, JSON, cesty
// a porty jsou pod „Podrobnosti“. Stav: `store.ts` (detekce, hooky), `assist.ts` (MCP); logika zobrazení
// `../claudeSetup.ts`. Nic nespouští — zapnutí je výslovný klik a jde vypnout i v Nastavení → Claude (MCP).
import { onMount, Show } from "solid-js";
import { t } from "../../i18n";
import { assist, chooseClaude, copy, disableMcp, loadMcp, promptText } from "../assist";
import { setupView } from "../claudeSetup";
import { installHooks, wiz } from "../store";

export function ClaudeStep() {
  onMount(() => void loadMcp());
  const hooked = () => wiz.hooks?.state === "full";
  const view = () => setupView(assist.load, assist.mode, assist.mcp);
  const mcpOn = () => assist.mcp?.enabled === true;
  return (
    <div class="ob-center">
      <div class="ob-lead">
        <h3>{t("onboarding.claude.title", "Claude")}</h3>
        <p>{t("onboarding.claude.lead", "Ledge ti ukáže, když Claude čeká na schválení, a umí ho nastavit za tebe.")}</p>
      </div>
      <div class="ob-card">
        <div class="ob-status">
          <span class="ob-dot on" />
          <div>
            <b>{wiz.claude?.version ? t("onboarding.claude.found", "Claude Code {v}", { v: wiz.claude.version }) : t("onboarding.claude.foundNoVer", "Claude Code je nainstalovaný")}</b>
          </div>
        </div>
        <div class="ob-opt">
          <span class="ob-opt-txt">
            <b>{t("onboarding.claude.waitTitle", "Ukázat, když Claude čeká")}</b>
            <small>{t("onboarding.claude.waitHint", "Žádost o schválení uvidíš v proužku a můžeš ji vyřídit v šuplíku.")}</small>
          </span>
          <Show when={!hooked()} fallback={<span class="pill g">{t("onboarding.claude.ok", "Zapojeno")}</span>}>
            <button class="btn pri" disabled={wiz.hooks?.state === "error"} onClick={() => void installHooks()}>
              {t("onboarding.claude.install", "Zapojit")}
            </button>
          </Show>
        </div>
        <Show when={wiz.hooks?.error}>
          <span class="sub err">{wiz.hooks?.error}</span>
        </Show>
        <Show when={view() !== "manualOnly"}>
          <div class="ob-opt">
            <span class="ob-opt-txt">
              <b>{t("onboarding.claude.setupTitle", "Nechat Clauda nastavit Ledge")}</b>
              <small>{mcpOn() ? t("onboarding.claude.setupOn", "Připojeno. Zkopíruj zadání a vlož ho Claudovi — zeptá se tě a po tvém schválení vše nastaví.") : t("onboarding.claude.setupHint", "Claude se tě zeptá, co používáš, a po tvém schválení nastaví rozložení.")}</small>
            </span>
            <Show
              when={mcpOn()}
              fallback={
                <button class="btn" disabled={assist.busy || view() === "loading"} onClick={() => void chooseClaude()}>
                  {assist.busy ? t("onboarding.assist.enabling", "Zapínám připojení…") : t("onboarding.claude.setupBtn", "Připojit")}
                </button>
              }
            >
              <button class="btn pri" onClick={() => void copy(promptText(), t("onboarding.assist.promptTitle", "Zadání pro Clauda"))}>
                {t("onboarding.assist.copyPrompt", "Zkopírovat zadání pro Clauda")}
              </button>
            </Show>
          </div>
        </Show>
        <details class="ob-details">
          <summary>{t("onboarding.claude.details", "Podrobnosti")}</summary>
          <ul class="ob-bullets">
            <li>{t("onboarding.claude.b1", "Hooky zapíšou do ~/.claude/settings.json pár řádků, které posílají Ledge události session — jen na 127.0.0.1.")}</li>
            <li>{t("onboarding.claude.b2", "Ostatní nastavení i cizí hooky zůstanou; poprvé se uloží záloha settings.json.bak-pred-ledge.")}</li>
            <li>{t("onboarding.claude.b3", "Bez běžícího Ledge Claude funguje dál normálně. Odpojit jde jedním klikem v Nastavení.")}</li>
            <Show when={wiz.claude?.exe}>
              <li>{wiz.claude?.exe}</li>
            </Show>
            <li>{t("onboarding.assist.allowed", "Claude smí číst a měnit nastavení Ledge (zástupce, akce, rozložení, vzhled). Nic nespouští a nic neposílá ven. Připojení jde kdykoli vypnout v Nastavení → Claude (MCP).")}</li>
          </ul>
          <Show when={view() === "connect"}>
            <Connect />
          </Show>
        </details>
      </div>
    </div>
  );
}

/** Ruční připojení (příkaz do terminálu, JSON pro Claude Desktop) + vypnutí — jen pod „Podrobnosti“. */
function Connect() {
  const m = () => assist.mcp!;
  return (
    <>
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
          <button class="mini" disabled={assist.busy} onClick={() => void disableMcp()}>
            {t("onboarding.assist.turnOff", "Vypnout připojení")}
          </button>
        </div>
      </div>
    </>
  );
}
