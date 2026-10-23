// Nenápadný chip pod textem poznámky: „⏰ Připomenout zítra 10:00?“ — jen když poznámka nemá připomínku
// a v textu je čas („zítra v 10“, „v pondělí“, „za 2 h“, „25. 10.“). Jádro (`note_suggest_remind`) se ptá až
// po 450 ms ticha a jen když text vypadá časově (`suggest.ts` `wantsAsk`); v klidu nedělá nic. Klik nastaví
// připomínku (stejné pole jako výběr níž), × návrh zavře.
import { createEffect, createSignal, onCleanup, Show } from "solid-js";
import { api } from "../api";
import { IconX } from "../icons";
import { t } from "../i18n";
import { SUGGEST_DELAY, SUGGEST_MAX_CHARS, toLocalValue, visibleSuggest, wantsAsk, type Suggest } from "./suggest";

export default function RemindSuggest(p: { text: () => string; remind: () => string; onPick: (value: string) => void }) {
  const [sg, setSg] = createSignal<Suggest | null>(null);
  const [dismissed, setDismissed] = createSignal("");
  let timer: ReturnType<typeof setTimeout> | undefined;
  let seq = 0;
  /** text, na který se jádro naposledy ptalo (stejný text se nevolá znovu) */
  let asked = "";

  createEffect(() => {
    const text = p.text();
    const remind = p.remind();
    clearTimeout(timer);
    if (remind) {
      asked = "";
      setSg(null);
      return;
    }
    if (!wantsAsk(text, remind, asked)) {
      if (asked !== text) setSg(null);
      return;
    }
    const id = ++seq;
    timer = setTimeout(() => {
      asked = text;
      api.noteSuggestRemind(text.slice(0, SUGGEST_MAX_CHARS)).then(
        (r) => id === seq && setSg(r ?? null),
        () => id === seq && setSg(null),
      );
    }, SUGGEST_DELAY);
  });
  onCleanup(() => {
    clearTimeout(timer);
    seq++;
  });

  const shown = () => visibleSuggest(sg(), p.remind(), dismissed());
  return (
    <Show when={shown()}>
      {(s) => (
        <div class="note-sug">
          <button
            class="note-sug-go"
            title={t("notes.suggest.title", "Nastaví připomínku podle času v textu")}
            onClick={() => {
              const v = toLocalValue(s().iso);
              if (v) p.onPick(v);
            }}
          >
            ⏰ {t("notes.suggest.ask", "Připomenout {when}?", { when: s().label })}
          </button>
          <button class="note-sug-x" aria-label={t("notes.suggest.dismiss", "Nenabízet")} title={t("notes.suggest.dismiss", "Nenabízet")} onClick={() => setDismissed(s().iso)}>
            <IconX />
          </button>
        </div>
      )}
    </Show>
  );
}
