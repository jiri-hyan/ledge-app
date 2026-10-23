// Krok Odkazy (návrhy + vlastní URL → rychlé akce). Krok Claude je v `Claude.tsx`.
import { createSignal, For } from "solid-js";
import { api } from "../../api";
import { t } from "../../i18n";
import { say } from "../../ui/toast";
import { linkSuggestions, normalizeUrl } from "../logic";
import { setWiz, wiz } from "../store";
import { Logo, Preview } from "./parts";

export function LinksStep() {
  const [label, setLabel] = createSignal("");
  const [url, setUrl] = createSignal("");
  const toggle = (id: string) => setWiz("linkSel", (s) => (s.includes(id) ? s.filter((x) => x !== id) : [...s, id]));
  function addCustom() {
    const u = normalizeUrl(url());
    if (!u) return say(t("onboarding.links.bad", "Adresa musí být webová (https://…)"), true);
    const name = label().trim() || new URL(u).hostname.replace(/^www\./, "");
    if (wiz.custom.some((c) => c.url === u)) return say(t("onboarding.links.dup", "Tenhle odkaz už v seznamu je"), true);
    setWiz("custom", (c) => [...c, { label: name, url: u }]);
    setLabel("");
    setUrl("");
  }
  const items = () => [
    ...linkSuggestions().filter((l) => wiz.linkSel.includes(l.id)).map((l) => ({ key: l.id, name: l.label, color: l.color })),
    ...wiz.custom.map((c) => ({ key: c.url, name: c.label })),
  ];
  const where = () =>
    t("onboarding.links.toActions", "Uloží se mezi odkazy — v kartě Odkazy a v Nastavení → Projekty, akce a odkazy.");
  const focus = () => api.focusDrawer();
  return (
    <div class="ob-split">
      <section class="ob-main">
        <div class="ob-lead">
          <h3>{t("onboarding.links.title", "Odkazy a akce")}</h3>
          <p>{t("onboarding.links.lead", "Weby, které otevíráš pořád dokola — pošta, kalendář, GitHub. Klikni na návrh, nebo přidej vlastní adresu.")}</p>
        </div>
        <div class="ob-chips">
          <For each={linkSuggestions()}>
            {(l) => (
              <button type="button" class="ob-chip" classList={{ on: wiz.linkSel.includes(l.id) }} aria-pressed={wiz.linkSel.includes(l.id)} onClick={() => toggle(l.id)}>
                <Logo src={null} name={l.label} color={l.color} />
                {l.label}
              </button>
            )}
          </For>
        </div>
        <form
          class="ob-custom"
          onSubmit={(e) => {
            e.preventDefault();
            addCustom();
          }}
        >
          <input type="text" placeholder={t("onboarding.links.name", "Název (nepovinné)")} value={label()} onPointerDown={focus} onInput={(e) => setLabel(e.currentTarget.value)} />
          <input type="text" class="grow" placeholder="https://…" value={url()} onPointerDown={focus} onInput={(e) => setUrl(e.currentTarget.value)} />
          <button class="btn" type="submit" disabled={!url().trim()}>
            {t("onboarding.links.add", "Přidat")}
          </button>
        </form>
        <div class="ob-list short">
          <For each={wiz.custom}>
            {(c) => (
              <div class="ob-row static">
                <Logo src={null} name={c.label} />
                <span class="ob-row-txt">
                  <span class="ob-row-name">{c.label}</span>
                  <span class="ob-row-meta">{c.url}</span>
                </span>
                <button class="mini" onClick={() => setWiz("custom", (x) => x.filter((y) => y.url !== c.url))}>
                  {t("onboarding.links.remove", "Odebrat")}
                </button>
              </div>
            )}
          </For>
        </div>
      </section>
      <Preview title={t("onboarding.links.preview", "Tvoje odkazy")} items={items()} empty={t("onboarding.links.previewEmpty", "Zatím žádný odkaz.")} note={where()} />
    </div>
  );
}
