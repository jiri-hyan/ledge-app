// Pojistka proti surovým technickým chybám v hláškách (`toast.tsx`): jádro (Rust) posílá lidské věty,
// ale kdyby přesto dorazil `os error 10048`, anglický stacktrace nebo JSON, ukážeme obecnou větu.

/** Vypadá text jako surová technická chyba (systém, panika, stacktrace, JSON)? */
export function looksTechnical(msg: string): boolean {
  const s = msg.trim();
  if (!s) return false;
  if (/\(os error \d+\)|\bos error \d+\b/i.test(s)) return true;
  if (/\(0x8[0-9a-f]{7}\)/i.test(s)) return true;
  if (/stack backtrace|panicked at|\n\s+at\s+\S+.*:\d+:\d+|^\s*(Type|Reference|Syntax)Error\b/m.test(s)) return true;
  if (/^[{[]/.test(s) && /"\s*:/.test(s)) return true; // JSON objekt / pole s klíči
  return false;
}

/** Text chyby pro člověka: prázdný → `generic`, technický → `technical`, jinak beze změny (bez „Error:“). */
export function humanError(raw: unknown, generic: string, technical: string): string {
  const msg = (raw instanceof Error ? raw.message : String(raw ?? "")).replace(/^Error:\s*/, "").trim();
  if (!msg) return generic;
  return looksTechnical(msg) ? technical : msg;
}
