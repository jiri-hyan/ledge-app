// Studio → Vzhled šuplíku → Akcent, rozšíření kola 25 (pod výběrem akcentu v `AppearanceThemes.tsx`): gradientní
// akcent (druhá barva → `--accent-grad` na zvýraznění, aktivní prvky a hlavičky karet), „akcent podle pozadí“
// (barvy se vezmou z vybraného gradientu a doladí na čitelnost) a živá kontrola kontrastu akcentu na kartách
// (`theme/backdrop.ts` `accentContrast`, stejné funkce jako test). Výchozí: vše vypnuto = dnešní akcent.
import { Show } from "solid-js";
import { t } from "../../i18n";
import { accentContrast, gradientOf, normalizeBackdrop } from "../../theme/backdrop";
import { appearanceVars, isHex } from "../../theme/appearance";
import { themeOf } from "../../theme/customThemes";
import { Toggle } from "../fields/controls";
import { type Ap } from "./apUi";

export default function AppearanceAccent(q: { ap: Ap }) {
  const a = () => q.ap.a();
  const vars = () => appearanceVars(a());
  const hasGrad = () => !!gradientOf(normalizeBackdrop(a().backdrop));
  const base = () => themeOf(a()).vars;
  const ratio = () => accentContrast(vars().accent ?? base().accent, base());
  return (
    <div class="card ap-top">
      <h4>{t("appearance.acg.grad.title", "Akcent: gradient a podle pozadí")}</h4>
      <div class="opt">
        <span>{t("appearance.acg.fromBd", "Akcent podle pozadí")}<small>{hasGrad() ? t("appearance.acg.fromBd.hint", "Barvy akcentu se vezmou z vybraného gradientu a doladí, aby byly na kartách čitelné.") : t("appearance.acg.fromBd.none", "Nejdřív vyber gradient v Pozadí šuplíku výše.")}</small></span>
        <Toggle label={t("appearance.acg.fromBd", "Akcent podle pozadí")} value={!!a().accentFromBackdrop} onChange={(v) => q.ap.set("accentFromBackdrop", v)} disabled={!hasGrad() && !a().accentFromBackdrop} />
      </div>
      <div class="opt">
        <span>{t("appearance.acg.second", "Druhá barva akcentu")}<small>{t("appearance.acg.second.hint", "Z akcentu a druhé barvy vznikne gradient: podtržení nadpisů karet, aktivní volby, ukazatele postupu. Prázdné = jen jedna barva.")}</small></span>
        <span class="gd-stop">
          <span class="ac-prev" style={{ background: `linear-gradient(90deg, ${vars().accent ?? base().accent}, ${isHex(a().accent2) ? a().accent2 : (vars()["accent-2"] ?? base()["accent-2"])})` }} aria-hidden="true" />
          <input type="color" value={isHex(a().accent2) ? a().accent2! : (vars()["accent-2"] ?? "#60a5fa")} aria-label={t("appearance.acg.second", "Druhá barva akcentu")}
            onInput={(e) => {
              if (!isHex(e.currentTarget.value)) return;
              if (!a().accent) q.ap.set("accent", vars().accent ?? base().accent);
              q.ap.set("accent2", e.currentTarget.value);
            }} />
          <button class="btn" disabled={!a().accent2} onClick={() => q.ap.set("accent2", "")}>{t("appearance.acg.second.clear", "Zrušit")}</button>
        </span>
      </div>
      <Show when={ratio() < 3}>
        <p class="ap-note" role="status">{t("appearance.acg.lowContrast", "Tahle barva akcentu je na kartách hůř vidět (kontrast {r}:1, doporučeno aspoň 3). Zkus světlejší nebo sytější.", { r: ratio().toFixed(1) })}</p>
      </Show>
    </div>
  );
}
