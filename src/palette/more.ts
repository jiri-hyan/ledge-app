// Položky palety mimo snapshot: skripty ze složky skriptů (`scripts_list` při otevření palety) a
// stránky Nastavení Windows (`winsettings.ts`, jen s dotazem). `Palette.tsx` je přidá k `buildItems`.
import type { ScriptView, Snapshot } from "../types";
import type { PItem } from "./items";
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
