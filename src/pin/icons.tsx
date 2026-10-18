// Ikony okna lístku (inline, styl jako `src/icons.tsx` — tahy 1.6, Tabler outline).
const s = { fill: "none", stroke: "currentColor", "stroke-width": 1.6, "stroke-linecap": "round", "stroke-linejoin": "round" } as const;

/** Špendlík (připnout na plochu / vždy navrchu). */
export const IconPin = () => (
  <svg viewBox="0 0 24 24" {...s}><path d="M15 4.5l-4 4-4 1.5-1.5 1.5 7 7 1.5-1.5 1.5-4 4-4M9 15l-4.5 4.5M14.5 4l5.5 5.5" /></svg>
);
export const IconMore = () => (
  <svg viewBox="0 0 24 24" {...s}><path d="M5 12h.01M12 12h.01M19 12h.01" stroke-width="2.6" /></svg>
);
export const IconLock = () => (
  <svg viewBox="0 0 24 24" {...s}><rect x="5" y="11" width="14" height="10" rx="2" /><path d="M8 11V7a4 4 0 0 1 8 0v4" /></svg>
);
export const IconPlus = () => (
  <svg viewBox="0 0 24 24" {...s}><path d="M12 5v14M5 12h14" /></svg>
);
export const IconEdit = () => (
  <svg viewBox="0 0 24 24" {...s}><path d="M4 20h4L19 9a2.8 2.8 0 0 0-4-4L4 16v4M13.5 6.5l4 4" /></svg>
);
/** Průchozí režim: šipka skrz rámeček (myš projde na stránku pod lístkem). */
export const IconThrough = () => (
  <svg viewBox="0 0 24 24" {...s}><rect x="4" y="4" width="16" height="16" rx="3" stroke-dasharray="3 3" /><path d="M12 7v10M8.5 13.5L12 17l3.5-3.5" /></svg>
);
