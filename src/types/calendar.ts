/** Výskyt události na kartě Kalendář (Rust `calendar::view::Item`). */
export interface CalendarItem {
  /** ms epochy; u celodenní půlnoc data jako UTC — čas se neukazuje */
  start: number;
  end: number;
  allDay: boolean;
  title: string;
  location: string;
  /** index v `CalendarView.calendars` (= `personal.calendars`) */
  cal: number;
  ongoing: boolean;
  /** pásmo události Ledge nezná → čas brán jako místní */
  tzGuess: boolean;
  /** odkaz Připojit (Rust `calendar::meeting::Join`); null / starý snapshot bez pole = žádný */
  join?: CalendarJoin | null;
}

/** Odkaz na schůzku (ověřený Rustem: jen https známé služby). */
export interface CalendarJoin {
  provider: "teams" | "meet" | "zoom" | "webex" | "jitsi";
  url: string;
}

/** Zdroj kalendáře (`personal.calendars[].source`, Rust `calendar::source::SourceKind`). */
export type CalendarSource = "ics" | "outlook-local" | "graph" | "google";

export interface CalendarDay {
  kind: "today" | "tomorrow" | "later";
  /** místní datum YYYY-MM-DD */
  date: string;
  items: CalendarItem[];
}

/** Jeden kalendář z configu — adresa nikdy celá, jen doména. */
export interface CalendarInfo {
  name: string;
  color: string | null;
  domain: string;
  /** starý snapshot bez pole = ics */
  source?: CalendarSource;
  /** `outlook-closed` = Outlook v PC je zavřený (karta nabídne Načíst z Outlooku) */
  hint?: "outlook-closed" | null;
  enabled: boolean;
  status: "off" | "invalid" | "ok" | "stale" | "loading" | "unavailable" | "idle";
  error: string | null;
  fetchedMs: number | null;
  events: number;
  /** zdroj z přihlášeného účtu (Nastavení → Účty); chybí = odkaz ICS z `personal.calendars` */
  account?: "microsoft" | "google";
}

export interface CalendarView {
  status: "off" | "unset" | "ok" | "stale" | "loading" | "unavailable" | "idle";
  days: CalendarDay[];
  more: number;
  calendars: CalendarInfo[];
  loading: boolean;
}

/** `calendar_outlook_probe`: klasický Outlook nainstalovaný / běží. */
export interface OutlookProbe {
  installed: boolean;
  running: boolean;
}

/** Výsledek `calendar_test` (Nastavení: přidání / Otestovat). */
export interface CalendarTest {
  events: number;
  upcoming: number;
  /** název z feedu; "" = feed ho nemá */
  name: string;
}
