// Panel „Claude“: most pro schvalování (zapnutí, port, jak dlouho čekat), otázky Clauda (`Questions.tsx`,
// dřív samostatný panel), limity, útrata, hlídač nedokončených session a zapojení hooků Ledge do
// `~/.claude/settings.json` (`HooksSetup` → Rust `hooksetup.rs`), MCP server pro Clauda (`mcp/McpSettings.tsx`).
import { createSignal, onMount, Show } from "solid-js";
import { api, type HooksStatus } from "../../api";
import { t } from "../../i18n";
import { say, sayError } from "../../ui/toast";
import { useSettings } from "../ctx";
import { Confirm } from "../../ui/Confirm";
import { Num, Seg, Slider, Toggle } from "../fields/controls";
import { Advanced } from "../fields/Advanced";
import { Row, Section } from "../fields/Row";
import type { PanelProps } from "../types";
import StatusLineSetup from "../../limits/StatusLineSetup";
import { fieldLabel } from "../labels";
import QuestionsSection from "./Questions";
import { McpSettings } from "../../mcp/McpSettings";

const HOOKS_TEXT: Record<HooksStatus["state"], [string, string]> = {
  full: ["settings.hooks.full", "Zapojené"],
  partial: ["settings.hooks.partial", "Zapojené jen zčásti"],
  none: ["settings.hooks.none", "Nezapojené"],
  error: ["settings.hooks.error", "settings.json nejde přečíst"],
};

/** Hooky Ledge v `~/.claude/settings.json`: stav + zapojit / odpojit s potvrzením. */
function HooksSetup() {
  const [st, setSt] = createSignal<HooksStatus | null>(null);
  const [ask, setAsk] = createSignal<"install" | "remove" | null>(null);
  const [busy, setBusy] = createSignal(false);
  onMount(async () => setSt((await api.hooksStatus()) ?? null));
  async function run(what: "install" | "remove") {
    setAsk(null);
    setBusy(true);
    try {
      setSt((await (what === "install" ? api.hooksInstall() : api.hooksRemove())) ?? null);
      say(what === "install" ? t("settings.hooks.installed", "Hooky zapojené — platí pro nově spuštěné session") : t("settings.hooks.removed", "Hooky Ledge odpojené"));
    } catch (e) {
      sayError(e);
    } finally {
      setBusy(false);
    }
  }
  const detail = (h: HooksStatus) => (h.error ? h.error : h.state === "partial" ? t("settings.hooks.missing", "Chybí: {m}", { m: h.missing.join(", ") }) : h.path);
  return (
    <Row id="claude.hooks" stack live>
      <Show when={st()} fallback={<div class="st-skel" style={{ width: "60%" }} />}>
        {(h) => (
          <>
            <div class="st-status">
              <span class="st-dot" classList={{ ok: h().state === "full", warn: h().state === "partial", bad: h().state === "error" }} />
              <span class="grow">
                {t(...HOOKS_TEXT[h().state])}
                <small class="mono">{detail(h())}</small>
              </span>
              <Show when={h().state !== "full" && h().state !== "error"}>
                <button class="mini ok" disabled={busy()} onClick={() => setAsk("install")}>{t("settings.hooks.install", "Zapojit hooky")}</button>
              </Show>
              <Show when={h().state !== "none" && h().state !== "error"}>
                <button class="mini" disabled={busy()} onClick={() => setAsk("remove")}>{t("settings.hooks.remove", "Odpojit")}</button>
              </Show>
            </div>
            <Confirm
              when={ask() !== null}
              text={ask() === "install" ? t("settings.hooks.ask.install", "Zapojit hooky Ledge do settings.json?") : t("settings.hooks.ask.remove", "Odpojit hooky Ledge ze settings.json?")}
              detail={
                ask() === "install"
                  ? t("settings.hooks.ask.install.detail", "Claude Code pak posílá Ledge události session: schvalování přímo v šuplíku, subagenti pod session a hlídač nezacommitované práce. Jen na 127.0.0.1, bez Ledge Claude běží dál normálně. Ostatní nastavení i cizí hooky zůstanou, poprvé se uloží záloha settings.json.bak-pred-ledge.")
                  : t("settings.hooks.ask.remove.detail", "Odeberou se jen hooky mířící na Ledge; ostatní nastavení i cizí hooky zůstanou.")
              }
              yes={ask() === "install" ? t("settings.hooks.install.yes", "Zapojit") : t("settings.hooks.remove.yes", "Odpojit")}
              onYes={() => run(ask()!)}
              onNo={() => setAsk(null)}
            />
          </>
        )}
      </Show>
    </Row>
  );
}

