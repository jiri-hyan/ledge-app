// Upozornění „Ledge běží v bublině MSIX“ + tlačítko Restartovat mimo (panel Šuplík → Spouštění a paměť).
// Mimo bublinu se nevykreslí nic. Běží-li servery, Rust vrátí `confirm` (s Ledge by umřely) → druhé
// potvrzení se seznamem, stejně jako u aktualizace.
import { createSignal, Show } from "solid-js";
import { t, tp } from "../i18n";
import { say, sayError } from "../ui/toast";
import { bubbleOf, type Bubble } from "./model";
import { pkgView, relaunchOutside } from "./pkgctx";
import "./pkgctx.css";

/** Stav bubliny pro ostatní místa UI (hlavička, panel aktualizace, nápověda autostartu). */
export const bubble = (): Bubble | null => bubbleOf(pkgView());

export default function BubbleNotice() {
  const [running, setRunning] = createSignal<string[] | null>(null);
  const [busy, setBusy] = createSignal(false);
  async function restart(confirmed: string[]) {
    setBusy(true);
    try {
      const out = await relaunchOutside(confirmed);
      if (out?.outcome === "confirm") setRunning(out.running);
      else if (out?.outcome === "started") say(t("settings.pkg.restarting", "Restartuji mimo balíček — Ledge za chvíli naběhne znovu"));
    } catch (e) {
      sayError(e);
    } finally {
      setBusy(false);
    }
  }
  return (
    <Show when={bubble()}>
      {(b) => (
        <div class="pkg-note" role="note">
          <p>{t("settings.pkg.notice", "Ledge běží uvnitř aplikace {app} (balíček MSIX). Nastavení, data a spouštění po startu se ukládají do její izolované kopie.", { app: b().app })}</p>
          <Show when={b().bubbleDir}>
            {(dir) => <p class="pkg-path">{t("settings.pkg.where", "Kopie v bublině:")} <code>{dir()}</code></p>}
          </Show>
          <p class="pkg-path">{t("settings.pkg.after", "Mimo bublinu Ledge použije")} <code>{b().configDir}</code> {t("settings.pkg.noCopy", "— nastavení z bubliny se samo nepřenese.")}</p>
          <Show when={running()} fallback={
            <div class="pkg-bar">
              <button class="mini" disabled={busy()} onClick={() => restart([])}>{busy() ? t("settings.pkg.busy", "Restartuji…") : t("settings.pkg.restart", "Restartovat mimo")}</button>
            </div>
          }>
            {(r) => (
              <div class="pkg-bar">
                <span class="pkg-warn">{tp("settings.pkg.stops", r().length, ["Restart zastaví {n} běžící server: {list}", "Restart zastaví {n} běžící servery: {list}", "Restart zastaví {n} běžících serverů: {list}"], { list: r().join(", ") })}</span>
                <button class="mini" disabled={busy()} onClick={() => setRunning(null)}>{t("settings.pkg.back", "Zpět")}</button>
                <button class="mini danger" disabled={busy()} onClick={() => restart(r())}>{t("settings.pkg.stopRestart", "Zastavit a restartovat")}</button>
              </div>
            )}
          </Show>
        </div>
      )}
    </Show>
  );
}
