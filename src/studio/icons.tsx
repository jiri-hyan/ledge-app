// Ikony Ledge Studia (kolo 24): 8 částí (páteř, domov), okenní tlačítka a drobnosti titulku. Tah 1.6,
// `currentColor`, 20 × 20 — žádná animace.
import type { JSX } from "solid-js";

const S = (p: { children: JSX.Element; size?: number }) => (
  <svg width={p.size ?? 20} height={p.size ?? 20} viewBox="0 0 20 20" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
    {p.children}
  </svg>
);

const PART: Record<string, () => JSX.Element> = {
  strip: () => <S><path d="M2.5 3.5h15" /><rect x="6" y="3.5" width="8" height="3.2" rx="1.6" /><path d="M4 16.5h12" opacity=".35" /></S>,
  open: () => <S><path d="M2.5 3.5h15" /><path d="M10 6.5v6" /><path d="m7.2 9.8 2.8 2.8 2.8-2.8" /><path d="M5 16.5h10" /></S>,
  look: () => <S><circle cx="10" cy="10" r="6.5" /><path d="M10 3.5a6.5 6.5 0 0 0 0 13z" fill="currentColor" stroke="none" opacity=".5" /></S>,
  cards: () => <S><rect x="3" y="3.5" width="6" height="6" rx="1.4" /><rect x="11" y="3.5" width="6" height="3.5" rx="1.2" /><rect x="11" y="9" width="6" height="7.5" rx="1.4" /><rect x="3" y="11.5" width="6" height="5" rx="1.2" /></S>,
  claude: () => <S><path d="M10 3v14M3 10h14M5 5l10 10M15 5 5 15" /></S>,
  projects: () => <S><path d="M3 6.2a1.7 1.7 0 0 1 1.7-1.7h3l1.6 1.8h6a1.7 1.7 0 0 1 1.7 1.7v6.3a1.7 1.7 0 0 1-1.7 1.7H4.7A1.7 1.7 0 0 1 3 14.3z" /><path d="m9 10.5 1.5 1.5 3-3" /></S>,
  notes: () => <S><path d="M5 3.5h7.5L16 7v8.8a.7.7 0 0 1-.7.7H5a.7.7 0 0 1-.7-.7V4.2a.7.7 0 0 1 .7-.7z" /><path d="M12.5 3.5V7H16M7 10.5h6M7 13.5h4" /></S>,
  app: () => <S><circle cx="10" cy="10" r="2.6" /><path d="M10 2.8v2M10 15.2v2M2.8 10h2M15.2 10h2M4.9 4.9l1.4 1.4M13.7 13.7l1.4 1.4M4.9 15.1l1.4-1.4M13.7 6.3l1.4-1.4" /></S>,
};

export const PartIcon = (p: { icon: string }) => (PART[p.icon] ?? PART.app)();

export const IconHome = () => <S><path d="M3.5 9.2 10 3.8l6.5 5.4" /><path d="M5.2 8v8.2h9.6V8" /></S>;
export const IconFind = () => <S size={16}><circle cx="8.8" cy="8.8" r="5" /><path d="m12.6 12.6 4 4" /></S>;
export const IconGuide = () => <S size={16}><path d="M4 16.5V3.5" /><path d="M4 4h9.5l-2 3 2 3H4" /></S>;
export const IconHistory = () => <S size={16}><path d="M3.5 10a6.5 6.5 0 1 0 1.9-4.6" /><path d="M3.5 3.8v2.8h2.8" /><path d="M10 6.5V10l2.4 1.6" /></S>;
export const IconUndo = () => <S size={14}><path d="M7 5 3.5 8.5 7 12" /><path d="M3.5 8.5H12a4.5 4.5 0 0 1 0 9h-1.5" /></S>;
export const IconCheck = () => <S size={14}><path d="m4 10.5 4 4 8-9" /></S>;
export const IconMin = () => <S size={14}><path d="M4.5 10h11" /></S>;
export const IconMax = () => <S size={14}><rect x="4.5" y="4.5" width="11" height="11" rx="1.5" /></S>;
export const IconRestore = () => <S size={14}><rect x="4" y="7" width="9" height="9" rx="1.5" /><path d="M7 7V5.5A1.5 1.5 0 0 1 8.5 4H14.5A1.5 1.5 0 0 1 16 5.5v6a1.5 1.5 0 0 1-1.5 1.5H13" /></S>;
export const IconClose = () => <S size={14}><path d="m5 5 10 10M15 5 5 15" /></S>;
export const IconScene = () => <S size={16}><rect x="2.5" y="4" width="15" height="11" rx="2" /><path d="M12 4v11" /></S>;
export const IconChevron = () => <S size={14}><path d="m8 5 5 5-5 5" /></S>;
