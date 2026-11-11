// „API klíč pro zadání“ (kolo 19): klíč jde jen do Správce pověření Windows (`apikey_set`); do configu, localStorage
// ani zpět do UI se nevrací — Ledge zná jen „uložen / chybí“ (`apikey_status`). Účtuje se z něj jen session ze zadání.
import { createSignal, onMount, Show } from "solid-js";
import { api } from "../../api";
import { t } from "../../i18n";
import { say, sayError } from "../../ui/toast";
import { hasKey, refreshKey } from "../../notes/apikey";

export default function ApiKeySetup() {
  const [val, setVal] = createSignal("");
  const [busy, setBusy] = createSignal(false);
  onMount(refreshKey);
  async function save() {
    const k = val().trim();
    if (!k || busy()) return;
    setBusy(true);
    try {
      await api.apikeySet(k);
      setVal(""); // z paměti UI pryč hned
      await refreshKey();
      say(t("settings.apikey.saved", "API klíč uložen do Správce pověření Windows"));
    } catch (e) {
      sayError(e);
    } finally {
      setBusy(false);
    }
  }
  async function clear() {
    setBusy(true);
    try {
      await api.apikeyClear();
      await refreshKey();
      say(t("settings.apikey.cleared", "API klíč smazán"));
    } catch (e) {
      sayError(e);
    } finally {
      setBusy(false);
    }
  }
  return (
    <>
      <div class="st-status">
        <span class="st-dot" classList={{ ok: hasKey() === true, warn: hasKey() === false }} />
        <span class="grow">
          {hasKey() === true ? t("settings.apikey.have", "Uložen") : hasKey() === false ? t("settings.apikey.missing", "Chybí") : t("settings.apikey.unknown", "Stav neznámý")}
        </span>
        <Show when={hasKey() === true}>
          <button class="mini" disabled={busy()} onClick={clear}>{t("settings.apikey.clear", "Smazat")}</button>
        </Show>
      </div>
      <div class="st-inline">
        <input
          type="password"
          class="st-text mono grow"
          aria-label={t("settings.f.claude.apiKey", "API klíč pro zadání")}
          placeholder={hasKey() ? t("settings.apikey.replace", "Nový klíč (nahradí uložený)") : "sk-ant-…"}
          autocomplete="off"
          spellcheck={false}
          value={val()}
          onInput={(e) => setVal(e.currentTarget.value)}
          onKeyDown={(e) => e.key === "Enter" && void save()}
        />
        <button class="mini ok" disabled={busy() || !val().trim()} onClick={save}>{t("settings.apikey.save", "Uložit")}</button>
      </div>
      <small class="st-note">{t("settings.apikey.note", "Klíč se ukládá do Správce pověření Windows (ne do configu) a účtuje se z něj jen session spuštěná ze zadání. Běžné session z terminálu ho nepoužívají.")}</small>
    </>
  );
}
