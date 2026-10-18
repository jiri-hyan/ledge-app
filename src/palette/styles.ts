// Paleta příkazů: „Styl: Neon“ atd. — rychlé přepnutí design stylu (`theme/looks.ts` STYLES) bez
// Nastavení. Uloží config hned (jako ⋯ „Skrýt kartu“): `get_config` → `configWithStyle` → `save_config`;
// Rust pošle událost `appearance` a šuplík se přebarví bez restartu. S chybou v configu jen hláška.
import { api, type Snapshot } from "../api";
import type { PItem } from "./items";
import { STYLES } from "../theme/looks";
import { configWithStyle } from "../theme/styles";
import { savedAppearance } from "../theme/apply";
import { lbl } from "../theme/label";
import { say, sayError } from "../ui/toast";
import { t } from "../i18n";

async function pickStyle(id: string, name: string, cfgError: string | null) {
  if (cfgError) return say(t("palette.style.cfgError", "Config má chybu — styl změníš v Nastavení."), true);
  try {
    const cfg = await api.getConfig();
    if (!cfg) return;
    await api.saveConfig(configWithStyle(cfg, id));
    say(t("palette.style.done", "Styl {name} použit.", { name }));
  } catch (e) {
    sayError(e);
  }
}

export function styleItems(s: Snapshot): PItem[] {
  const now = savedAppearance().style;
  return STYLES.map((st) => {
    const name = lbl(st.name);
    return {
      id: `style:${st.id}`,
      section: "app",
      order: 6,
      title: t("palette.style.item", "Styl: {name}", { name }),
      sub: st.id === now ? t("palette.style.current", "aktuální") : lbl(st.desc),
      keywords: `styl style vzhled design motiv ${st.id}`,
      run: () => pickStyle(st.id, name, s.cfgError),
    };
  });
}
