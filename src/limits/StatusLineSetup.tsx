// Nastavení → Claude Code: zapojení stavového řádku Ledge (`statusLine` v `~/.claude/settings.json`)
// pro limity 5h / týden. Ledge je průchozí: původní stavový řádek uživatele běží dál beze změny.
// Rust: `commands/limits.rs` → `statuslinesetup.rs`. Řádek s kotvou `claude.statusLine` je v `panels/Claude.tsx`.
import { createSignal, onMount, Show } from "solid-js";
import { api, type StatusLineStatus } from "../api";
import { t } from "../i18n";
import { say, sayError } from "../ui/toast";
import { Confirm } from "../settings/fields/Confirm";

const stateText = (st: StatusLineStatus["state"]) =>
  st === "full"
    ? t("sessions.limits.setup.full", "Zapojený")
    : st === "foreign"
      ? t("sessions.limits.setup.foreign", "Nezapojený (máš vlastní stavový řádek — Ledge ho obalí, poběží dál)")
      : st === "none"
        ? t("sessions.limits.setup.none", "Nezapojený")
        : t("sessions.limits.setup.error", "settings.json nejde přečíst");

export default function StatusLineSetup() {
  const [st, setSt] = createSignal<StatusLineStatus | null>(null);
  const [ask, setAsk] = createSignal<"install" | "remove" | null>(null);
  const [busy, setBusy] = createSignal(false);
  onMount(async () => {
    try {
      setSt((await api.statuslineStatus()) ?? null);
    } catch (e) {
      sayError(e);
    }
  });
  async function run(what: "install" | "remove") {
    setAsk(null);
    setBusy(true);
    try {
      setSt((await (what === "install" ? api.statuslineInstall() : api.statuslineRemove())) ?? null);
      say(what === "install" ? t("sessions.limits.setup.installed", "Stavový řádek zapojený — limity se ukážou po další odpovědi Clauda") : t("sessions.limits.setup.removed", "Stavový řádek Ledge odpojený, původní vrácený"));
    } catch (e) {
      sayError(e);
    } finally {
      setBusy(false);
    }
  }
  const detail = (s: StatusLineStatus) =>
    s.error ? s.error : s.command ? t("sessions.limits.setup.wraps", "Tvůj příkaz: {cmd}", { cmd: s.command }) : s.path;
  return (
    <Show when={st()} fallback={<div class="st-skel" style={{ width: "60%" }} />}>
        {(s) => (
          <>
            <div class="st-status">
              <span class="st-dot" classList={{ ok: s().state === "full", bad: s().state === "error" }} />
              <span class="grow">
                {stateText(s().state)}
                <small class="mono">{detail(s())}</small>
              </span>
              <Show when={s().state === "none" || s().state === "foreign"}>
                <button class="mini ok" disabled={busy()} onClick={() => setAsk("install")}>{t("sessions.limits.setup.install", "Zapojit stavový řádek")}</button>
              </Show>
              <Show when={s().state === "full"}>
                <button class="mini" disabled={busy()} onClick={() => setAsk("remove")}>{t("sessions.limits.setup.remove", "Odpojit")}</button>
              </Show>
            </div>
            <Confirm
              when={ask() !== null}
              text={ask() === "install" ? t("sessions.limits.setup.ask.install", "Zapojit stavový řádek Ledge do settings.json?") : t("sessions.limits.setup.ask.remove", "Odpojit stavový řádek Ledge?")}
              detail={
                ask() === "install"
                  ? t("sessions.limits.setup.ask.install.detail", "Claude Code pak po každé odpovědi předá Ledge limity 5h a týden (jen do souboru na tomhle PC, žádná síť). Tvůj dosavadní stavový řádek poběží dál a vypíše totéž co dřív; Ledge si ho uloží a při odpojení vrátí přesně. Poprvé se uloží záloha settings.json.bak-pred-ledge.")
                  : t("sessions.limits.setup.ask.remove.detail", "Klíč statusLine se vrátí na původní hodnotu (nebo zmizí, když dřív nebyl). Ostatní nastavení zůstane.")
              }
              yes={ask() === "install" ? t("sessions.limits.setup.install.yes", "Zapojit") : t("sessions.limits.setup.remove.yes", "Odpojit")}
              onYes={() => run(ask()!)}
              onNo={() => setAsk(null)}
            />
          </>
        )}
    </Show>
  );
}
