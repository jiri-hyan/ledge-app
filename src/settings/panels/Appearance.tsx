// Panel Nastavení „Vzhled a animace“: motiv (galerie s náhledy), akcent, hustota, písmo, rohy, krytí
// a rozmazání pozadí, šířka šuplíku, styl a rychlost animací, proužek (styl, velikost, pozice, linka,
// krytí), pořadí karet. Každá změna se hned náhledově aplikuje na celý šuplík (`applyAppearance`);
// odchod bez uložení vrátí uložený vzhled. Ukládá JEN společná lišta Nastavení (Uložit / Zahodit, jako
// všechny panely — vlastní tlačítko tu není) → Rust pošle `appearance` a překreslí
// proužek. Logika (normalizace, pořadí, proměnné) je v `theme/appearance.ts`. Pozadí (`drawer.glass`),
// šířka (`drawer.width`) a animace (`appearance.motion`, řídí i vysunutí okna v Rustu) se nastavují JEN
// tady — panel Šuplík a proužek má jen chování. Kotvy hledání `f-<id>` = klíče z `labels.ts`.
// Jazyk rozhraní (`ui.lang`) je tu taky: patří k tomu, jak UI vypadá (jako motiv a písmo), samostatný
// panel „Obecné“ pro jednu volbu nemáme. Přepne se po Uložit (`ctx.loadBase` → `setLang`).
import { For, Show, createEffect, onCleanup } from "solid-js";
import type { PanelProps } from "../types";
import type { Appearance, MotionStyle } from "../../types";
import { lang, t, type Lang } from "../../i18n";
import { THEMES, THEME_IDS } from "../../theme/themes";
import { DEFAULT_APPEARANCE, cardOrder, isHex, normalizeAppearance, type CardId } from "../../theme/appearance";
import { applyAppearance, savedAppearance, scopeStyle } from "../../theme/apply";
import { hit } from "../ctx";
import { fieldHint, fieldLabel } from "../labels";
import AppearancePreview from "./AppearancePreview";
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
const CARD_LABEL: Record<CardId, [string, string]> = {
  running: ["appearance.cards.running", "Běží"],
  claude: ["appearance.cards.claude", "Claude"],
  repos: ["appearance.cards.repos", "Repa"],
  notes: ["appearance.cards.notes", "Poznámky"],
};
/** Kotva hledání je `id="f-<klíč>"`; tohle = krátké zvýraznění po skoku z hledání (jako `Row`). */
const lit = (id: string) => hit() === id;
/** Výchozí hodnoty polí mimo `appearance`, která panel také edituje (config.rs `Default`). */
const DRAWER_WIDTH = 1080;
const STRIP_OPACITY = 0.82;

function Seg<T extends string>(q: { label: string; value: T; options: [T, string][]; set: (v: T) => void }) {
  return (
    <div class="seg" role="radiogroup" aria-label={q.label}>
      <For each={q.options}>
        {([v, l]) => (
          <button classList={{ on: q.value === v }} role="radio" aria-checked={q.value === v} onClick={() => q.set(v)}>{l}</button>
        )}
      </For>
    </div>
  );
}

function Slider(q: { label: string; hint?: string; anchor?: string; min: number; max: number; step: number; value: number; fmt: (v: number) => string; set: (v: number) => void }) {
  return (
    <div class="opt" id={q.anchor ? `f-${q.anchor}` : undefined} classList={{ hit: !!q.anchor && lit(q.anchor) }}>
      <span>{q.label}<Show when={q.hint}><small>{q.hint}</small></Show></span>
      <span class="ap-range">
        <input type="range" min={q.min} max={q.max} step={q.step} value={q.value} aria-label={q.label} onInput={(e) => q.set(+e.currentTarget.value)} />
        <output>{q.fmt(q.value)}</output>
      </span>
    </div>
  );
}

