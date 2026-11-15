// Připnutí čerstvě vytvořené poznámky (`saveDraft(pin)`). Poznámka už existuje, takže selhání připnutí
// nesmí zrušit reset konceptu — jinak další „Uložit“ vytvoří duplicitu. Čistá funkce bez DOM a Tauri
// (test `pinafter.test.mjs`).

/** Připne `path` přes `toggle`; vrací text chyby, nebo null (připnuto / není co připínat). Nikdy nevyhodí. */
export async function pinAfterCreate(pin: boolean, path: string | null | undefined, toggle: (p: string) => Promise<unknown>, human: (e: unknown) => string = (e) => (e instanceof Error ? e.message : `${e}`)): Promise<string | null> {
  if (!pin || !path) return null;
  try {
    await toggle(path);
    return null;
  } catch (e) {
    return human(e);
  }
}
