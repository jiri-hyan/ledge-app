# Účty Microsoft a Google v Ledge

Ledge umí ukázat kalendář z Outlooku / Microsoftu 365 a z Kalendáře Google přímo v kartě Kalendář a
počet nepřečtených zpráv z Outlooku v kartě Pošta (a volitelně v proužku nahoře). Všechno je **jen
čtení**, nic se nikam neposílá a nic se v účtu nemění.

> Nechceš se přihlašovat? Kalendář jde i bez účtu — přes tajný odkaz ICS (Nastavení → Osobní →
> Kalendáře). Přihlášení přidává poštu, odkazy na online schůzky a kalendáře, které odkaz nemají.

## Proč si musíš vytvořit vlastní „registraci aplikace“

Ledge je veřejná open-source aplikace a **nemá vlastní ověřeného OAuth klienta** u Microsoftu ani
Googlu. Ověření stojí čas a u Googlu i placený bezpečnostní audit; neověřený sdílený klient by navíc
narážel na limity:

- **Google** u neověřené aplikace se scopem kalendáře ukáže varování „Google tuto aplikaci neověřil“
  a pustí nejvýš **100 testovacích uživatelů**. V režimu „Testing“ navíc přihlášení **po 7 dnech
  vyprší** (řešení níž: přepnout na „In production“).
- **Microsoft** u aplikace pro „kohokoli“ ve firmách často **blokuje správce** (vyžaduje jeho
  schválení). U osobního účtu to nevadí.
- **Gmail Ledge číst neumí:** oprávnění k poště Gmail jsou „restricted“ a vyžadují roční bezpečnostní
  audit. Proto u Google jen kalendář.

Proto si každý vytvoří svou registraci (zdarma, 5–10 minut) a vloží její **Client ID** do Ledge.
Výchozí Client ID je prázdné — Ledge žádné nevymýšlí a bez tvého nic neodesílá.

## Co Ledge s účtem dělá

| | Microsoft | Google |
|---|---|---|
| Oprávnění | `offline_access User.Read Calendars.Read Mail.ReadBasic` | `calendar.readonly` |
| Čte | jméno účtu, události kalendáře (−1 / +14 dní), počet nepřečtených v Doručené poště a hlavičky 5 nejnovějších zpráv (předmět, odesílatel, čas) | události vybraných kalendářů (−1 / +14 dní), seznam kalendářů |
| Nečte | obsah zpráv, přílohy, kontakty, soubory | Gmail, Disk, kontakty |
| Kdy | jen když je karta vidět (kalendář nejvýš 1× za 15 min, pošta 1× za 2 min); s položkou proužku Pošta i se zavřeným šuplíkem nejvýš 1× za 5 min | jen když je karta vidět (1× za 15 min) |

- Přihlášení probíhá ve **tvém prohlížeči** (ne uvnitř Ledge) — Ledge heslo nikdy nevidí. Používá
  OAuth 2.0 Authorization Code s **PKCE** a jednorázovou adresou `127.0.0.1` / `localhost` na náhodném
  portu, která po přihlášení (nebo po 5 minutách) zmizí.
- Trvalé přihlášení (refresh token) je jen ve **Správci pověření Windows** (Ovládací panely →
  Správce pověření → Pověření systému Windows → Obecná pověření → `Ledge/accounts/microsoft` nebo
  `Ledge/accounts/google`). V configu je jen Client ID. Do logu ani do chybových hlášek se token nedostane.
- **Odhlásit** (Nastavení → Účty) smaže přihlášení ze Správce pověření. U Googlu ho Ledge i odvolá.
  Microsoft odvolání pro veřejné aplikace nenabízí — přístup můžeš odebrat ručně: osobní účet na
  account.microsoft.com → Soukromí → Aplikace a služby; pracovní účet na myapps.microsoft.com.

## Microsoft (Outlook, Microsoft 365) — krok za krokem

1. Otevři **portal.azure.com** a přihlas se (stačí osobní účet Microsoft; Azure předplatné není
   potřeba). Najdi **Microsoft Entra ID → App registrations → New registration**.
