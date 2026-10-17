// Osobní: odkazy z `personal.links` (web, složka, soubor, aplikace). Cíl na disku ověřuje Rust na
// pozadí (`pending` → `invalid` s důvodem); otevření jde přes index + target (`api.openLink`).
import { For } from "solid-js";
import { api, type LinkView, type Snapshot } from "../api";
import { IconApp, IconFile, IconFolder, IconLink, IconWorld, IconX } from "../icons";
import { sayError } from "../ui/toast";
import { t } from "../i18n";
import "./personal.css";

const LINK_ICON: Record<LinkView["kind"], () => any> = { web: IconWorld, folder: IconFolder, file: IconFile, app: IconApp, invalid: IconX, pending: IconLink };

async function openLink(i: number, target: string) {
  try {
    await api.openLink(i, target);
  } catch (e) {
    sayError(e);
  }
}

export default function LinksCard(p: { s: Snapshot; onSettings: () => void }) {
  return (
    <div class="card" data-anim>
      <h4><span class="ic" style="background:var(--violet)" />{t("cards.links.title", "Odkazy")} <span class="n">{p.s.personal.links.length || ""}</span></h4>
      <For each={p.s.personal.links} fallback={<div class="empty">{t("cards.links.empty", "Přidej odkazy v Nastavení — web, složku nebo aplikaci.")} <button class="mini" onClick={p.onSettings}>{t("cards.links.settings", "Nastavení")}</button></div>}>
        {(l, i) => (
          <button class="row link-row" classList={{ bad: l.kind === "invalid", wait: l.kind === "pending" }} disabled={l.kind === "invalid" || l.kind === "pending"} title={l.problem ? `${l.target}\n${l.problem}` : l.target} onClick={() => openLink(i(), l.target)}>
            <span class="lmono">{l.monogram}</span>
            <div class="grow">
              <div class="name">{l.name || l.target}</div>
              <div class="sub">{l.problem ?? (l.kind === "pending" ? t("cards.links.checking", "ověřuji…") : l.target)}</div>
            </div>
            <span class="lkind">{LINK_ICON[l.kind]()}</span>
          </button>
        )}
      </For>
    </div>
  );
}
