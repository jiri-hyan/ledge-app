// Most k jádru (Rust): `api.*` = Tauri příkazy (`invoke`), `on` = události (`listen`). Mimo Tauri
// (plain vite v prohlížeči) je obojí no-op a UI se vykreslí prázdné. Typy dat jsou v `types.ts`.
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type { AppInfo, DecideExtra, DetectedProject, FieldError, ImportPreview, PickKind, RepoCandidate } from "./types";
import type { StatusLineStatus, TimerOp, TimerState } from "./types";
import type { EditPreview, GitView } from "./types";
import type { CalendarTest } from "./types";
import type { MediaAction } from "./types";
import type { Draft, DraftMeta, EditDraft, HooksStatus, InstallOutcome, NewNote, NoteDetail, NoteQuery, NoteView, QueryResult, SaveNote, SaveOutcome, Snapshot, View } from "./types";

export type * from "./types";

/** Mimo Tauri (plain vite v prohlížeči) neexistuje IPC — UI se pak jen vykreslí prázdné. */
export const inTauri = typeof (window as any).__TAURI_INTERNALS__ !== "undefined";

async function call<T>(cmd: string, args?: Record<string, unknown>): Promise<T | undefined> {
  if (!inTauri) return undefined;
  return invoke<T>(cmd, args);
}

