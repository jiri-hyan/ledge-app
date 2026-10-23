// Nastavení → Claude → „Claude (MCP)“: zapnutí jedním klikem (okamžitě, jako zapojení hooků v tomtéž
// panelu — `mcp_set_enabled` uloží config sám a Nastavení se přenačte událostí `config-changed`),
// příkazy pro Claude Code a Claude Desktop ke zkopírování, klíč pro http zálohu jen na výslovný klik,
// výměna klíče s potvrzením. Klíč se nikdy neukazuje sám od sebe.
import { createSignal, onMount, Show } from "solid-js";
import { t } from "../i18n";
import { Confirm } from "../ui/Confirm";
import { Advanced } from "../settings/fields/Advanced";
import { Row, Section } from "../settings/fields/Row";
import { useSettings } from "../settings/ctx";
import { fieldLabel } from "../settings/labels";
import { say, sayError } from "../ui/toast";
import { mcpRotateKey, mcpSetEnabled, mcpState, type McpState } from "./api";
import { mcpRevealKey, type McpKey } from "./key";

async function copy(text: string) {
  try {
    await navigator.clipboard.writeText(text);
    say(t("settings.mcp.copied", "Zkopírováno — vlož to do terminálu nebo do configu Claude"));
  } catch (e) {
    sayError(e);
  }
}

export function McpSettings() {
  const c = useSettings();
  const [st, setSt] = createSignal<McpState | null>(null);
  const [key, setKey] = createSignal<McpKey | null>(null);
  const [ask, setAsk] = createSignal(false);
  const [busy, setBusy] = createSignal(false);
  onMount(async () => {
    try {
      setSt(await mcpState());
    } catch {
      // mimo Tauri (náhled) — sekce zůstane jen s popisem
    }
  });
  async function run(fn: () => Promise<McpState>, done: string) {
    setBusy(true);
    try {
      setSt(await fn());
      say(done);
    } catch (e) {
      sayError(e);
    } finally {
      setBusy(false);
    }
  }
  const toggle = () => {
    const on = !st()?.enabled;
    setKey(null);
    void run(() => mcpSetEnabled(on), on ? t("settings.mcp.on", "Claude teď může nastavovat Ledge — zkopíruj příkaz níž") : t("settings.mcp.off", "MCP vypnuté — Claude už Ledge nenastaví"));
  };
  async function reveal() {
    try {
      setKey(key() ? null : await mcpRevealKey());
    } catch (e) {
      sayError(e);
    }
  }
  return (
    <Section
      title={t("settings.mcp.title", "Claude (MCP)")}
      desc={t("settings.mcp.desc", "Claude Code nebo Claude Desktop může Ledge nastavit za tebe: přidá aplikace, weby, odkazy a akce, které používáš, upraví záložky, karty a vzhled. Smí jen číst a měnit nastavení Ledge — nic nespouští a nic neposílá ven (jen tento počítač). Každou změnu vrátíš.")}
    >
      <Row id="mcp.enabled" live>
        <button
          class="st-switch"
          classList={{ on: !!st()?.enabled }}
          role="switch"
          aria-checked={!!st()?.enabled}
          aria-label={fieldLabel("mcp.enabled")}
          disabled={busy() || !st()}
          onClick={toggle}
        />
      </Row>
      <Show when={st()?.enabled && st()}>
        {(s) => (
          <>
            <Show when={!c.cfg.claude?.enabled}>
              <small class="st-note">{t("settings.mcp.bridgeOff", "Schvalování z Ledge (výš) je vypnuté — bez něj MCP neběží. Zapni ho a ulož.")}</small>
            </Show>
            <Row id="mcp.cli" stack label={t("settings.mcp.cli", "Připojit Claude Code")} hint={t("settings.mcp.cli.hint", "Spusť jednou v terminálu. Klíč si Ledge předá sám, v příkazu není.")}>
              <div class="st-status">
                <code class="mono grow">{s().cliCommand}</code>
                <button class="mini ok" onClick={() => void copy(s().cliCommand)}>{t("settings.mcp.copy", "Kopírovat")}</button>
              </div>
            </Row>
            <Advanced id="claude.mcp" inline hint={t("settings.mcp.adv.hint", "Claude Desktop, záloha přes http, výměna klíče")}>
            <Row id="mcp.desktop" label={t("settings.mcp.desktop", "Připojit Claude Desktop")} hint={t("settings.mcp.desktop.hint", "Úryvek do claude_desktop_config.json (sekce mcpServers), pak Claude Desktop restartuj.")}>
              <button class="mini" onClick={() => void copy(s().desktopConfig)}>{t("settings.mcp.copy", "Kopírovat")}</button>
            </Row>
            <Row id="mcp.key" stack label={t("settings.mcp.key", "Záloha přes http")} hint={t("settings.mcp.key.hint", "Jen když klient neumí spustit příkaz: adresa {url} a klíč v hlavičce. Klíč nikomu neposílej.", { url: s().url })}>
              <div class="st-status">
                <button class="mini" onClick={() => void reveal()}>{key() ? t("settings.mcp.key.hide", "Skrýt klíč") : t("settings.mcp.key.show", "Zobrazit klíč")}</button>
                <button class="mini" disabled={busy()} onClick={() => setAsk(true)}>{t("settings.mcp.rotate", "Vyměnit klíč")}</button>
              </div>
              <Show when={key()}>
                {(k) => (
                  <div class="st-status">
                    <code class="mono grow">{k().httpCommand}</code>
                    <button class="mini" onClick={() => void copy(k().httpCommand)}>{t("settings.mcp.copy", "Kopírovat")}</button>
                  </div>
                )}
              </Show>
              <Confirm
                when={ask()}
                text={t("settings.mcp.rotate.ask", "Vyměnit klíč MCP?")}
                detail={t("settings.mcp.rotate.detail", "Starý klíč přestane platit hned. Připojení přes příkaz si nový klíč vezme samo po restartu klienta; zálohu přes http musíš přidat znovu.")}
                yes={t("settings.mcp.rotate.yes", "Vyměnit")}
                onYes={() => {
                  setAsk(false);
                  setKey(null);
                  void run(mcpRotateKey, t("settings.mcp.rotated", "Klíč vyměněný"));
                }}
                onNo={() => setAsk(false)}
              />
            </Row>
            </Advanced>
          </>
        )}
      </Show>
    </Section>
  );
}
