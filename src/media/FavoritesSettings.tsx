// Nastavení → Osobní → Přehrávač → Oblíbené: správa `media.favorites` (přidat, přejmenovat, pořadí, smazat
// s Vrátit). Mění jen návrh Nastavení — uloží ho společná lišta. Rychlé přidání je přímo v kartě Právě hraje
// („☆ Přidat, co hraje“, `FavoriteTiles.tsx`); tady jde odkaz i vložit rovnou.
import { Show, createSignal } from "solid-js";
import { t } from "../i18n";
import { offerUndo, useSettings } from "../settings/ctx";
import { Section } from "../settings/fields/Row";
import { SortList } from "../settings/fields/SortList";
import { moveItem } from "../actions/model";
import { MAX_FAVORITES, favoriteError, favoriteSource, withFavorite, withoutFavorite, type Favorite } from "./favorites";
import { favErrText } from "./FavoriteTiles";
import "./media.css";

/** `id` = kotva hledání (`#f-<id>`), předává ji MediaSettings. */
export default function FavoritesSettings(p: { id: string }) {
  const c = useSettings();
  const list = (): Favorite[] => c.get("media.favorites") ?? [];
  const commit = (next: Favorite[]) => c.set("media.favorites", next);
  const [name, setName] = createSignal("");
  const [uri, setUri] = createSignal("");
  const [err, setErr] = createSignal<string | null>(null);

  const add = () => {
    const f = { name: name(), uri: uri() };
    const e = favoriteError(f, list());
    if (e) return setErr(favErrText(e));
    commit(withFavorite(list(), f));
    setName("");
    setUri("");
    setErr(null);
  };
  const remove = (i: number) => {
    const gone = structuredClone(list()[i]);
    commit(withoutFavorite(list(), i));
    offerUndo(t("settings.media.fav.deleted", "Oblíbená položka „{name}“ smazána.", { name: gone.name }), () => {
      const next = list().slice();
      next.splice(Math.min(i, next.length), 0, gone);
      commit(next);
    });
  };
  const srcName = (u: string) => {
    const s = favoriteSource(u);
    return s === "spotify" ? "Spotify" : s === "ytMusic" ? "YouTube Music" : s === "web" ? t("cards.media.fav.web", "Web") : t("settings.media.fav.badUri", "nepovolený odkaz");
  };
  return (
    <Section
      id={p.id}
      title={t("settings.media.fav.title", "Oblíbené playlisty a stanice")}
      desc={t("settings.media.fav.desc", "Dlaždice v kartě Právě hraje: klik otevře odkaz ve Spotify nebo v prohlížeči (Ledge nic nepřehrává sám, play / pauza / další zůstávají v kartě). Odkaz zkopíruješ ve Spotify přes Sdílet → Kopírovat odkaz.")}
    >
      <Show when={list().length} fallback={<div class="empty">{t("settings.media.fav.empty", "Zatím žádné oblíbené. Přidej odkaz níže, nebo v kartě Právě hraje přes „☆ Přidat, co hraje“.")}</div>}>
        <p class="st-note">{t("settings.media.fav.order", "Pořadí změníš přetažením za úchyt (nebo Alt+šipkou). Dlaždic je nejvýš {n}.", { n: MAX_FAVORITES })}</p>
        <SortList items={list()} label={t("settings.media.fav.title", "Oblíbené playlisty a stanice")} onMove={(from, to) => commit(moveItem(list(), from, to))}>
          {(f, i, handle) => (
            <div class="med-fs-row">
              {handle()}
              <div class="med-fs-main">
                <input
                  type="text"
                  class="med-fs-name"
                  value={f.name}
                  maxLength={60}
                  aria-label={t("cards.media.fav.name", "Název")}
                  classList={{ bad: !!c.error(`media.favorites.${i()}.name`) }}
                  onInput={(e) => c.set(`media.favorites.${i()}.name`, e.currentTarget.value)}
                />
                <div class="med-fs-sub" classList={{ err: !!c.error(`media.favorites.${i()}.uri`) || !favoriteSource(f.uri) }} title={f.uri}>
                  {srcName(f.uri)} · {f.uri}
                </div>
                <Show when={c.error(`media.favorites.${i()}.name`) ?? c.error(`media.favorites.${i()}.uri`)}>{(m) => <div class="st-err" role="alert">{m()}</div>}</Show>
              </div>
              <button class="mini" onClick={() => remove(i())}>{t("settings.media.fav.delete", "Smazat")}</button>
            </div>
          )}
        </SortList>
      </Show>
      <Show when={list().length < MAX_FAVORITES}>
        <div class="med-fs-add">
          <input type="text" class="med-fs-name" value={name()} maxLength={60} placeholder={t("cards.media.fav.name", "Název")} aria-label={t("cards.media.fav.name", "Název")} onInput={(e) => setName(e.currentTarget.value)} />
          <input type="text" class="med-fs-uri" value={uri()} placeholder="spotify:playlist:… / https://…" aria-label={t("cards.media.fav.uri", "Odkaz")} onInput={(e) => { setUri(e.currentTarget.value); setErr(null); }} onKeyDown={(e) => e.key === "Enter" && add()} />
          <button class="mini" onClick={add}>+ {t("settings.media.fav.add", "Přidat")}</button>
        </div>
        <Show when={err()}>{(m) => <div class="st-err" role="alert">{m()}</div>}</Show>
      </Show>
    </Section>
  );
}
