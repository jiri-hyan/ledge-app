// Panel Vzhled, krok Otevřený → „Barvy“: galerie motivů (vestavěných i vlastních) jako barevné vzorníky,
// najetí = dočasný náhled na celém šuplíku (`apUi` `peekOn`), klik = výběr; pod ní akcent. Vlastní motivy
// (`appearance.customThemes`, logika `theme/customThemes.ts`): „Uložit aktuální barvy jako vlastní motiv“,
// editor 11 barev se živým náhledem a ŽIVOU kontrolou kontrastu textu (≥ 4.5:1, stejné funkce jako test
// motivů) — uložení s nízkým kontrastem je zablokované —, přejmenování, smazání a export / import jako JSON.
// Úpravy drží pracovní kopie (`work`); do rozpracovaného vzhledu se dostanou až tlačítkem „Uložit motiv“,
// do configu pak společnou lištou Nastavení.
import { For, Show, createEffect, createSignal, on, onCleanup } from "solid-js";
import { lang, t } from "../../i18n";
import type { Appearance, CustomTheme } from "../../types";
import { THEMES, THEME_IDS } from "../../theme/themes";
import { accentPair, isHex } from "../../theme/appearance";
import { scopeStyle } from "../../theme/apply";
import { lbl } from "../../theme/label";
import { contrast, parseColor, toHex } from "../../theme/contrast";
import {
  CUSTOM_KEYS, CUSTOM_TEXT_KEYS, MAX_CUSTOM_THEMES, MAX_NAME, addCustomTheme, customProblems, exportTheme, importTheme, themeOf, type Colors, type CustomKey,
} from "../../theme/customThemes";
import { dropThemeRefs } from "../../theme/pages";
import { clearPeek, lit, peekOn, setThemeDraft, withThemeDraft, type Ap } from "./apUi";

const ACCENTS: [string, string, string][] = [
  ["#8b5cf6", "appearance.accent.violet", "Fialová"],
  ["#3b82f6", "appearance.accent.blue", "Modrá"],
  ["#14b8a6", "appearance.accent.teal", "Tyrkysová"],
  ["#22c55e", "appearance.accent.green", "Zelená"],
  ["#f59e0b", "appearance.accent.amber", "Jantarová"],
  ["#ef4444", "appearance.accent.red", "Červená"],
  ["#ec4899", "appearance.accent.pink", "Růžová"],
];

const KEY_LABEL: Record<CustomKey, () => string> = {
  bg: () => t("appearance.ct.bg", "Pozadí"),
  raised: () => t("appearance.ct.raised", "Vyvýšené plochy"),
  t1: () => t("appearance.ct.t1", "Text"),
  t2: () => t("appearance.ct.t2", "Text tlumený"),
  t3: () => t("appearance.ct.t3", "Text slabý"),
  accent: () => t("appearance.ct.accent", "Akcent"),
  "accent-2": () => t("appearance.ct.accent2", "Akcent 2"),
  green: () => t("appearance.ct.green", "Úspěch"),
  amber: () => t("appearance.ct.amber", "Varování"),
  red: () => t("appearance.ct.red", "Chyba"),
  blue: () => t("appearance.ct.blue", "Info"),
};

const ratioText = (r: number) => `${lang() === "cs" ? r.toFixed(1).replace(".", ",") : r.toFixed(1)}:1`;

/** Barvy aktuálního vzhledu v tvaru editoru (včetně vlastního akcentu) — výchozí bod „Uložit jako vlastní motiv“. */
function currentColors(a: Appearance): Colors {
  const v = themeOf(a).vars;
  const out = Object.fromEntries(CUSTOM_KEYS.map((k) => [k, toHex(parseColor(v[k]))])) as Colors;
  if (isHex(a.accent)) {
    const [c1, c2] = accentPair(a.accent);
    out.accent = c1;
    out["accent-2"] = c2;
  }
  return out;
}

