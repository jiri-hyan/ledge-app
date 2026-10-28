// Teplota počasí: Open-Meteo vrací °C vždy; ve °F (`personal.fahrenheit`) se jen přepočítá při zobrazení.
// Čistá funkce bez DOM — test v `src/cards/temp.test.mjs`.

/** °C → hodnota v zvolené jednotce (nezaokrouhlená). */
export const tempIn = (c: number, fahrenheit: boolean | undefined): number => (fahrenheit ? (c * 9) / 5 + 32 : c);

/** Značka jednotky: `°C` / `°F`. */
export const tempUnit = (fahrenheit: boolean | undefined): string => (fahrenheit ? "°F" : "°C");
