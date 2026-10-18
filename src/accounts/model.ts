// Účty Microsoft / Google — čistá logika bez importů za běhu (test `model.test.mjs`): kontrola
// Client ID (zrcadlo Rust `accounts/conf.rs`), co Ledge z účtu čte (z udělených oprávnění), výběr
// kalendářů Google, popisky pošty.

export type ProviderId = "microsoft" | "google";
export const PROVIDERS: readonly ProviderId[] = ["microsoft", "google"];

/** Nejvíc vybraných kalendářů Google (Rust `MAX_GOOGLE_CALENDARS`). */
export const MAX_GOOGLE_CALENDARS = 10;

/** Microsoft Application (client) ID = GUID. */
export function msClientOk(id: string): boolean {
  return /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i.test(id.trim());
}

/** Google Client ID končí `.apps.googleusercontent.com`. */
export function googleClientOk(id: string): boolean {
  const m = /^([A-Za-z0-9_-]{1,120})\.apps\.googleusercontent\.com$/.exec(id.trim());
  return !!m;
}

/** Client secret: jeden řádek bez mezer. */
export function secretOk(s: string): boolean {
  const t = s.trim();
  return t.length > 0 && t.length <= 200 && !/\s/.test(t);
}

/** Lze kliknout „Přihlásit“? (Rust to ověří znovu.) */
export function canConnect(p: ProviderId, clientId: string, secret: string): boolean {
  return p === "microsoft" ? msClientOk(clientId) : googleClientOk(clientId) && secretOk(secret);
}

/** Co účet smí číst — z udělených oprávnění (jak je vrátil poskytovatel). */
export type Grant = "profile" | "calendar" | "mail" | "offline";
export function grants(scopes: readonly string[]): Grant[] {
  const out = new Set<Grant>();
  for (const raw of scopes) {
    const s = raw.toLowerCase();
    if (s === "offline_access") out.add("offline");
    else if (s.endsWith("user.read") || s === "openid" || s === "email" || s === "profile") out.add("profile");
    else if (s.endsWith("calendars.read") || s.endsWith("/auth/calendar.readonly")) out.add("calendar");
    else if (s.endsWith("mail.readbasic")) out.add("mail");
  }
  return (["profile", "calendar", "mail", "offline"] as Grant[]).filter((g) => out.has(g));
}

/** Přidá / odebere kalendář z výběru (prázdný výběr = jen hlavní). Nad strop nic nepřidá. */
export function toggleCalendar(list: readonly string[], id: string): string[] {
  if (list.includes(id)) return list.filter((x) => x !== id);
  return list.length >= MAX_GOOGLE_CALENDARS ? [...list] : [...list, id];
}

/** Je kalendář vybraný? Prázdný výběr = hlavní (`primary`). */
export function calendarOn(list: readonly string[], id: string, primary: boolean): boolean {
  return list.length ? list.includes(id) : primary;
}

/** Počet nepřečtených na odznak (999+). */
export function unreadBadge(n: number | null | undefined): string {
  if (!n || n <= 0) return "";
  return n > 999 ? "999+" : String(n);
}

/** Odkaz do Outlooku na webu — jen https (Rust filtruje taky). */
export function safeLink(u: string | null | undefined): string | null {
  return u && /^https:\/\/[^\s]+$/i.test(u) ? u : null;
}
