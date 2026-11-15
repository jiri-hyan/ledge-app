# Codemapa: Poslat do propojené aplikace (`connector`)

Poznámku (nebo její `- [ ]` body) jedním klikem do propojené aplikace přes její MCP server: jako poznámku, nebo jako úkoly s řešitelem a prioritou. Ledge je tu klient (JSON-RPC `tools/call` přes HTTPS, Bearer klíč); opačný směr (Ledge jako server pro Clauda) je `mcp.md`. Adresa (`connector.url`), adresa webu (`connector.appUrl`) i zobrazovaný název (`connector.name`) jsou v configu, výchozí prázdné = funkce nenastavená. Nic nejde ven bez Odeslat v náhledu.

| soubor | účel | klíčové symboly |
|---|---|---|
| `src-tauri/src/connector/mod.rs` | config `connector` (`url`, `appUrl`, `name`; prázdné výchozí), kontrola adresy (https, http jen loopback), `endpoint()` / `app_base()` vrací `Option` | `ConnectorConfig` `url_ok` `validate` |
| `connector/plan.rs` | ČISTÁ logika: poznámka → plán. Body `- [ ]` → úkoly (jinak 1 poznámka), řešitel („řešitel Jméno Příjmení“, `@jmeno`, `assignee`) proti členům týmu bez diakritiky a celými slovy (`Ambiguous` / `Unknown` se nehádá), priorita `!` / `!!` / `!N`, idempotenční `request_id(path,nth,text)` (hash normalizované cesty + textu bez `⏰` + pořadí stejného textu; NE řádek), `sent` podle značky `SENT_MARK` | `plan` `Plan` `Item` `Assignee` `Mode` `request_id` |
| `connector/client.rs` | JSON-RPC `tools/call` (`accounts::http::post_json`), parsování odpovědi / chyb (JSON i SSE, klíč se z textu maže), argumenty úkolu / poznámky, odeslání po bodech (401/403/429 smyčku zastaví) | `call` `send` `parse_reply` `task_args` `note_args` `list_members` `Outcome` `task_link` |
| `connector/key.rs` | klíč (16–256 znaků bez bílých znaků) ve Správci pověření (`Ledge/connector`, vzor `briefrun/key.rs`), nikdy v configu / logu / hlášce | `get` `set` `clear` `valid` |
| `connector/link.rs` | odkaz zpět do poznámky: pod bodem (a jeho snímky) řádek `↳ [{název}](url)` (bez názvu `↳ [Úkol](url)`, bez adresy `↳ [{název}] id`); poznámka jako celek `connector: <id>` ve frontmatteru; bod se hledá podle TEXTU (`Outcome::text`), nenalezený se vrací v `missed` a příkaz ohlásí `link_error` | `add_links` `link_line` `connector_id` |
| `connector/tests.rs` | tabulky plánu (CZ), klient proti lokálnímu falešnému serveru (`tiny_http`), odkazy, starý config s neznámou sekcí | `fake_server` |
| `src-tauri/src/commands/connector.rs` | Tauri příkazy: `connector_state` (`hasKey`, `host`, `name`, `configured`), klíč, `connector_preview` (čte jen členy; vrací i `name`), `connector_send` (kontrola otisku poznámky, odeslání, zápis odkazů přes `notes::ops::rewrite`) | `connector_state` `connector_set_key` `connector_clear_key` `connector_preview` `connector_send` |
| `src/connector/api.ts` · `model.ts` | IPC kontrakt · čisté pomocné (předvolba řádků, souhrn; test `model.test.mjs`) | `connectorPreview` `connectorSend` · `defaultLines` `summarize` |
| `src/connector/SendDialog.tsx` · `connector.css` | náhled odeslání v plovoucím panelu (`ui/Popover`): seznam bodů s řešitelem / prioritou, přepnutí Úkoly / Celá poznámka, Odeslat | `SendDialog` |
| `src/connector/ConnectorKey.tsx` · `en.ts` | pole klíče v Nastavení → Claude (uložit / smazat, nikdy zobrazit) · angličtina | `ConnectorKey` |
| `src/types/connector.ts` | zrcadlo configu `connector` | `ConnectorConfig` `CONNECTOR_DEFAULT` |

Napojení mimo oblast (jednořádkové): `commands/calendar.rs` `calendar_add` (cíl `"connector"`) znovu používá `endpoint` / `need_key` a `client::call`; položka „Poslat do {název}“ v `src/notes/NoteMenu.tsx`; sekce Propojená aplikace (klíč, adresy, název) v `src/mcp/McpSettings.tsx`; `notes::ops::rewrite`; `accounts::http::post_json`; `briefrun::key::decode_with`.

## Invarianty
- Klíč jen ve Správci pověření; testy používají `MemVault` a falešný server na `127.0.0.1`.
- Nenastavená adresa (`configured=false`) = hláška „Nejdřív nastav adresu propojené aplikace: Nastavení → Claude.“, nic se nevolá.
- Řešitele nikdy nehádat: nejednoznačný / neznámý bod je v náhledu odškrtnutý a řekne proč.
- Opakovaný klik nezdvojí úkol: `clientRequestId` + řádek začínající `↳ [` pod bodem (`SENT_MARK`, jedna značka v Rustu i TS; pozná starý i nový tvar).
- `url` z odpovědi je jen dotaz; absolutní odkaz = `connector.appUrl` + dotaz a jen když dotaz projde bezpečným tvarem (`task_link`); bez `appUrl` řádek bez odkazu.
- Čas dne a termín se neposílají — řeší `notes/when.rs` / budoucí fáze.
