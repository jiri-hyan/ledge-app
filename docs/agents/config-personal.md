# Konfigurace — časovač, kalendář, účty, Claude

Součást návodu pro agenty — index je v [`AGENTS.md`](../../AGENTS.md). Obsah je doslovný výřez původního AGENTS.md (odkazy typu „viz sekce …“ míří na sekce v souborech podle indexu).

- **Časovač** — sekce `timer` (Nastavení → Osobní → Časovač, `src-tauri/src/timer.rs`): `presets`
  (minuty, 1–600, nejvýš 6; výchozí `[5, 15, 25, 50]`), `toast` (výchozí `true`: oznámení Windows po
  doběhnutí odpočtu — bez fokusu, ne ve hře / herním režimu / při „nerušit“ / s otevřeným šuplíkem), `strip`
  (výchozí `true`: běžící časovač se ukáže na začátku proužku i bez položky `timer`; přepínač je i v Nastavení →
  Vzhled → Co proužek ukazuje). Odpočet (předvolby, vlastní minuty, pauza, +5 min, zrušit) a stopky (start, pauza,
  reset) v kartě Časovač na záložce Osobní a v paletě Ctrl+K („Časovač 25 min“, „Stopky“, „Zrušit
  časovač“…). Proužek ukazuje „⏱ 18 min“ (zbývá, zaokrouhleno nahoru), stopky „⏱ 0:42“ (h:mm), pauza „⏸“,
  konec „⏱ čas vypršel“ (10 min, pak zmizí); překresluje se jen po minutách, sekundy počítá otevřený šuplík.
  Konec odpočtu = modré bliknutí + puls proužku (jako upozornění Clauda) a volitelný toast; ve fullscreen
  hře nic. Stav (časy, ne tik) je v `%APPDATA%\Ledge\timer.json` — zapisuje se jen při akci, takže běžící
  odpočet přežije restart Ledge (aktualizace); doběhl-li mezitím, ukáže se jen tiše „čas vypršel“. Smazání
  souboru = nic neběží po dalším startu.
- **Kalendář (jen čtení, bez přihlášení)** — `personal.calendars` (Nastavení → Osobní → Kalendáře; kód
  `src-tauri/src/calendar/`, mapa `docs/codemap/calendar.md`): seznam `{ "name", "url", "color", "enabled", "source", "remindMin", "remindToast" }`
  (nejvýš 8). `url` = **tajná adresa ve formátu iCal** (Google: Nastavení kalendáře → Integrovat kalendář;
  Outlook: Nastavení → Kalendář → Sdílené kalendáře → Publikovat kalendář → ICS), jen `https://`
  (`webcal://` se bere jako https, `http://` neprojde validací). Adresa je tajemství: Ledge ji neloguje, v chybách
  ani v UI ji neukazuje (jen doménu), cache `%LOCALAPPDATA%\Ledge\calendar-cache.json` je klíčovaná otiskem
  SHA-256; export zálohy configu ji obsahuje (jako zbytek configu). `name` prázdné = název z feedu
  (`X-WR-CALNAME`), `color` `#rrggbb` nebo `null` (podle pořadí), `enabled` výchozí `true`. Přidání v Nastavení:
  vložit odkaz → Přidat (jedno zkušební stažení, Tauri `calendar_test`). Karta Kalendář na Osobní: Dnes (bez
  skončených), Zítra, a když obojí chybí, nejbližší další den do 14 dní; celodenní nahoře, probíhající
  zvýrazněná; patička = kalendáře (klik vypne/zapne). Stahuje se jen při otevřeném šuplíku na Osobní se
  zapnutou kartou, když jsou data starší 15 min (po pokusu 2 min pauza), odpověď nejvýš 5 MB / 15 s; v klidu
  nic. Opakování (RRULE běžné tvary, EXDATE, RECURRENCE-ID) a pásma (VTIMEZONE z feedu, běžné IANA
  i Windows názvy; neznámé = místní čas, karta ho označí „≈“).
  - **Zdroj** `source` (výchozí `"ics"` = starý config beze změny): `"outlook-local"` = výchozí kalendář
    **klasického Outlooku v tomto PC** přes COM (bez přihlášení a sítě; `url` se nepoužije, v seznamu nejvýš
    jednou). Automaticky se načte jen když Outlook běží (připojí se, žádné okno ani fokus); zavřený Outlook
    jen po kliku „Načíst z Outlooku“ (COM ho spustí na pozadí). Text pozvánky (odkaz Teams) čte jen když
    Outlook volajícímu věří (`Application.IsTrusted`) — jinak by vyskočila bezpečnostní výzva Outlooku.
    Nový Outlook COM nemá → odkaz ICS. `"graph"` / `"google"` + `account` = účty Microsoft / Google
    (`calendar/graph.rs`, `google.rs` přes `source::adapter`; přihlášení v Nastavení → Účty). Bez řádku tady
    dá kalendář účtu do karty přepínač `accounts.*.calendar` (bez připomínky); je-li zapnutý řádek téhož
    účtu tady, vede ho tenhle (připomínka, Připojit) a přepínač účtu se nezdvojí.
  - **Připojit:** z vlastností schůzky (`X-MICROSOFT-SKYPETEAMSMEETINGURL`, `X-GOOGLE-CONFERENCE`,
    `CONFERENCE`), `URL`, místa a popisu (i HTML, Safe Links) Ledge vezme odkaz Teams / Meet / Zoom / Webex /
    Jitsi a u události ukáže tlačítko Připojit (Teams rovnou v aplikaci `msteams:`, jinak https; Tauri
    `calendar_join` otevře jen ověřený odkaz známé služby).
  - **Připomínka:** `remindMin` (výchozí `5`, `0` = vypnuto, nejvýš `120`) minut před začátkem časové
    události proužek ukáže „📅 Název za 5 min“ (modře, i s vypnutým proužkem; priorita mezi „čeká na
    tebe“ a „hotovo“), blikne a pulsne; `remindToast: true` (výchozí `false`) navíc pošle toast Windows
    s tlačítkem Připojit. Budík jen na čas připomínky, nic se kvůli ní nestahuje (data z posledního
    načtení / cache). Ve fullscreen hře / herním režimu nic, toast ani při „nerušit“ nebo otevřeném šuplíku.
