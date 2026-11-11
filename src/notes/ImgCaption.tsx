// Pole „Co je na snímku?“ pod čerstvě vloženým snímkem + kam patří („pod: <řádek>“). Prázdné je v pořádku.
import { Show } from "solid-js";
import { t } from "../i18n";
import { entryOf, setCaption } from "./imgmeta";

export default function ImgCaption(p: { url: string; focus?: boolean }) {
  const e = () => entryOf(p.url);
  return (
    <div class="img-cap">
      <input
        class="brief-in"
        ref={(el) => p.focus && queueMicrotask(() => el.focus())}
        placeholder={t("notes.brief.captionPh", "Co je na snímku?")}
        value={e()?.caption ?? ""}
        maxLength={200}
        onInput={(ev) => setCaption(p.url, ev.currentTarget.value)}
      />
      <Show when={e()?.anchor}>{(a) => <small class="hint" title={a().text}>{t("notes.brief.under", "pod: {line}", { line: a().text.replace(/^\s*(?:[-*+]|\d+[.)])\s*(?:\[[ xX~]\]\s*)?/, "").slice(0, 28) })}</small>}</Show>
    </div>
  );
}
