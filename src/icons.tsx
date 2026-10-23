// Ikony inline (žádné CDN, žádná knihovna). Tahy 1.6, ve stylu Tabler outline.
const s = { fill: "none", stroke: "currentColor", "stroke-width": 1.6, "stroke-linecap": "round", "stroke-linejoin": "round" } as const;

export const IconNote = () => (
  <svg viewBox="0 0 24 24" {...s}><path d="M13 20l7-7M13 20v-6a1 1 0 0 1 1-1h6V6a2 2 0 0 0-2-2H6a2 2 0 0 0-2 2v12a2 2 0 0 0 2 2h7" /></svg>
);
export const IconTerminal = () => (
  <svg viewBox="0 0 24 24" {...s}><path d="M5 7l5 5-5 5M12 19h7" /></svg>
);
export const IconFolder = () => (
  <svg viewBox="0 0 24 24" {...s}><path d="M5 4h4l3 3h7a2 2 0 0 1 2 2v8a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V6a2 2 0 0 1 2-2" /></svg>
);
export const IconGithub = () => (
  <svg viewBox="0 0 24 24" {...s}><path d="M9 19c-4.3 1.4-4.3-2.5-6-3m12 5v-3.5c0-1 .1-1.4-.5-2 2.8-.3 5.5-1.4 5.5-6a4.6 4.6 0 0 0-1.3-3.2 4.2 4.2 0 0 0-.1-3.2s-1.1-.3-3.5 1.3a12.3 12.3 0 0 0-6.2 0C6.5 2.8 5.4 3.1 5.4 3.1a4.2 4.2 0 0 0-.1 3.2A4.6 4.6 0 0 0 4 9.5c0 4.6 2.7 5.7 5.5 6-.6.6-.6 1.2-.5 2V21" /></svg>
);
export const IconLink = () => (
  <svg viewBox="0 0 24 24" {...s}><path d="M12 6H6a2 2 0 0 0-2 2v10a2 2 0 0 0 2 2h10a2 2 0 0 0 2-2v-6M11 13l9-9M15 4h5v5" /></svg>
);
export const IconSettings = () => (
  <svg viewBox="0 0 24 24" {...s}><path d="M10.3 4.3c.4-1.8 3-1.8 3.4 0a1.7 1.7 0 0 0 2.6 1.1c1.5-.9 3.3.8 2.4 2.4a1.7 1.7 0 0 0 1 2.5c1.9.5 1.9 3.1 0 3.5a1.7 1.7 0 0 0-1 2.6c.9 1.5-.9 3.3-2.4 2.4a1.7 1.7 0 0 0-2.6 1c-.4 1.9-3 1.9-3.4 0a1.7 1.7 0 0 0-2.6-1c-1.5.9-3.3-.9-2.4-2.4a1.7 1.7 0 0 0-1-2.6c-1.9-.4-1.9-3 0-3.5a1.7 1.7 0 0 0 1-2.5c-.9-1.6.9-3.3 2.4-2.4 1 .6 2.3.1 2.6-1.1" /><circle cx="12" cy="12" r="3" /></svg>
);
export const IconBack = () => (
  <svg viewBox="0 0 24 24" {...s}><path d="M15 6l-6 6 6 6" /></svg>
);
export const IconShield = () => (
  <svg viewBox="0 0 24 24" {...s}><path d="M12 3a12 12 0 0 0 8.5 3A12 12 0 0 1 12 21 12 12 0 0 1 3.5 6 12 12 0 0 0 12 3" /><path d="M12 9v4M12 16h.01" /></svg>
);
export const IconBell = () => (
  <svg viewBox="0 0 24 24" {...s}><path d="M10 5a2 2 0 1 1 4 0 7 7 0 0 1 4 6v3a4 4 0 0 0 2 3H4a4 4 0 0 0 2-3v-3a7 7 0 0 1 4-6M9 17v1a3 3 0 0 0 6 0v-1" /></svg>
);
export const IconX = () => (
  <svg viewBox="0 0 24 24" {...s}><path d="M18 6L6 18M6 6l12 12" /></svg>
);
export const IconWorld = () => (
  <svg viewBox="0 0 24 24" {...s}><circle cx="12" cy="12" r="9" /><path d="M3.6 9h16.8M3.6 15h16.8M11.5 3a17 17 0 0 0 0 18M12.5 3a17 17 0 0 1 0 18" /></svg>
);
export const IconFile = () => (
  <svg viewBox="0 0 24 24" {...s}><path d="M14 3v4a1 1 0 0 0 1 1h4M17 21H7a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h7l5 5v11a2 2 0 0 1-2 2" /></svg>
);
export const IconApp = () => (
  <svg viewBox="0 0 24 24" {...s}><rect x="3" y="4" width="18" height="16" rx="2" /><path d="M3 8h18M7 6h.01M10 6h.01" /></svg>
);

/** Lupa (hlavička i paleta) — tady kvůli tomu, aby hlavička netahala líně načtenou paletu. */
export const IconSearch = () => (
  <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
    <circle cx="11" cy="11" r="7" />
    <path d="m20 20-3.5-3.5" />
  </svg>
);