export default function AppearanceThemes(q: { ap: Ap }) {
  const a = () => q.ap.a();
  const set = q.ap.set;
  const sel = () => (a().customTheme ? a().customThemes.find((c) => c.id === a().customTheme) ?? null : null);
  const [work, setWork] = createSignal<CustomTheme | null>(null);
  const [io, setIo] = createSignal<"" | "export" | "import">("");
  const [text, setText] = createSignal("");
  const [msg, setMsg] = createSignal("");

  // jiný nebo změněný uložený motiv (výběr, Zahodit v liště) → pracovní kopie znovu
  createEffect(on(() => JSON.stringify(sel()), () => {
    const s = sel();
    setWork(s ? structuredClone(s) : null);
    setMsg("");
  }));
  const dirty = () => !!work() && JSON.stringify(work()) !== JSON.stringify(sel());
  createEffect(() => setThemeDraft(dirty() ? work() : null));
  onCleanup(() => setThemeDraft(null));
  const problems = () => (work() ? customProblems(work()!) : []);

  const view = () => withThemeDraft(a());
  const pickBuiltin = (id: Appearance["theme"]) => {
    clearPeek();
    q.ap.setAll({ ...a(), theme: id, customTheme: "" });
  };
  const pickCustom = (id: string) => {
    clearPeek();
    q.ap.setAll({ ...a(), customTheme: id });
  };
  const full = () => a().customThemes.length >= MAX_CUSTOM_THEMES;
  const create = () => {
    const name = t("appearance.ct.newName", "Vlastní motiv {n}", { n: a().customThemes.length + 1 });
    const r = addCustomTheme(a().customThemes, name, a().theme, currentColors(a()));
    if (r) q.ap.setAll({ ...a(), customThemes: r.list, customTheme: r.id });
  };
  const commit = () => {
    const w = work();
    if (!w || problems().length) return;
    q.ap.setAll({ ...a(), customThemes: a().customThemes.map((c) => (c.id === w.id ? w : c)) });
  };
  const remove = () => {
    const s = sel();
    if (s) q.ap.setAll(dropThemeRefs({ ...a(), customThemes: a().customThemes.filter((c) => c.id !== s.id) }, s.id));
  };
  const setColor = (k: CustomKey, hex: string) => isHex(hex) && setWork((w) => (w ? { ...w, colors: { ...w.colors, [k]: hex.toLowerCase() } } : w));
  const rename = (name: string) => setWork((w) => (w ? { ...w, name } : w));

  const doExport = async () => {
    const w = work();
    if (!w) return;
    const json = exportTheme(w);
    setText(json);
    setIo("export");
    try {
      await navigator.clipboard.writeText(json);
      setMsg(t("appearance.ct.copied", "Zkopírováno do schránky."));
    } catch {
      setMsg(t("appearance.ct.copyManual", "Schránka není dostupná — zkopíruj text níže."));
    }
  };
  const doImport = () => {
    const r = importTheme(text(), a().customThemes);
    if (r.ok) {
      q.ap.setAll({ ...a(), customThemes: [...a().customThemes, r.theme], customTheme: r.theme.id });
      setIo("");
      setText("");
      setMsg("");
      return;
    }
    setMsg(
      r.error === "json" ? t("appearance.ct.errJson", "Tohle není platný JSON.")
        : r.error === "full" ? t("appearance.ct.errFull", "Už máš {n} vlastních motivů — jeden smaž.", { n: MAX_CUSTOM_THEMES })
        : r.error === "contrast" ? t("appearance.ct.errContrast", "Text má málo kontrastu ({keys}) — motiv se nepřidá.", { keys: (r.keys ?? []).map((k) => KEY_LABEL[k]()).join(", ") })
        : t("appearance.ct.errFormat", "Není to motiv z Ledge (chybí značka ledgeTheme nebo některá z 11 barev)."),
    );
  };

  return (
    <div class="card ap-top" id="f-appearance.theme" classList={{ hit: lit("appearance.theme") }}>
      <h4>{t("appearance.theme.colors", "Barvy")}</h4>
      <div class="ap-themes" role="radiogroup" aria-label={t("appearance.theme.title", "Motiv")}>
        <For each={THEME_IDS}>
          {(id) => (
            <button class="ap-tile" classList={{ on: !a().customTheme && a().theme === id }} role="radio" aria-checked={!a().customTheme && a().theme === id}
              {...peekOn(() => ({ ...a(), theme: id, customTheme: "" }))} onClick={() => pickBuiltin(id)}>
              <span class="ap-swatch theme-scope" style={scopeStyle({ ...view(), theme: id, customTheme: "", glass: { opacity: 1 } })}>
                <i class="l1" /><i class="l2" /><i class="l3" /><i class="acc" />
              </span>
              <span class="ap-name">{lbl([THEMES[id].key, THEMES[id].cs])}</span>
            </button>
          )}
        </For>
        <For each={a().customThemes}>
          {(c) => (
            <button class="ap-tile" classList={{ on: a().customTheme === c.id }} role="radio" aria-checked={a().customTheme === c.id}
              {...peekOn(() => ({ ...a(), customTheme: c.id }))} onClick={() => pickCustom(c.id)}>
              <span class="ap-swatch theme-scope" style={scopeStyle({ ...view(), customTheme: c.id, glass: { opacity: 1 } })}>
                <i class="l1" /><i class="l2" /><i class="l3" /><i class="acc" />
              </span>
              <span class="ap-name">{view().customThemes.find((x) => x.id === c.id)?.name ?? c.name}<small class="ap-own">{t("appearance.ct.own", "vlastní")}</small></span>
            </button>
          )}
        </For>
      </div>

      <div class="ap-tbar">
        <button class="mini" disabled={full()} title={full() ? t("appearance.ct.full", "Už máš {n} vlastních motivů.", { n: MAX_CUSTOM_THEMES }) : undefined} onClick={create}>
          {t("appearance.ct.save", "Uložit aktuální barvy jako vlastní motiv")}
        </button>
        <button class="mini" classList={{ "ap-chip-on": io() === "import" }} aria-pressed={io() === "import"} onClick={() => { setIo(io() === "import" ? "" : "import"); setText(""); setMsg(""); }}>
          {t("appearance.ct.import", "Importovat motiv…")}
        </button>
      </div>

      <Show when={io() === "import"}>
        <div class="ap-tio">
          <textarea class="ap-tarea" rows={5} value={text()} placeholder={t("appearance.ct.pasteHint", "Sem vlož JSON motivu (z „Exportovat“ u jiného motivu).")} aria-label={t("appearance.ct.import", "Importovat motiv…")}
            onInput={(e) => { setText(e.currentTarget.value); setMsg(""); }} />
          <button class="mini" disabled={!text().trim()} onClick={doImport}>{t("appearance.ct.add", "Přidat motiv")}</button>
        </div>
      </Show>

      <Show when={work()}>
        {(w) => (
          <div class="ap-ted" role="group" aria-label={t("appearance.ct.editor", "Úprava vlastního motivu")}>
            <div class="ap-tname">
              <input type="text" value={w().name} maxLength={MAX_NAME} aria-label={t("appearance.ct.name", "Název motivu")} onInput={(e) => rename(e.currentTarget.value)} />
              <button class="mini" onClick={doExport}>{t("appearance.ct.export", "Exportovat")}</button>
              <button class="mini" onClick={remove}>{t("appearance.ct.delete", "Smazat")}</button>
            </div>
            <div class="ap-tcolors">
              <For each={CUSTOM_KEYS}>
                {(k) => {
                  const bad = () => problems().find((p) => p.key === k);
                  const ratio = () => contrast(parseColor(w().colors[k]), parseColor(w().colors.bg));
                  return (
                    <label class="ap-tcolor" classList={{ bad: !!bad() }}>
                      <input type="color" value={w().colors[k]} aria-label={KEY_LABEL[k]()} onInput={(e) => setColor(k, e.currentTarget.value)} />
                      <span class="ap-tkey">{KEY_LABEL[k]()}</span>
                      <Show when={CUSTOM_TEXT_KEYS.includes(k)}>
                        <span class="ap-ratio" classList={{ warn: !!bad() }} title={t("appearance.ct.ratioHint", "Kontrast textu na pozadí; potřeba aspoň 4,5:1")}>{ratioText(bad()?.ratio ?? ratio())}</span>
                      </Show>
                    </label>
                  );
                }}
              </For>
            </div>
            <Show when={problems().length}>
              <p class="ap-note warn" role="alert">
                {t("appearance.ct.lowContrast", "Málo kontrastu u: {keys}. Uprav barvy (aspoň 4,5:1 na pozadí), pak jde motiv uložit.", { keys: problems().map((p) => KEY_LABEL[p.key]()).join(", ") })}
              </p>
            </Show>
            <div class="ap-bar">
              <span class="ap-state" classList={{ warn: dirty() }}>{dirty() ? t("appearance.ct.unsaved", "Úpravy motivu ještě nejsou uložené do vzhledu.") : t("appearance.ct.clean", "Motiv odpovídá vzhledu.")}</span>
              <button class="mini" disabled={!dirty()} onClick={() => setWork(structuredClone(sel()))}>{t("appearance.ct.revert", "Zrušit úpravy")}</button>
              <button class="btn pri" disabled={!dirty() || problems().length > 0} onClick={commit}>{t("appearance.ct.commit", "Uložit motiv")}</button>
            </div>
          </div>
        )}
      </Show>

      <Show when={io() === "export"}>
        <textarea class="ap-tarea" rows={5} readOnly value={text()} aria-label={t("appearance.ct.export", "Exportovat")} onFocus={(e) => e.currentTarget.select()} />
      </Show>
      <Show when={msg()}><p class="ap-note" role="status">{msg()}</p></Show>

      <div class="opt" id="f-appearance.accent" classList={{ hit: lit("appearance.accent") }}>
        <span>{t("appearance.accent.title", "Akcent")}<small>{t("appearance.accent.hint", "Linka, záře a fokus. Výchozí = barva motivu.")}</small></span>
        <span class="ap-accents">
          <button class="ap-acc def" classList={{ on: !a().accent }} title={t("appearance.accent.default", "Podle motivu")} aria-label={t("appearance.accent.default", "Podle motivu")} onClick={() => set("accent", "")} />
          <For each={ACCENTS}>
            {([hex, key, cs]) => (
              <button class="ap-acc" classList={{ on: a().accent === hex }} style={{ background: hex }} title={lbl([key, cs])} aria-label={lbl([key, cs])} onClick={() => set("accent", hex)} />
            )}
          </For>
          <label class="ap-acc own" classList={{ on: !!a().accent && !ACCENTS.some(([h]) => h === a().accent) }} title={t("appearance.accent.custom", "Vlastní barva")}>
            <input type="color" value={a().accent || "#a78bfa"} aria-label={t("appearance.accent.custom", "Vlastní barva")} onInput={(e) => isHex(e.currentTarget.value) && set("accent", e.currentTarget.value)} />
          </label>
        </span>
      </div>
    </div>
  );
}
