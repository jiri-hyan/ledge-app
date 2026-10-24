// Oblíbené karty Právě hraje (FavoriteTiles.tsx): řada dlaždic (název + ikona zdroje) a „☆ Přidat, co hraje“. Klik na dlaždici
// otevře odkaz v Spotify / prohlížeči (`media_favorite_open`) — Ledge nic nepřehrává sám, ovládání pak
// jde přes GSMTC (`Controls`). GSMTC odkaz nedává, takže „Přidat“ otevře malý formulář s názvem z toho, co
// hraje, a polem pro odkaz (předvyplní ho odkaz na hudbu ze schránky, je-li zapnutá historie schránky).
// Správa (přejmenovat, pořadí, smazat) je v Nastavení → Obsah karet → Přehrávač (`MediaSettings.tsx`).
import { For, Show, createSignal, type JSX } from "solid-js";
import { api, type MediaView } from "../api";
import { clipApi } from "../cliphist/api";
import { t } from "../i18n";
import { say, sayError } from "../ui/toast";
import { favApi } from "./api";
import { MAX_FAVORITES, favoriteError, favoriteSource, musicLinkFrom, suggestedName, withFavorite, type FavError, type FavSource, type Favorite } from "./favorites";

const stroke = { viewBox: "0 0 24 24", fill: "none", stroke: "currentColor", "stroke-width": 1.8, "stroke-linecap": "round", "stroke-linejoin": "round", "aria-hidden": true } as const;

/** Ikona zdroje: Spotify = tři oblouky, YouTube Music = kruh s přehráním, jiný web = zeměkoule. */
const SOURCE_ICON: Record<FavSource, () => JSX.Element> = {
  spotify: () => <svg {...stroke}><circle cx="12" cy="12" r="9" /><path d="M7.5 9.5c3-1 6.5-.7 9 .8M8 12.6c2.6-.8 5.4-.5 7.6.8M8.6 15.4c2-.5 4.1-.3 5.8.6" /></svg>,
  ytMusic: () => <svg {...stroke}><circle cx="12" cy="12" r="9" /><path d="M10 8.6v6.8l5.6-3.4z" /></svg>,
  web: () => <svg {...stroke}><circle cx="12" cy="12" r="9" /><path d="M3 12h18M12 3c2.5 2.7 2.5 15.3 0 18M12 3c-2.5 2.7-2.5 15.3 0 18" /></svg>,
};

const sourceName = (s: FavSource | null) => (s === "spotify" ? "Spotify" : s === "ytMusic" ? "YouTube Music" : t("cards.media.fav.web", "Web"));

/** Text chyby rozepsané položky (jazyk podle `t()`). */
export const favErrText = (e: FavError): string =>
  e === "max"
    ? t("cards.media.fav.err.max", "Víc než 24 oblíbených nejde.")
    : e === "name"
      ? t("cards.media.fav.err.name", "Doplň název.")
      : e === "nameLong"
        ? t("cards.media.fav.err.nameLong", "Název je moc dlouhý.")
        : t("cards.media.fav.err.uri", "Tenhle odkaz nejde otevřít — vlož odkaz ze Spotify nebo YouTube Music (nebo adresu https://…).");

/** Uloží seznam oblíbených do configu. Rozbitý config (v paměti výchozí hodnoty) neukládá — přepsal by uživatelův. */
export async function saveFavorites(list: Favorite[], cfgError: string | null): Promise<boolean> {
  if (cfgError) {
    say(t("cards.media.cfgError", "Nastavení obsahuje chybu — vzhled teď změníš jen v Nastavení."), true);
    return false;
  }
  try {
    const cfg = await api.getConfig();
    if (!cfg) throw new Error(t("cards.media.noConfig", "Nastavení se nepodařilo načíst."));
    await api.saveConfig({ ...cfg, media: { ...(cfg.media ?? {}), favorites: list } });
    return true;
  } catch (e) {
    sayError(e);
    return false;
  }
}

