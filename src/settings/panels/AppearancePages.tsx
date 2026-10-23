// Panel Vzhled, krok Otevřený → „Vzhled záložek“: každá záložka šuplíku (Práce, Osobní, vlastní) může mít
// jiný styl, motiv a akcent než celek (`appearance.pages`, logika `theme/pages.ts`); prázdné pole = jako
// celek. Použije se, jakmile na záložku přepneš (plynulý přechod barev, `theme/apply.ts` `setActivePage`).
// Proužek, šířka okna a animace zůstávají společné. Záložky se berou z rozložení (`layout.pages`) — jejich
// přidávání, přejmenování a pořadí zůstává v liště záložek šuplíku.
import { For, Show } from "solid-js";
import { t } from "../../i18n";
import type { PageLook } from "../../types";
import { THEMES, THEME_IDS } from "../../theme/themes";
import { STYLES } from "../../theme/looks";
import { isHex } from "../../theme/appearance";
import { lbl } from "../../theme/label";
import { setPageLook } from "../../theme/pages";
import { normalizeLayout } from "../../layout/model";
import { pageName } from "../../layout/text";
import { lit, type Ap } from "./apUi";

export default function AppearancePages(q: { ap: Ap }) {
  const a = () => q.ap.a();
  const pages = () => normalizeLayout(q.ap.p.cfg.layout).pages;
  const look = (id: string): PageLook => a().pages[id] ?? {};
  const put = (id: string, patch: Partial<Record<keyof PageLook, string | undefined>>) => q.ap.setAll(setPageLook(a(), id, patch));
  const theme = (id: string) => look(id).theme ?? "";
  return (
    <div class="card ap-top" id="f-appearance.pages" classList={{ hit: lit("appearance.pages") }}>
      <h4>{t("appearance.pages.title", "Vzhled záložek")}</h4>
      <p class="ap-note">{t("appearance.pages.hint", "Každá záložka může mít vlastní styl, motiv a akcent — platí, jakmile na ni přepneš. Proužek, šířka a animace zůstávají společné.")}</p>
      <div class="ap-prows">
        <For each={pages()}>
          {(pg) => (
            <div class="ap-prow" classList={{ own: !!a().pages[pg.id] }}>
              <b class="ap-pname">{pageName(pg)}</b>
              <label class="ap-pctl">
                <small>{t("appearance.pages.style", "Styl")}</small>
                <select aria-label={t("appearance.pages.styleAria", "Styl záložky {name}", { name: pageName(pg) })} onChange={(e) => put(pg.id, { style: e.currentTarget.value })}>
                  <option value="" selected={!look(pg.id).style}>{t("appearance.pages.asWhole", "Jako celek")}</option>
                  <For each={STYLES}>{(s) => <option value={s.id} selected={look(pg.id).style === s.id}>{lbl(s.name)}</option>}</For>
                </select>
              </label>
              <label class="ap-pctl">
                <small>{t("appearance.pages.theme", "Motiv")}</small>
                <select aria-label={t("appearance.pages.themeAria", "Motiv záložky {name}", { name: pageName(pg) })} onChange={(e) => put(pg.id, { theme: e.currentTarget.value })}>
                  <option value="" selected={!theme(pg.id)}>{t("appearance.pages.asWhole", "Jako celek")}</option>
                  <For each={THEME_IDS}>{(id) => <option value={id} selected={theme(pg.id) === id}>{lbl([THEMES[id].key, THEMES[id].cs])}</option>}</For>
                  <For each={a().customThemes}>{(c) => <option value={c.id} selected={theme(pg.id) === c.id}>{c.name} ({t("appearance.ct.own", "vlastní")})</option>}</For>
                </select>
              </label>
              <span class="ap-pctl">
                <small>{t("appearance.pages.accent", "Akcent")}</small>
                <span class="ap-pacc">
                  <label class="ap-acc own" classList={{ on: !!look(pg.id).accent }} title={t("appearance.accent.custom", "Vlastní barva")}>
                    <input type="color" value={look(pg.id).accent || "#a78bfa"} aria-label={t("appearance.pages.accentAria", "Akcent záložky {name}", { name: pageName(pg) })}
                      onInput={(e) => isHex(e.currentTarget.value) && put(pg.id, { accent: e.currentTarget.value })} />
                  </label>
                  <Show when={look(pg.id).accent} fallback={<small>{t("appearance.pages.asWhole", "Jako celek")}</small>}>
                    <button class="mini" onClick={() => put(pg.id, { accent: undefined })}>{t("appearance.pages.reset", "Zrušit")}</button>
                  </Show>
                </span>
              </span>
              <button class="mini" disabled={!a().pages[pg.id]} onClick={() => q.ap.setAll({ ...a(), pages: Object.fromEntries(Object.entries(a().pages).filter(([k]) => k !== pg.id)) })}>
                {t("appearance.pages.clear", "Jako celek")}
              </button>
            </div>
          )}
        </For>
      </div>
    </div>
  );
}
