# Lístky na plochu

Lístek je **poznámka z Ledge připnutá na plochu** — malé průhledné okno, které zůstane vidět nad
prohlížečem i jinými okny. Hodí se třeba na QA webu: checklist toho, co projít, a snímky chyb
rovnou u sebe. Lístek není nic nového vedle poznámek: je to ta samá poznámka, co vidíš v kartě
Poznámky. Co změníš v lístku, je hned v kartě, a naopak.

## Jak lístek dostat na plochu

- **Nový lístek:** v kartě Poznámky klikni na špendlík vedle „+ Nová“ (nebo ⋯ → „Nový lístek na
  plochu“, nebo Ctrl+K → „Nový lístek na plochu“). Vznikne nová poznámka a rovnou se otevře pole pro
  první bod.
- **Rovnou z nové poznámky:** v editoru nové poznámky je vedle Uložit tlačítko „Uložit a připnout“ —
  poznámka se uloží a objeví se jako lístek na ploše.
- **Připnout existující poznámku:** najeď na řádek poznámky a klikni na špendlík vpravo (připnutá ho
  má modrý). Jde to i v editoru poznámky nebo přes Ctrl+K → „Připnout na plochu: …“.

- **Klávesovou zkratkou odkudkoli:** v Nastavení → Šuplík a karty → Otevírání a klávesy → Klávesové zkratky si nastav „Nový lístek na
  plochu“ (výchozí je bez zkratky). Zkratka funguje, i když máš jiné zkratky Ledge vypnuté. Při hře nebo
  v herním režimu nedělá nic — hře nic nevezme.

Nový lístek se objeví vpravo nahoře a vypadá jako lístek, který jsi naposledy upravoval (styl a průhlednost).
Připnutí lístku ti nevezme fokus — v prohlížeči můžeš psát dál.

**Lístek si pamatuje, kde byl.** Když ho zavřeš a poznámku později připneš znovu, objeví se na stejném místě,
ve stejné velikosti, na stejném monitoru a se stejným vzhledem. Když ten monitor zrovna není připojený, ukáže
se vpravo nahoře na hlavním. Poloha a vzhled lístku patří jen tomuhle počítači — na druhém počítači (sync
poznámek) si lístek pamatuje svoje místo zvlášť.

## Co v lístku jde

| Chceš | Udělej |
|---|---|
| posunout lístek | táhni za horní lištu |
| zmenšit / zvětšit | táhni za kterýkoli okraj nebo roh |
| sbalit na lištu a zase rozbalit | dvojklik na lištu |
| přidat bod checklistu | „+ bod“, napiš text, Enter (další bod hned za ním), Esc zavře. Končí-li poznámka číslovaným seznamem (`1.`, `2.`), „+ bod“ pokračuje dalším číslem |
| odškrtnout bod | klikni na čtvereček |
| vložit snímek obrazovky | Win+Shift+S, pak klikni do lístku a Ctrl+V — obrázek se uloží k poznámce |
| vložit odkaz nebo text | klikni do lístku a Ctrl+V — přidá se na konec |
| upravit celý text | tužka dole, nebo dvojklik do textu (kurzor skočí k místu kliku) — ukládá se samo (štítek „Uloženo ✓“), Hotovo / Esc / Ctrl+Enter ukončí úpravu |
| psát seznamy | při psaní (v lístku i v editoru) Enter na řádku `1.` / `-` / `- [ ]` založí další položku, Enter na prázdné položce seznam ukončí, Tab / Shift+Tab ji vnoří / vrátí. Ctrl+Enter zůstává „Hotovo“ |
| uložit nebo předat poznámku | ⋯ → Kopírovat jako Markdown, nebo ⋯ → Exportovat… → Uložit jako Markdown… / HTML… / PDF…, Otevřít v … (aplikace, které má Windows pro `.md` — editor, Typora, poznámková aplikace…) |
| obarvit lístek | ⋯ → Barva: osm barev, první kroužek = výchozí. Barva se uloží k poznámce (uvidíš ji i v kartě Poznámky a na druhém počítači) |
| změnit vzhled | ⋯ → Vzhled a průhlednost: dlaždice Tmavé sklo (původní), Papír, Sklo, Barevný proužek, Minimal, Tmavý — klik a hned vidíš výsledek. Pod nimi posuvníky Pozadí a Text |
| stejný vzhled všude | ⋯ → Vzhled a průhlednost → „Použít na všechny lístky“ — styl a průhlednost dostanou všechny lístky i každý nový |
| štítky | ⋯ → Štítky: napiš štítek a Enter (bez mezer, třeba `qa`, `práce`), křížek ho odebere. Štítkem je i `#štítek` napsaný přímo v textu poznámky (ten smažeš v textu). Štítky se ukazují pod textem lístku |
| odkaz na jinou poznámku | napiš `[[` a začni psát název — Ledge nabídne poznámky (šipky, Enter). Klik na odkaz v lístku otevře tu poznámku jako další lístek, v editoru ji otevře v editoru. Když taková poznámka ještě není, nabídne „Vytvořit“ |
| schovat lístek do zítřka | ⋯ → Schovat do…: Za hodinu / Dnes v 18:00 / Zítra v 8:00 / V pondělí / Za týden, nebo vlastní čas. Lístek zmizí a v ten čas se sám vrátí na stejné místo. Dřív ho vrátíš špendlíkem u poznámky v kartě |
| mít lístek vždy navrchu / jako běžné okno | špendlík na liště |
| zamknout na místě | ⋯ → Zamknout na místě (nejde posunout ani zvětšit, lišta ukáže zámek) |
| průhlednost | ⋯ → Vzhled a průhlednost → Pozadí a Text (text zůstane vždy čitelný) |
| zavřít | ✕ na liště, nebo křížek nahoře v ⋯ — lístek zmizí z plochy, **poznámka zůstane** v kartě Poznámky a lístek si pamatuje své místo a vzhled |
| splnit, archivovat nebo smazat poznámku | ⋯ → Splnit · Archivovat · Smazat — lístek zůstane pár sekund s lištou „Zpět“ a pak se zavře. Smazaná poznámka jde do Koše Windows |