- **Účty Microsoft a Google (OAuth)** — sekce `accounts` (Nastavení → Účty; kód `src-tauri/src/accounts/`,
  mapa `docs/codemap/accounts.md`, návod pro uživatele `docs/ucty.md`). Výchozí = nic nepřihlášeno, žádná síť.
  ```json
  "accounts": {
    "microsoft": { "clientId": "", "tenant": "common", "calendar": true, "mail": true, "color": null },
    "google": { "clientId": "", "calendar": true, "calendars": [], "color": null }
  }
  ```
  `clientId` = Client ID **vlastní registrace aplikace** uživatele (Ledge žádné výchozí nemá — veřejná
  open-source appka bez ověřeného OAuth klienta): Microsoft = GUID z Azure Portal (veřejný klient, redirect
  `http://localhost`), Google = `….apps.googleusercontent.com` (klient „Desktop app“). `tenant` = `common` /
  `organizations` / `consumers` / ID tenantu. `calendar` = události účtu v kartě Kalendář (za odkazy ICS),
  `mail` = karta Pošta + položka proužku `mail`; `calendars` = id kalendářů Google (prázdné = hlavní, nejvýš
  10); `color` `#rrggbb` nebo `null`. **Přihlášení samo v configu není:** refresh token (a u Google „client
  secret“, který u desktopových klientů tajný není) leží ve Správci pověření Windows (`Ledge/accounts/<účet>`),
  access token jen v paměti; config, log ani chyby žádné tokeny neobsahují. Oprávnění: Microsoft `offline_access
  User.Read Calendars.Read Mail.ReadBasic`, Google `calendar.readonly` (Gmail ne — „restricted“ scopes chtějí
  audit). Stahuje se jen s viditelnou kartou (kalendář jako ICS: 15 min, pošta 2 min), termín 15 s na dotaz.
  Tauri: `accounts_status`, `account_connect`, `account_cancel`, `account_disconnect`, `account_google_calendars`.
- **Připomínka resetu limitu** — `claude.limitsReminder` (výchozí `true`): když 5h nebo týdenní limit
  Claude dosáhl 100 %, v čase resetu krátký peek (viz „Limity Claude 5h / týden“).
- **Útrata Claude (odhad)** — `claude.costs` (výchozí `true`): sekce dole v kartě Claude s tokeny a odhadem
  ceny v cenách API podle projektu a dne (viz „Útrata Claude (odhad)“); `false` = sekce zmizí, nic se nečte.