2. **Name:** třeba „Ledge (moje)“.
   **Supported account types:** „Accounts in any organizational directory and personal Microsoft
   accounts“ (osobní i pracovní účet). Jen osobní účet: „Personal Microsoft accounts only“.
3. **Redirect URI:** platforma **Public client/native (mobile & desktop)**, adresa `http://localhost`
   (bez portu — u `localhost` Microsoft port nepočítá a Ledge si ho volí sám). Klikni **Register**.
   (Adresu `http://127.0.0.1` jde v portálu přidat jen úpravou manifestu, proto `localhost`.)
4. Na stránce **Overview** zkopíruj **Application (client) ID** a vlož ho v Ledge do Nastavení →
   Účty → Microsoft. Client secret **nevytvářej** — Ledge je veřejný klient.
5. **Authentication → Allow public client flows = Yes → Save.**
6. **API permissions** přidávat nemusíš — Ledge si o `User.Read`, `Calendars.Read` a `Mail.ReadBasic`
   řekne při přihlášení a ty je potvrdíš.
7. V Ledge klikni **Přihlásit v prohlížeči**, přihlas se a povol přístup. Pak **Uložit** v liště Nastavení
   (ať si Ledge Client ID pamatuje).
8. Kartu **Pošta** přidáš v Nastavení → Šuplík a karty (nebo ✎ v šuplíku → + Přidat kartu). Počet
   nepřečtených v proužku: Nastavení → Vzhled → Zavřený → Pokročilé → položka „Pošta“.

Když to nejde:
- „chybí adresa přesměrování“ → bod 3 (musí to být platforma *Mobile and desktop*, ne *Web*).
- „Microsoft nezná tohle Client ID / nemá povolené veřejné klienty“ → bod 4 a 5.
- „správce tvé organizace nepovoluje…“ → pracovní účet; požádej správce o schválení aplikace,
  nebo použij osobní účet.

## Google (Kalendář Google) — krok za krokem

1. Otevři **console.cloud.google.com** a vlevo nahoře vytvoř **nový projekt** (třeba „Ledge“).
2. **APIs & Services → Library →** vyhledej **Google Calendar API → Enable**.
3. **Google Auth Platform** (dřív „OAuth consent screen“) **→ Get started**: název aplikace, tvůj e-mail,
   **Audience: External**, kontaktní e-mail → Create. V **Data access → Add or remove scopes** přidej
   `https://www.googleapis.com/auth/calendar.readonly`.
4. **Audience → Test users → Add users →** přidej svůj Gmail (bez toho Google přihlášení odmítne).
5. **Clients → Create client →** Application type **Desktop app** → Create. Zkopíruj **Client ID**
   a **Client secret** a vlož je v Ledge do Nastavení → Účty → Google. (U desktopové aplikace secret
   tajný není — Google to tak sám popisuje —, Ledge ho i tak ukládá jen do Správce pověření.) Adresu
   přesměrování nezadáváš: desktopový klient bere `http://127.0.0.1:<port>` automaticky.
6. V Ledge klikni **Přihlásit v prohlížeči**. Google ukáže **„Google tuto aplikaci neověřil“** —
   je to tvoje vlastní aplikace: **Pokročilé → Přejít na Ledge (nebezpečné)** → povol čtení kalendáře.
   Pak **Uložit** v liště Nastavení.
7. **Ať přihlášení nevyprší za 7 dní:** v **Audience → Publishing status** klikni **Publish app**
   („In production“). Ověření podávat nemusíš (aplikace je jen pro tebe, limit 100 uživatelů platí dál),
   varování při přihlášení zůstane, ale refresh token už po týdnu nepropadne.
8. Které kalendáře se ukážou: Nastavení → Účty → Google → **Vybrat kalendáře…** (nic = jen hlavní).

## Soukromí v kostce

- Bez přihlášení Ledge na Microsoft ani Google nic neposílá.
- Data jsou jen v paměti (a zobrazená v šuplíku), nic se neukládá do souborů ani neposílá třetím stranám.
- Ve fullscreen hře Ledge nic nestahuje.