## Průchozí režim — klikání skrz lístek

⋯ → „Průchozí“: myš jde lístkem na stránku pod ním, takže můžeš klikat na web „přes“ lístek. Lístek
zůstane vidět, jen ho nejde ovládat myší — v rohu to ukazuje štítek „průchozí“.

**Jak z průchozího režimu ven:**

- zkratkou **Ctrl+Alt+P** (přepne průchozí režim u všech lístků najednou; platí, jen když je na ploše
  nějaký lístek, i když máš jinak klávesové zkratky v Ledge vypnuté), nebo
- v Ledge: Ctrl+K → „Lístky: vypnout průchozí režim“ (taky v ⋯ karty Poznámky).

Zkratku změníš v Nastavení → Šuplík a karty → Otevírání a klávesy → Klávesové zkratky („Lístky na ploše: průchozí režim“).

## Barvy a štítky v kartě Poznámky

- Obarvená poznámka má v kartě barevný proužek vlevo, štítky vidíš v řádku (`#qa #web`).
- Pod záložkami karty jsou štítky, které tvoje nehotové poznámky mají — klik na štítek ukáže jen poznámky
  s ním, druhý klik filtr zruší.
- Barva podle štítku: Nastavení → Poznámky → Lístky na ploše → Barvy štítků. Lístek bez vlastní barvy pak
  dostane barvu svého prvního obarveného štítku (třeba všechno `#qa` červeně).
- Barva i štítky jsou obyčejné řádky v hlavičce poznámky (`color: blue`, `tags: [qa, web]`), takže je přečte
  i jiný editor Markdownu (třeba Obsidian) a putují s poznámkou na druhý počítač.

## Výchozí vzhled nových lístků

Nastavení → Poznámky → Lístky na ploše: styl (galerie dlaždic s náhledem), pozadí a text nového lístku a
„Nový lístek jako naposledy upravený“ (zapnuté = nový lístek převezme vzhled, který jsi naposledy nastavil
v nějakém lístku; vypnuté = vždy výchozí z téhle sekce).

## Dobré vědět

- Po restartu počítače nebo Ledge se připnuté lístky vrátí na svá místa i s nastavením. Když je
  monitor odpojený, lístek se ukáže na hlavním monitoru.
- Když se poznámka mezitím změní jinde (druhý počítač, editor), lístek se sám obnoví. Pokud zrovna
  píšeš, ukáže nabídku „Načíst znovu“ / „Uložit jako kopii“ — nic se nepřepíše naslepo.
- Platí to i obráceně: když máš tu samou poznámku otevřenou v editoru Ledge a píšeš v lístku, text se
  v editoru sám objeví (kurzor zůstane). Pokud v editoru zrovna něco rozepisuješ, nabídne
  „Načíst znovu“ / „Ponechat moje“ / „Uložit jako kopii“.
- Rozepsanou úpravu jedné poznámky ti otevření jiné nepřepíše: nahoře v editoru se objeví „Máš rozepsanou
  úpravu «název» — Otevřít ji · Zahodit“.
- Když na monitoru s proužkem Ledge běží hra nebo video přes celou obrazovku, prezentace, nebo je
  zapnutý herní režim, lístky ustoupí do pozadí a potom se vrátí navrch. Prohlížeč po F11 (Chrome,
  Edge, Firefox, Brave, Opera, Vivaldi, Arc, Zen) lístky nechá navrchu — hodí se při kontrole webů
  s poznámkami po ruce; přepnutí z takového prohlížeče do hry je hned schová.
