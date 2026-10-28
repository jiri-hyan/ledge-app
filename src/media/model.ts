// Záložka Osobní — právě hraje, hlasitost, baterie: čistá logika pro karty (bez DOM a bez `t()`).
// Data posílá Rust (`media.rs`, `volume.rs`, `battery.rs`); tady se nic nevymýšlí, jen tvaruje.

/** Zbývající čas baterie (sekundy od Windows) → hodiny a minuty (dolů na minutu); neznámo = null. */
export function remaining(s: number | null | undefined): { h: number; m: number } | null {
  if (s == null || !Number.isFinite(s) || s <= 0) return null;
  const min = Math.floor(s / 60);
  return { h: Math.floor(min / 60), m: min % 60 };
}

/** Baterie je „nízko“ (červeně): na baterii a ≤ 15 %. */
export function batteryLow(percent: number, plugged: boolean): boolean {
  return !plugged && percent <= 15;
}

/** Ikona hlasitosti podle úrovně: ztlumeno / 0 / do 33 / do 66 / víc. */
export function volumeLevel(level: number, muted: boolean): "mute" | "low" | "mid" | "high" {
  if (muted || level <= 0) return "mute";
  if (level <= 33) return "low";
  if (level <= 66) return "mid";
  return "high";
}

/**
 * Omezení volání při tahu posuvníku: první hodnota hned, další nejvýš 1× za `ms`, poslední vždy
 * (na konci intervalu). `flush` pošle čekající hned (puštění myši). Hodiny a časovač lze podstrčit
 * (testy).
 */
export function throttle<T>(
  send: (v: T) => void,
  ms: number,
  clock: { now: () => number; later: (fn: () => void, ms: number) => unknown; cancel: (h: unknown) => void } = {
    now: () => Date.now(),
    later: (fn, d) => setTimeout(fn, d),
    cancel: (h) => clearTimeout(h as ReturnType<typeof setTimeout>),
  },
): { push: (v: T) => void; flush: () => void } {
  let last = -Infinity;
  let pending: { v: T } | null = null;
  let timer: unknown = null;
  const fire = () => {
    timer = null;
    if (!pending) return;
    const { v } = pending;
    pending = null;
    last = clock.now();
    send(v);
  };
  return {
    push(v) {
      pending = { v };
      const wait = last + ms - clock.now();
      if (wait <= 0) {
        if (timer != null) clock.cancel(timer);
        fire();
      } else if (timer == null) {
        timer = clock.later(fire, wait);
      }
    },
    flush() {
      if (timer != null) clock.cancel(timer);
      fire();
    },
  };
}

/** Menu výstupu v kartě Zvuk: jen když je z čeho vybírat (>1 zařízení) — beze změny oproti kolu 18. */
export function outputPickerShown(devices: number): boolean {
  return devices > 1;
}
