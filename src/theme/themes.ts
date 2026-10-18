// Barevné motivy jako sady hodnot CSS tokenů (`styles.css` :root). Každý motiv definuje VŠECHNY
// klíče z `COLOR_KEYS` (aby nic neprosáklo z jiného motivu); „dark“ = hodnoty z `styles.css` (hlídá
// `theme.test.mjs`, i kontrast textů ≥ 4.5:1). Čisté údaje bez DOM — aplikuje `apply.ts`.

import type { ThemeId } from "../types.ts";

export type { ThemeId };

export const COLOR_KEYS = [
  "bg", "card", "card-2", "card-3", "well", "well-2", "pop", "raised", "line", "line-2", "line-soft", "thumb",
  "shell-edge", "shell-side", "spot", "pill-n", "switch-off", "logo-plate", "logo-shadow", "code-bg", "scrim",
  "t0", "t1", "t2", "t3", "on-light", "pri-hover",
  "green", "amber", "red", "blue", "violet", "teal", "amber-ink", "red-ink",
  "accent", "accent-2", "glow-1", "glow-2", "focus",
  "shadow-pop", "shadow-toast", "ring-logo",
] as const;
export type ColorKey = (typeof COLOR_KEYS)[number];

export type ThemeDef = {
  id: ThemeId;
  /** klíč a česká podoba názvu pro `t()` */
  key: string;
  cs: string;
  light: boolean;
  vars: Record<ColorKey, string>;
};

/** Neutrální vrstvy pro tmavé motivy (bílá/černá s krytím) — liší se jen plochou a texty. */
const DARK_LAYERS = {
  card: "rgba(255, 255, 255, 0.045)",
  "card-2": "rgba(255, 255, 255, 0.085)",
  "card-3": "rgba(255, 255, 255, 0.12)",
  well: "rgba(0, 0, 0, 0.24)",
  "well-2": "rgba(0, 0, 0, 0.32)",
  line: "rgba(255, 255, 255, 0.075)",
  "line-2": "rgba(255, 255, 255, 0.14)",
  "line-soft": "rgba(255, 255, 255, 0.05)",
  thumb: "rgba(255, 255, 255, 0.15)",
  "shell-edge": "rgba(255, 255, 255, 0.06)",
  "shell-side": "rgba(255, 255, 255, 0.04)",
  spot: "rgba(255, 255, 255, 0.075)",
  "pill-n": "rgba(255, 255, 255, 0.07)",
  "switch-off": "rgba(255, 255, 255, 0.14)",
  "logo-plate": "rgba(255, 255, 255, 0.94)",
  "logo-shadow": "rgba(0, 0, 0, 0.6)",
  "code-bg": "rgba(255, 255, 255, 0.08)",
  scrim: "rgba(0, 0, 0, 0.65)",
  t0: "#fff",
  "on-light": "#111",
  "pri-hover": "#fff",
  "shadow-pop": "0 14px 34px rgba(0, 0, 0, 0.5)",
  "shadow-toast": "0 10px 28px rgba(0, 0, 0, 0.45)",
  "ring-logo": "inset 0 0 0 1px rgba(255, 255, 255, 0.14)",
  "glow-1": "16%",
  "glow-2": "13%",
} as const;

const STATUS = {
  green: "#4ade80",
  amber: "#fbbf24",
  red: "#f87171",
  blue: "#60a5fa",
  violet: "#a78bfa",
  teal: "#2dd4bf",
  "amber-ink": "#fde7a7",
  "red-ink": "#fecaca",
} as const;

