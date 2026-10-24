// Typy snapshotu a IPC (zrcadlo serde struktur v `src-tauri/src`) — jen tvary dat, žádná logika.
// Volání jádra jsou v `api.ts`, který tyto typy zároveň re-exportuje (`import type { … } from "./api"`).
export * from "./actions";
export * from "./appearance";
export * from "./calendar";
export * from "./claude";
export * from "./desk";
export * from "./media";
export * from "./notes";
export * from "./onboarding";
export * from "./personal";
export * from "./projects";
export * from "./settings";
export * from "./snapshot";
export * from "./suggest";
export * from "./timer";
export * from "./widgets";
