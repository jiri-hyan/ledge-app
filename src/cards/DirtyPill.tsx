// Chip „N neuloženo · M nepushnuto“ v hlavičce šuplíku (zadání týden 43, bod 1). Klik = menu (`ui/Popover`
// `PopMenu`): u každého repa větev a stav, „Ztišit u tohoto repa“ (config `projects[].muteDirty`, zapíše hned
// přes `saveProjectFields`) a Terminál; dole „Přečteno“ — zvýraznění zhasne, dokud se počty nezmění (otisk
// `gitAlerts.read` v configu, přežije restart). Ztišená repa se nepočítají (ani v proužku, `stripview.rs`),
// dlaždice a karta Repa jejich stav ukazují dál; vypínač všeho `gitAlerts.enabled` (Nastavení → Projekty).
// Logika je `projects/dirty.ts`.
import { createEffect, createSignal, For, Show } from "solid-js";
import { api, type Snapshot } from "../api";
import { t } from "../i18n";
import { rich } from "../ui/rich";
import { createMenu, PopCap, PopItem, PopMenu, PopSep } from "../ui/Popover";
import { dirtySummary, isAcked, sigHash } from "../projects/dirty";
import { saveProjectFields } from "../projects/state";
import { sayError } from "../ui/toast";

/** otisk „Přečteno“ hned po kliku (než přijde snapshot s uloženým configem); null = podle snapshotu */
const [localRead, setLocalRead] = createSignal<string | null>(null);

/** Zapsat otisk do configu (`gitAlerts.read`; prázdný = smazat klíč). */
async function saveRead(read: string) {
  setLocalRead(read);
  try {
    const cfg = await api.getConfig();
    if (!cfg) return;
    const ga = { ...(cfg.gitAlerts ?? {}) };
    if (read) ga.read = read;
    else delete ga.read;
    await api.saveConfig({ ...cfg, gitAlerts: ga });
  } catch (e) {
    setLocalRead(null);
    sayError(e);
  }
}

export default function DirtyPill(p: { s: Snapshot }) {
  const menu = createMenu();
  const sum = () => dirtySummary(p.s.projects);
  const acked = () => isAcked(sum().sig, localRead() ?? p.s.gitAlerts?.read);
  // uložený otisk dorazil snapshotem → místní už není potřeba
  createEffect(() => localRead() !== null && (p.s.gitAlerts?.read ?? "") === localRead() && setLocalRead(null));
  const state = (pr: { git?: { branch?: string | null; dirty: number; ahead: number } | null }) =>
    [
      pr.git?.branch ?? t("cards.head.dirtyDetached", "bez větve"),
      (pr.git?.dirty ?? 0) > 0 ? t("cards.repos.dirty", "●{n} neuloženo", { n: pr.git!.dirty }) : "",
      (pr.git?.ahead ?? 0) > 0 ? t("cards.repos.ahead", "↑{n} nepushnuto", { n: pr.git!.ahead }) : "",
    ]
      .filter(Boolean)
      .join(" · ");
  return (
    <Show when={p.s.gitAlerts?.enabled !== false && sum().dirty + sum().unpushed > 0}>
      <button
        class={`chip pz-pill ${acked() ? "dim" : "amber"}`}
        aria-haspopup="menu"
        aria-expanded={menu.open()}
        title={t("cards.head.dirtyTitle", "Neuložené a nepushnuté změny v repech — klik = přehled, Přečteno nebo Ztišit")}
        onClick={menu.toggle}
      >
        <span class="d" />
        {rich(t("cards.head.dirty", "<b>{n}</b> neuloženo", { n: sum().dirty }))}
        {sum().unpushed ? t("cards.head.unpushed", " · {n} nepushnuto", { n: sum().unpushed }) : ""}
        <span class="pz-caret">▾</span>
      </button>
      <Show when={menu.at()}>
        {(at) => (
          <PopMenu anchor={at().anchor} keyboard={at().keyboard} trigger={menu.trigger()} onClose={menu.close} label={t("cards.head.dirtyAria", "Neuložené změny v repech")}>
            <For each={sum().repos}>
              {(pr) => (
                <>
                  <PopCap>
                    {pr.name} <small>· {state(pr)}</small>
                  </PopCap>
                  <PopItem
                    checked={!!pr.muteDirty}
                    title={pr.muteDirty ? t("cards.repos.unmuteTitle", "Znovu upozorňovat v proužku a v hlavičce") : t("cards.repos.muteTitle", "Nezobrazovat v proužku a v hlavičce šuplíku (dlaždice stav ukáže dál)")}
                    onClick={() => void saveProjectFields(pr.id, { muteDirty: !pr.muteDirty })}
                  >
                    {t("cards.head.dirtyMute", "Ztišit u tohoto repa")}
                  </PopItem>
                  <PopItem onClick={() => { menu.close(); void api.openIn(pr.id, "terminal"); }}>{t("cards.repos.terminal", "Terminál")}</PopItem>
                </>
              )}
            </For>
            <PopSep />
            <PopItem
              tone={acked() ? "undo" : undefined}
              onClick={() => {
                void saveRead(acked() ? "" : sigHash(sum().sig));
                menu.close();
              }}
            >
              {acked() ? t("cards.head.dirtyUnack", "Vrátit zvýraznění") : t("cards.head.dirtyAck", "✓ Přečteno (do další změny)")}
            </PopItem>
          </PopMenu>
        )}
      </Show>
    </Show>
  );
}