export default function ClaudePanel(p: PanelProps) {
  const c = useSettings();
  return (
    <>
      <Section title={t("settings.claude.connect", "Propojit s Claude Code (nutné)")}>
        <HooksSetup />
      </Section>
      <Section title={t("settings.claude.approve", "Schvalování")}>
        <Row id="claude.enabled">
          <Toggle path="claude.enabled" label={fieldLabel("claude.enabled")} />
        </Row>
        <Row id="claude.approvalTimeoutS" dim={!c.cfg.claude.enabled}>
          <Slider path="claude.approvalTimeoutS" min={5} max={550} step={5} label={fieldLabel("claude.approvalTimeoutS")} fmt={(v) => (v >= 60 ? `${Math.floor(v / 60)} min ${v % 60 ? `${v % 60} s` : ""}` : `${v} s`)} />
        </Row>
        <Row id="claude.toast" dim={!c.cfg.claude.enabled}>
          <Toggle path="claude.toast" label={fieldLabel("claude.toast")} />
        </Row>
        <Row id="smart.approvalSound.enabled" dim={!c.cfg.claude.enabled}>
          <Toggle path="smart.approvalSound.enabled" label={fieldLabel("smart.approvalSound.enabled")} />
        </Row>
        <Show when={c.cfg.smart?.approvalSound?.enabled}>
          <Row id="smart.approvalSound.minGapS" nested>
            <Slider path="smart.approvalSound.minGapS" min={5} max={600} step={5} label={fieldLabel("smart.approvalSound.minGapS")} fmt={(v) => (v >= 60 ? `${Math.floor(v / 60)} min ${v % 60 ? `${v % 60} s` : ""}` : `${v} s`)} />
          </Row>
        </Show>
      </Section>
      <QuestionsSection s={p.s} />
      <Section title={t("settings.claude.limits", "Limity 5h / týden")}>
        <Row id="claude.statusLine" stack live>
          <StatusLineSetup />
        </Row>
        <Row id="claude.limitsDesktop" stack>
          <Toggle path="claude.limitsDesktop" label={fieldLabel("claude.limitsDesktop")} />
          <small class="st-note">{t("settings.claude.limitsDesktop.sentence", "Pracuješ v aplikaci Claude? Ta si limity ukládá sama; Ledge je jen přečte (nic neposílá).")}</small>
        </Row>
        <Row id="claude.limitsFetch" stack>
          <Toggle path="claude.limitsFetch" label={fieldLabel("claude.limitsFetch")} />
          <small class="st-note">{t("settings.claude.limitsFetch.sentence", "Ledge se jednou za 10 min zeptá Anthropicu na tvé limity tvým přihlášením Claude Code; token nikam neukládá ani neposílá jinam. Jen když je karta s limity vidět nebo je ukazuje proužek, nikdy ve hře.")}</small>
        </Row>
        <Row id="claude.limitsReminder">
          <Toggle path="claude.limitsReminder" label={fieldLabel("claude.limitsReminder")} />
        </Row>
        <Row id="claude.limitsAlert">
          <Toggle path="claude.limitsAlert" label={fieldLabel("claude.limitsAlert")} />
        </Row>
        <Row id="claude.limitsPace">
          <Toggle path="claude.limitsPace" label={fieldLabel("claude.limitsPace")} />
        </Row>
        <Row id="smart.incidents.enabled" stack>
          <Toggle path="smart.incidents.enabled" label={fieldLabel("smart.incidents.enabled")} />
          <small class="st-note">
            {t("settings.smart.incidents.sentence", "Ledge se bude ptát status.anthropic.com každých {n} min, jen když je karta s limity vidět. Posílá se prázdný dotaz bez tvých dat; ukáže se jen probíhající incident.", { n: c.cfg.smart?.incidents?.everyMin ?? 10 })}
          </small>
        </Row>
        <Show when={c.cfg.smart?.incidents?.enabled}>
          <Row id="smart.incidents.everyMin" nested>
            <Slider path="smart.incidents.everyMin" min={5} max={120} step={5} label={fieldLabel("smart.incidents.everyMin")} fmt={(v) => `${v} min`} />
          </Row>
        </Show>
      </Section>
      <div class="st-cols">
      <Section title={t("settings.claude.costs", "Hodnota v cenách API")}>
        <Row id="claude.costs">
          <Toggle path="claude.costs" label={fieldLabel("claude.costs")} />
        </Row>
        <Row id="claude.contextBar">
          <Toggle path="claude.contextBar" label={fieldLabel("claude.contextBar")} />
        </Row>
      </Section>
      <Section title={t("settings.claude.watch", "Hlídač nedokončené práce")}>
        <Row id="claude.enforceCommit" stack>
          <Seg
            path="claude.enforceCommit"
            label={fieldLabel("claude.enforceCommit")}
            options={[
              { v: "off", label: t("settings.claude.enforce.off", "Nehlídat") },
              { v: "notify", label: t("settings.claude.enforce.notify", "Jen ukázat (doporučeno)") },
              { v: "block", label: t("settings.claude.enforce.block", "Vrátit session k dokončení") },
            ]}
          />
          <small class="st-note">
            {c.cfg.claude.enforceCommit === "block"
              ? t("settings.claude.enforce.block.hint", "Když session skončí s nezacommitovanou prací, Ledge ji jednou vrátí, ať dokončí commit a push.")
              : c.cfg.claude.enforceCommit === "off"
                ? t("settings.claude.enforce.off.hint", "Ledge kvůli tomu vůbec nevolá git.")
                : t("settings.claude.enforce.notify.hint", "Štítek u session v šuplíku a v proužku, nic víc.")}
          </small>
        </Row>
      </Section>
      </div>
      <McpSettings />
      <Advanced id="claude" hint={t("settings.claude.adv.hint", "Port pro spojení s Claude Code")}>
        <Section>
          <Row id="claude.port" dim={!c.cfg.claude.enabled}>
            <Num path="claude.port" min={1024} max={65535} label={fieldLabel("claude.port")} width="84px" />
          </Row>
        </Section>
      </Advanced>
    </>
  );
}
