// Účty: čistá logika (bez DOM) a shoda se zrcadly v Rustu.
//   node --experimental-strip-types --test src/accounts/model.test.mjs
import { test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { MAX_GOOGLE_CALENDARS, calendarOn, canConnect, googleClientOk, grants, msClientOk, safeLink, secretOk, toggleCalendar, unreadBadge } from "./model.ts";

const rust = (f) => readFileSync(new URL(`../../src-tauri/src/${f}`, import.meta.url), "utf8");

test("Client ID a secret jako v Rustu", () => {
  for (const [id, want] of [
    ["11111111-2222-3333-4444-55555555abcd", true],
    [" 11111111-2222-3333-4444-55555555ABCD ", true],
    ["1111-2222", false],
    ["{11111111-2222-3333-4444-555555555555}", false],
    ["", false],
  ]) assert.equal(msClientOk(id), want, id);
  for (const [id, want] of [
    ["123456789012-abcdefghijklmnop0123456789abcdef.apps.googleusercontent.com", true],
    [".apps.googleusercontent.com", false],
    ["123.apps.example.com", false],
    ["a b.apps.googleusercontent.com", false],
  ]) assert.equal(googleClientOk(id), want, id);
  assert.ok(secretOk("GOCSPX-abc_DEF") && !secretOk("") && !secretOk("a b"));
  assert.equal(canConnect("microsoft", "11111111-2222-3333-4444-555555555555", ""), true);
  assert.equal(canConnect("google", "1-x.apps.googleusercontent.com", ""), false, "Google bez secretu ne");
  assert.equal(canConnect("google", "1-x.apps.googleusercontent.com", "GOCSPX-x"), true);
  assert.match(rust("accounts/conf.rs"), new RegExp(`MAX_GOOGLE_CALENDARS: usize = ${MAX_GOOGLE_CALENDARS};`));
});

test("co účet smí číst z oprávnění", () => {
  assert.deepEqual(grants(["offline_access", "User.Read", "Calendars.Read", "Mail.ReadBasic"]), ["profile", "calendar", "mail", "offline"]);
  // Graph vrací scopes i s adresou prostředku
  assert.deepEqual(grants(["https://graph.microsoft.com/Calendars.Read", "https://graph.microsoft.com/User.Read"]), ["profile", "calendar"]);
  assert.deepEqual(grants(["https://www.googleapis.com/auth/calendar.readonly"]), ["calendar"]);
  assert.deepEqual(grants(["https://www.googleapis.com/auth/calendar"]), [], "plný přístup ke kalendáři Ledge nežádá ani neukazuje jako čtení");
  assert.deepEqual(grants([]), []);
  // scopes v Rustu jsou minimální a bez Gmailu
  const mod = rust("accounts/mod.rs");
  assert.match(mod, /MS_SCOPES: &str = "offline_access User\.Read Calendars\.Read Mail\.ReadBasic"/);
  assert.match(mod, /GOOGLE_SCOPES: &str = "https:\/\/www\.googleapis\.com\/auth\/calendar\.readonly"/);
  assert.doesNotMatch(mod, /gmail\./);
});

test("výběr kalendářů Google", () => {
  assert.deepEqual(toggleCalendar([], "a"), ["a"]);
  assert.deepEqual(toggleCalendar(["a", "b"], "a"), ["b"]);
  const full = Array.from({ length: MAX_GOOGLE_CALENDARS }, (_, i) => `c${i}`);
  assert.equal(toggleCalendar(full, "navic").length, MAX_GOOGLE_CALENDARS, "nad strop nic");
  assert.equal(calendarOn([], "jana@example.com", true), true, "prázdný výběr = hlavní");
  assert.equal(calendarOn([], "rodina", false), false);
  assert.equal(calendarOn(["rodina"], "jana@example.com", true), false);
});

test("pošta: odznak a odkazy", () => {
  assert.equal(unreadBadge(0), "");
  assert.equal(unreadBadge(null), "");
  assert.equal(unreadBadge(7), "7");
  assert.equal(unreadBadge(1200), "999+");
  assert.equal(safeLink("https://outlook.office365.com/owa/?ItemID=1"), "https://outlook.office365.com/owa/?ItemID=1");
  assert.equal(safeLink("javascript:alert(1)"), null);
  assert.equal(safeLink("http://x.cz"), null);
  // karta a položka proužku jsou v katalozích a výchozí NEpřidané / vypnuté
  const layout = rust("layout.rs");
  assert.match(layout, /OPT_IN: \[&str; \d+\] = \[[^\]]*"mail"/);
  assert.match(rust("stripconf.rs"), /\bMail,\n/);
  assert.doesNotMatch(rust("stripconf.rs").match(/items: vec!\[[^\]]*\]/)[0], /Mail/);
});
