// Ikony panelů Nastavení (inline SVG, tahy jako `src/icons.tsx`). Neznámý panel = tečka.
const st = { fill: "none", stroke: "currentColor", "stroke-width": 1.6, "stroke-linecap": "round", "stroke-linejoin": "round" } as const;

const PATHS: Record<string, string> = {
  vitejte: "M12 3l2.5 5.5L20 9l-4 4 1 6-5-3-5 3 1-6-4-4 5.5-.5z",
  projekty: "M4 5h6v6H4zM14 5h6v6h-6zM4 15h6v4H4zM14 15h6v4h-6z",
  suplik: "M3 4h18M7 4v3a2 2 0 0 0 2 2h6a2 2 0 0 0 2-2V4M5 13h14v6a1 1 0 0 1-1 1H6a1 1 0 0 1-1-1zM10 16h4",
  vzhled: "M12 3a9 9 0 1 0 0 18c1 0 1.5-.8 1.5-1.6 0-1.2-1-1.4-1-2.4 0-.9.7-1.5 1.6-1.5H17a4 4 0 0 0 4-4c0-4.7-4-8.5-9-8.5zM7.5 11h.01M10 7.5h.01M15 7.5h.01",
  akce: "M13 3L5 14h6l-1 7 8-11h-6z",
  claude: "M12 3v4M12 17v4M3 12h4M17 12h4M5.6 5.6l2.8 2.8M15.6 15.6l2.8 2.8M5.6 18.4l2.8-2.8M15.6 8.4l2.8-2.8",
  otazky: "M4 5h16v11H9l-5 4zM10 9a2 2 0 1 1 2.5 1.9c-.3.1-.5.4-.5.7V12M12 14h.01",
  poznamky: "M6 3h9l4 4v14H6zM14 3v5h5M9 13h7M9 17h5",
  osobni: "M12 12a4 4 0 1 0 0-8 4 4 0 0 0 0 8zM5 21a7 7 0 0 1 14 0",
  zkratky: "M3 7h18v11H3zM7 11h.01M11 11h.01M15 11h.01M8 15h8",
  zaloha: "M12 4v11M8 11l4 4 4-4M5 19h14",
  oaplikaci: "M12 3a9 9 0 1 0 0 18 9 9 0 0 0 0-18zM12 11v5M12 8h.01",
};

export function PanelIcon(p: { id: string }) {
  return (
    <svg viewBox="0 0 24 24" {...st} aria-hidden="true">
      {PATHS[p.id] ? <path d={PATHS[p.id]} /> : <circle cx="12" cy="12" r="3" />}
    </svg>
  );
}

export const IconSearch = () => (
  <svg viewBox="0 0 24 24" {...st} aria-hidden="true"><circle cx="11" cy="11" r="6" /><path d="M20 20l-4.2-4.2" /></svg>
);