export const THEMES: Record<ThemeId, ThemeDef> = {
  dark: {
    id: "dark", key: "appearance.theme.dark", cs: "Tmavý", light: false,
    vars: {
      ...DARK_LAYERS, ...STATUS,
      bg: "16 18 27", pop: "rgba(28, 31, 42, 0.98)", raised: "#262a37",
      t1: "#f4f5f9", t2: "#aeb2c2", t3: "#8d92a3",
      accent: "#a78bfa", "accent-2": "#60a5fa", focus: "#60a5fa",
    },
  },
  midnight: {
    id: "midnight", key: "appearance.theme.midnight", cs: "Půlnoc", light: false,
    vars: {
      ...DARK_LAYERS, ...STATUS,
      bg: "7 10 24", pop: "rgba(16, 21, 44, 0.98)", raised: "#18203d",
      card: "rgba(120, 140, 255, 0.05)", "card-2": "rgba(120, 140, 255, 0.09)", "card-3": "rgba(120, 140, 255, 0.13)",
      line: "rgba(140, 160, 255, 0.1)", "line-2": "rgba(140, 160, 255, 0.18)",
      t1: "#eef1ff", t2: "#aab3da", t3: "#8590bd",
      accent: "#818cf8", "accent-2": "#38bdf8", focus: "#38bdf8", "glow-1": "20%", "glow-2": "15%",
    },
  },
  graphite: {
    id: "graphite", key: "appearance.theme.graphite", cs: "Grafit", light: false,
    vars: {
      ...DARK_LAYERS, ...STATUS,
      bg: "24 24 26", pop: "rgba(36, 36, 39, 0.98)", raised: "#2c2c30",
      t1: "#f3f3f4", t2: "#b6b6bc", t3: "#98989f",
      accent: "#a1a1aa", "accent-2": "#d4d4d8", focus: "#60a5fa", "glow-1": "8%", "glow-2": "6%",
    },
  },
  nord: {
    id: "nord", key: "appearance.theme.nord", cs: "Nord", light: false,
    vars: {
      ...DARK_LAYERS,
      bg: "46 52 64", pop: "rgba(59, 66, 82, 0.98)", raised: "#434c5e",
      card: "rgba(236, 239, 244, 0.05)", "card-2": "rgba(236, 239, 244, 0.09)", "card-3": "rgba(236, 239, 244, 0.13)",
      well: "rgba(0, 0, 0, 0.18)", "well-2": "rgba(0, 0, 0, 0.26)",
      t1: "#eceff4", t2: "#d8dee9", t3: "#b3bccc",
      green: "#a3be8c", amber: "#ebcb8b", red: "#f2a2a9", blue: "#88c0d0", violet: "#cdaec8", teal: "#94bfbe",
      "amber-ink": "#f3dfb6", "red-ink": "#f6c6cb",
      "on-light": "#2e3440", "pri-hover": "#ffffff",
      accent: "#88c0d0", "accent-2": "#81a1c1", focus: "#88c0d0", "glow-1": "12%", "glow-2": "10%",
    },
  },
  light: {
    id: "light", key: "appearance.theme.light", cs: "Světlý", light: true,
    vars: {
      bg: "246 247 250", pop: "rgba(255, 255, 255, 0.98)", raised: "#ffffff",
      card: "rgba(255, 255, 255, 0.7)", "card-2": "rgba(15, 23, 42, 0.05)", "card-3": "rgba(15, 23, 42, 0.09)",
      well: "rgba(15, 23, 42, 0.05)", "well-2": "rgba(15, 23, 42, 0.07)",
      line: "rgba(15, 23, 42, 0.1)", "line-2": "rgba(15, 23, 42, 0.18)", "line-soft": "rgba(15, 23, 42, 0.07)",
      thumb: "rgba(15, 23, 42, 0.22)",
      "shell-edge": "rgba(15, 23, 42, 0.14)", "shell-side": "rgba(15, 23, 42, 0.1)",
      spot: "rgba(15, 23, 42, 0.04)", "pill-n": "rgba(15, 23, 42, 0.07)", "switch-off": "rgba(15, 23, 42, 0.2)",
      "logo-plate": "#ffffff", "logo-shadow": "rgba(15, 23, 42, 0.22)", "code-bg": "rgba(15, 23, 42, 0.06)",
      scrim: "rgba(15, 23, 42, 0.6)",
      t0: "#000000", t1: "#14161f", t2: "#434a5c", t3: "#5b6274",
      "on-light": "#ffffff", "pri-hover": "#000000",
      green: "#137638", amber: "#985707", red: "#c81e1e", blue: "#1d4ed8", violet: "#6d28d9", teal: "#0f746c",
      "amber-ink": "#7c4a03", "red-ink": "#991b1b",
      accent: "#7c3aed", "accent-2": "#2563eb", focus: "#1d4ed8", "glow-1": "9%", "glow-2": "8%",
      "shadow-pop": "0 14px 34px rgba(15, 23, 42, 0.18)", "shadow-toast": "0 10px 28px rgba(15, 23, 42, 0.16)",
      "ring-logo": "inset 0 0 0 1px rgba(15, 23, 42, 0.14)",
    },
  },
  contrast: {
    id: "contrast", key: "appearance.theme.contrast", cs: "Vysoký kontrast", light: false,
    vars: {
      ...DARK_LAYERS,
      bg: "0 0 0", pop: "rgba(0, 0, 0, 0.99)", raised: "#111111",
      card: "rgba(255, 255, 255, 0.06)", "card-2": "rgba(255, 255, 255, 0.12)", "card-3": "rgba(255, 255, 255, 0.2)",
      well: "rgba(0, 0, 0, 0.6)", "well-2": "rgba(0, 0, 0, 0.75)",
      line: "rgba(255, 255, 255, 0.45)", "line-2": "rgba(255, 255, 255, 0.7)", "line-soft": "rgba(255, 255, 255, 0.3)",
      thumb: "rgba(255, 255, 255, 0.5)", "shell-edge": "rgba(255, 255, 255, 0.6)", "shell-side": "rgba(255, 255, 255, 0.45)",
      spot: "rgba(255, 255, 255, 0.1)", "pill-n": "rgba(255, 255, 255, 0.16)", "switch-off": "rgba(255, 255, 255, 0.35)",
      t1: "#ffffff", t2: "#e8e8e8", t3: "#cfcfcf",
      green: "#5cf08f", amber: "#ffd23f", red: "#ff8f8f", blue: "#7cc4ff", violet: "#cbb8ff", teal: "#5eead4",
      "amber-ink": "#ffeeb0", "red-ink": "#ffd6d6",
      "on-light": "#000000", "pri-hover": "#ffd400",
      accent: "#ffd400", "accent-2": "#00e5ff", focus: "#ffd400", "glow-1": "6%", "glow-2": "5%",
      "ring-logo": "inset 0 0 0 1px rgba(255, 255, 255, 0.6)",
    },
  },
  // ---------- barvy design stylů (vlna 6, `styles.ts`); jdou vybrat i samostatně ----------
  terminal: {
    id: "terminal", key: "appearance.theme.terminal", cs: "Terminál", light: false,
    vars: {
      ...DARK_LAYERS,
      bg: "3 6 4", pop: "rgba(4, 12, 6, 0.98)", raised: "#0b160d",
      card: "rgba(124, 255, 138, 0.04)", "card-2": "rgba(124, 255, 138, 0.08)", "card-3": "rgba(124, 255, 138, 0.13)",
      well: "rgba(0, 0, 0, 0.4)", "well-2": "rgba(0, 0, 0, 0.55)",
      line: "rgba(124, 255, 138, 0.16)", "line-2": "rgba(124, 255, 138, 0.32)", "line-soft": "rgba(124, 255, 138, 0.1)",
      thumb: "rgba(124, 255, 138, 0.25)", "shell-edge": "rgba(124, 255, 138, 0.22)", "shell-side": "rgba(124, 255, 138, 0.14)",
      spot: "rgba(124, 255, 138, 0.06)", "pill-n": "rgba(124, 255, 138, 0.08)", "switch-off": "rgba(124, 255, 138, 0.18)",
      t0: "#eaffee", t1: "#b8ffc2", t2: "#86e094", t3: "#62c070",
      green: "#7cff8a", amber: "#ffd166", red: "#ff7b7b", blue: "#6cb6ff", violet: "#c3a6ff", teal: "#5eead4",
      "amber-ink": "#ffe9a8", "red-ink": "#ffc9c9",
      "on-light": "#031006", "pri-hover": "#e6ffea",
      accent: "#7cff8a", "accent-2": "#2dd4bf", focus: "#7cff8a", "glow-1": "7%", "glow-2": "5%",
      "ring-logo": "inset 0 0 0 1px rgba(124, 255, 138, 0.3)",
    },
  },
  paper: {
    id: "paper", key: "appearance.theme.paper", cs: "Papír", light: true,
    vars: {
      bg: "247 242 232", pop: "rgba(255, 252, 245, 0.98)", raised: "#fffdf8",
      card: "rgba(255, 253, 248, 0.75)", "card-2": "rgba(90, 60, 20, 0.05)", "card-3": "rgba(90, 60, 20, 0.09)",
      well: "rgba(90, 60, 20, 0.05)", "well-2": "rgba(90, 60, 20, 0.07)",
      line: "rgba(90, 60, 20, 0.12)", "line-2": "rgba(90, 60, 20, 0.2)", "line-soft": "rgba(90, 60, 20, 0.08)",
      thumb: "rgba(90, 60, 20, 0.25)", "shell-edge": "rgba(90, 60, 20, 0.16)", "shell-side": "rgba(90, 60, 20, 0.12)",
      spot: "rgba(90, 60, 20, 0.04)", "pill-n": "rgba(90, 60, 20, 0.07)", "switch-off": "rgba(90, 60, 20, 0.22)",
      "logo-plate": "#ffffff", "logo-shadow": "rgba(90, 60, 20, 0.22)", "code-bg": "rgba(90, 60, 20, 0.06)",
      scrim: "rgba(40, 30, 15, 0.55)",
      t0: "#000000", t1: "#2b2419", t2: "#54483a", t3: "#6a5d4c",
      "on-light": "#ffffff", "pri-hover": "#000000",
      green: "#116a32", amber: "#8a4f06", red: "#b91c1c", blue: "#1d4ed8", violet: "#6d28d9", teal: "#0c6660",
      "amber-ink": "#7c4a03", "red-ink": "#991b1b",
      accent: "#b5651d", "accent-2": "#8c6d46", focus: "#1d4ed8", "glow-1": "7%", "glow-2": "5%",
      "shadow-pop": "0 14px 34px rgba(60, 40, 10, 0.18)", "shadow-toast": "0 10px 28px rgba(60, 40, 10, 0.16)",
      "ring-logo": "inset 0 0 0 1px rgba(90, 60, 20, 0.16)",
    },
  },
  neon: {
    id: "neon", key: "appearance.theme.neon", cs: "Neon", light: false,
    vars: {
      ...DARK_LAYERS, ...STATUS,
      bg: "13 5 28", pop: "rgba(24, 10, 48, 0.98)", raised: "#1d0d38",
      card: "rgba(170, 90, 255, 0.07)", "card-2": "rgba(170, 90, 255, 0.12)", "card-3": "rgba(170, 90, 255, 0.17)",
      line: "rgba(255, 80, 220, 0.18)", "line-2": "rgba(255, 80, 220, 0.32)", "line-soft": "rgba(255, 80, 220, 0.1)",
      "shell-edge": "rgba(255, 63, 216, 0.3)", "shell-side": "rgba(122, 92, 255, 0.22)",
      t0: "#ffffff", t1: "#f7ecff", t2: "#d4b3ff", t3: "#b896f0",
      teal: "#00f5d4", "on-light": "#1a0630", "pri-hover": "#ffffff",
      accent: "#ff3fd8", "accent-2": "#7a5cff", focus: "#ff7bf2", "glow-1": "26%", "glow-2": "22%",
      "ring-logo": "inset 0 0 0 1px rgba(255, 63, 216, 0.3)",
    },
  },
  brutal: {
    id: "brutal", key: "appearance.theme.brutal", cs: "Brutal", light: true,
    vars: {
      bg: "250 250 247", pop: "rgba(255, 255, 255, 0.99)", raised: "#ffffff",
      card: "#ffffff", "card-2": "rgba(0, 0, 0, 0.06)", "card-3": "rgba(0, 0, 0, 0.1)",
      well: "rgba(0, 0, 0, 0.05)", "well-2": "rgba(0, 0, 0, 0.08)",
      line: "rgba(0, 0, 0, 0.85)", "line-2": "#000000", "line-soft": "rgba(0, 0, 0, 0.25)",
      thumb: "rgba(0, 0, 0, 0.5)", "shell-edge": "#000000", "shell-side": "#000000",
      spot: "rgba(0, 0, 0, 0.04)", "pill-n": "rgba(0, 0, 0, 0.08)", "switch-off": "rgba(0, 0, 0, 0.3)",
      "logo-plate": "#ffffff", "logo-shadow": "rgba(0, 0, 0, 0.5)", "code-bg": "rgba(0, 0, 0, 0.06)",
      scrim: "rgba(0, 0, 0, 0.6)",
      t0: "#000000", t1: "#000000", t2: "#262626", t3: "#404040",
      "on-light": "#ffe600", "pri-hover": "#262626",
      green: "#006b2e", amber: "#8a4f06", red: "#b3001b", blue: "#0047d6", violet: "#5b21b6", teal: "#0c6660",
      "amber-ink": "#6b3d00", "red-ink": "#8b0000",
      accent: "#ff3d7f", "accent-2": "#ffe600", focus: "#0047d6", "glow-1": "0%", "glow-2": "0%",
      "shadow-pop": "6px 6px 0 #000000", "shadow-toast": "4px 4px 0 #000000",
      "ring-logo": "inset 0 0 0 2px #000000",
    },
  },
  pastel: {
    id: "pastel", key: "appearance.theme.pastel", cs: "Pastel", light: true,
    vars: {
      bg: "233 237 245", pop: "rgba(240, 243, 249, 0.98)", raised: "#f3f5fa",
      card: "#e9edf5", "card-2": "rgba(80, 95, 140, 0.07)", "card-3": "rgba(80, 95, 140, 0.11)",
      well: "rgba(80, 95, 140, 0.07)", "well-2": "rgba(80, 95, 140, 0.1)",
      line: "rgba(80, 95, 140, 0.14)", "line-2": "rgba(80, 95, 140, 0.24)", "line-soft": "rgba(80, 95, 140, 0.09)",
      thumb: "rgba(80, 95, 140, 0.25)", "shell-edge": "rgba(80, 95, 140, 0.12)", "shell-side": "rgba(80, 95, 140, 0.08)",
      spot: "rgba(255, 255, 255, 0.35)", "pill-n": "rgba(80, 95, 140, 0.09)", "switch-off": "rgba(80, 95, 140, 0.25)",
      "logo-plate": "#ffffff", "logo-shadow": "rgba(80, 95, 140, 0.3)", "code-bg": "rgba(80, 95, 140, 0.08)",
      scrim: "rgba(30, 36, 56, 0.55)",
      t0: "#141824", t1: "#2a3142", t2: "#4a5468", t3: "#56607a",
      "on-light": "#ffffff", "pri-hover": "#141824",
      green: "#116a32", amber: "#8a4f06", red: "#b91c1c", blue: "#1d4ed8", violet: "#6d28d9", teal: "#0c6660",
      "amber-ink": "#7c4a03", "red-ink": "#991b1b",
      accent: "#8b9cff", "accent-2": "#f0a6ca", focus: "#4f5fd0", "glow-1": "12%", "glow-2": "12%",
      "shadow-pop": "0 14px 34px rgba(80, 95, 140, 0.22)", "shadow-toast": "0 10px 28px rgba(80, 95, 140, 0.2)",
      "ring-logo": "inset 0 0 0 1px rgba(80, 95, 140, 0.16)",
    },
  },
};

export const THEME_IDS = Object.keys(THEMES) as ThemeId[];
