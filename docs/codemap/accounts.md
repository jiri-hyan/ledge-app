# Codemapa: účty Microsoft a Google (OAuth) — kalendář, pošta

Uživatel si přináší vlastní registraci aplikace (Client ID; Ledge nemá ověřeného klienta, výchozí je
prázdné). Návod pro uživatele [`docs/ucty.md`](../ucty.md), config `accounts`: [config-personal.md](../agents/config-personal.md).

## Rust `src-tauri/src/accounts/`

| soubor | co dělá | hlavní symboly |
|---|---|---|
| `mod.rs` | poskytovatelé, minimální scopes, adresy API | `Provider`, `MS_SCOPES`, `GOOGLE_SCOPES`, `GRAPH`, `GCAL` |
| `conf.rs` | config `accounts` (Client ID, tenant, co číst, barvy, kalendáře Google) + validace | `AccountsConfig`, `MicrosoftAccount`, `GoogleAccount`, `validate`, `ms_client_ok`, `google_client_ok` |
| `pkce.rs` | PKCE S256 (RFC 7636), base64url, náhoda `BCryptGenRandom`, porovnání `state` | `fresh`, `challenge`, `b64url`, `random`, `same` |
| `oauth.rs` | čistá logika: adresy přihlášení, formuláře, odpověď tokenu, chyby lidsky, politika obnovy, maskování tajemství, rozbor přesměrování | `authorize_url`, `redirect_uri`, `code_form`, `refresh_form`, `parse_token`, `classify`, `TokenFail`, `human`, `fresh`, `next_refresh`, `retry_after`, `scrub`, `parse_callback` |
| `loopback.rs` | jednorázový server `127.0.0.1:0` (+ `[::1]` stejný port), termín 5 min, zrušení, jen shodný `state` | `bind`, `Listener::wait`, `WaitEnd`, `WAIT` |
| `vault.rs` | Správce pověření Windows (`CredWriteW/CredReadW/CredDeleteW`), záznam po dílech 2 560 B, `MemVault` pro testy | `Stored`, `Vault`, `WinCred`, `save`, `load`, `remove`, `chunks`, `target` |
| `http.rs` | reqwest bez přesměrování, 15 s, 2 MB, chyby sítě a API lidsky | `post_form`, `get`, `api_error`, `net_error` |
| `session.rs` | připojení (prohlížeč → loopback → výměna kódu → profil → Správce pověření), access token v paměti s obnovou (serializovaná na účet), odhlášení + revokace, stav pro UI | `connect`, `cancel`, `disconnect`, `access_token`, `api_get`, `connected`, `status`, `AccountView` |
| `calsync.rs` | kalendáře účtů v kartě Kalendář: politika ICS (`needs_fetch`), vlastní vlákno přes `calendar::source::Source`, zdroje jako `CalInfo` za odkazy ICS | `maybe_refresh`, `infos`, `source`, `forget`, `on_config`, `next_deadline`, `block_on` |
| `mail.rs` | pošta Microsoft: nepřečtené + 5 hlaviček, obnova (karta 2 min / proužek 5 min), karta, segment proužku | `maybe_refresh`, `view`, `MailView`, `strip_segment`, `due`, `cadence`, `parse_folder`, `parse_messages`, `web_of` |

Adaptéry kalendáře (worker Q, v oblasti kalendáře): `calendar/graph.rs` (`/me/calendarView`, UTC, celodenní
podle data, `onlineMeeting.joinUrl`, další stránky jen na Graph), `calendar/google.rs` (`events` se
`singleEvents=true`, `hangoutLink`/`conferenceData`, `calendar_list`), kontrakt `calendar/source.rs`
(`Window`, `Event` = `Occ`, `SourceInfo`, `Source`; `adapter` → `graph::adapter` / `google::adapter` pro řádky
`personal.calendars` se `source` `graph`/`google` — ty mají přednost před `calsync`). Připojit: `graph::join_of`
(`https_only` + `meeting::classify`, záloha `meeting::find` z místa) → `Occ.join`.

Napojení (jednořádkové): `config/*` (`accounts`, `sections!` `mail` vypnuté přes `layout::OPT_IN`, `validate_field`),
`layout.rs` (`CARDS` + `OPT_IN`), `stripconf.rs` (`StripItem::Mail`), `stripview.rs`, `looks/mod.rs`
(`without`), `snapshot.rs` (`PersonalView.mail`), `calendar/live.rs` (`CalInfo.account`, `view` přidá
`calsync::infos`), `settings.rs` (`on_config`), `threads/ui.rs` + `threads/emit.rs` (`maybe_refresh`,
`next_deadline`), `commands/accounts.rs`, `lib.rs`, `Cargo.toml` (`Win32_Security_Credentials`,
`Win32_Security_Cryptography`).

**Tajemství:** refresh token a client secret jen ve Správci pověření, access token jen v paměti, `Stored`
má Debug bez tajemství, `AccountView` je nenese (test), chyby přes `oauth::human` / `http::api_error`
(bez kódů a adres), log přes `oauth::scrub`. **Výkon:** nic nepolluje; síť jen po přihlášení a s viditelnou
kartou (pošta s položkou proužku 1× / 5 min), ve hře nic.

## UI `src/accounts/`

| soubor | co dělá |
|---|---|
| `AccountsSettings.tsx` | Nastavení → Osobní → Kalendáře → „Pokročilé: připojit účet Microsoft / Google“ (skládá `settings/panels/Personal.tsx`; panel Účty od kola 15 není): dlaždice, přihlášení, stav, co smí číst, Odhlásit, průvodci Azure / Google Cloud |
| `MailCard.tsx` | karta Pošta (`openAccountsSettings`) |
| `model.ts` (+ `model.test.mjs`) | Client ID (zrcadlo Rust), `grants`, výběr kalendářů, odznak |
| `api.ts` · `en.ts` · `accounts.css` | příkazy · angličtina · třídy `mail-*`, `acc-*` |

## Testy

`cargo test accounts` (PKCE vektor RFC 7636, base64url, Správce pověření přes `MemVault` vč. dílů, formuláře,
adresy, odpovědi tokenu, `classify`, lidské chyby bez kódů, `scrub`, rozbor přesměrování, loopback přes
skutečný socket, profily, pošta ze vzorku `fixtures/graph-messages.json`, politika obnovy), `cargo test
calendar::graph calendar::google` (vzorky `fixtures/graph-calendarview.json`, `google-events.json`);
`node --test src/accounts/model.test.mjs`.