function AddForm(p: { start: { name: string; uri: string }; list: Favorite[]; cfgError: string | null; onDone: () => void }) {
  const [name, setName] = createSignal(p.start.name);
  const [uri, setUri] = createSignal(p.start.uri);
  const [err, setErr] = createSignal<string | null>(null);
  const [busy, setBusy] = createSignal(false);
  const submit = async () => {
    const f = { name: name(), uri: uri() };
    const e = favoriteError(f, p.list);
    if (e) return setErr(favErrText(e));
    setBusy(true);
    const ok = await saveFavorites(withFavorite(p.list, f), p.cfgError);
    setBusy(false);
    if (ok) p.onDone();
  };
  const onKey = (e: KeyboardEvent) => {
    if (e.key === "Enter") void submit();
    else if (e.key === "Escape") {
      e.stopPropagation();
      p.onDone();
    }
  };
  return (
    <div class="med-fav-form" onKeyDown={onKey}>
      <input type="text" class="med-fav-in" value={name()} maxLength={60} placeholder={t("cards.media.fav.name", "Název")} aria-label={t("cards.media.fav.name", "Název")} onInput={(e) => setName(e.currentTarget.value)} />
      <input type="text" class="med-fav-in" value={uri()} placeholder="spotify:playlist:… / https://…" aria-label={t("cards.media.fav.uri", "Odkaz")} onInput={(e) => { setUri(e.currentTarget.value); setErr(null); }} ref={(el) => queueMicrotask(() => el.focus())} />
      <p class="med-fav-hint">{t("cards.media.fav.hint", "Ve Spotify: Sdílet → Kopírovat odkaz. V YouTube Music: Sdílet → Kopírovat.")}</p>
      <Show when={err()}>{(m) => <p class="med-fav-err" role="alert">{m()}</p>}</Show>
      <div class="med-fav-btns">
        <button class="mini" onClick={p.onDone}>{t("cards.media.fav.cancel", "Zrušit")}</button>
        <button class="mini ok" disabled={busy()} onClick={() => void submit()}>{t("cards.media.fav.save", "Uložit")}</button>
      </div>
    </div>
  );
}

export default function Favorites(p: { list: Favorite[]; playing: MediaView | null; cfgError: string | null }) {
  const [form, setForm] = createSignal<{ name: string; uri: string } | null>(null);
  const full = () => p.list.length >= MAX_FAVORITES;
  const open = async () => {
    // výslovný klik: šuplík si vezme fokus, ať jde psát (po zavření formuláře ho vrátí)
    void api.focusDrawer();
    const clip = await clipApi.list().catch(() => undefined);
    setForm({ name: suggestedName(p.playing), uri: musicLinkFrom(clip?.items) });
  };
  const close = () => {
    setForm(null);
    void api.releaseFocus();
  };
  return (
    <div class="med-favs">
      <Show when={p.list.length > 0}>
        <div class="med-fav-row" role="list">
          <For each={p.list}>
            {(f, i) => {
              const src = () => favoriteSource(f.uri);
              const Icon = () => SOURCE_ICON[src() ?? "web"]();
              return (
                <button class="med-fav" role="listitem" title={`${f.name} · ${sourceName(src())}`} onClick={() => void favApi.open(i(), f.uri).catch(sayError)}>
                  <span class="med-fav-ico" data-src={src() ?? "web"}><Icon /></span>
                  <span class="med-fav-name">{f.name}</span>
                </button>
              );
            }}
          </For>
        </div>
      </Show>
      <Show when={form()} fallback={
        <Show when={!full()}>
          <button class="mini med-fav-add" onClick={() => void open()} title={t("cards.media.fav.addTitle", "Uloží odkaz na playlist nebo stanici; Ledge ho jen otevře")}>
            {p.playing ? t("cards.media.fav.addNow", "☆ Přidat, co hraje") : t("cards.media.fav.add", "☆ Přidat oblíbené")}
          </button>
        </Show>
      }>
        {(f) => <AddForm start={f()} list={p.list} cfgError={p.cfgError} onDone={close} />}
      </Show>
    </div>
  );
}
