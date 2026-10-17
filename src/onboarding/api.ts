// IPC průvodce prvním spuštěním (Rust `commands::onboarding`). Mimo Tauri vrací prázdno jako `api.ts`.
import { invoke } from "@tauri-apps/api/core";
import { inTauri } from "../api";
import type { ClaudeInfo, FoundApp, FoundRepo, OnboardingApplied, OnboardingPatch, ThemeOption } from "../types";

async function call<T>(cmd: string, args?: Record<string, unknown>): Promise<T | undefined> {
  if (!inTauri) return undefined;
  return invoke<T>(cmd, args);
}

export const ob = {
  /** mělký sken obvyklých složek (+ `extra`) na git repa s návrhem projektu */
  repos: (extra: string[]) => call<FoundRepo[]>("onboarding_repos", { extra }),
  apps: () => call<FoundApp[]>("onboarding_apps"),
  /** klíče `repo:<cesta>` / `app:<exe>` → data URL (jen položky ze skenu) */
  icons: (keys: string[]) => call<(string | null)[]>("onboarding_icons", { keys }),
  claude: () => call<ClaudeInfo | null>("onboarding_claude"),
  pickFolder: () => call<string | null>("onboarding_pick_folder"),
  themes: () => call<ThemeOption[]>("onboarding_themes"),
  apply: (patch: OnboardingPatch) => call<OnboardingApplied>("onboarding_apply", { patch }),
};
