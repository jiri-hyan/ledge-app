# Codemapa: vlastní SVG grafy (`src/charts`, kolo 22)

Malé grafy pro karty, široký notch a widgety: sparkline, prstenec, sloupce, pás (heat) a „číslo jako hrdina“.
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
| `Ring` | **`value`** (0–1), `size` 40, `stroke` 4, `label`, `children` (obsah uprostřed) | oblouk od 12 hodin + stopa |
| `Bars` | **`values`**, `w` 96, `h` 28, `gap` 2, `max`, `hi` (zvýrazněný index, −1 = poslední), `fluid`, `title(i)`, `label` | sloupce zdola |
| `Heat` | **`values`**, `w` 120, `h` 12, `rows` 1 (7 = týdny), `gap` 2, `max`, `cellTitle(i)`, `label` | buňky úrovní 0–4 |
| `Stat` | **`value`** (text nebo null), **`label`**, `unit`, `sub`, `series` (sparkline + šipka), `min`, `max`, `size` lg/md | velké číslo (2,6 × tělo, `tabular-nums`) |

`label` = popis pro čtečky (bez něj je graf ozdoba, `aria-hidden`). Příklad:
`<Sparkline values={hist} max={100} label="CPU za minutu" />`, `<Ring value={0.42}>42 %</Ring>`.

Používají: karta Claude dnes (`src/board/UsageCard.tsx`: `<Bars values={0–1} max={1} h={32} fluid hi={…} title={(i) => …} />`;
bez zvýraznění `hi={-99}`) a široký notch (`src/notch/NotchBar.tsx`: `<Sparkline min={0} max={100} area={false} dot={false} fluid />`).
Sloupců méně než 2 = nic.

## Soubory

| soubor | odpovědnost | klíčové symboly |
|---|---|---|
| `src/charts/geom.ts` | čistá geometrie: body a path sparklinu (plocha uzavřená dole), prstenec jako `stroke-dasharray`, obdélníky sloupců, buňky pásu po sloupcích s úrovní, trend; souřadnice na 2 desetiny | `finite` `extent` `sparkPoints` `sparkline` `ring` `bars` `heat` `heatLevel` `trend` |
| `src/charts/visible.ts` | „kreslit jen viditelné“: IntersectionObserver obalu + otevřený šuplík; bez observeru = viditelné | `useVisible` |
| `src/charts/Sparkline.tsx` · `Ring.tsx` · `Bars.tsx` · `Heat.tsx` | tenké komponenty nad `geom.ts` (viz API) | `Sparkline` `Ring` `Bars` `Heat` |
| `src/charts/Stat.tsx` | číslo jako hrdina: číslo, jednotka, popisek, druhý řádek, šipka trendu a sparkline z `series`; `value` null = „—“ | `Stat` |
| `src/charts/charts.css` | barvy a typografie grafů (`chart-*`), žádná animace ani filtr | |

## Testy

`node --experimental-strip-types --test src/charts/geom.test.mjs` (prázdný stav, vynechání nekonečných čísel, tabulky
sparklinu / prstence / sloupců / pásu / trendu, komponenty bez sítě, časovače, animace a knihovny a s `useVisible`).
