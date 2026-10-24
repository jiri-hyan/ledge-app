// Pole „Město“ s našeptávačem pro počasí (Nastavení → Obsah karet): píšeš název, po krátké odmlce (450 ms, aspoň
// 2 znaky) se Open-Meteo zeptá Rust (`geo_search`), výběr vyplní zeměpisnou šířku a délku i název místa.
// V klidu se neptá na nic; souřadnice zůstávají vidět v „Pokročilé“ (`Personal.tsx`). Logika `cityModel.ts`.
import { createSignal, For, onCleanup, Show } from "solid-js";
import { t } from "../../i18n";
import { useSettings } from "../ctx";
import { GEO_DEBOUNCE_MS, geoQuery, placeLabel, placeToWeather, type Place } from "../cityModel";
import { geoSearch } from "../geoApi";

export function CityPicker() {
  const c = useSettings();
  const [text, setText] = createSignal("");
  const [found, setFound] = createSignal<Place[] | null>(null);
  const [busy, setBusy] = createSignal(false);
  const [err, setErr] = createSignal<string | null>(null);
  let timer: number | undefined;
  let seq = 0;
  onCleanup(() => clearTimeout(timer));

  function typed(raw: string) {
    setText(raw);
    clearTimeout(timer);
    const q = geoQuery(raw);
    const mine = ++seq;
    setErr(null);
    if (!q) {
      setFound(null);
      setBusy(false);
      return;
    }
    setBusy(true);
    timer = window.setTimeout(async () => {
      try {
        const list = await geoSearch(q);
        if (mine === seq) setFound(list);
      } catch (e) {
        if (mine === seq) setErr(typeof e === "string" ? e : e instanceof Error ? e.message : String(e));
      } finally {
        if (mine === seq) setBusy(false);
      }
    }, GEO_DEBOUNCE_MS);
  }

  function pick(p: Place) {
    seq++;
    c.set("personal.weather", placeToWeather(p));
    setText("");
    setFound(null);
    setBusy(false);
  }

  return (
    <div class="st-city">
      <input
        type="text"
        class="st-text"
        spellcheck={false}
        autocomplete="off"
        maxLength={80}
        aria-label={t("settings.personal.city", "Město")}
        placeholder={t("settings.personal.city.ph", "Začni psát název města")}
        value={text()}
        onInput={(e) => typed(e.currentTarget.value)}
        onKeyDown={(e) => e.key === "Escape" && text() && (e.stopPropagation(), typed(""))}
      />
      <Show when={busy()}><small class="st-note">{t("settings.personal.city.busy", "Hledám…")}</small></Show>
      <Show when={err()}>{(m) => <small class="st-city-err" role="alert">{m()}</small>}</Show>
      <Show when={!busy() && !err() && found() && found()!.length === 0}>
        <small class="st-note">{t("settings.personal.city.none", "Nic nenalezeno — zkus jiný název nebo napiš souřadnice v Pokročilé.")}</small>
      </Show>
      <Show when={found()?.length}>
        <ul class="st-city-list" role="listbox" aria-label={t("settings.personal.city.results", "Nalezená místa")}>
          <For each={found()!}>
            {(p) => (
              <li>
                <button role="option" aria-selected="false" onClick={() => pick(p)}>{placeLabel(p)}</button>
              </li>
            )}
          </For>
        </ul>
      </Show>
    </div>
  );
}
