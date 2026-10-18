// Kroky Claude Code (detekce + zapojení hooků) a Odkazy (návrhy + vlastní URL → rychlé akce,
// případně odkazy záložky Osobní, když config sekci `actions` ještě nemá).
import { createSignal, For, Show } from "solid-js";
import { api } from "../../api";
import { t } from "../../i18n";
import { say } from "../../ui/toast";
import { linkSuggestions, normalizeUrl } from "../logic";
import { installHooks, setWiz, wiz } from "../store";
import { Logo, Preview, Skeleton } from "./parts";

export function ClaudeStep() {
  const hooked = () => wiz.hooks?.state === "full";
  return (
    <div class="ob-center">
      <div class="ob-lead">
        <h3>{t("onboarding.claude.title", "Claude Code")}</h3>
        <p>{t("onboarding.claude.lead", "Ledge umí ukazovat běžící session Clauda, schvalovat jeho akce přímo v šuplíku a hlídat nezacommitovanou práci.")}</p>
      </div>
      <Show when={wiz.claudeState === "ready" || wiz.claudeState === "error"} fallback={<Skeleton rows={2} label={t("onboarding.claude.detecting", "Zjišťuji, jestli tu Claude Code je…")} />}>
        <Show
          when={wiz.claude?.installed}
          fallback={<div class="ob-card ob-quiet">{t("onboarding.claude.missing", "Claude Code na tomhle počítači nevidím — krok klidně přeskoč, hooky jdou zapojit později v Nastavení.")}</div>}
        >
          <div class="ob-card">
            <div class="ob-status">
              <span class="ob-dot on" />
              <div>
                <b>{wiz.claude?.version ? t("onboarding.claude.found", "Claude Code {v}", { v: wiz.claude.version }) : t("onboarding.claude.foundNoVer", "Claude Code je nainstalovaný")}</b>
                <small>{wiz.claude?.exe ?? t("onboarding.claude.dirOnly", "složka .claude existuje, spustitelný soubor jsem v PATH nenašel")}</small>
              </div>
            </div>
            <ul class="ob-bullets">
              <li>{t("onboarding.claude.b1", "Hooky zapíšou do ~/.claude/settings.json pár řádků, které posílají Ledge události session — jen na 127.0.0.1.")}</li>
              <li>{t("onboarding.claude.b2", "Ostatní nastavení i cizí hooky zůstanou; poprvé se uloží záloha settings.json.bak-pred-ledge.")}</li>
              <li>{t("onboarding.claude.b3", "Bez běžícího Ledge Claude funguje dál normálně. Odpojit jde jedním klikem v Nastavení.")}</li>
            </ul>
            <div class="ob-actions">
              <Show when={!hooked()} fallback={<span class="pill g">{t("onboarding.claude.ok", "Hooky jsou zapojené")}</span>}>
                <button class="btn pri" disabled={wiz.hooks?.state === "error"} onClick={() => void installHooks()}>
                  {t("onboarding.claude.install", "Zapojit hooky")}
                </button>
                <Show when={wiz.hooks?.error}>
                  <span class="sub err">{wiz.hooks?.error}</span>
                </Show>
              </Show>
            </div>
          </div>
        </Show>
      </Show>
    </div>
  );
}

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
          <h3>{t("onboarding.links.title", "Odkazy a rychlé akce")}</h3>
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
