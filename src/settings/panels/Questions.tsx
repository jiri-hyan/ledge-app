// Sekce „Otázky Clauda“ v panelu Claude (dřív samostatný panel): odpovídat na AskUserQuestion z Ledge
// (`claude.questionHoldS`, 0 = jen ukázat), jak dlouho čekat, pojistka neviděné otázky (`claude.questionUnseenS`,
// 0 = vypnuto) a upozornění, že chybí hook PreToolUse(AskUserQuestion). Zapojení hooků je jen na jednom místě
// — nahoře v panelu Claude (`HooksSetup`); volby se ukládají hned.
import { For, Show } from "solid-js";
import type { Snapshot } from "../../api";
import { fmtClock } from "../../format";
import { t } from "../../i18n";
import { useSettings } from "../ctx";
import { Toggle } from "../fields/controls";
import { Row, Section } from "../fields/Row";
import { fieldLabel } from "../labels";

const DEFAULT_HOLD = 540;
const MAX_HOLD = 550;
const MIN_HOLD = 30;
/** pojistka neviděné otázky (Rust `claude::unseen`: výchozí 20 s, 5–550, 0 = vypnuto) */
const DEFAULT_UNSEEN = 20;
const MIN_UNSEEN = 5;

/** Jak otázka v deníku skončila (Rust `QuestionLog.outcome`). */
const OUTCOME: Record<string, () => string> = {
  answered: () => t("questions.log.answered", "Odpovězeno"),
  holding: () => t("questions.log.holding", "Čeká v Ledge"),
  shown: () => t("questions.log.shown", "Jen ukázáno"),
  released: () => t("questions.log.released", "Předáno do Claude"),
  expired: () => t("questions.log.expired", "Vypršelo"),
  turnEnded: () => t("questions.log.turnEnded", "Konec tahu"),
  releasedUnseen: () => t("questions.log.releasedUnseen", "Nikdo neviděl — předáno do Claude"),
};
const outcomeName = (o: string) => OUTCOME[o]?.() ?? t("questions.log.skipped", "Přeskočeno");

export default function QuestionsSection(p: { s: Snapshot }) {
  const c = useSettings();
  const hold = (): number => c.cfg?.claude?.questionHoldS ?? DEFAULT_HOLD;
  const on = () => hold() > 0;
  const hooks = () => p.s.hooks;
  const qMissing = () => hooks().state === "none" || hooks().missing.includes("PreToolUse");
  const setHold = (v: number) => c.set("claude.questionHoldS", v);
  const unseen = (): number => c.cfg?.claude?.questionUnseenS ?? DEFAULT_UNSEEN;
  const setUnseen = (v: number) => c.set("claude.questionUnseenS", v);

  return (
    <Section title={t("questions.panel.title", "Otázky Clauda")}>
      <Row id="claude.questionHoldS" hint={on() ? t("questions.panel.onHint", "Claude počká na tvou odpověď v Ledge") : t("questions.panel.offHint", "Ledge otázku jen ukáže, odpovídáš v okně Claude")} dim={!p.s.claudeEnabled}>
        <Toggle value={on()} onChange={(v) => setHold(v ? DEFAULT_HOLD : 0)} label={fieldLabel("claude.questionHoldS")} />
      </Row>
      <Show when={on()}>
        <Row id="claude.questionHold.wait">
          <span class="st-num">
            <input
              type="number"
              min={MIN_HOLD}
              max={MAX_HOLD}
              style={{ width: "72px" }}
              value={hold()}
              aria-label={fieldLabel("claude.questionHold.wait")}
              onChange={(e) => setHold(Math.min(MAX_HOLD, Math.max(MIN_HOLD, Math.round(+e.currentTarget.value) || DEFAULT_HOLD)))}
            />
            <span class="unit">s</span>
          </span>
        </Row>
      </Show>
      <Show when={on()}>
        <Row id="claude.questionUnseenS" hint={unseen() > 0 ? t("questions.panel.unseenOn", "Když otázku do {n} s nikdo v Ledge neuvidí (šuplík zavřený, hra přes celou obrazovku), Claude se zeptá ve svém okně", { n: unseen() }) : t("questions.panel.unseenOff", "Ledge drží otázku celou dobu, i když ji nikdo nevidí")}>
          <Toggle value={unseen() > 0} onChange={(v) => setUnseen(v ? DEFAULT_UNSEEN : 0)} label={fieldLabel("claude.questionUnseenS")} />
        </Row>
        <Show when={unseen() > 0}>
          <Row id="claude.questionUnseen.wait">
            <span class="st-num">
              <input
                type="number"
                min={MIN_UNSEEN}
                max={MAX_HOLD}
                style={{ width: "72px" }}
                value={unseen()}
                aria-label={fieldLabel("claude.questionUnseen.wait")}
                onChange={(e) => setUnseen(Math.min(MAX_HOLD, Math.max(MIN_UNSEEN, Math.round(+e.currentTarget.value) || DEFAULT_UNSEEN)))}
              />
              <span class="unit">s</span>
            </span>
          </Row>
        </Show>
      </Show>
      <Show when={qMissing() && hooks().state !== "error"}>
        <p class="st-note">{t("questions.panel.hookMissingNote", "Ledge otázky zatím nedostává — zapoj hooky nahoře v části „Propojit s Claude Code“.")}</p>
      </Show>
      <Show when={!p.s.claudeEnabled}>
        <p class="st-note">{t("questions.panel.bridgeOff", "Most pro Claude je vypnutý — Ledge otázky neuvidí.")}</p>
      </Show>
      <Show when={p.s.questionLog?.length}>
        <Row id="claude.questionLog" stack label={t("questions.panel.log", "Deník posledních otázek")} hint={t("questions.panel.logHint", "Posledních 20 otázek Clauda od spuštění Ledge — jestli k tobě došly a jak skončily")}>
          <div class="st-list">
            <For each={p.s.questionLog ?? []}>
              {(item) => (
                <div class="st-item">
                  <span class="mono">{fmtClock(new Date(item.at))}</span>
                  <span class="grow">
                    <strong>{item.title || t("questions.log.untitled", "Otázka Clauda")}</strong>
                    <Show when={item.detail}>{(d) => <small> · {d()}</small>}</Show>
                  </span>
                  <span class="st-tag">{outcomeName(item.outcome)}</span>
                </div>
              )}
            </For>
          </div>
        </Row>
      </Show>
    </Section>
  );
}
