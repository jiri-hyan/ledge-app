# Codemapa: vlastní SVG grafy (`src/charts`, kolo 22)

Malé grafy pro karty, široký notch a widgety: sparkline, sloupce a „číslo jako hrdina“.
Bez knihoven, bez CDN, bez animací a časovačů. Cituj symbolem, ne číslem řádku.

## Rozhodnutí

- **Geometrie = čisté funkce** (`geom.ts`, tabulkový test `geom.test.mjs`), **komponenty = tenké obaly** (vezmou čísla,
  zavolají geometrii, nakreslí `<svg>`). Nová logika grafu patří do `geom.ts` + test, ne do komponenty.
- **Poctivý prázdný stav:** méně než 2 konečná čísla = graf se nekreslí (prázdný obal drží místo, nic se nevymýšlí).
  `NaN` / `Infinity` / `null` se vynechají, nikdy se nenahradí nulou. Prstenec ukazuje jedno číslo; chybí = nic.
- **Kreslit jen viditelné:** `useVisible` (IntersectionObserver + `media/shown.ts` `drawerShown`) — zavřený šuplík
  nebo karta mimo zorné pole = žádné `<svg>`. Žádný `setTimeout` / `setInterval` / `requestAnimationFrame`, žádné
  `@keyframes` (hlídá `geom.test.mjs`). Data dodává volající (snapshot, události) — grafy nic nestahují ani nevzorkují.
- **Barvy:** `currentColor` obalu, výchozí `var(--accent)`; přebiješ `--chart-c` nebo `color` na obalu / rodiči.
  Stopy a nulové buňky = `--line-2`. Třídy mají prefix `chart-` (`charts.css`).

## API (stabilní — používají KARTY, DESIGN-LEDGE i WIDGETY)

| komponenta | props (povinné **tučně**) | kreslí |
|---|---|---|
| `Sparkline` | **`values`**, `w` 96, `h` 28, `min`, `max`, `area` (ano), `dot` (ano), `fluid`, `label` | čára + plocha + tečka posledního bodu |
| `Bars` | **`values`**, `w` 96, `h` 28, `gap` 2, `max`, `hi` (zvýrazněný index, −1 = poslední), `fluid`, `title(i)`, `label` | sloupce zdola |
| `Stat` | **`value`** (text nebo null), **`label`**, `unit`, `sub`, `series` (sparkline + šipka), `min`, `max`, `noTrend`, `trendMin`, `seriesLabel` (tooltip sparklinu), `size` lg/md | velké číslo (2,6 × tělo, `tabular-nums`) |

`label` = popis pro čtečky (bez něj je graf ozdoba, `aria-hidden`). Příklad:
`<Sparkline values={hist} max={100} label="CPU za minutu" />`.

Používají: karta Claude dnes (`src/board/UsageCard.tsx`: `<Bars values={0–1} max={1} h={32} fluid hi={…} title={(i) => …} />`;
bez zvýraznění `hi={-99}`) a široký notch (kolo 27 vlastní mini graf `src/notch/SecSystem.tsx` z `charts/geom.ts` `sparkPoints` — tloušťka, vyhlazení, plocha, přechod).
Sloupců méně než 2 = nic.

## Soubory

| soubor | odpovědnost | klíčové symboly |
|---|---|---|
| `src/charts/geom.ts` | čistá geometrie: body a path sparklinu (plocha uzavřená dole), prstenec jako `stroke-dasharray`, obdélníky sloupců, buňky pásu po sloupcích s úrovní, trend; souřadnice na 2 desetiny | `finite` `extent` `sparkPoints` `sparkline` `ring` `bars` `heat` `heatLevel` `trend` |
| `src/charts/dial.ts` · `Gauge.tsx` · `Rings.tsx` | kolo 25, budík (půlkruh s ručičkou a ryskou plánu) a soustředné prstence pro vzhledy karet Claude a Systém: čistá geometrie (`dialPath`, `needleEnd`, `ringDash`, `concentric`, `toneOf`, test `dial.test.mjs`), šířka podle obalu (viewBox), tooltip jako SVG `<title>`, kreslí jen viditelné | `Gauge` `Rings` `dialPath` `ringDash` |
| `src/charts/visible.ts` | „kreslit jen viditelné“: IntersectionObserver obalu + otevřený šuplík; bez observeru = viditelné | `useVisible` |
| `src/charts/Sparkline.tsx` · `Bars.tsx` | tenké komponenty nad `geom.ts` (viz API); `Ring`/`Heat` smazány (kolo 31b, nic je neimportovalo; `geom.ts` `ring`/`heatLevel` zůstávají jako testovaná geometrie bez komponenty) | `Sparkline` `Bars` |
| `src/charts/Stat.tsx` | číslo jako hrdina: číslo, jednotka, popisek, druhý řádek, šipka trendu a sparkline z `series`; `value` null = „—“ | `Stat` |
| `src/charts/charts.css` | barvy a typografie grafů (`chart-*`), žádná animace ani filtr | |

## Testy

`node --experimental-strip-types --test src/charts/geom.test.mjs` (prázdný stav, vynechání nekonečných čísel, tabulky
sparklinu / prstence / sloupců / pásu / trendu, komponenty bez sítě, časovače, animace a knihovny a s `useVisible`).
