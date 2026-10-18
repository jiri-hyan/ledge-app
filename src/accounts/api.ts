// Most k jádru pro účty Microsoft / Google (`src-tauri/src/commands/accounts.rs`). Mimo Tauri no-op.
// Tajemství (tokeny, client secret) do UI nikdy nejdou — jen stav účtu.
import { invoke } from "@tauri-apps/api/core";
import { inTauri } from "../api";
import type { ProviderId } from "./model";

/** Stav účtu (Rust `accounts::session::AccountView`). */
export interface AccountView {
  provider: ProviderId;
  connected: boolean;
  connecting: boolean;
  /** přihlášení vypršelo / bylo odvoláno → přihlásit znovu */
  expired: boolean;
  account: string;
  name: string;
  scopes: string[];
  clientId: string;
  error: string | null;
}

/** Kalendář účtu Google pro výběr (Rust `calendar::google::CalendarEntry`). */
export interface GoogleCalendar {
  id: string;
  name: string;
  color: string | null;
  primary: boolean;
}

const call = <T>(cmd: string, args?: Record<string, unknown>): Promise<T | undefined> => (inTauri ? invoke<T>(cmd, args) : Promise.resolve(undefined));

export const accountsApi = {
  status: () => call<AccountView[]>("accounts_status"),
  /** otevře přihlášení v systémovém prohlížeči a čeká na něj (nejvýš 5 min) */
  connect: (providerId: ProviderId, clientId: string, clientSecret?: string, tenant?: string) => call<AccountView>("account_connect", { providerId, clientId, clientSecret, tenant }),
  cancel: (providerId: ProviderId) => call("account_cancel", { providerId }),
  disconnect: (providerId: ProviderId) => call("account_disconnect", { providerId }),
  googleCalendars: () => call<GoogleCalendar[]>("account_google_calendars"),
};
