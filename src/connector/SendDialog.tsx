// „Poslat do {název}“ z ⋯ poznámky: náhled (úkoly s řešitelem a prioritou, nebo celá poznámka) → jeden klik
// Odeslat. Nic nejde ven před kliknutím; řešitele, kterého nešlo určit, Ledge nehádá — bod je odškrtnutý a řekne proč.
// Plovoucí panel (`ui/Popover`), fokus nekrade.
import { createSignal, For, onMount, Show } from "solid-js";
import { t } from "../i18n";
import Popover from "../ui/Popover";
import type { Box } from "../ui/place";
import { errorText, say, sayError } from "../ui/toast";
import { connectorPreview, connectorSend, type Preview, type SendOut } from "./api";
import { defaultLines, needsDecision, priorityBand, summarize, toggled, type Assignee, type Item } from "./model";
import "./connector.css";

const assigneeText = (a: Assignee): string => {
  switch (a.kind) {
    case "resolved":
      return t("notes.connector.assignee.resolved", "řešitel {name}", { name: a.name });
    case "ambiguous":
      return t("notes.connector.assignee.ambiguous", "řešitel „{q}“ sedí na víc lidí ({names}) — pošle se bez řešitele, tedy tobě", { q: a.query, names: a.names.join(", ") });
    case "unknown":
      return t("notes.connector.assignee.unknown", "řešitel „{q}“ v týmu není — pošle se bez řešitele, tedy tobě", { q: a.query });
    default:
      return t("notes.connector.assignee.unset", "řešitel: ty (vlastník klíče)");
  }
};

const prioText = (p: number | null): string | null => {
  const b = priorityBand(p);
  return b === null ? null : b === "high" ? t("notes.connector.prio.high", "priorita vysoká") : b === "mid" ? t("notes.connector.prio.mid", "priorita střední") : t("notes.connector.prio.low", "priorita nízká");
};

