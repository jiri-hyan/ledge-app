// Nastavení → Obsah karet → Kontextové návrhy: které zdroje smí krmit pruh návrhů v šuplíku (`suggest.*`) a puls
// proužku u GitHubu. Mění jen návrh Nastavení — uloží ho společná lišta.
import { Show } from "solid-js";
import { t } from "../i18n";
import { useSettings } from "../settings/ctx";
import { Toggle } from "../settings/fields/controls";
import { Row } from "../settings/fields/Row";
import { fieldLabel } from "../settings/labels";

export default function SuggestSettings() {
  const c = useSettings();
  return (
    <>
      <Row id="suggest.clipboard"><Toggle path="suggest.clipboard" label={fieldLabel("suggest.clipboard")} /></Row>
      <Show when={c.get("suggest.clipboard") && !c.get("clipboard.enabled")}>
        <p class="st-note">{t("settings.suggest.clipNote", "Historie schránky je vypnutá — zapni ji níže v „Historie schránky“, jinak tenhle zdroj nemá co číst.")}</p>
      </Show>
      <Row id="suggest.devServer"><Toggle path="suggest.devServer" label={fieldLabel("suggest.devServer")} /></Row>
      <Row id="suggest.github"><Toggle path="suggest.github" label={fieldLabel("suggest.github")} /></Row>
      <Row id="suggest.reminders"><Toggle path="suggest.reminders" label={fieldLabel("suggest.reminders")} /></Row>
      <Row id="suggest.githubPulse"><Toggle path="suggest.githubPulse" label={fieldLabel("suggest.githubPulse")} /></Row>
    </>
  );
}
