// Ikony panelů Nastavení (inline SVG, tahy jako `src/icons.tsx`). Neznámý panel = tečka.
const st = { fill: "none", stroke: "currentColor", "stroke-width": 1.6, "stroke-linecap": "round", "stroke-linejoin": "round" } as const;

const PATHS: Record<string, string> = {
  // Začínáme: hvězda
  zaciname: "M12 3l2.5 5.5L20 9l-4 4 1 6-5-3-5 3 1-6-4-4 5.5-.5z",
  // Šuplík a karty: mřížka karet pod lištou
  suplik: "M3 4h18M5 8h6v5H5zM13 8h6v5h-6zM5 15h14v5H5z",
  // Vzhled: paleta
  vzhled: "M12 3a9 9 0 1 0 0 18c1 0 1.5-.8 1.5-1.6 0-1.2-1-1.4-1-2.4 0-.9.7-1.5 1.6-1.5H17a4 4 0 0 0 4-4c0-4.7-4-8.5-9-8.5zM7.5 11h.01M10 7.5h.01M15 7.5h.01",
  // Projekty, akce a odkazy: blesk + článek řetězu
  akce: "M11 3L5 12h5l-1 6 6-9h-5zM15.5 16.5l1.5-1.5a2.5 2.5 0 0 1 3.5 3.5l-1.5 1.5a2.5 2.5 0 0 1-3.5 0",
  // Claude: hvězdice
  claude: "M12 3v4M12 17v4M3 12h4M17 12h4M5.6 5.6l2.8 2.8M15.6 15.6l2.8 2.8M5.6 18.4l2.8-2.8M15.6 8.4l2.8-2.8",
  // Osobní: postava
  osobni: "M12 12a4 4 0 1 0 0-8 4 4 0 0 0 0 8zM5 21a7 7 0 0 1 14 0",
  // Poznámky: list s přehnutým rohem
  poznamky: "M6 3h9l4 4v14H6zM14 3v5h5M9 13h7M9 17h5",
  // Chování: posuvníky
  chovani: "M4 6h9M17 6h3M4 12h3M11 12h9M4 18h11M19 18h1M15 4v4M9 10v4M17 16v4",
  // Záloha a aplikace: krabice se šipkou
  aplikace: "M4 8l8-4 8 4v9l-8 4-8-4zM4 8l8 4 8-4M12 12v9",
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