export default function SendDialog(p: { path: string; anchor: Box | null; onClose: () => void }) {
  const [pv, setPv] = createSignal<Preview | null>(null);
  const [err, setErr] = createSignal<string | null>(null);
  const [sel, setSel] = createSignal<number[]>([]);
  const [busy, setBusy] = createSignal(true);
  const [out, setOut] = createSignal<SendOut | null>(null);
  const name = () => pv()?.name ?? "";
  const title = () => (name().trim() ? t("notes.connector.title", "Poslat do {name}", { name: name().trim() }) : t("notes.connector.title.none", "Poslat do propojené aplikace"));

  async function load(mode?: "tasks" | "note") {
    setBusy(true);
    setErr(null);
    try {
      const v = await connectorPreview(p.path, mode);
      setPv(v);
      setSel(defaultLines(v.plan));
    } catch (e) {
      setErr(errorText(e));
    } finally {
      setBusy(false);
    }
  }
  onMount(() => void load());

  const plan = () => pv()?.plan;
  const count = () => (plan()?.mode === "note" ? 1 : sel().length);
  const canSend = () => !busy() && !out() && !!plan() && (plan()!.mode === "note" ? !plan()!.noteSent : sel().length > 0);

  async function send() {
    const v = pv();
    if (!v || !canSend()) return;
    setBusy(true);
    try {
      const r = await connectorSend(p.path, v.hash, v.plan.mode, sel());
      setOut(r);
      const s = summarize(r.results);
      if (s.failed === 0) say(t("notes.connector.sent", "Odesláno: {n}", { n: s.ok }));
    } catch (e) {
      sayError(e);
    } finally {
      setBusy(false);
    }
  }

  return (
    <Popover anchor={() => p.anchor} align="end" label={title()} onDismiss={p.onClose}>
      <div class="mono-dlg">
        <header>
          <strong>{title()}</strong>
          <Show when={pv()}>{(v) => <small>{t("notes.connector.to", "do {host}", { host: v().host })}</small>}</Show>
        </header>
        <Show when={busy() && !pv() && !err()}>
          <p class="mono-note">{t("notes.connector.loading", "Načítám tým…")}</p>
        </Show>
        <Show when={err()}>
          <p class="mono-warn" role="alert">{err()}</p>
        </Show>
        <Show when={plan()}>
          {(pl) => (
            <>
              <Show when={pl().hasTasks}>
                <div class="mono-mode" role="group" aria-label={t("notes.connector.mode", "Co poslat")}>
                  <button class="mini" classList={{ ok: pl().mode === "tasks" }} disabled={busy() || !!out()} onClick={() => void load("tasks")}>{t("notes.connector.mode.tasks", "Úkoly z bodů")}</button>
                  <button class="mini" classList={{ ok: pl().mode === "note" }} disabled={busy() || !!out()} onClick={() => void load("note")}>{t("notes.connector.mode.note", "Celá poznámka")}</button>
                </div>
              </Show>
              <Show when={pl().mode === "tasks"}>
                <Show when={pl().items.length === 0}>
                  <p class="mono-note">{t("notes.connector.none", "V poznámce není žádný otevřený bod „- [ ]“.")}</p>
                </Show>
                <ul class="mono-items">
                  <For each={pl().items}>
                    {(it: Item) => (
                      <li classList={{ sent: it.sent, warn: needsDecision(it) }}>
                        <label>
                          <input type="checkbox" checked={sel().includes(it.line)} disabled={it.sent || !!out() || busy()} onChange={() => setSel(toggled(sel(), it.line))} />
                          <span class="mono-title">{it.title}</span>
                        </label>
                        <small>
                          {it.sent ? t("notes.connector.sent.already", "už odesláno") : assigneeText(it.assignee)}
                          <Show when={!it.sent && prioText(it.priority)}>{(pt) => <> · {pt()}</>}</Show>
                        </small>
                      </li>
                    )}
                  </For>
                </ul>
                <Show when={pl().truncated}>
                  <p class="mono-warn">{t("notes.connector.truncated", "Bodů je víc než 20 — odešle se prvních 20, zbytek až při dalším kliknutí.")}</p>
                </Show>
                <p class="mono-note">{t("notes.connector.saved", "Posílá se uložená verze poznámky.")}</p>
              </Show>
              <Show when={pl().mode === "note"}>
                <p class="mono-note">
                  {pl().noteSent
                    ? t("notes.connector.note.sent", "Tahle poznámka už byla odeslána (connector: ve frontmatteru).")
                    : t("notes.connector.note.what", "Poznámka „{title}“ se pošle jako poznámka. Obrázky z ní se nepřenesou.", { title: pl().noteTitle })}
                </p>
              </Show>
            </>
          )}
        </Show>
        <Show when={out()}>
          {(o) => {
            const s = summarize(o().results);
            return (
              <div class="mono-result" role="status">
                <p>
                  {t("notes.connector.result", "Odesláno {ok}, selhalo {failed}.", { ok: s.ok, failed: s.failed })}
                  {s.duplicate ? " " + t("notes.connector.result.dup", "({n} už bylo odesláno dříve.)", { n: s.duplicate }) : ""}
                </p>
                <Show when={s.error}><p class="mono-warn">{s.error}</p></Show>
                <Show when={o().linkError}><p class="mono-warn">{t("notes.connector.linkerr", "Odkazy se do poznámky nezapsaly: {e}", { e: o().linkError ?? "" })}</p></Show>
              </div>
            );
          }}
        </Show>
        <footer>
          <button class="mini" onClick={p.onClose}>{out() ? t("notes.connector.close", "Zavřít") : t("ui.confirm.cancel", "Zrušit")}</button>
          <Show when={!out()}>
            <button class="mini ok" disabled={!canSend()} onClick={() => void send()}>
              {plan()?.mode === "note" ? t("notes.connector.send.note", "Odeslat poznámku") : t("notes.connector.send.tasks", "Odeslat ({n})", { n: count() })}
            </button>
          </Show>
        </footer>
      </div>
    </Popover>
  );
}
