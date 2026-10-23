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

Nový lístek se objeví vpravo nahoře. Připnutí lístku ti nevezme fokus — v prohlížeči můžeš psát dál.

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
| uložit nebo předat poznámku | ⋯ → Export: Uložit jako Markdown… / HTML…, Kopírovat jako Markdown, Otevřít v … (aplikace, které má Windows pro `.md` — editor, Typora, poznámková aplikace…) |
| mít lístek vždy navrchu / jako běžné okno | špendlík na liště |
| zamknout na místě | ⋯ → Zamknout na místě (nejde posunout ani zvětšit, lišta ukáže zámek) |
| průhlednost | ⋯ → Pozadí a Text (text zůstane vždy čitelný) |
| zavřít | ✕ — lístek zmizí z plochy, **poznámka zůstane** v kartě Poznámky |
| splnit, archivovat nebo smazat poznámku | ⋯ → Splnit · Archivovat · Smazat — lístek zůstane pár sekund s lištou „Zpět“ a pak se zavře. Smazaná poznámka jde do Koše Windows |

## Průchozí režim — klikání skrz lístek

⋯ → „Průchozí“: myš jde lístkem na stránku pod ním, takže můžeš klikat na web „přes“ lístek. Lístek
zůstane vidět, jen ho nejde ovládat myší — v rohu to ukazuje štítek „průchozí“.

**Jak z průchozího režimu ven:**

- zkratkou **Ctrl+Alt+P** (přepne průchozí režim u všech lístků najednou; platí, jen když je na ploše
  nějaký lístek, i když máš jinak klávesové zkratky v Ledge vypnuté), nebo
- v Ledge: Ctrl+K → „Lístky: vypnout průchozí režim“ (taky v ⋯ karty Poznámky).

Zkratku změníš v Nastavení → Chování → Klávesové zkratky („Lístky na ploše: průchozí režim“).

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
