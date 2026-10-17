// Panel Nastavení „Otázky Clauda“: odpovídat na AskUserQuestion z Ledge (`claude.questionHoldS`, 0 = jen
// ukázat), vysvětlení a stav hooku PreToolUse(AskUserQuestion) s tlačítkem doplnit / zapojit
// (Rust `hooksetup.rs`, příkaz `hooks_install`). Změna se ukládá hned přes `save()` shellu.
import { createSignal, Show } from "solid-js";
import { api } from "../../api";
import { t } from "../../i18n";
import { say, sayError } from "../../ui/toast";
import type { PanelProps } from "../types";

const DEFAULT_HOLD = 540;
const MAX_HOLD = 550;
const MIN_HOLD = 30;

export default function QuestionsPanel(p: PanelProps) {
  const [ask, setAsk] = createSignal(false);
  const [busy, setBusy] = createSignal(false);
  const hold = (): number => p.cfg?.claude?.questionHoldS ?? DEFAULT_HOLD;
  const on = () => hold() > 0;
  const hooks = () => p.s.hooks;
  const qMissing = () => hooks().state === "none" || hooks().missing.includes("PreToolUse");

  async function setHold(v: number) {
    p.setCfg("claude", "questionHoldS", v);
    await p.save();
  }

  async function install() {
    setAsk(false);
    setBusy(true);
    try {
      await api.hooksInstall();
      say(t("questions.hooksAdded", "Otázky Clauda zapojené — platí pro nově spuštěné session"));
    } catch (e) {
      sayError(e);
    } finally {
      setBusy(false);
    }
  }

  const hookText = () => {
    const h = hooks();
    if (h.state === "error") return h.error ?? t("questions.panel.hookError", "settings.json nejde přečíst");
    if (h.state === "none") return t("questions.panel.hookNone", "Hooky Ledge nejsou v Claude Code zapojené.");
    if (qMissing()) return t("questions.panel.hookMissing", "Chybí hook pro otázky (hooky zapojila starší verze Ledge).");
    return t("questions.panel.hookOk", "Zapojený — Ledge otázky dostává.");
  };

  return (
    <div class="card">
      <h4>{t("questions.panel.title", "Otázky Clauda")}</h4>
      <div class="opt">
        <span>
          {t("questions.panel.what", "Co to dělá")}
          <small>
            {t("questions.panel.explain", "Když se Claude Code zeptá (nástroj AskUserQuestion), Ledge ukáže otázku s možnostmi jako v chatu a odpovíš kliknutím. Když neodpovíš včas nebo zvolíš „Odpovědět v Claude“, otázka se ukáže v okně Claude jako dřív.")}
          </small>
        </span>
      </div>
      <div class="opt">
        <span>
          {t("questions.panel.toggle", "Odpovídat na otázky z Ledge")}
          <small>{on() ? t("questions.panel.onHint", "Claude počká na tvou odpověď v Ledge") : t("questions.panel.offHint", "Ledge otázku jen ukáže, odpovídáš v okně Claude")}</small>
        </span>
        <button class="switch" classList={{ on: on() }} role="switch" aria-checked={on()} aria-label={t("questions.panel.toggle", "Odpovídat na otázky z Ledge")} onClick={() => void setHold(on() ? 0 : DEFAULT_HOLD)} />
      </div>
      <Show when={on()}>
        <div class="opt">
          <span>
            {t("questions.panel.hold", "Čekat na odpověď")}
            <small>{t("questions.panel.holdHint", "Pak otázka přejde do okna Claude (nejvýš {max} s)", { max: MAX_HOLD })}</small>
          </span>
          <span>
            <input type="number" min={MIN_HOLD} max={MAX_HOLD} value={hold()} onChange={(e) => void setHold(Math.min(MAX_HOLD, Math.max(MIN_HOLD, Math.round(+e.currentTarget.value) || DEFAULT_HOLD)))} /> s
          </span>
        </div>
      </Show>
      <div class="opt">
        <span>
          {t("questions.panel.hook", "Hook pro otázky")}
          <small>{hookText()} {t("questions.panel.newSessions", "Platí pro nově spuštěné session.")}</small>
        </span>
        <Show when={qMissing() && hooks().state !== "error"}>
          <span>
            <button class="mini" disabled={busy()} onClick={() => (hooks().state === "none" ? setAsk(true) : void install())}>
              {hooks().state === "none" ? t("questions.panel.install", "Zapojit hooky") : t("questions.panel.add", "Doplnit otázky Clauda")}
            </button>
          </span>
        </Show>
      </div>
      <Show when={ask()}>
        <div class="opt">
          <span>
            {t("questions.panel.confirm", "Zapojit hooky Ledge do ~/.claude/settings.json?")}
            <small>{t("questions.panel.confirmHint", "Schvalování, stav session, subagenti a otázky. Cizí hooky zůstanou, záloha se udělá jednou.")}</small>
          </span>
          <span>
            <button class="mini" onClick={() => setAsk(false)}>{t("questions.panel.no", "Ne")}</button>{" "}
            <button class="mini ok" disabled={busy()} onClick={() => void install()}>{t("questions.panel.yes", "Zapojit")}</button>
          </span>
        </div>
      </Show>
      <Show when={!p.s.claudeEnabled}>
        <div class="opt"><span><small>{t("questions.panel.bridgeOff", "Most pro Claude je vypnutý — Ledge otázky neuvidí.")}</small></span></div>
      </Show>
    </div>
  );
}