export const api = {
  getState: () => call<Snapshot>("get_state"),
  launch: (id: string) => call<{ outcome: string; url?: string }>("launch", { id }),
  stop: (id: string) => call("stop", { id }),
  dismiss: (id: string) => call("dismiss", { id }),
  logs: (id: string) => call<string[]>("logs", { id }),
  decide: (id: number | null, allow: boolean, extra?: DecideExtra) => call<boolean>("decide", { id, allow, extra: extra ?? null }),
  setMode: (mode: string) => call("set_mode", { mode }),
  /** Herní režim: zapnutí zavře šuplík; vypíná se zkratkou nebo dalším spuštěním Ledge (Rust `gamemode.rs`). */
  setGameMode: (on: boolean) => call("set_game_mode", { on }),
  reportSize: (view: View, w: number, h: number) => call("report_size", { view, w, h }),
  close: () => call("close_drawer"),
  /** Jen na výslovný klik do poznámky: okno otevřené najetím aktivuje (jinak by nešlo psát). */
  focusDrawer: () => call("focus_drawer"),
  /** Odchod ze stránky poznámky: šuplík otevřený najetím zase zavře odjetí myší. */
  releaseFocus: () => call("release_focus"),
  uiReady: () => call("ui_ready"),
  animAck: (kind: "prepared" | "closed") => call("anim_ack", { kind }),
  animStats: (stats: { kind: string; frames: number; avgMs: number; worstMs: number; durationMs: number }) => call("anim_stats", { stats }),
  openUrl: (url: string) => call("open_url", { url }),
  openIn: (id: string, target: "terminal" | "folder") => call("open_in", { id, target }),
  gitPull: (id: string) => call<string>("git_pull", { id }),
  projectLogo: (id: string) => call<string | null>("project_logo", { id }),
  cloneProject: (id: string) => call<string>("clone_project", { id }),
  getConfig: () => call<any>("get_config"),
  saveConfig: (cfg: any) => call("save_config", { cfg }),
  openConfigFile: () => call("open_config_file"),
  noteCreate: (note: NewNote) => call<string>("note_create", { note }),
  noteList: (limit?: number) => call<NoteView[]>("note_list", { limit }),
  noteDone: (path: string, done: boolean) => call("note_done", { path, done }),
  noteOpen: (path: string) => call("note_open", { path }),
  noteQuery: (q: NoteQuery) => call<QueryResult>("note_query", { q }),
  noteRead: (path: string) => call<NoteDetail>("note_read", { path }),
  noteSave: (note: SaveNote, copy = false) => call<SaveOutcome>("note_save", { note, copy }),
  noteToggleTask: (path: string, line: number, checked: boolean, expectedHash: string) => call<SaveOutcome>("note_toggle_task", { path, line, checked, expectedHash }),
  noteRender: (body: string) => call<string>("note_render", { body }),
  noteEditDraftSave: (draft: EditDraft | null) => call("note_edit_draft_save", { draft }),
  noteEditDraftLoad: () => call<EditDraft | null>("note_edit_draft_load"),
  notePrefill: () => call<string | null>("note_prefill"),
  noteDraftLoad: () => call<Draft | null>("note_draft_load"),
  noteDraftSave: (meta: DraftMeta) => call("note_draft_save", { meta }),
  noteDraftImages: (images: string[]) => call("note_draft_images", { images }),
  noteDraftClear: () => call("note_draft_clear"),
  /** Synchronizace poznámek hned — počká na doběhnutí, vrátí krátký výsledek (chyba = text). */
  notesSyncNow: () => call<string>("notes_sync_now"),
  takePage: () => call<string | null>("take_page"),
  /** Rust ověří, že na `index` je pořád tenhle `target` (config se mezitím mohl změnit). */
  openLink: (index: number, target: string) => call("open_link", { index, target }),
  updateCheckNow: () => call("update_check_now"),
  /** `confirmed` = běžící servery, které šéf odsouhlasil zastavit; běží-li jiné, vrátí `confirm`. */
  updateInstall: (confirmed: string[]) => call<InstallOutcome>("update_install", { confirmed }),
  /** Klik na session: přepnout do jejího okna / skončenou obnovit v terminálu. Chyba = česky. */
  sessionOpen: (id: string) => call("session_open", { id }),
  /** Panel Git session (jen na klik; null = cwd není v gitu). Destruktivní akce Rust znovu ověří. */
  sessionGit: (id: string) => call<GitView | null>("session_git", { id }),
  sessionGitDiff: (id: string, path: string, staged: boolean) => call<EditPreview>("session_git_diff", { id, path, staged }),
  sessionGitCommit: (id: string, message: string, paths: string[]) => call<string>("session_git_commit", { id, message, paths }),
  sessionGitPush: (id: string) => call<string>("session_git_push", { id }),
  sessionGitPull: (id: string) => call<string>("session_git_pull", { id }),
  sessionGitDiscard: (id: string, path: string) => call("session_git_discard", { id, path }),
  sessionGitRemoveWorktree: (id: string) => call<string>("session_git_remove_worktree", { id }),
  sessionGitOpen: (id: string, target: "terminal" | "folder") => call("session_git_open", { id, target }),
  hooksStatus: () => call<HooksStatus>("hooks_status"),
  hooksInstall: () => call<HooksStatus>("hooks_install"),
  hooksRemove: () => call<HooksStatus>("hooks_remove"),
  statuslineStatus: () => call<StatusLineStatus>("statusline_status"),
  statuslineInstall: () => call<StatusLineStatus>("statusline_install"),
  statuslineRemove: () => call<StatusLineStatus>("statusline_remove"),
  /** Útrata Claude: přepočet indexu na pozadí (Rust ho pustí nejvýš 1× / 60 s); výsledek ve snapshotu. */
  costsRefresh: () => call("costs_refresh"),
  /** Časovač: akce (`{ op: "start", minutes: 25 }`, pause, resume, cancel, add, swStart, swPause, swReset) → nový stav. */
  timer: (op: TimerOp) => call<TimerState>("timer", { op }),
  // --- Osobní: právě hraje, hlasitost, baterie (Rust `media.rs`, `volume.rs`, `battery.rs`) ---
  /** Záložka Osobní je vidět: Rust spustí odběry událostí (líně, jen jednou). */
  personalLive: () => call("personal_live"),
  mediaControl: (action: MediaAction) => call("media_control", { action }),
  /** Obal skladby jako data URL (jednou na `coverKey`); null = není / skladba se změnila. */
  mediaCover: (key: string) => call<string | null>("media_cover", { key }),
  volumeSet: (level: number) => call("volume_set", { level }),
  volumeMute: (muted: boolean) => call("volume_mute", { muted }),
  /** Kalendář: jedno zkušební stažení odkazu ICS (nic se neukládá); chyba = lidská věta bez adresy. */
  calendarTest: (url: string) => call<CalendarTest>("calendar_test", { url }),
  // --- settings ---
  /** Ověří rozpracovaný config (nic nezapisuje): první chyba s polem, `null` = v pořádku. */
  checkConfig: (cfg: any) => call<FieldError | null>("check_config", { cfg }),
  configDefaults: () => call<any>("config_defaults"),
  configExport: (path: string) => call("config_export", { path }),
  configImportPreview: (path: string) => call<ImportPreview>("config_import_preview", { path }),
  /** Nativní dialog nad šuplíkem; zrušení = null. */
  pickPath: (kind: PickKind, title?: string, start?: string) => call<string | null>("pick_path", { kind, title, start }),
  pickSave: (name: string) => call<string | null>("pick_save", { name }),
  projectDetect: (path: string) => call<DetectedProject>("project_detect", { path }),
  projectCandidates: () => call<RepoCandidate[]>("project_candidates"),
  logoPreview: (project: any) => call<string | null>("logo_preview", { project }),
  appInfo: () => call<AppInfo>("app_info"),
  openAppFile: (which: "log" | "dir") => call("open_app_file", { which }),
};

export function on<T>(event: string, fn: (payload: T) => void): Promise<UnlistenFn> {
  if (!inTauri) return Promise.resolve(() => {});
  return listen<T>(event, (e) => fn(e.payload));
}
