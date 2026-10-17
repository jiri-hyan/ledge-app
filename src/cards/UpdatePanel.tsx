// Panel „Nová verze“ (otevírá chip v hlavičce): poznámky k vydání → Instalovat. Běží-li servery, Rust
// vrátí `confirm` se seznamem (Ledge je drží v Job Objectu, s ním by umřely) a chce druhé potvrzení.
import { createSignal, Show } from "solid-js";
import { api, type UpdateView } from "../api";
import { IconShield } from "../icons";
import { fmtTime } from "../format";
import { say, sayError } from "../ui/toast";
import { t, tp } from "../i18n";
import "./head.css";

export default function UpdatePanel(p: { u: UpdateView & { state: "dostupna" }; onClose: () => void }) {
  const [running, setRunning] = createSignal<string[] | null>(null);
  const [busy, setBusy] = createSignal(false);
  const published = () => {
    const d = new Date(p.u.publikovano);
    return Number.isNaN(d.getTime()) ? "" : t("cards.update.date", "{d}. {m}. {time}", { d: d.getDate(), m: d.getMonth() + 1, time: fmtTime(d) });
  };
  async function install(confirmed: string[]) {
    setBusy(true);
    try {
      const out = await api.updateInstall(confirmed);
      if (out?.outcome === "confirm") setRunning(out.running);
      else if (out?.outcome === "started") say(t("cards.update.installing", "Instaluji — Ledge se za chvíli spustí znovu"));
    } catch (e) {
      sayError(e);
    } finally {
      setBusy(false);
    }
  }
  const working = () => busy() || p.u.installing;
  return (
    <div class="card upd-panel" data-anim>
      <h4>Ledge {p.u.verze}<Show when={published()}><span class="n">{t("cards.update.published", "vydáno {date} · teď {current}", { date: published(), current: p.u.current })}</span></Show></h4>
      <pre class="upd-notes">{p.u.poznamky || t("cards.update.noNotes", "Bez poznámek k vydání.")}</pre>
      <Show when={p.u.installError}><div class="banner"><IconShield /> <span>{p.u.installError}</span></div></Show>
      <Show when={running()} fallback={
        <div class="upd-bar">
          <span class="msg">{working() ? t("cards.update.downloading", "Stahuji a ověřuji SHA256…") : t("cards.update.how", "Stáhne se přes gh, ověří SHA256 a tiše nainstaluje; Ledge se restartuje.")}</span>
          <button class="btn" onClick={p.onClose} disabled={working()}>{t("cards.update.later", "Později")}</button>
          <button class="btn pri" onClick={() => install([])} disabled={working()}>{t("cards.update.install", "Instalovat")}</button>
        </div>
      }>
        {(r) => (
          <div class="upd-bar">
            <span class="msg warn">{tp("cards.update.stops", r().length, ["Aktualizace zastaví {n} běžící server: {list}", "Aktualizace zastaví {n} běžící servery: {list}", "Aktualizace zastaví {n} běžících serverů: {list}"], { list: r().join(", ") })}</span>
            <button class="btn" onClick={() => setRunning(null)} disabled={working()}>{t("cards.update.back", "Zpět")}</button>
            <button class="btn pri" onClick={() => install(r())} disabled={working()}>{t("cards.update.stopInstall", "Zastavit a instalovat")}</button>
          </div>
        )}
      </Show>
    </div>
  );
}
