// Karta Odkazy: odkazy = rychlé akce, které jen něco otevřou (web, okno s webem, složka, soubor —
// `actions/model.ts` `linkActions`), viditelné na téhle záložce, v pořadí seznamu akcí; ikona = favicon
// nebo ikona souboru (`ActionIcon`), stav/problém z jádra (`ActionView.problem`). Upravují se v Nastavení
// → Projekty, akce a odkazy (jeden seznam). Starý `personal.links` převádí Rust na akce při načtení
// (`linkmigrate.rs`); co převést nešlo (třeba relativní cesta), ukáže karta pod nimi postaru — cíl
// ověřuje Rust na pozadí, otevření jde přes index + target (`api.openLink`). Vlna 8 P: vlastní vzhled
// odkazu (ikona, barva, tvar, popisek — pravý klik nebo klik v režimu úprav; panel pod seznamem).
import { For, Show, type JSX } from "solid-js";
import { api, type LinkView, type Snapshot } from "../api";
import type { ActionView } from "../types";
import { IconApp, IconFile, IconFolder, IconLink, IconWorld, IconX } from "../icons";
import { sayError } from "../ui/toast";
import { t } from "../i18n";
import { gotoSettingsPanel } from "../palette/goto";
import { linkActions } from "../actions/model";
import { runAction } from "../actions/api";
import { ActionIcon } from "../actions/ui";
import { editing } from "../layout/edit";
import LookEditor from "../tilelook/LookEditor";
import { tileLabel } from "../tilelook/model";
import { lookOf, lookTarget } from "../tilelook/state";
import { actionAuto, actionLookMenu, openActionLook } from "./ActionsCard";
import "./personal.css";

const LINK_ICON: Record<LinkView["kind"], () => any> = { web: IconWorld, folder: IconFolder, file: IconFile, app: IconApp, invalid: IconX, pending: IconLink };
const KIND_ICON: Partial<Record<ActionView["kind"], () => any>> = { url: IconWorld, web: IconWorld, folder: IconFolder, file: IconFile };

async function openLink(i: number, target: string) {
  try {
    await api.openLink(i, target);
  } catch (e) {
    sayError(e);
  }
}

/** Odkaz nikdy nepotvrzuje (jen otevírá) — kdyby jádro přesto chtělo potvrzení, nespustí se. */
const noConfirm = async () => false;

export default function LinksCard(p: { s: Snapshot; mode: string; onSettings: () => void; menu?: JSX.Element }) {
  const links = () => linkActions(p.s.actions ?? [], p.mode);
  const legacy = () => p.s.personal.links;
  const count = () => links().length + legacy().length;
  /** prázdná karta → tlačítko „Přidat akci“ v Nastavení (odkaz = akce typu web) */
  const addLink = () => {
    p.onSettings();
    gotoSettingsPanel("f-actions.add");
  };
  return (
    <div class="card lnk-card" data-anim data-edit={editing() ? "" : undefined}>
      <h4><span class="ic" style="background:var(--violet)" />{t("cards.links.title", "Odkazy")} <span class="n">{count() || ""}</span>{p.menu}</h4>
      <Show when={count()} fallback={<div class="empty">{t("cards.links.empty", "Přidej odkazy v Nastavení — web, složku nebo soubor.")} <button class="mini" onClick={addLink}>{t("cards.links.add", "Přidat odkaz…")}</button></div>}>
        <For each={links()}>
          {(a) => (
            <button
              class="row link-row"
              data-tl-tile={`a:${a.id}`}
              classList={{ bad: !!a.problem }}
              title={a.problem ? `${a.target}\n${a.problem}` : a.target}
              onClick={() => (editing() ? openActionLook(a, p.s.cfgError) : void runAction(a, noConfirm))}
              onContextMenu={(e) => actionLookMenu(e, a, p.s.cfgError)}
            >
              <ActionIcon a={a} size={28} />
              <div class="grow">
                <div class="name">{tileLabel(a.label, lookOf("action", a.id, a.look))}</div>
                <div class="sub">{a.problem ?? a.target}</div>
              </div>
              <span class="lkind">{(KIND_ICON[a.kind] ?? IconLink)()}</span>
            </button>
          )}
        </For>
        <For each={legacy()}>
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
      </Show>
      <Show when={lookTarget()?.kind === "action" ? links().find((a) => a.id === lookTarget()!.id) : null} keyed>
        {(a) => <LookEditor auto={actionAuto(a)} editing={editing()} />}
      </Show>
    </div>
  );
}
