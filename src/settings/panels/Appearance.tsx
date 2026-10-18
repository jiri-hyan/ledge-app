// Panel Nastavení „Vzhled“: nahoře galerie design stylů (`AppearanceStyles.tsx`), živý
// náhled, barvy (motiv + akcent), šířka a rozložení (`AppearanceLayout.tsx`), animace s ukázkami
// (`AppearanceMotion.tsx`), CELÝ proužek — vzhled (styl, velikost, pozice, linka, krytí) a hned pod ním
// co ukazuje (`StripSettings.tsx`: položky, barvy, klid, vlastní text, monitor) —, rozbalovací „Upravit
// detaily“ (písmo, materiál, hustota, rohy, pozadí), jazyk. Pořadí a velikost karet je v `layout` (panel Šuplík a karty). Každá změna se hned náhledově aplikuje na celý šuplík (`applyAppearance`);
// odchod bez uložení vrátí uložený vzhled. Ukládá JEN společná lišta Nastavení (Uložit / Zahodit, jako
// všechny panely — vlastní tlačítko tu není) → Rust pošle `appearance` a překreslí
// proužek. Logika (normalizace, proměnné) je v `theme/appearance.ts`. Pozadí (`drawer.glass`),
// šířka (`drawer.width`) a animace (`appearance.motion`, řídí i vysunutí okna v Rustu) se nastavují JEN
// tady — panel Chování má jen chování. Kotvy hledání `f-<id>` = klíče z `labels.ts`.
// Jazyk rozhraní (`ui.lang`) je tu taky: patří k tomu, jak UI vypadá (jako motiv a písmo), samostatný
// panel „Obecné“ pro jednu volbu nemáme. Přepne se po Uložit (`ctx.loadBase` → `setLang`).
import { For, Show, createEffect, onCleanup } from "solid-js";
import type { PanelProps } from "../types";
import type { Appearance, StripStyle } from "../../types";
import { t, type Lang } from "../../i18n";
import { THEMES, THEME_IDS } from "../../theme/themes";
import { DEFAULT_APPEARANCE, STRIP_STYLES, isHex, normalizeAppearance } from "../../theme/appearance";
import { applyAppearance, savedAppearance, scopeStyle } from "../../theme/apply";
import { fieldHint, fieldLabel } from "../labels";
import AppearancePreview from "./AppearancePreview";
import AppearanceStyles from "./AppearanceStyles";
import AppearanceMotion from "./AppearanceMotion";
import StripSettings from "./StripSettings";
import { DetailsCard, LayoutCard } from "./AppearanceLayout";
import { Seg, Slider, lit, pct, type Ap } from "./apUi";
import "./appearance.css";

const ACCENTS: [string, string, string][] = [
  ["#8b5cf6", "appearance.accent.violet", "Fialová"],
  ["#3b82f6", "appearance.accent.blue", "Modrá"],
  ["#14b8a6", "appearance.accent.teal", "Tyrkysová"],
  ["#22c55e", "appearance.accent.green", "Zelená"],
  ["#f59e0b", "appearance.accent.amber", "Jantarová"],
  ["#ef4444", "appearance.accent.red", "Červená"],
  ["#ec4899", "appearance.accent.pink", "Růžová"],
];
/** Výchozí krytí proužku (`strip.opacity`, config.rs `Default`). */
const STRIP_OPACITY = 0.82;
const STRIP_LABEL: Record<StripStyle, () => string> = {
  pill: () => t("appearance.strip.pill", "Pilulka"),
  line: () => t("appearance.strip.line", "Pruh"),
  dot: () => t("appearance.strip.dot", "Tečky"),
  term: () => t("appearance.strip.term", "Terminál"),
  tag: () => t("appearance.strip.tag", "Štítek"),
  glow: () => t("appearance.strip.glow", "Neon"),
  block: () => t("appearance.strip.block", "Blok"),
  drop: () => t("appearance.strip.drop", "Kapka"),
};

