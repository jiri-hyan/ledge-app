// Klíč propojené aplikace v Nastavení → Claude: jde jen do Správce pověření Windows (`connector_set_key`); do configu, localStorage
// ani zpět do UI se nevrací — Ledge zná jen „uložen / chybí“. Vzor `settings/panels/ApiKey.tsx`.
import { createSignal, onMount, Show } from "solid-js";
import { t } from "../i18n";
import { say, sayError } from "../ui/toast";
import { connectorClearKey, connectorSetKey, connectorState } from "./api";

export default function ConnectorKey() {
  const [has, setHas] = createSignal<boolean | undefined>(undefined);
  const [val, setVal] = createSignal("");
  const [busy, setBusy] = createSignal(false);
  const refresh = async () => {
    try {
      setHas((await connectorState()).hasKey);
    } catch {
      setHas(undefined);
    }
  };
  onMount(refresh);
  async function save() {
    const k = val().trim();
    if (!k || busy()) return;
    setBusy(true);
    try {
      await connectorSetKey(k);
      setVal(""); // z paměti UI pryč hned
      await refresh();
      say(t("settings.connector.saved", "Klíč propojené aplikace uložen do Správce pověření Windows"));
    } catch (e) {
      sayError(e);
    } finally {
      setBusy(false);
    }
  }
  async function clear() {
    setBusy(true);
    try {
      await connectorClearKey();
      await refresh();
      say(t("settings.connector.cleared", "Klíč propojené aplikace smazán"));
    } catch (e) {
      sayError(e);
    } finally {
      setBusy(false);
    }
  }
  return (
    <>
      <div class="st-status">
        <span class="st-dot" classList={{ ok: has() === true, warn: has() === false }} />
        <span class="grow">{has() === true ? t("settings.connector.have", "Uložen") : has() === false ? t("settings.connector.missing", "Chybí") : t("settings.connector.unknown", "Stav neznámý")}</span>
        <Show when={has() === true}>
          <button class="mini" disabled={busy()} onClick={clear}>{t("settings.connector.clear", "Smazat")}</button>
        </Show>
      </div>
      <div class="st-inline">
        <input
          type="password"
          class="st-text mono grow"
          aria-label={t("settings.f.connector.key", "Klíč propojené aplikace")}
          placeholder={has() ? t("settings.connector.replace", "Nový klíč (nahradí uložený)") : "…"}
          autocomplete="off"
          spellcheck={false}
          value={val()}
          onInput={(e) => setVal(e.currentTarget.value)}
          onKeyDown={(e) => e.key === "Enter" && void save()}
        />
        <button class="mini ok" disabled={busy() || !val().trim()} onClick={save}>{t("settings.connector.save", "Uložit")}</button>
      </div>
    </>
  );
}
