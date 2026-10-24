// Most k jádru (Rust): `api.*` = Tauri příkazy (`invoke`), `on` = události (`listen`). Mimo Tauri
// (plain vite v prohlížeči) je obojí no-op a UI se vykreslí prázdné. Typy dat jsou v `types.ts`.
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type { AppInfo, DecideExtra, DetectedProject, FieldError, ImportPreview, PickKind, RepoCandidate } from "./types";
import type { StatusLineStatus, TimerOp, TimerState } from "./types";
import type { EditPreview, GitView } from "./types";
import type { CalendarTest, OutlookProbe } from "./types";
import type { MediaAction } from "./types";
import type { Deleted, Draft, DraftMeta, EditDraft, HooksStatus, InstallOutcome, NewNote, NoteDetail, NoteQuery, NoteView, QueryResult, SaveNote, SaveOutcome, Snapshot, View } from "./types";
import type { PinInit, PinPrefs } from "./types";
import type { FolderCandidates, PortHolder } from "./types";

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
  /** Stop → počkat na port → spustit znovu (`commands/projects.rs` `restart`). */
  restart: (id: string) => call<{ outcome: string; url?: string }>("restart", { id }),
  /** Zastaví všechno, co spustil Ledge; vrací kolik. */
  stopAllServers: () => call<number>("stop_all_servers"),
  /** Složky s repem projektu k výběru (git se ptá jen tady). */
  projectFolders: (id: string) => call<FolderCandidates>("project_folders", { id }),
  /** Cizí procesy na portech projektů (jen ukázat; ukončit po potvrzení). */
  portHolders: () => call<PortHolder[]>("port_holders"),
  killPortHolder: (port: number, pid: number) => call("kill_port_holder", { port, pid }),
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
  /** Náhled po najetí (Compact / Island) → celý šuplík: klik do náhledu nebo jeho setrvání. Fokus nebere. */
  expandDrawer: () => call("expand_drawer"),
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
  /** Ctrl+K: poznámky s dotazem v titulku / těle + úryvek (min. 2 znaky). */
  noteSearch: (query: string, limit?: number) => call<(NoteView & { snippet: string })[]>("note_search", { query, limit }),
  /** Čas v textu („zítra v 10“) → nabídka připomínky `{ label, iso }`, jinak null. */
  noteSuggestRemind: (text: string) => call<{ label: string; iso: string } | null>("note_suggest_remind", { text }),
  noteRead: (path: string) => call<NoteDetail>("note_read", { path }),
  /** text poznámky do schránky (přes jádro — šuplík nemá fokus) */
  noteCopyText: (path: string) => call("note_copy_text", { path }),
  noteSave: (note: SaveNote, copy = false) => call<SaveOutcome>("note_save", { note, copy }),
  noteArchive: (path: string, keepPin = false) => call<string>("note_archive", { path, keepPin }),
  noteUnarchive: (path: string) => call<string>("note_unarchive", { path }),
  noteDelete: (path: string, keepPin = false) => call<Deleted>("note_delete", { path, keepPin }),
  noteDeleteUndo: (id: string) => call<string>("note_delete_undo", { id }),
  noteToggleTask: (path: string, line: number, checked: boolean, expectedHash: string) => call<SaveOutcome>("note_toggle_task", { path, line, checked, expectedHash }),
  noteRender: (body: string) => call<string>("note_render", { body }),
  /** Export poznámky do souboru (dialog Uložit jako): vrací uloženou cestu, null = uživatel zrušil. */
  noteExport: (path: string, format: "md" | "html" | "pdf") => call<string | null>("note_export", { path, format }),
  /** Aplikace registrované ve Windows pro `.md` (nabídka „Otevřít v …“). */
  noteOpenWithApps: () => call<{ id: string; name: string }[]>("note_open_with_apps"),
  /** Otevře poznámku v aplikaci `app` (id z `noteOpenWithApps`); chyba = český text z jádra. */
  noteOpenWith: (path: string, app: string) => call("note_open_with", { path, app }),
  noteEditDraftSave: (draft: EditDraft | null) => call("note_edit_draft_save", { draft }),
  noteEditDraftLoad: () => call<EditDraft | null>("note_edit_draft_load"),
  notePrefill: () => call<string | null>("note_prefill"),
  noteDraftLoad: () => call<Draft | null>("note_draft_load"),
  noteDraftSave: (meta: DraftMeta) => call("note_draft_save", { meta }),
  noteDraftImages: (images: string[]) => call("note_draft_images", { images }),
  noteDraftClear: () => call("note_draft_clear"),
  /** Synchronizace poznámek hned — počká na doběhnutí, vrátí krátký výsledek (chyba = text). */
  notesSyncNow: () => call<string>("notes_sync_now"),
  /** propojení se složkou: „Synchronizovat teď“ (podle uloženého nastavení), kontrola složky, „Rozumím“ u konfliktů */
  noteFolderSyncNow: () => call<string>("note_folder_sync_now"),
  noteFolderCheck: (path: string) => call<{ n: number; warn?: string | null }>("note_folder_check", { path }),
  noteFolderClearConflicts: () => call("note_folder_clear_conflicts"),
  // --- lístky na plochu (Rust `pins/`, `commands/pins.rs`) ---
  /** Šuplík: připnout poznámku na plochu / odepnout; true = je připnutá. */
  pinToggle: (path: string) => call<boolean>("pin_toggle", { path }),
  /** Z okna lístku: otevře šuplík na editoru téhle poznámky (Rust pošle šuplíku `edit-note`). */
  pinEditInLedge: (path: string) => call("pin_edit_in_ledge", { path }),
  /** Šuplík / paleta: nová poznámka hned jako lístek na ploše (s fokusem). Vrací cestu. */
  pinNew: () => call<string>("pin_new"),
  /** Průchozí režim u všech lístků přepnout; null = žádný lístek není na ploše. */
  pinThroughAll: () => call<boolean | null>("pin_through_all"),
  /** Okno lístku: start (poznámka, volby, zkratka). Rust pozná lístek podle volajícího okna. */
  pinInit: () => call<PinInit>("pin_init"),
  pinSet: (patch: Partial<PinPrefs>) => call<PinPrefs>("pin_set", { patch }),
  /** Okno lístku: tah za lištu (zamčený / průchozí se nehne). */
  pinDrag: () => call("pin_drag"),
  pinUnpin: () => call("pin_unpin"),
  pinHintSeen: () => call("pin_hint_seen"),
  takePage: () => call<string | null>("take_page"),
  /** Rust ověří, že na `index` je pořád tenhle `target` (config se mezitím mohl změnit). */
  openLink: (index: number, target: string) => call("open_link", { index, target }),
  updateCheckNow: () => call("update_check_now"),
  /** `confirmed` = běžící servery, které šéf odsouhlasil zastavit; běží-li jiné, vrátí `confirm`. */
  updateInstall: (confirmed: string[]) => call<InstallOutcome>("update_install", { confirmed }),
  /** „Zkusit znovu“ po selhaném tichém stažení (výsledek ve snapshotu `update.fetch`). */
  updateRetry: () => call("update_retry"),
  /** „Co je nového“ přečteno — upozornění v šuplíku zmizí. */
  updateNotesRead: () => call("update_notes_read"),
  /** Návrat k předchozí verzi; stejné potvrzení běžících serverů jako `updateInstall`. */
  updateRollback: (confirmed: string[]) => call<InstallOutcome>("update_rollback", { confirmed }),
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
  /** Export útrat a limitů do CSV / Markdownu (dialog Uložit jako v jádře); vrací cestu, `null` = zrušeno. */
  exportUsage: (format: "csv" | "md") => call<string | null>("export_usage", { format }),
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
  /** ztlumit / zapnout výchozí mikrofon; vrací nový stav */
  volumeMicMute: () => call<boolean>("volume_mic_mute"),
  /** přepnout výchozí výstup na zařízení z `VolumeView.devices` */
  volumeSetDevice: (id: string) => call("volume_set_device", { id }),
  /** Kalendář: jedno zkušební stažení odkazu ICS (nic se neukládá); chyba = lidská věta bez adresy. */
  calendarTest: (url: string) => call<CalendarTest>("calendar_test", { url }),
  /** Kalendář: Připojit ke schůzce — Rust otevře jen ověřený odkaz Teams/Meet/Zoom/Webex/Jitsi. */
  calendarJoin: (url: string) => call("calendar_join", { url }),
  /** Kalendář: je tu klasický Outlook (registr, nic nespouští) a běží? */
  calendarOutlookProbe: () => call<OutlookProbe>("calendar_outlook_probe"),
  /** Kalendář: výslovné „Načíst z Outlooku“ (smí Outlook spustit na pozadí); počet nadcházejících výskytů. */
  calendarOutlookLoad: () => call<number>("calendar_outlook_load"),
  // --- settings ---
  /** Ověří rozpracovaný config (nic nezapisuje): první chyba s polem, `null` = v pořádku. */
  checkConfig: (cfg: any) => call<FieldError | null>("check_config", { cfg }),
  configDefaults: () => call<any>("config_defaults"),
  configExport: (path: string) => call("config_export", { path }),
  configImportPreview: (path: string) => call<ImportPreview>("config_import_preview", { path }),
  /** Nativní dialog nad šuplíkem; zrušení = null. */
  pickPath: (kind: PickKind, title?: string, start?: string) => call<string | null>("pick_path", { kind, title, start }),
  /** katalog Launcheru (`catalog.json` vybraný v dialogu) jako JSON — import zástupců */
  launcherCatalog: (path: string) => call<unknown>("launcher_catalog_read", { path }),
  /** pojistka spouštění (`runguard`) pro zástupce z katalogu: `problem` = proč se importuje vypnutý */
  launcherImportCheck: (items: { id: string; path?: string; run?: string; exe?: string; args?: string[] }[]) =>
    call<{ id: string; problem: string | null }[]>("launcher_import_check", { items }),
  /** Vzhled zástupce: vybraný obrázek → zmenšený do cache → klíč (`tilelook.rs`); data URL podle klíče. */
  lookImageAdd: (path: string) => call<string>("look_image_add", { path }),
  lookImage: (key: string) => call<string | null>("look_image", { key }),
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
