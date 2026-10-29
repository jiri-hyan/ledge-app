# Kolo 25 / vzhled — hodnocení vzhledů (archy v `shots-k25/`, necommitováno)

Karty měřeny ve 3 šířkách (~300 / 460 / 680 px), čas pevně 14:30 (sky noc 23:20), motiv výchozí tmavý.

## Claude (9)
- classic OK · rings OK (úzká: prstenec nad pruhy) · pace OK · gauge opraveno (ručička vedla přes číslo → začíná nad ním)
- hero / heroWeek / heroSpend: opraveno (hero 5 h kreslilo pod číslem limitu sparkline ÚTRATY; teď skutečná historie limitu, týden bez řady); na široké kartě slabé (číslo vlevo, prázdná plocha vpravo)
- spend OK · sessions: slabé na ~680 (4 dlaždice, čas „před 4 m…“ oříznutý)
## Systém (7)
- lines OK (úzká: „11…“ oříznuto) · rings OK · gauges opraveno (ručička přes text) · bars OK · terminal OK · tiles OK (úzká: 3. dlaždice pod sebou) · net OK
## Počasí (7 + noc + déšť)
- classic OK · sky opraveno (bílý nadpis na světlé obloze nečitelný → tmavší gradienty + jemný stín) · thermo OK (štítek „teď“ přetéká o pár px) · poster OK (široká: prázdná plocha vpravo, slabé) · curve OK · week OK · arc OK (široká: prázdné boky)
## Týden (6)
- strip opraveno (široká: události svisle na střed; stále slabé — čas uprostřed, malé události) · month OK · tearoff OK · timeline slabé (názvy událostí oříznuté na „R…“ v úzké/střední) · dial OK (štítek „−18“ je rysky + 18) · agenda OK
## Pozadí šuplíku (14 gradientů + 6 živých, motiv oled, materiál plné)
- po opravě 1 jsou karty průsvitné (tint) a pozadí prosvítá; na oled je efekt jemný (černá výplň 55–70 %), sklo (`glass`) je vidět víc. float: pozadí je teď vidět pod panelem. notch: pozadí OK.
- Živá pozadí jsou snímána statickým snímkem (reduced motion) — animaci nelze ověřit v archu.
## Neověřeno
- `backdrop-filter` (režim glass) ve WebView2 se software renderem; živé pozadí v pohybu; notch s pevnou šířkou (v showcase je okno 1200).
