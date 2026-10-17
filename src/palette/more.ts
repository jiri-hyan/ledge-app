// Položky palety mimo snapshot: skripty ze složky skriptů (`scripts_list` při otevření palety),
// stránky Nastavení Windows (`winsettings.ts`, jen s dotazem), rychlé odpovědi z dotazu (`quick.ts`:
// výpočet, převod jednotek, emoji), příkaz „Emoji…“ a nápověda, co paleta umí. `Palette.tsx` je
// přidá k `buildItems`.
import type { ScriptView, Snapshot } from "../types";
import type { PItem } from "./items";
import type { Ranked } from "./search";
import { quick } from "./quick";
import { EMOJI_PREFIX } from "./emoji";
import { t, type Lang } from "../i18n";
import { runAny, scriptsApi } from "../actions/scripts";
import { askConfirm } from "../actions/ui";
import { winItems } from "./winsettings";
import { sayError } from "../ui/toast";

export function scriptItems(list: readonly ScriptView[]): PItem[] {
  return list.map((s) => ({
    id: s.id,
    section: "script",
    order: 1,
    title: s.label,
    sub: s.problem ?? s.description ?? s.target,
    keywords: `skript script ${s.target} ${s.group ?? ""} ${s.lang}`,
    action: s,
    run: () => runAny(s, askConfirm("palette")),
  }));
}

export function windowsItems(s: Snapshot, query: string): PItem[] {
  return winItems(s.paletteWinSettings !== false, query).map((w) => ({
    ...w,
    section: "windows",
    run: () => scriptsApi.winSettings(w.target).catch(sayError),
  }));
}

/**
 * Rychlé odpovědi pro dotaz: `only` = emoji režim (řádky nahradí běžný seznam), jinak nanejvýš jeden
 * řádek s výsledkem výpočtu / převodu nad běžnými výsledky. Enter na nich kopíruje (`PItem.copy`).
 */
export function quickRows(query: string, lang: Lang): { only: boolean; rows: Ranked<PItem>[] } {
  const r = quick(query, lang);
  if (!r) return { only: false, rows: [] };
  const noop = () => undefined;
  if (r.kind === "emoji") {
    // bez podtitulu: řádky emoji jsou husté, co dělá Enter říká patička
    return {
      only: true,
      rows: r.rows.map((x) => ({ item: { id: x.item.id, title: x.item.title, section: "emoji", glyph: x.item.emoji, copy: x.item.emoji, run: noop }, score: x.score, ranges: x.ranges })),
    };
  }
  const item: PItem = { id: `quick:${r.kind}`, title: r.title, sub: t("palette.quick.sub", "Enter zkopíruje výsledek ({value})", { value: r.copy }), section: r.kind, glyph: r.kind === "calc" ? "=" : "⇄", copy: r.copy, run: noop };
  return { only: false, rows: [{ item, score: 0, ranges: [] }] };
}

/** Příkaz „Emoji…“: přepne dotaz na `:` (paleta zůstane otevřená). */
export function emojiCommand(open: (query: string) => void): PItem {
  return { id: "app:emoji", section: "emoji", order: 6, glyph: "😀", stay: true, title: t("palette.emoji.open", "Emoji…"), sub: t("palette.emoji.openSub", "Nebo napiš : a slovo, třeba :srdce"), keywords: "emoji smajlik smajliky emotikon ikona symbol smiley", run: () => open(EMOJI_PREFIX) };
}

/** Nápověda pod prázdnou paletou: co umí kromě hledání. */
export const paletteTips = () => t("palette.tips", "Umím i počty (2^10), převody (10 km na mi) a emoji (:srdce)");
