# Widgety ze skriptů — vlastní karta za 2 minuty

Widget je obyčejný skript, který něco vypíše. Ledge ho čas od času spustí (bez okna) a z toho, co vypsal,
nakreslí kartu v šuplíku a/nebo krátký text v proužku nahoře. Kurz měny, stav serveru, běžící kontejnery,
poslední build — cokoli, na co umíš napsat pár řádků v PowerShellu, Pythonu, Node.js nebo dávce `.cmd`.

## Nejrychlejší cesta: příklad z galerie

1. **Nastavení → Projekty, akce a odkazy → Widgety ze skriptů → Galerie příkladů.**
2. U příkladu (třeba *Kurz ČNB*) klikni **Přidat** — zkopíruje se do složky widgetů
   (`%USERPROFILE%\Ledge\widgets`). Nic se zatím nespustí.
3. V šuplíku klikni na **✎** (upravit šuplík) → **+ Přidat kartu** → dole v katalogu je skupina
   **Widgety ze skriptů** → klikni na widget → **Hotovo**.
4. Karta ukáže začátek skriptu a zeptá se. Když ti skript dává smysl, klikni **Povolit a spustit**.

Do proužku dáš widget přes **⋯ na kartě → Ukázat i v proužku** (nebo Nastavení → Vzhled → Co proužek
ukazuje).

## Vlastní widget

Vytvoř ve složce widgetů soubor, třeba `muj.ps1` (tlačítko **Otevřít složku widgetů** ji založí):

```powershell
# @ledge.widget Můj první widget
# @ledge.icon 🧩
# @ledge.refresh 5m
# @ledge.show both

$volno = [math]::Round((Get-PSDrive C).Free / 1GB)
@{
    text  = "Na disku C: je volno $volno GB"
    strip = "🧩 $volno GB"
    items = @(
        @{ label = 'Otevřít disk C:'; open = 'C:\' },
        @{ label = 'Nápověda Ledge'; url = 'https://github.com/jiri-hyan/ledge-app' }
    )
} | ConvertTo-Json -Depth 5 -Compress
```

Ulož ho jako **UTF-8 s BOM** (Windows PowerShell jinak špatně čte diakritiku). Pak ho přidej z katalogu
karet a povol — hotovo.

### Hlavička

Řádky `@ledge.…` v komentáři na začátku souboru (`#` v `.ps1` / `.py`, `::` nebo `REM` v `.cmd`, `//` v `.js`):

| řádek | co dělá | výchozí |
|---|---|---|
| `@ledge.widget Název` | **povinný** — z tohohle Ledge pozná, že je to widget | — |
| `@ledge.icon 🐳` | emoji před názvem | žádná |
| `@ledge.description …` | popis v katalogu a v Nastavení | žádný |
| `@ledge.refresh 5m` | jak často: `30s`, `5m`, `1h`, `1h30m` (nejméně 30 s, nejvýš 24 h) | `5m` |
| `@ledge.show both` | kam patří: `card` (karta), `strip` (proužek), `both` | `card` |
| `@ledge.timeout 15` | kolik sekund smí běžet, pak ho Ledge ukončí (1–120) | `10` |
| `@ledge.args -x "a b"` | pevné argumenty | žádné |

Program `.exe` hlavičku mít nemůže — dej ji vedle do textového souboru `program.exe.ledge`.

### Co vypsat (JSON)

```json
{
  "title": "Docker",
  "text": "2 kontejnery běží",
  "badge": "2/3",
  "progress": 0.66,
  "strip": "🐳 2 běží",
  "color": "green",
  "items": [
    { "label": "web", "value": "Up 2 h", "color": "green", "url": "http://localhost:8080" },
    { "label": "db", "value": "Up 2 h", "copy": "postgres://localhost:5432" },
    { "label": "Logy", "open": "C:\\logs" },
    { "label": "Restartovat web", "run": "restart.ps1", "arg": "web" }
  ]
}
```

Všechno je nepovinné. `strip` má nejvýš 24 znaků (chybí-li, vezme se `badge`, pak první řádek `text`).
Barvy: `green`, `amber`, `red`, `blue`, `teal`, `violet`, `muted`, `text`, `accent` (a srozumitelné
`ok`, `warn`, `error`, `gray` nebo `#rrggbb`).

Klik na položku umí jen tohle — nic jiného ze skriptu Ledge nespustí:

- `url` — otevře web v prohlížeči (jen `http://` a `https://`),
- `open` — otevře složku nebo dokument (pdf, obrázek, txt…; programy ne),
- `copy` — zkopíruje text do schránky,
- `run` + `arg` — spustí skript ze složky **skriptů** (Script Commands), který jsi už jednou povolil
  z palety (Ctrl+K); `arg` dostane jako poslední argument.

### Nebo jen text (jako xbar)

```text
☀ 18 °C | color=amber
Lisabon, polojasno
---
Předpověď | href=https://open-meteo.com
Zkopírovat teplotu | copy="18 °C"
```

První řádek jde do proužku, další řádky (do `---`) jsou text karty, za `---` jsou položky. Parametry za `|`:
`href`, `color`, `copy`, `open`, `run`, `arg`, `icon`, `value`. Příkazy typu `bash=` Ledge záměrně
nespouští.

## Bezpečnost a výkon

- **Nic neběží bez tvého souhlasu.** Nový skript i každá jeho změna se zastaví a karta se zeptá znovu
  (ukáže začátek souboru a jeho otisk). Souhlas je v `%LOCALAPPDATA%\Ledge\widgets-consent.json`, ne
  v nastavení — nepřenese se zálohou ani na jiný počítač.
- Widget běží **jen když je vidět**: karta na otevřené záložce, nebo položka v proužku. Ve hře na celou
  obrazovku a v herním režimu se nespouští nic.
- Bez okna, s termínem (pak Ledge ukončí celý strom procesů), výstup nejvýš 256 KB, nikdy dvakrát
  zároveň, bez zvýšení práv. Výsledek zůstává jen v paměti Ledge.
- Když se něco pokazí, karta to řekne lidsky („Skript skončil chybou…“) a v **⋯ → Zobrazit výstup**
  uvidíš, co skript opravdu vypsal.

## Galerie příkladů

| příklad | co ukazuje | síť |
|---|---|---|
| Kurz ČNB (PowerShell, Python) | euro, dolar, libra z denního kurzovního lístku | www.cnb.cz |
| Počasí jinde (PowerShell, Python) | teplota a obloha v jiném městě | api.open-meteo.com |
| Docker | běžící a zastavené kontejnery (`docker ps`) | ne |
| GitHub CI | poslední commit a běhy Actions jednoho repa (`gh`) | api.github.com přes `gh` |
| Ping (PowerShell, Python) | odezva serveru | jeden ping |
| Síť a VPN | aktivní připojení, jestli běží VPN | ne |
| Místo na disku (PowerShell, Python) | volné místo na discích | ne |
| Uptime (PowerShell, Python) | jak dlouho počítač běží | ne |
| Odpočet (PowerShell, Python) | kolik dní zbývá do data (Python = textový formát) | ne |

Město, server nebo repo si v příkladu změníš přes **⋯ → Upravit skript** (otevře se v editoru).
