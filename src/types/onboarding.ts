// --- onboarding ---
/** Repo z průvodce (Rust `onboarding::repos::FoundRepo`); `project` = hotový návrh pro config. */
export interface FoundRepo {
  name: string;
  path: string;
  remote: string | null;
  lastMs: number | null;
  kind: "dev" | "app" | "none";
  hint: string;
  project: Record<string, unknown> | null;
  already: boolean;
}
/** Aplikace ze Start menu (Rust `onboarding::apps::FoundApp`). */
export interface FoundApp {
  name: string;
  exe: string;
  args: string[];
  already: boolean;
}
export interface ClaudeInfo {
  exe: string | null;
  dir: boolean;
  version: string | null;
  installed: boolean;
}
export interface ThemeOption {
  value: string;
  label: string;
}
export interface OnboardingPatch {
  lang?: "cs" | "en";
  projects?: Record<string, unknown>[];
  links?: { label: string; url: string }[];
  theme?: string;
  done?: boolean;
}
export interface OnboardingApplied {
  added: string[];
  skipped: string[];
  linksAdded: number;
  linksTo: "actions" | "personal.links" | null;
  themeSaved: boolean | null;
}
