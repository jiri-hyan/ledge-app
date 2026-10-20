// Ikony akcí nad poznámkou (inline, styl jako `src/icons.tsx` — tahy 1.6).
const s = { fill: "none", stroke: "currentColor", "stroke-width": 1.6, "stroke-linecap": "round", "stroke-linejoin": "round" } as const;

/** Splnit ✓. */
export const IconCheck = () => (
  <svg viewBox="0 0 24 24" {...s}><path d="M5 12.5l4.5 4.5L19 7.5" /></svg>
);
/** Archivovat: krabice se šipkou dolů. */
export const IconArchive = () => (
  <svg viewBox="0 0 24 24" {...s}><rect x="3" y="4" width="18" height="4" rx="1" /><path d="M5 8v10a2 2 0 0 0 2 2h10a2 2 0 0 0 2-2V8M12 11v5M9.5 13.5L12 16l2.5-2.5" /></svg>
);
/** Vrátit z archivu: krabice se šipkou nahoru. */
export const IconUnarchive = () => (
  <svg viewBox="0 0 24 24" {...s}><rect x="3" y="4" width="18" height="4" rx="1" /><path d="M5 8v10a2 2 0 0 0 2 2h10a2 2 0 0 0 2-2V8M12 16v-5M9.5 13.5L12 11l2.5 2.5" /></svg>
);
/** Smazat (do Koše). */
export const IconTrash = () => (
  <svg viewBox="0 0 24 24" {...s}><path d="M4 7h16M10 11v6M14 11v6M5.5 7l1 12a2 2 0 0 0 2 2h7a2 2 0 0 0 2-2l1-12M9 7V4h6v3" /></svg>
);