const pct = (v: number) => (lang() === "cs" ? `${Math.round(v * 100)} %` : `${Math.round(v * 100)}%`);
const times = (v: number) => (lang() === "cs" ? `${v.toFixed(1).replace(".", ",")}×` : `${v.toFixed(1)}×`);
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
  const setMotion = (patch: Partial<Appearance["motion"]>) => set("motion", { ...a().motion, ...patch });

  // náhled na celém šuplíku hned; odchod bez uložení → zpět uložený vzhled
  createEffect(() => applyAppearance(a()));
  onCleanup(() => applyAppearance(savedAppearance()));

  const dirty = () => JSON.stringify(a()) !== JSON.stringify(savedAppearance());
  const reduced = typeof matchMedia === "function" && matchMedia("(prefers-reduced-motion: reduce)").matches;
  const blur = () => p.cfg.drawer?.glass === "acrylic";
  const width = () => p.cfg.drawer?.width ?? DRAWER_WIDTH;
  const stripOpacity = () => p.cfg.strip?.opacity ?? STRIP_OPACITY;

  function reset() {
    p.setCfg("appearance", structuredClone(DEFAULT_APPEARANCE));
    p.setCfg("drawer", "width", DRAWER_WIDTH);
    p.setCfg("drawer", "glass", "solid");
    p.setCfg("strip", "opacity", STRIP_OPACITY);
  }
  function move(order: CardId[], i: number, d: -1 | 1) {
    const next = [...order];
    [next[i], next[i + d]] = [next[i + d], next[i]];
    set("cards", next);
  }
  const hidden = (id: CardId) => p.cfg.drawer?.sections?.[id] === false;
  const uiLang = (): Lang => (p.cfg.ui?.lang === "en" ? "en" : "cs");
  const setUiLang = (v: Lang) => (p.cfg.ui ? p.setCfg("ui", "lang", v) : p.setCfg("ui", { lang: v }));

  return (
    <div class="ap">
      <div class="card ap-top">
        <AppearancePreview a={a()} stripOpacity={stripOpacity()} blur={blur()} />
        <div class="ap-bar">
          <Show when={dirty()} fallback={<span class="ap-state">{t("appearance.state.saved", "Odpovídá uloženému vzhledu.")}</span>}>
            <span class="ap-state warn">{t("appearance.state.dirty", "Náhled — ulož nebo zahoď dole v liště.")}</span>
          </Show>
          <button class="mini" onClick={reset}>{t("appearance.reset", "Obnovit vzhled")}</button>
        </div>
      </div>

      <div class="card ap-lang" id="f-ui.lang" classList={{ hit: lit("ui.lang") }}>
        <div class="opt">
          <span>{fieldLabel("ui.lang")}<small>{fieldHint("ui.lang")}</small></span>
          <Seg<Lang> label={fieldLabel("ui.lang")} value={uiLang()} set={setUiLang} options={LANGS} />
        </div>
      </div>

      <div class="card" id="f-appearance.theme" classList={{ hit: lit("appearance.theme") }}>
        <h4>{t("appearance.theme.title", "Motiv")}</h4>
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

      <div class="card" id="f-appearance.layout" classList={{ hit: lit("appearance.layout") }}>
        <h4>{t("appearance.layout.title", "Písmo a rozložení")}</h4>
        <div class="opt">
          <span>{t("appearance.density", "Hustota")}</span>
          <Seg label={t("appearance.density", "Hustota")} value={a().density} set={(v) => set("density", v)}
            options={[["compact", t("appearance.density.compact", "Úsporná")], ["normal", t("appearance.density.normal", "Běžná")], ["comfy", t("appearance.density.comfy", "Vzdušná")]]} />
        </div>
        <Slider label={t("appearance.font", "Velikost písma")} min={0.85} max={1.25} step={0.05} value={a().fontScale} fmt={pct} set={(v) => set("fontScale", +v.toFixed(2))} />
        <div class="opt">
          <span>{t("appearance.radius", "Rohy")}</span>
          <Seg label={t("appearance.radius", "Rohy")} value={a().radius} set={(v) => set("radius", v)}
            options={[["sharp", t("appearance.radius.sharp", "Ostré")], ["normal", t("appearance.radius.normal", "Běžné")], ["round", t("appearance.radius.round", "Kulaté")]]} />
        </div>
        <Slider anchor="drawer.width" label={t("appearance.width", "Šířka šuplíku")} hint={t("appearance.width.hint", "Na menším monitoru se ořízne na 95 % šířky.")}
          min={720} max={2400} step={20} value={width()} fmt={(v) => `${v} px`} set={(v) => p.setCfg("drawer", "width", v)} />
      </div>

      <div class="card" id="f-drawer.glass" classList={{ hit: lit("drawer.glass") }}>
        <h4>{t("appearance.glass.title", "Pozadí šuplíku")}</h4>
        <Slider label={t("appearance.glass.opacity", "Krytí pozadí")} hint={t("appearance.glass.opacityHint", "Méně = víc prosvítá plocha za šuplíkem.")}
          min={0.25} max={1} step={0.05} value={a().glass.opacity} fmt={pct} set={(v) => set("glass", { opacity: +v.toFixed(2) })} />
        <div class="opt">
          <span>{t("appearance.glass.blur", "Rozmazat plochu za šuplíkem")}<small>{t("appearance.glass.blurHint", "Akryl Windows 11 — dražší na výkon a šuplík místo morfu z proužku jen vyjede shora.")}</small></span>
          <button class="switch" classList={{ on: blur() }} role="switch" aria-checked={blur()} aria-label={t("appearance.glass.blur", "Rozmazat plochu za šuplíkem")}
            onClick={() => p.setCfg("drawer", "glass", blur() ? "solid" : "acrylic")} />
        </div>
      </div>

      <div class="card" id="f-appearance.motion" classList={{ hit: lit("appearance.motion") }}>
        <h4>{t("appearance.motion.title", "Animace")}</h4>
        <div class="opt">
          <span>{t("appearance.motion.style", "Styl")}<Show when={reduced}><small class="warn">{t("appearance.motion.reduced", "Windows má zapnuté omezení animací — Ledge je proto nepřehrává.")}</small></Show>
            <Show when={!reduced && a().motion.style === "none"}><small>{t("appearance.motion.noneHint", "Šuplík se ukáže a schová hned, bez vysunutí; Nastavení se rozšíří bez přechodu.")}</small></Show></span>
          <Seg<MotionStyle> label={t("appearance.motion.style", "Styl")} value={a().motion.style} set={(v) => setMotion({ style: v })}
            options={[["spring", t("appearance.motion.spring", "Pružina")], ["smooth", t("appearance.motion.smooth", "Plynulý")], ["snappy", t("appearance.motion.snappy", "Svižný")], ["none", t("appearance.motion.none", "Bez animací")]]} />
        </div>

        <Show when={a().motion.style !== "none"}>
          <Slider label={t("appearance.motion.speed", "Rychlost")} min={0.5} max={2} step={0.1} value={a().motion.speed} fmt={times} set={(v) => setMotion({ speed: +v.toFixed(1) })} />
        </Show>
      </div>

      <div class="card" id="f-appearance.strip" classList={{ hit: lit("appearance.strip") }}>
        <h4>{t("appearance.strip.title", "Vzhled proužku")}</h4>
        <div class="opt">
          <span>{t("appearance.strip.style", "Styl")}<small>{t("appearance.strip.styleHint", "„Tečky“ ukazují jen stavy, bez textu a času.")}</small></span>
          <Seg label={t("appearance.strip.style", "Styl")} value={a().strip.style} set={(v) => setStrip({ style: v })}
            options={[["pill", t("appearance.strip.pill", "Pilulka")], ["line", t("appearance.strip.line", "Pruh")], ["dot", t("appearance.strip.dot", "Tečky")]]} />
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

      <div class="card" id="f-appearance.cards" classList={{ hit: lit("appearance.cards") }}>
        <h4>{t("appearance.cards.title", "Pořadí karet na hlavní stránce")}</h4>
        <For each={cardOrder(a().cards)}>
          {(id, i) => (
            <div class="opt">
              <span>{t(CARD_LABEL[id][0], CARD_LABEL[id][1])}<Show when={hidden(id)}><small>{t("appearance.cards.hidden", "skrytá — zapneš ji v Šuplík a proužek → Karty na hlavní stránce")}</small></Show></span>
              <span class="ap-move">
                <button class="mini" disabled={i() === 0} aria-label={t("appearance.cards.up", "Posunout výš")} onClick={() => move(cardOrder(a().cards), i(), -1)}>↑</button>
                <button class="mini" disabled={i() === 3} aria-label={t("appearance.cards.down", "Posunout níž")} onClick={() => move(cardOrder(a().cards), i(), 1)}>↓</button>
              </span>
            </div>
          )}
        </For>
      </div>
    </div>
  );
}
