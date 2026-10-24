# Podpis vydání Ledge

Ledge se aktualizuje sám z veřejných vydání na GitHubu. Kontrolní součet SHA256 v poznámkách vydání chrání
jen před poškozeným stažením: kdo by ovládl vydání (účet, token), přepsal by instalátor i řádek se součtem.
Proto je každý instalátor od kola 21 **podepsaný** a Ledge bez platného podpisu nic nespustí.

## Model hrozeb (co podpis chrání a co ne)

- **Chrání proti** převzetí GitHubu nebo vydání: kdo ovládne účet, token nebo stránku vydání, může přepsat
  instalátor i `SHA256SUMS.txt`, ale nemá soukromý klíč, takže Ledge jeho soubor nespustí.
- **Nechrání proti** kompromitaci stroje vydavatele. Heslo v DPAPI (`CurrentUser`) odemkne každý proces
  běžící pod stejným účtem Windows, takže malware na tomto účtu může podepsat cokoli. Podpis tedy dokládá
  „vydáno z tohoto účtu“, ne „vydáno člověkem po kontrole“.
- **Budoucí krok:** oddělený účet jen pro vydávání, nebo hardware klíč (podpis v zařízení, které soukromý klíč
  nikdy neopustí).
- `signing-setup.ps1` odmítne běžet, když je `src-tauri/signing/update.pub` už vyplněný; vědomá výměna klíče
  je `-Rotate` (viz Ztráta nebo vyzrazení klíče).

## Jak to funguje

**Při vydání** (`scripts/publish.ps1`, lokálně na stroji vydavatele):

1. Ještě před snapshotem a buildem skript ověří, že soukromý klíč existuje, leží mimo repo a jeho veřejná
   půlka je ta, která je v repu v `src-tauri/signing/update.pub`. Jinak skončí s vysvětlením.
2. Po buildu podepíše instalátor příkazem `tauri signer sign --app-version <verze>` (minisign, Ed25519).
   Vznikne `<instalátor>.sig`; jeho podepsaný komentář obsahuje jméno souboru a verzi.
3. Vydání vznikne nejdřív jako **koncept**, nahrají se všechny soubory (instalátor, `.sig`, `SHA256SUMS.txt`,
   `pro-claude.md`), skript zkontroluje jejich seznam a teprve pak koncept publikuje.

**V aplikaci** (`src-tauri/src/updsign.rs`, volá `update.rs`):

- Veřejný klíč je zapečený v binárce (`src-tauri/signing/update.pub` při sestavení).
- Vydání bez `<instalátor>.sig` se vůbec nenabídne („Vydání … není podepsané“).
- Po stažení se ověří SHA256 a pak podpis. Podpis musí patřit zapečenému klíči, sedět na každý bajt
  souboru a jeho komentář musí jmenovat přesně tenhle soubor a verzi — starý platně podepsaný instalátor tak
  nejde podstrčit pod novější číslo verze.
- Instalátor v cache (`%LOCALAPPDATA%\Ledge\updates`) se před spuštěním ověřuje znovu, včetně podpisu.
  Položky stažené dřívější verzí Ledge podpis nemají, proto se smažou a stáhnou znovu.
- Cokoli neprojde = instalátor se smaže a nic se nespustí. Sestavení bez klíče (prázdný `update.pub`,
  vývoj) neinstaluje žádnou aktualizaci.
- **Vrátit předchozí verzi** platí stejně: na verzi vydanou před podpisy se Ledge sám nevrátí. Konkrétně
  návrat z 0.8.0 na 0.7.x nejde (0.7.x není podepsaná) — tu je potřeba nainstalovat ručně ze stránky vydání
  (stáhnout instalátor, ověřit SHA256 proti `SHA256SUMS.txt`, spustit).

## Jednorázové nastavení klíče

Vydávání běží bez člověka (lead pouští `publish.ps1` na pozadí přes frontu), proto výchozí cesta je
neinteraktivní:

```powershell
powershell -NoProfile -File scripts/signing-setup.ps1 -NonInteractive
```

- Soukromý klíč vznikne v `%USERPROFILE%\.ledge-signing\ledge-update.key` (jiná složka: `-Dir`, vždy mimo
  repo), veřejný vedle něj (`.pub`) a zkopíruje se do `src-tauri/signing/update.pub`. Složka má práva jen
  pro aktuální účet.
- Heslo je náhodné (256 bitů z kryptografického generátoru), nikde se nevypíše a uloží se přes **DPAPI**
  (Windows, vázané na účet) do `%USERPROFILE%\.ledge-signing\ledge-update.pass.dpapi`. Jedno omezení:
  `tauri signer generate` bere heslo jen jako argument, takže je po dobu generování (pár sekund) vidět
  v příkazové řádce procesu pro tentýž účet.
- Bez `-NonInteractive` se tauri na heslo zeptá (2×) a DPAPI soubor nevznikne — vydávat pak jde jen
  s heslem v `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`.
- Commitni `src-tauri/signing/update.pub` na `main` a pushni. Teprve pak jde vydávat.
- Skript existující klíč ani heslo nikdy nepřepíše a při už vyplněném `update.pub` odmítne běžet vůbec
  (na jiném stroji klíč obnov ze zálohy, nevytvářej nový).

