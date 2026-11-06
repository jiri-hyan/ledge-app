// Hláška o configu pod chybovým bannerem: kopie původního config.json z posledního zápisu (s cestou,
// Obnovit / Složka / Zavřít) a varování o hodnotách mimo rozsah (`claude.*`). Text varování skládá
// jádro (`config/recovery.rs`), tady jen zobrazení.
import { For, Show } from "solid-js";
import { api } from "./api";
import { t } from "./i18n";
import { IconShield } from "./icons";
import { say, sayError } from "./ui/toast";

export function ConfigNotice(p: { n: { warnings: string[]; backup: string | null; restorable?: boolean } }) {
  const restore = () => api.configRestoreBackup()?.then(() => say(t("ui.configNotice.restored", "Config vrácen z kopie."))).catch(sayError);
  return (
    <>
      <Show when={p.n.backup}>{(path) => (
        <div class="banner warn" data-anim>
          <IconShield />
          <span>{t("ui.configNotice.backup", "Ledge uschoval kopii předchozího config.json: {path}", { path: path() })}</span>
          {/* kopie, která není platný JSON, by skončila chybou parseru: tlačítko schováme (chybí pole = starší jádro) */}
          <Show when={p.n.restorable !== false}>
            <button class="mini" onClick={restore}>{t("ui.configNotice.restore", "Obnovit")}</button>
          </Show>
          <button class="mini" onClick={() => api.configBackupReveal()?.catch(sayError)}>{t("ui.configNotice.reveal", "Ukázat složku")}</button>
          <button class="mini" onClick={() => api.configBackupDismiss()?.catch(sayError)}>{t("ui.configNotice.dismiss", "Zavřít")}</button>
        </div>
      )}</Show>
      <Show when={p.n.warnings.length}>
        <div class="banner warn" data-anim>
          <IconShield />
          <span><For each={p.n.warnings}>{(w) => <div>{w}</div>}</For></span>
        </div>
      </Show>
    </>
  );
}
