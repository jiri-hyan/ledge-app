// Skok do Nastavení odjinud (paleta, karty, hlavička): id panelu z `settings/registry.ts` (`osobni`, `akce`…; i staré id),
// id části Ledge Studia (`studio/parts.ts`) nebo kotva pole `f-<pole>` (`f-actions.add`, `f-autostart`).
// Od kola 24 je Nastavení samostatné okno Ledge Studio: `openSettings` / `gotoSettingsPanel` v jednom tiku
// sloučí na JEDNO `studio_open(cíl)`; když okno nejde vytvořit, zavolá záložní cestu šuplíku (stránka
// `settings` se Studiem, `setStudioFallback` v `App.tsx`), která si cíl přečte ze `settingsTarget`.
// V samotném okně Studia jen nastaví `settingsTarget` (Studio skočí samo). Vždy jde i událost `ledge:settings-panel`.
import { createSignal } from "solid-js";
import { api, inStudio, inTauri } from "../api";

const [settingsTarget, setSettingsTarget] = createSignal<string | null>(null);
export { settingsTarget, setSettingsTarget };

let fallback: (() => void) | null = null;
/** Šuplík: co dělat, když okno Studia nevznikne (stránka `settings` v šuplíku). */
export const setStudioFallback = (f: () => void) => void (fallback = f);

let pending: { target: string | null } | null = null;
/** Otevřít Studio (a skočit na cíl). Víc volání v jednom tiku = jedno otevření s posledním cílem. */
export function openSettings(target: string | null = null) {
  if (inStudio) return void (target && setSettingsTarget(target));
  if (pending) return void (target && (pending.target = target));
  pending = { target };
  queueMicrotask(async () => {
    const t = pending?.target ?? null;
    pending = null;
    try {
      if (!inTauri) throw new Error("mimo Tauri");
      await api.studioOpen(t);
      setSettingsTarget(null);
    } catch {
      fallback?.();
    }
  });
}

/** Skok na panel / volbu (volá se i po `onSettings()` — sloučí se s ním). */
export function gotoSettingsPanel(id: string) {
  setSettingsTarget(id);
  window.dispatchEvent(new CustomEvent("ledge:settings-panel", { detail: id }));
  openSettings(id);
}