Při vydání: `publish.ps1` si klíč najde sám (jiná cesta: `-SigningKey <soubor>` nebo proměnná
`LEDGE_SIGNING_KEY`). Heslo vezme z `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`, a když není, z DPAPI souboru
(`-PasswordFile` pro jinou cestu); **nic se neptá**. Heslo drží jen jako SecureString, nikdy ho nevypisuje
(ani při chybě), proměnnou prostředí dostane jen podproces podpisu a hned po něm se smaže — build ani `gh`
ji nevidí. Soukromý klíč ani heslo nepatří do repa, do GitHub Actions ani do žádného configu.

## Záloha klíče

Záloha = **soukromý klíč + heslo**. DPAPI soubor zálohou **není**: odemkne ho jen tentýž účet Windows na
tomtéž stroji, takže po přeinstalaci nebo na jiném PC je k ničemu. Zálohu dělá šéf ručně v okně konzole:

```powershell
# klíč na USB / šifrovaný disk + heslo jednou na obrazovku (po potvrzení napsáním ZOBRAZIT, pak se smaže)
powershell -NoProfile -File scripts/signing-backup.ps1 -Dest E:\ledge-zaloha -ShowPassword
# nebo heslo do souboru (práva jen pro tvůj účet, ale PROSTÝ TEXT): vlož ho do správce hesel a hned smaž
powershell -NoProfile -File scripts/signing-backup.ps1 -Dest E:\ledge-zaloha -PasswordOut $env:USERPROFILE\Desktop\ledge-heslo.txt
```

- Klíč a heslo ukládej **odděleně** (klíč na USB / do trezoru jako přílohu, heslo do správce hesel).
  Jedno bez druhého nestačí.
- `-ShowPassword` heslo ukáže jen v interaktivní konzoli bez přesměrování výstupu (do logu se nedostane);
  skript nic nepřepisuje a zálohu neukládá do repa.
- Veřejný klíč zálohovat není potřeba — je v repu.
- Zálohu jednou vyzkoušej: podepiš záložním klíčem s heslem ze správce hesel libovolný soubor
  (`node node_modules/@tauri-apps/cli/tauri.js signer sign -f <záloha klíče> <soubor>` s heslem
  v `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`) a ověř, že `<záloha>.pub` je stejný jako `src-tauri/signing/update.pub`.
- Obnova na novém stroji: klíč zpět do `%USERPROFILE%\.ledge-signing\` a heslo buď do
  `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`, nebo nový DPAPI soubor:
  `Read-Host -AsSecureString | ConvertFrom-SecureString | Set-Content -Encoding ASCII $env:USERPROFILE\.ledge-signing\ledge-update.pass.dpapi`.

## Ztráta nebo vyzrazení klíče

Nainstalované kopie Ledge znají jen starý veřejný klíč, takže vydání podepsané novým klíčem **neověří**
a neinstalují ho (hlásí, že vydání podepsal jiný klíč). Postup:

1. `signing-setup.ps1 -NonInteractive -Rotate -Dir <nová složka>` (nebo starý klíč i DPAPI soubor nejdřív přesuň
   stranou) → nový klíč a nový `update.pub`, commit. Bez `-Rotate` skript při vyplněném `update.pub` odmítne běžet
   (ochrana před náhodným přepsáním z jiného stroje či účtu).
2. Vydej novou verzi. Do poznámek napiš, že ji je potřeba **jednou nainstalovat ručně** ze stránky vydání
   (stáhnout instalátor, ověřit SHA256 proti `SHA256SUMS.txt`, spustit). Od té verze dál aktualizace zase
   fungují samy.
3. Při vyzrazení navíc: zneplatni přístup k vydáním (tokeny GitHubu, hesla), zkontroluj, že žádné vydání
   nebylo podepsané cizí rukou, a v poznámkách uživatele varuj.

## Přechod z verze 0.6.0

Ledge 0.6.0 podpisy neumí ověřit — kontroluje jen SHA256. Vydání s podpisem pro něj zůstává běžným
vydáním: soubor `.sig` nekončí na `-setup.exe`, takže ho 0.6.0 přehlédne a nainstaluje první podepsanou
verzi jako dřív. Teprve ta (a každá další) vyžaduje podpis. Podmínka: první podepsané vydání musí mít
v binárce skutečný klíč (`publish.ps1` bez neprázdného a odpovídajícího `update.pub` nevydá).

## Neměnná vydání (immutable releases)

Po zapnutí na GitHubu nejde do publikovaného vydání nic nahrát, smazat ani ho přepsat a tag se nedá
přesunout. `publish.ps1` s tím počítá: všechny soubory nahraje do konceptu a publikuje až nakonec. Když běh
spadne v půlce, koncept zůstane neveřejný a další běh vydá **stejnou** verzi a koncept doplní (koncepty se
do čísla další verze nepočítají). Už publikovanou verzi skript nikdy nepřepisuje — chyba ve vydání = vydat
vyšší verzi.

## Authenticode (podpis Windows) — budoucí krok

Instalátor zatím nemá podpis Windows (Authenticode), takže SmartScreen při ruční instalaci varuje před
neznámým vydavatelem. Na stroji vydavatele není certifikát pro podepisování kódu a self-signed certifikát
by SmartScreen nepomohl. Až bude certifikát (OV/EV, nebo Azure Trusted Signing), přidá se do
`publish.ps1` krok `signtool sign` před výpočtem SHA256 a podpisem minisign; ověření v Ledge se nemění.
