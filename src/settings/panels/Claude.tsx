// Panel „Claude“: most pro schvalování (zapnutí, port, jak dlouho čekat), otázky Clauda (`Questions.tsx`,
// dřív samostatný panel), limity, útrata, hlídač nedokončených session a zapojení hooků Ledge do
// `~/.claude/settings.json` (`HooksSetup` → Rust `hooksetup.rs`).
import { createSignal, onMount, Show } from "solid-js";
import { api, type HooksStatus } from "../../api";
import { t } from "../../i18n";
import { say, sayError } from "../../ui/toast";
import { useSettings } from "../ctx";
import { Confirm } from "../fields/Confirm";
import { Num, Seg, Slider, Toggle } from "../fields/controls";
import { PanelHead, Row, Section } from "../fields/Row";
import type { PanelProps } from "../types";
import StatusLineSetup from "../../limits/StatusLineSetup";
import { fieldLabel } from "../labels";
import QuestionsSection from "./Questions";

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
    <Row id="claude.hooks" stack>
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
      <PanelHead title={t("settings.nav.claude", "Claude")} desc={t("settings.claude.desc", "Ledge ukazuje session Claude Code a jejich žádosti o povolení. Povolit smí vždy jen člověk.")} />
      <Section title={t("settings.claude.approve", "Schvalování")}>
        <Row id="claude.enabled" hint={t("settings.claude.enabled.hint", "Žádosti Clauda o povolení se objeví v šuplíku (most na 127.0.0.1:{port})", { port: c.cfg.claude.port })}>
          <Toggle path="claude.enabled" label={fieldLabel("claude.enabled")} />
        </Row>
        <Row id="claude.approvalTimeoutS" dim={!c.cfg.claude.enabled}>
          <Slider path="claude.approvalTimeoutS" min={5} max={550} step={5} label={fieldLabel("claude.approvalTimeoutS")} fmt={(v) => (v >= 60 ? `${Math.floor(v / 60)} min ${v % 60 ? `${v % 60} s` : ""}` : `${v} s`)} />
        </Row>
        <Row id="claude.toast" dim={!c.cfg.claude.enabled}>
          <Toggle path="claude.toast" label={fieldLabel("claude.toast")} />
        </Row>
        <Row id="claude.port" dim={!c.cfg.claude.enabled}>
          <Num path="claude.port" min={1024} max={65535} label={fieldLabel("claude.port")} width="84px" />
        </Row>
        <HooksSetup />
      </Section>
      <QuestionsSection s={p.s} />
      <Section title={t("settings.claude.limits", "Limity 5h / týden")}>
        <Row id="claude.statusLine" stack>
          <StatusLineSetup />
        </Row>
        <Row id="claude.limitsReminder">
          <Toggle path="claude.limitsReminder" label={fieldLabel("claude.limitsReminder")} />
        </Row>
      </Section>
      <Section title={t("settings.claude.costs", "Útrata (odhad v cenách API)")}>
        <Row id="claude.costs">
          <Toggle path="claude.costs" label={fieldLabel("claude.costs")} />
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
              { v: "block", label: t("settings.claude.enforce.block", "Postrčit session") },
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
    </>
  );
}
