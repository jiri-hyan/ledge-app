// Typy a příkazy karet GitHub a Systém (Rust `github/`, `sysmon/`, `commands/integrations.rs`). Vlastní
// soubor, ať se nesahá do společných `types.ts` / `api.ts`. Mimo Tauri (plain vite) vrací příkazy `undefined`.
import { invoke } from "@tauri-apps/api/core";
import { demoCall, inTauri } from "../api";
import type { Ci, NetUnit } from "./format";

export type Disk = { letter: string; total: number; free: number; system: boolean };

/** Rust `sysmon::SystemView`. Rychlosti jsou v bajtech za sekundu; `null` = čeká se na druhý vzorek. */
export interface SystemView {
  atMs: number;
  cpu: number | null;
  memUsed: number;
  memTotal: number;
  memPct: number;
  disks: Disk[];
  rx: number | null;
  tx: number | null;
  cpuHist: number[];
  memHist: number[];
  rxHist: number[];
  txHist: number[];
  netUnit: NetUnit;
  /** písmena všech pevných disků (`disks` je podle configu vyfiltrované) */
  allDisks: string[];
}

export type GithubStatus = "off" | "noGh" | "login" | "ok" | "error";

/** Rust `github::parse::Pr`. */
export interface GithubPr {
  repo: string;
  number: number;
  title: string;
  /** adresa na github.com; prázdná = neotevře se */
  url: string;
  author: string;
  draft: boolean;
  updated: string;
  ci: Ci;
  /** `APPROVED` / `CHANGES_REQUESTED` / `REVIEW_REQUIRED` nebo prázdné */
  decision: string;
}

/** Rust `github::parse::Notif`. */
export interface GithubNotif {
  title: string;
  repo: string;
  kind: string;
  reason: string;
  url: string;
  updated: string;
}

/** Rust `github::GithubView`. */
export interface GithubView {
  status: GithubStatus;
  error: string | null;
  reviewOn: boolean;
  mineOn: boolean;
  notifsOn: boolean;
  review: GithubPr[];
  reviewCount: number;
  reviewMore: boolean;
  mine: GithubPr[];
  mineCount: number;
  notifs: GithubNotif[];
  notifCount: number;
  notifMore: boolean;
  fetchedMs: number | null;
  loading: boolean;
  /** za kolik ms smí karta znovu zavolat `githubView` */
  nextInMs: number;
}

/** Sekce `system` v configu (Rust `sysmon::conf::SystemConfig`). */
export interface SystemConfig {
  disks: string[];
  netUnit: NetUnit;
}

/** Sekce `github` v configu (Rust `github::conf::GithubConfig`). */
export interface GithubConfig {
  enabled: boolean;
  review: boolean;
  mine: boolean;
  notifications: boolean;
  refreshMin: number;
}

const call = <T>(cmd: string, args?: Record<string, unknown>): Promise<T | undefined> => (inTauri ? invoke<T>(cmd, args) : Promise.resolve(demoCall<T>(cmd, args)));

export const intApi = {
  /** Jeden vzorek systému (Rust ho vezme, nejvýš 1× za 1,5 s). */
  systemView: () => call<SystemView | null>("system_view"),
  /** Otevře Správce úloh a zavře šuplík. */
  systemTaskmgr: () => call("system_taskmgr"),
  /** Data GitHubu; obnoví je jen, když to odstup dovolí (`force` = tlačítko Obnovit). Blokuje až ~15 s. */
  githubView: (force = false) => call<GithubView>("github_view", { force }),
  githubOpen: (url: string) => call("github_open", { url }),
  /** Zkopíruje `gh auth login` do schránky (Rust — šuplík nemá fokus). */
  githubCopyLogin: () => call("github_copy_login"),
};