/** Názvy jazyků vždy v jejich vlastním jazyce (ať se k češtině vrátí i ten, kdo anglicky neumí). */
const LANGS: [Lang, string][] = [["cs", "Čeština"], ["en", "English"]];

export default function AppearancePanel(p: PanelProps) {
  const a = (): Appearance => normalizeAppearance(p.cfg.appearance);
  /** Změna jednoho pole vzhledu (chybí-li sekce ve starém configu, založí ji s výchozími hodnotami). */
  function set<K extends keyof Appearance>(k: K, v: Appearance[K]) {
    if (!p.cfg.appearance) p.setCfg("appearance", structuredClone(DEFAULT_APPEARANCE));
    p.setCfg("appearance", k, structuredClone(v));
  }
  const setStrip = (patch: Partial<Appearance["strip"]>) => set("strip", { ...a().strip, ...patch });

  // náhled na celém šuplíku hned; odchod bez uložení → zpět uložený vzhled
  createEffect(() => applyAppearance(a()));
  onCleanup(() => applyAppearance(savedAppearance()));

  const dirty = () => JSON.stringify(a()) !== JSON.stringify(savedAppearance());
  const reduced = typeof matchMedia === "function" && matchMedia("(prefers-reduced-motion: reduce)").matches;
  const blur = () => p.cfg.drawer?.glass === "acrylic";
  const stripOpacity = () => p.cfg.strip?.opacity ?? STRIP_OPACITY;
  const ap: Ap = {
    p,
    a,
    set,
    setAll: (next) => p.setCfg("appearance", structuredClone(next)),
    blur,
    // jen skutečná změna (výběr stylu bez rozmazání nezapíše `drawer.glass`, který v configu chybí)
    setBlur: (on) => {
      if (blur() !== on) p.setCfg("drawer", (d: any) => ({ ...(d ?? {}), glass: on ? "acrylic" : "solid" }));
    },
  };
  const uiLang = (): Lang => (p.cfg.ui?.lang === "en" ? "en" : "cs");
  const setUiLang = (v: Lang) => (p.cfg.ui ? p.setCfg("ui", "lang", v) : p.setCfg("ui", { lang: v }));

  return (
    <div class="ap">
      <AppearanceStyles ap={ap} dirty={dirty()} />
      <div class="card ap-top">
        <AppearancePreview a={a()} stripOpacity={stripOpacity()} blur={blur()} />
      </div>

      <div class="card" id="f-appearance.theme" classList={{ hit: lit("appearance.theme") }}>
        <h4>{t("appearance.theme.colors", "Barvy")}</h4>
        <div class="ap-themes" role="radiogroup" aria-label={t("appearance.theme.title", "Motiv")}>
          <For each={THEME_IDS}>
            {(id) => (
              <button class="ap-tile" classList={{ on: a().theme === id }} role="radio" aria-checked={a().theme === id} onClick={() => set("theme", id)}>
                <span class="ap-swatch theme-scope" style={scopeStyle({ ...a(), theme: id, glass: { opacity: 1 } })}>
                  <i class="l1" /><i class="l2" /><i class="l3" /><i class="acc" />
                </span>
                <span class="ap-name">{t(THEMES[id].key, THEMES[id].cs)}</span>
              </button>
            )}
          </For>
        </div>
        <div class="opt" id="f-appearance.accent" classList={{ hit: lit("appearance.accent") }}>
          <span>{t("appearance.accent.title", "Akcent")}<small>{t("appearance.accent.hint", "Linka, záře a fokus. Výchozí = barva motivu.")}</small></span>
          <span class="ap-accents">
            <button class="ap-acc def" classList={{ on: !a().accent }} title={t("appearance.accent.default", "Podle motivu")} aria-label={t("appearance.accent.default", "Podle motivu")} onClick={() => set("accent", "")} />
            <For each={ACCENTS}>
              {([hex, key, cs]) => (
                <button class="ap-acc" classList={{ on: a().accent === hex }} style={{ background: hex }} title={t(key, cs)} aria-label={t(key, cs)} onClick={() => set("accent", hex)} />
              )}
            </For>
            <label class="ap-acc own" classList={{ on: !!a().accent && !ACCENTS.some(([h]) => h === a().accent) }} title={t("appearance.accent.custom", "Vlastní barva")}>
              <input type="color" value={a().accent || "#a78bfa"} aria-label={t("appearance.accent.custom", "Vlastní barva")} onInput={(e) => isHex(e.currentTarget.value) && set("accent", e.currentTarget.value)} />
            </label>
          </span>
        </div>
      </div>

      <LayoutCard ap={ap} />
      <AppearanceMotion ap={ap} reduced={reduced} />

      <div class="card" id="f-appearance.strip" classList={{ hit: lit("appearance.strip") }}>
        <h4>{t("appearance.strip.title", "Vzhled proužku")}</h4>
        <div class="opt">
          <span>{t("appearance.strip.style", "Styl")}<small>{t("appearance.strip.styleHint", "„Tečky“ ukazují jen stavy, bez textu a času.")}</small></span>
          <Seg label={t("appearance.strip.style", "Styl")} value={a().strip.style} set={(v) => setStrip({ style: v })}
            options={STRIP_STYLES.map((x) => [x, STRIP_LABEL[x]()])} />
        </div>
        <div class="opt">
          <span>{t("appearance.strip.size", "Velikost")}</span>
          <Seg label={t("appearance.strip.size", "Velikost")} value={a().strip.size} set={(v) => setStrip({ size: v })}
            options={[["small", t("appearance.strip.small", "Malý")], ["normal", t("appearance.strip.normal", "Střední")], ["large", t("appearance.strip.large", "Velký")]]} />
        </div>
        <div class="opt">
          <span>{t("appearance.strip.position", "Pozice")}<small>{t("appearance.strip.positionHint", "Proužek i šuplík u horní hrany; myš otevírá tam, kde proužek je.")}</small></span>
          <Seg label={t("appearance.strip.position", "Pozice")} value={a().strip.position} set={(v) => setStrip({ position: v })}
            options={[["left", t("appearance.strip.left", "Vlevo")], ["center", t("appearance.strip.center", "Uprostřed")], ["right", t("appearance.strip.right", "Vpravo")]]} />
        </div>
        <div class="opt">
          <span>{t("appearance.strip.color", "Linka dole")}</span>
          <span class="ap-accents">
            <Seg label={t("appearance.strip.color", "Linka dole")} value={a().strip.color === "" || a().strip.color === "accent" ? a().strip.color : "own"}
              set={(v) => setStrip({ color: v === "own" ? (isHex(a().strip.color) ? a().strip.color : "#ffffff") : v })}
              options={[["", t("appearance.strip.lineGradient", "Přechod")], ["accent", t("appearance.strip.lineAccent", "Akcent")], ["own", t("appearance.strip.lineOwn", "Vlastní")]]} />
            <Show when={isHex(a().strip.color)}>
              <label class="ap-acc own on"><input type="color" value={a().strip.color} aria-label={t("appearance.strip.lineOwn", "Vlastní")} onInput={(e) => setStrip({ color: e.currentTarget.value })} /></label>
            </Show>
          </span>
        </div>
        <Slider label={t("appearance.strip.opacity", "Krytí proužku")} min={0.3} max={1} step={0.05} value={stripOpacity()} fmt={pct} set={(v) => p.setCfg("strip", "opacity", +v.toFixed(2))} />
      </div>
      <div class="ap-wide"><StripSettings /></div>

      <DetailsCard ap={ap} />

      <div class="card ap-lang" id="f-ui.lang" classList={{ hit: lit("ui.lang") }}>
        <div class="opt">
          <span>{fieldLabel("ui.lang")}<small>{fieldHint("ui.lang")}</small></span>
          <Seg<Lang> label={fieldLabel("ui.lang")} value={uiLang()} set={setUiLang} options={LANGS} />
        </div>
      </div>

    </div>
  );
}
