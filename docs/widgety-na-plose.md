# Widgety na ploše

Kartu z Ledge si můžeš dát na plochu nebo na druhý monitor jako malé okno. Widget žije i tehdy, když je šuplík
zavřený, a ukazuje to, co Ledge stejně ví — nic nového se kvůli němu nestahuje.

## Co jde dát na plochu

| Widget | Co ukazuje | Odkud ho připneš |
|---|---|---|
| **Limity Clauda** | 5h okno a týden, kdy se obnoví a za jak dlouho | karta Claude → ⋯ → Připnout na plochu: Limity Clauda |
| **Čeká na tebe** | schválení a otázky Clauda, projekt a jak dlouho čekají | karta Claude → ⋯ → Připnout na plochu: Čeká na tebe |
| **Dev servery** | co běží, na jakém portu, tlačítka Otevřít, Log a Stop | karta Běží → ⋯ → Připnout na plochu |
| **Agenda dne** | dnešek a zítřek z kalendáře, tlačítko Připojit u schůzky | karta Kalendář → ⋯ → Připnout na plochu |
| **Widget ze skriptu** | to, co vypíše tvůj skript ([widgety ze skriptů](widgets.md)) | karta widgetu → ⋯ → Připnout na plochu |

Totéž najdeš v Ctrl+K (napiš „plocha“) a v Nastavení → Šuplík a karty → **Na ploše**.

„Čeká na tebe“ nic neschvaluje. Klik na řádek otevře Ledge s náhledem schválení — rozhoduješ vždycky ty, v šuplíku.

## Ovládání widgetu

- **Posun:** chyť widget za horní okraj (nadpis) a táhni. Po puštění se přilepí k okraji obrazovky nebo k jinému
  widgetu, když je blízko (vypneš v Nastavení → Na ploše → Přichytit widgety k okrajům).
- **Velikost:** táhni za okraj okna.
- **⋯ v pravém horním rohu** (ukáže se po najetí myší):
  - **Vrstva:** *Navrchu* (přes všechna okna), *Normální* (jako běžné okno), *Jen na ploše* (pod všemi okny).
  - **Kompaktní:** menší widget s tím nejdůležitějším; přepnutím zpět se vrátí plná velikost.
  - **Zamknout na místě:** widget nejde posunout ani zvětšit.
  - **Průchozí:** kliknutí projde na okno pod widgetem. Widget pak myší nevypneš — průchozí vypneš
    v Nastavení → Šuplík a karty → Na ploše, nebo v Ctrl+K („Widgety na ploše: vypnout průchozí“).
  - **Schovat ve hře** (zapnuté): ve hře přes celou obrazovku a v herním režimu widget zmizí a nic nepočítá.
    Po hře se vrátí a nevezme ti fokus.
  - **Průhlednost** 30–100 %.
  - **Odepnout z plochy** — widget zmizí, karta v šuplíku zůstane.

## Profily rozmístění

V Nastavení → Šuplík a karty → Na ploše napiš jméno (třeba *Práce*) a klikni **Uložit rozmístění**. Pak si
rozmísti widgety jinak a ulož třeba *Hraní*. Mezi profily přepneš jedním klikem (**Obnovit**) nebo v Ctrl+K
(„Rozmístění plochy: Práce“). Profil si pamatuje, které widgety jsou na ploše, kde a jak jsou nastavené.

## Monitory

Poloha widgetů platí jen pro tento počítač (soubor `%LOCALAPPDATA%\Ledge\desk.json`, nesynchronizuje se).
Když odpojíš monitor, widgety z něj se přesunou na monitor s proužkem Ledge; po jeho připojení se samy vrátí
na své místo.

## Kolik widgetů

Nejvýš 6. Každý widget je samostatné okno a bere trochu paměti. Schovaný widget (ve hře, nebo „Schovat
všechny“) se uspí a nic nepočítá.

## Výchozí chování

V Nastavení → Šuplík a karty → Na ploše nastavíš, jak se mají chovat **nové** widgety: vrstva, schovat ve hře,
průhlednost a přichycení. Uložíš je lištou Uložit dole. Bez widgetů se nic nemění — Ledge žádné okno neotevře.
