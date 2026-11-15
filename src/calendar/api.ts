// IPC kontrakt „Do kalendáře“ (Rust `commands/calendar.rs` `calendar_add`, logika `calendar/write.rs`).
// `ics` zapíše soubor do dočasné složky Ledge a otevře ho výchozí aplikací (otevře se okno cizí aplikace — volat
// jen na výslovný klik); `connector` pošle `create_event` (jen po potvrzení; vyžaduje klíč propojené aplikace s právem
// zapisovat události). Bez nového oprávnění; stejné `uid` = stejná událost.
import { invoke } from "@tauri-apps/api/core";

export type CalAddTarget = "ics" | "connector";

export interface CalAddReq {
  title: string;
  /** `YYYY-MM-DD` (celodenní) | `YYYY-MM-DDTHH:MM` (místní čas) | ISO s offsetem */
  startIso: string;
  /** výchozí 30 */
  durMin?: number;
  /** výchozí 10, `0` = bez upozornění (celodenní ho nemá nikdy) */
  remindMin?: number;
  allDay?: boolean;
  /** semeno stabilního uid / clientRequestId (např. `CalDraft.uid`) */
  uid: string;
  target: CalAddTarget;
}

export interface CalAddOut {
  target: CalAddTarget;
  /** id události v propojené aplikaci (u `ics` null) */
  id: string | null;
  /** propojená aplikace ji už znala (opakovaný klik) */
  duplicate: boolean;
}

export const calendarAdd = (req: CalAddReq) => invoke<CalAddOut>("calendar_add", { req });
