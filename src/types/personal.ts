import type { CalendarView } from "./calendar";
import type { BatteryView, MediaView, VolumeView } from "./media";
import type { NoteView } from "./notes";

/** Počasí z Open-Meteo (stahuje Rust, jen při otevření Osobní a datech starších 30 min). */
export interface Weather {
  /** °C */
  temp: number;
  /** WMO kód */
  code: number;
  desc: string;
  icon: string;
  isDay: boolean;
  /** dnešní min/max °C */
  min: number | null;
  max: number | null;
  /** dnešní srážky celkem (mm) a nejvyšší pravděpodobnost (%) */
  precipSum: number | null;
  precipProb: number | null;
  /** čas měření v místním čase místa */
  observed: string;
}

export interface WeatherView {
  /** unset = místo nenastavené (žádná síť) · stale = poslední dobrá data, poslední pokus selhal */
  status: "unset" | "loading" | "ok" | "stale" | "unavailable" | "idle";
  label: string;
  data: Weather | null;
  fetchedMs: number | null;
  error: string | null;
  loading: boolean;
}

export interface LinkView {
  name: string;
  target: string;
  /** pending = ověření cíle na disku ještě běží (na pozadí v Rustu) */
  kind: "web" | "folder" | "file" | "app" | "invalid" | "pending";
  monogram: string;
  /** proč nejde otevřít (jen kind=invalid) */
  problem: string | null;
}

export interface PersonalView {
  weather: WeatherView;
  /** pořadí = `personal.links` v configu (index + target pro `openLink`) */
  links: LinkView[];
  /** obecné poznámky (bez projektu): připomínky na dnes / po termínu, pak nejnovější — nejvýš 8 */
  notes: NoteView[];
  /** kolik obecných poznámek se nevešlo */
  notesMore: number;
  /** právě hraje; null = nic nehraje / karta vypnutá / odběr ještě neběží */
  media: MediaView | null;
  /** null = bez výstupního zařízení / vypnuto */
  volume: VolumeView | null;
  /** null = stroj bez baterie / vypnuto */
  battery: BatteryView | null;
  /** karta Kalendář (Rust `calendar::live::view`); starý snapshot bez pole = nic */
  calendar?: CalendarView;
  /** karta Pošta — účet Microsoft (Rust `accounts::mail::view`); starý snapshot bez pole = nic */
  mail?: MailView;
  /** sekce `media` v configu (vzhled karty); chybí jen ve starém jádře = výchozí */
  mediaLook?: import("../media/timeline").MediaLook;
}

/** Zpráva v kartě Pošta (jen hlavička — tělo Ledge nečte). */
export interface MailItem {
  subject: string;
  from: string;
  receivedMs: number;
  unread: boolean;
  /** odkaz na zprávu v Outlooku na webu (https) */
  link: string | null;
}

/** Karta Pošta (Rust `accounts::mail::MailView`). */
export interface MailView {
  status: "off" | "unset" | "idle" | "loading" | "ok" | "stale" | "unavailable";
  account: string;
  unread: number | null;
  items: MailItem[];
  fetchedMs: number | null;
  error: string | null;
  loading: boolean;
  /** kam vede „Otevřít Outlook“ */
  web: string;
}
