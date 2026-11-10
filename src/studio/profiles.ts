// Osobní archetypy / profily (Pillar 1): 4 předpřipravené profily pro rychlé nastavení Ledge.
// Každý profil sladí horní proužek (designová předvolba), chování po najetí i rozložení karet.
// Čistý modul bez DOM a bez t().
import { designOf, sameCompose, normCompose, type DesignId, type StripCompose } from "../strip/compose.ts";
import { normalizeLayout, removeCard, UP_PERSONAL_PROJECTS, WORK, PERSONAL, type CardId, type CardSize, type Layout } from "../layout/model.ts";

export type { StripCompose };

export type ProfileId = "ai_builder" | "power_hw" | "focus_zen" | "organizer";

export type PersonalProfile = {
  id: ProfileId;
  icon: string;
  design: DesignId;
  hover: "full" | "compact" | "island";
  buildLayout: () => Layout;
};

export const PROFILE_IDS: readonly ProfileId[] = ["ai_builder", "power_hw", "focus_zen", "organizer"];

const mkCard = (id: CardId, size: CardSize) => ({ id, size });

export const PROFILES: readonly PersonalProfile[] = [
  {
    id: "ai_builder",
    icon: "✦",
    design: "claudefocus",
    hover: "full",
    buildLayout: (): Layout => ({
      pages: [
        {
          id: WORK,
          name: "",
          cards: [
            mkCard("claude", "l"),
            mkCard("activity", "s"),
            mkCard("projects", "l"),
            mkCard("repos", "s"),
            mkCard("running", "xl"),
            mkCard("notes", "xl"),
          ],
        },
        {
          id: PERSONAL,
          name: "",
          cards: [
            mkCard("usage", "m"),
            mkCard("system", "m"),
            mkCard("media", "m"),
            mkCard("links", "m"),
          ],
        },
      ],
      hiddenParts: [],
      upgrades: [UP_PERSONAL_PROJECTS],
    }),
  },
  {
    id: "power_hw",
    icon: "⚡",
    design: "hardware",
    hover: "full",
    buildLayout: (): Layout => ({
      pages: [
        {
          id: WORK,
          name: "",
          cards: [
            mkCard("system", "l"),
            mkCard("running", "s"),
            mkCard("projects", "xl"),
            mkCard("claude", "m"),
            mkCard("repos", "m"),
            mkCard("winquick", "xl"),
          ],
        },
        {
          id: PERSONAL,
          name: "",
          cards: [
            mkCard("media", "l"),
            mkCard("toggles", "s"),
            mkCard("notes", "m"),
            mkCard("links", "m"),
          ],
        },
      ],
      hiddenParts: [],
      upgrades: [UP_PERSONAL_PROJECTS],
    }),
  },
  {
    id: "focus_zen",
    icon: "🎯",
    design: "zen",
    hover: "island",
    buildLayout: (): Layout => ({
      pages: [
        {
          id: WORK,
          name: "",
          cards: [
            mkCard("timer", "m"),
            mkCard("claude", "m"),
            mkCard("projects", "xl"),
            mkCard("notes", "xl"),
          ],
        },
        {
          id: PERSONAL,
          name: "",
          cards: [
            mkCard("bigclock", "m"),
            mkCard("weather", "m"),
            mkCard("media", "xl"),
          ],
        },
      ],
      hiddenParts: [],
      upgrades: [UP_PERSONAL_PROJECTS],
    }),
  },
  {
    id: "organizer",
    icon: "📅",
    design: "islands",
    hover: "compact",
    buildLayout: (): Layout => ({
      pages: [
        {
          id: WORK,
          name: "",
          cards: [
            mkCard("today", "m"),
            mkCard("calendar", "m"),
            mkCard("notes", "l"),
            mkCard("timer", "s"),
            mkCard("projects", "xl"),
            mkCard("actions", "xl"),
          ],
        },
        {
          id: PERSONAL,
          name: "",
          cards: [
            mkCard("week", "l"),
            mkCard("weather", "s"),
            mkCard("links", "m"),
            mkCard("media", "m"),
          ],
        },
      ],
      hiddenParts: [],
      upgrades: [UP_PERSONAL_PROJECTS],
    }),
  },
];

export function applyProfileToConfig<T extends Record<string, any>>(cfg: T, id: ProfileId): T {
  const prof = PROFILES.find((p) => p.id === id);
  if (!prof) return cfg;
  return {
    ...cfg,
    strip: { ...(cfg.strip ?? {}), look: "custom", compose: designOf(prof.design) },
    drawer: { ...(cfg.drawer ?? {}), hover: prof.hover },
    layout: prof.buildLayout(),
  } as T;
}

export function activeProfile(cfg: any): ProfileId | null {
  if (cfg?.strip?.look !== "custom") return null;
  const currentCompose = normCompose(cfg?.strip?.compose);
  const currentHover = cfg?.drawer?.hover;
  const firstPageCards = Array.isArray(cfg?.layout?.pages?.[0]?.cards)
    ? cfg.layout.pages[0].cards.map((c: any) => c.id)
    : [];

  for (const p of PROFILES) {
    if (currentHover !== p.hover) continue;
    if (!sameCompose(currentCompose, designOf(p.design))) continue;
    const targetCards = p.buildLayout().pages[0].cards.map((c) => c.id);
    if (firstPageCards.length !== targetCards.length) continue;
    if (firstPageCards.every((id: string, idx: number) => id === targetCards[idx])) {
      return p.id;
    }
  }
  return null;
}

export function hideCardEverywhere(rawLayout: unknown, cardId: CardId): Layout {
  let layout = normalizeLayout(rawLayout);
  const pageIds = layout.pages.map((p) => p.id);
  for (const pageId of pageIds) {
    layout = removeCard(layout, pageId, cardId).layout;
  }
  return layout;
}
