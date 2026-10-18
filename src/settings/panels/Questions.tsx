// Sekce „Otázky Clauda“ v panelu Claude (dřív samostatný panel): odpovídat na AskUserQuestion z Ledge
// (`claude.questionHoldS`, 0 = jen ukázat), jak dlouho čekat a stav hooku PreToolUse(AskUserQuestion)
// s tlačítkem doplnit / zapojit (Rust `hooksetup.rs`, příkaz `hooks_install`). Volba jde do draftu a
// uloží ji společná lišta (Uložit); zapojení hooku je akce (zápis do ~/.claude), ne volba configu.
import { createSignal, Show } from "solid-js";
import { api, type Snapshot } from "../../api";
import { t } from "../../i18n";
import { say, sayError } from "../../ui/toast";
import { useSettings } from "../ctx";
import { Toggle } from "../fields/controls";
import { Row, Section } from "../fields/Row";
import { fieldLabel } from "../labels";

const DEFAULT_HOLD = 540;
const MAX_HOLD = 550;
const MIN_HOLD = 30;

export default function QuestionsSection(p: { s: Snapshot }) {
  const c = useSettings();
  const [ask, setAsk] = createSignal(false);
  const [busy, setBusy] = createSignal(false);
  const hold = (): number => c.cfg?.claude?.questionHoldS ?? DEFAULT_HOLD;
  const on = () => hold() > 0;
  const hooks = () => p.s.hooks;
  const qMissing = () => hooks().state === "none" || hooks().missing.includes("PreToolUse");
  const setHold = (v: number) => c.set("claude.questionHoldS", v);

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
    <Section title={t("questions.panel.title", "Otázky Clauda")} desc={t("questions.panel.explain", "Když se Claude Code zeptá (nástroj AskUserQuestion), Ledge ukáže otázku s možnostmi jako v chatu a odpovíš kliknutím. Když neodpovíš včas nebo zvolíš „Odpovědět v Claude“, otázka se ukáže v okně Claude jako dřív.")}>
      <Row id="claude.questionHoldS" hint={on() ? t("questions.panel.onHint", "Claude počká na tvou odpověď v Ledge") : t("questions.panel.offHint", "Ledge otázku jen ukáže, odpovídáš v okně Claude")} dim={!p.s.claudeEnabled}>
        <Toggle value={on()} onChange={(v) => setHold(v ? DEFAULT_HOLD : 0)} label={fieldLabel("claude.questionHoldS")} />
      </Row>
      <Show when={on()}>
        <Row id="claude.questionHold.wait" label={t("questions.panel.hold", "Čekat na odpověď")} hint={t("questions.panel.holdHint", "Pak otázka přejde do okna Claude (nejvýš {max} s)", { max: MAX_HOLD })}>
          <span class="st-num">
            <input
              type="number"
              min={MIN_HOLD}
              max={MAX_HOLD}
              style={{ width: "72px" }}
              value={hold()}
              aria-label={t("questions.panel.hold", "Čekat na odpověď")}
              onChange={(e) => setHold(Math.min(MAX_HOLD, Math.max(MIN_HOLD, Math.round(+e.currentTarget.value) || DEFAULT_HOLD)))}
            />
            <span class="unit">s</span>
          </span>
        </Row>
      </Show>
      <Row id="claude.questionHook" label={t("questions.panel.hook", "Hook pro otázky")} hint={`${hookText()} ${t("questions.panel.newSessions", "Platí pro nově spuštěné session.")}`}>
        <Show when={qMissing() && hooks().state !== "error"}>
          <button class="mini" disabled={busy()} onClick={() => (hooks().state === "none" ? setAsk(true) : void install())}>
            {hooks().state === "none" ? t("questions.panel.install", "Zapojit hooky") : t("questions.panel.add", "Doplnit otázky Clauda")}
          </button>
        </Show>
      </Row>
      <Show when={ask()}>
        <Row id="claude.questionHook.confirm" label={t("questions.panel.confirm", "Zapojit hooky Ledge do ~/.claude/settings.json?")} hint={t("questions.panel.confirmHint", "Schvalování, stav session, subagenti a otázky. Cizí hooky zůstanou, záloha se udělá jednou.")}>
          <span>
            <button class="mini" onClick={() => setAsk(false)}>{t("questions.panel.no", "Ne")}</button>{" "}
            <button class="mini ok" disabled={busy()} onClick={() => void install()}>{t("questions.panel.yes", "Zapojit")}</button>
          </span>
        </Row>
      </Show>
      <Show when={!p.s.claudeEnabled}>
        <p class="st-note">{t("questions.panel.bridgeOff", "Most pro Claude je vypnutý — Ledge otázky neuvidí.")}</p>
      </Show>
    </Section>
  );
}
