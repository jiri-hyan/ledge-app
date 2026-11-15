// Cesta „Nastavení → Stránka → Sekce“ do hlášek: skládá se ze SKUTEČNÝCH názvů stránek (`registry.ts` `PANEL_META`)
// a sekcí (`SECTIONS` níže), v jazyce UI. Jeden zdroj proti dvojímu slovníku (Studio části × stránky) a proti
// ručně psaným, zastaralým cestám (kolo 33, L107). Pravidlo pro `t()` text: `{path}` + `settingsPath(...)` místo
// literálu „Nastavení → …“; čisté moduly bez `t()` (např. `layout/hero.ts`) píší literál a hlídá ho
// `settings/path.test.mjs` (stejně jako Rust texty).
import { t } from "../i18n";
import { PANEL_META, SECTIONS, type SectionId } from "./registry";

/** „Nastavení → Obsah karet → Kalendáře“: `panel` = id stránky z `PANEL_META`, pak sekce (po jedné i víc). */
export function settingsPath(panel: string, ...subs: SectionId[]): string {
  const m = PANEL_META.find((p) => p.id === panel);
  if (!m) throw new Error(`settingsPath: unknown panel ${panel}`);
  const parts = [t("settings.path.root", "Nastavení"), t(m.key, m.cs), ...subs.map((s) => t(SECTIONS[s].key, SECTIONS[s].cs))];
  return parts.join(" → ");
}
