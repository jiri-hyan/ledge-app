# ZBÝVÁ — Ledge Studio (kolo 24), po FIX (B–F)

## Hotovo
- Etapa A (okno, modely, UI) + FIX: průvodce bez páteře se spodní lištou Zpět / Přeskočit / Další (`.studio.guiding`),
  scéna 280–320 px se souhrnem volby, titulek „Uloženo ✓“ + „Historie (n)“, `#root` na celou výšku (bez toho se obsah
  neposouval), `Ctrl+S` pryč / `Ctrl+Z` Vrátit v `shortcuts.ts`, názvy hodnot v historii (`named.ts`), mrtvé `.st-live` pryč,
  tipy „Dobré vědět“ + „Co je nového“ (jen po upgradu, zavřít; `tips.ts`) na domově. Snímky 8× přepsané.
- Snímky: CDP přes Chrome (headless) nad `dist` — skript jen v scratchpadu; každý snímek = nová navigace (hash sám app nepřepne).

## Zbývá jen živě
- Vznik okna z ⚙ / palety / karty / tray / widgetu, `studio-goto`, zapamatovaná poloha, tažení titulku, dvojklik = max.
- ACL okna Studio (chybí-li příkaz, tichý reject v konzoli), autosave při změně zvenku, „Spustit průvodce“ / „Upravit
  přímo“ → šuplík, výkon v klidu (zavřené Studio = žádné okno).
- „Co je nového“: localStorage `ledge.studio.seenVersion`; poprvé se jen zapíše, karta se ukáže až po vyšší verzi.
- Domov: skutečný `StripLookPreview` a karty z `layout.pages` už byly (bod 5 kontroly); ověř vzhled na reálném layoutu.
- Starý rám `Section`/intro uvnitř otázek a komentáře v panelech řeší P1–P4 (mimo tuto větev).

## MAPA
`docs/codemap/studio.md`, panely `docs/codemap/settings.md`.
