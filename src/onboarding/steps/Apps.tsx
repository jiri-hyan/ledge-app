// Krok Aplikace: zástupci ze Start menu s ikonou, hledání, výběr → projekty `kind: app`.
// Ikony se dotahují jen pro to, co je právě vidět (dávka ≤ 48), s krátkým zpožděním při psaní.
import { createEffect, createMemo, For, on, onCleanup, Show } from "solid-js";
import { api } from "../../api";
import { t } from "../../i18n";
import { loadIcons, setWiz, wiz } from "../store";
import { CheckRow, Logo, Preview, Skeleton } from "./parts";

const LIMIT = 80;

export default function AppsStep() {
  const match = createMemo(() => {
    const q = wiz.query.trim().toLowerCase();
    return q ? wiz.apps.filter((a) => a.name.toLowerCase().includes(q) || a.exe.toLowerCase().includes(q)) : wiz.apps;
  });
  const shown = () => match().slice(0, LIMIT);
  const selected = () => wiz.apps.filter((a) => !a.already && wiz.appSel.includes(a.exe));
  const toggle = (exe: string) => setWiz("appSel", (s) => (s.includes(exe) ? s.filter((x) => x !== exe) : [...s, exe]));

  let timer: number | undefined;
  createEffect(
    on(shown, (list) => {
      clearTimeout(timer);
      timer = window.setTimeout(() => void loadIcons(list.map((a) => `app:${a.exe}`)), 140);
    }),
  );
  onCleanup(() => clearTimeout(timer));

  return (
    <div class="ob-split">
      <section class="ob-main">
        <div class="ob-lead">
          <h3>{t("onboarding.apps.title", "Aplikace, které spouštíš")}</h3>
          <p>{t("onboarding.apps.lead", "Vyber programy ze Start menu, které chceš mít v šuplíku po ruce (spuštění jedním klikem, logo podle aplikace).")}</p>
        </div>
        <div class="ob-toolbar">
          <input
            type="text"
            class="ob-search"
            placeholder={t("onboarding.apps.search", "Hledat aplikaci…")}
            value={wiz.query}
            onPointerDown={() => api.focusDrawer()}
            onInput={(e) => setWiz("query", e.currentTarget.value)}
          />
          <span class="ob-count">
            <Show when={wiz.appsState === "ready"} fallback={t("onboarding.apps.searching", "Načítám Start menu…")}>
              {t("onboarding.apps.count", "{n} aplikací · vybráno {k}", { n: match().length, k: selected().length })}
            </Show>
          </span>
        </div>
        <div class="ob-list">
          <Show when={wiz.appsState === "ready" || wiz.appsState === "error"} fallback={<Skeleton label={t("onboarding.apps.loading", "Čtu zástupce ve Start menu…")} />}>
            <Show when={shown().length} fallback={<div class="ob-empty">{wiz.query ? t("onboarding.apps.noMatch", "Nic neodpovídá hledání.") : t("onboarding.apps.empty", "Ve Start menu jsem žádnou aplikaci nenašel. Aplikace ze Storu tu nejsou — přidáš je v Nastavení.")}</div>}>
              <For each={shown()}>
                {(a) => (
                  <CheckRow
                    on={a.already || wiz.appSel.includes(a.exe)}
                    disabled={a.already}
                    onToggle={() => toggle(a.exe)}
                    logo={<Logo src={wiz.icons[`app:${a.exe}`]} name={a.name} />}
                    name={a.name}
                    meta={a.exe}
                    tags={a.already ? <span class="pill g">{t("onboarding.already", "už v Ledge")}</span> : undefined}
                  />
                )}
              </For>
              <Show when={match().length > LIMIT}>
                <div class="ob-more">{t("onboarding.apps.limit", "Zobrazeno {n} z {m} — upřesni hledání.", { n: LIMIT, m: match().length })}</div>
              </Show>
            </Show>
          </Show>
        </div>
      </section>
      <Preview
        title={t("onboarding.preview.title", "Náhled v šuplíku")}
        items={selected().map((a) => ({ key: a.exe, name: a.name, src: wiz.icons[`app:${a.exe}`] }))}
        empty={t("onboarding.apps.previewEmpty", "Zatím žádná aplikace — krok klidně přeskoč.")}
      />
    </div>
  );
}
