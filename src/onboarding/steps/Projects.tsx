// Krok Projekty: repa nalezená na disku (Plocha, Dokumenty, source\repos…, vlastní složka) se
// zaškrtáváním; výchozí = 6 naposledy měněných. Náhled vpravo ukazuje dlaždice, které přibudou.
import { For, Show } from "solid-js";
import { t } from "../../i18n";
import type { FoundRepo } from "../../types";
import { importable, since } from "../logic";
import { addFolder, setWiz, wiz } from "../store";
import { CheckRow, Logo, Preview, Skeleton } from "./parts";

/** Textové nápovědy skenu přeloží UI podle druhu — sken mohl proběhnout ještě v předchozím jazyce
 *  (průvodce ho spouští hned při otevření, jazyk se volí až v prvním kroku). */
const hintOf = (r: FoundRepo) =>
  r.kind === "none"
    ? t("onboarding.hint.none", "nevím, jak ho spustit (chybí skript dev/start, Cargo.toml i .sln)")
    : r.kind === "app" && !r.project
      ? t("onboarding.hint.slnNoExe", "Visual Studio řešení bez sestaveného .exe — sestav ho a přidej později")
      : r.hint;
const kindLabel = (r: FoundRepo) =>
  r.kind === "dev" ? t("onboarding.kind.dev", "web / dev server") : r.kind === "app" ? t("onboarding.kind.app", "aplikace") : t("onboarding.kind.none", "jen složka");

export default function ProjectsStep() {
  const toggle = (path: string) => setWiz("repoSel", (s) => (s.includes(path) ? s.filter((x) => x !== path) : [...s, path]));
  const usable = () => wiz.repos.filter(importable);
  const allOn = () => usable().length > 0 && usable().every((r) => wiz.repoSel.includes(r.path));
  const selected = () => wiz.repos.filter((r) => importable(r) && wiz.repoSel.includes(r.path));
  return (
    <div class="ob-split">
      <section class="ob-main">
        <div class="ob-lead">
          <h3>{t("onboarding.projects.title", "Tvoje projekty")}</h3>
          <p>{t("onboarding.projects.lead", "Prošel jsem obvyklá místa (Plocha, Dokumenty, source\\repos, repos, dev, projects). Zaškrtni, co chceš mít v šuplíku — dev servery spustíš jedním klikem, aplikace taky.")}</p>
        </div>
        <div class="ob-toolbar">
          <span class="ob-count">
            <Show when={wiz.reposState === "ready"} fallback={t("onboarding.projects.searching", "Hledám…")}>
              {t("onboarding.projects.count", "Nalezeno {n} · vybráno {k}", { n: wiz.repos.length, k: selected().length })}
            </Show>
          </span>
          <Show when={usable().length > 1}>
            <button class="mini" onClick={() => setWiz("repoSel", allOn() ? [] : usable().map((r) => r.path))}>
              {allOn() ? t("onboarding.selectNone", "Zrušit výběr") : t("onboarding.selectAll", "Vybrat vše")}
            </button>
          </Show>
          <button class="mini" disabled={wiz.reposState === "loading"} onClick={() => void addFolder()}>
            {t("onboarding.projects.addFolder", "Přidat složku…")}
          </button>
        </div>
        <div class="ob-list">
          <Show when={wiz.reposState !== "loading" || wiz.repos.length} fallback={<Skeleton label={t("onboarding.projects.scanning", "Procházím složky s repy…")} />}>
            <Show
              when={wiz.repos.length}
              fallback={<div class="ob-empty">{t("onboarding.projects.empty", "Žádné git repo jsem nenašel. Přidej složku, kde repa máš, nebo krok přeskoč — projekty jdou přidat i později v Nastavení.")}</div>}
            >
              <For each={wiz.repos}>
                {(r) => (
                  <CheckRow
                    on={r.already || (importable(r) && wiz.repoSel.includes(r.path))}
                    disabled={!importable(r)}
                    onToggle={() => toggle(r.path)}
                    logo={<Logo src={wiz.icons[`repo:${r.path}`]} name={r.name} />}
                    name={r.name}
                    meta={`${hintOf(r)} · ${since(r.lastMs)}`}
                    tags={
                      <>
                        <Show when={r.already}>
                          <span class="pill g">{t("onboarding.already", "už v Ledge")}</span>
                        </Show>
                        <span class={`pill ${r.kind === "dev" ? "b" : r.kind === "app" ? "v" : "n"}`}>{kindLabel(r)}</span>
                      </>
                    }
                  />
                )}
              </For>
            </Show>
          </Show>
        </div>
      </section>
      <Preview
        title={t("onboarding.preview.title", "Náhled v šuplíku")}
        items={selected().map((r) => ({ key: r.path, name: r.name, src: wiz.icons[`repo:${r.path}`] }))}
        empty={t("onboarding.projects.previewEmpty", "Zatím nic — zaškrtni projekty vlevo.")}
        note={t("onboarding.projects.note", "Příkaz, port i logo jde kdykoli upravit v Nastavení → Projekty.")}
      />
    </div>
  );
}
