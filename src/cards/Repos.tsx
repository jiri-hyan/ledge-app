// Karta Repa: projekty s neuloženou / nepushnutou prací nebo pozadu za remote (z `ProjectView.git`);
// Pull jen u čistého repa, které je pozadu.
import { createSignal, For, Show } from "solid-js";
import { api, type ProjectView, type Snapshot } from "../api";
import { say, sayError } from "../ui/toast";
import { t } from "../i18n";

/** Repa s neuloženou / nepushnutou prací a ta, co zaostávají za remote. */
export default function Repos(p: { s: Snapshot }) {
  const [busy, setBusy] = createSignal<string | null>(null);
  const list = () =>
    p.s.projects
      .filter((x) => x.git && (x.git.dirty > 0 || x.git.ahead > 0 || x.git.behind > 0))
      .sort((a, b) => b.git!.dirty + b.git!.ahead * 2 - (a.git!.dirty + a.git!.ahead * 2));
  async function pull(pr: ProjectView) {
    setBusy(pr.id);
    try {
      const out = await api.gitPull(pr.id);
      say(t("cards.repos.pulled", "{name}: {out}", { name: pr.name, out: out || t("cards.repos.upToDate", "aktuální") }));
    } catch (e) {
      sayError(e);
    } finally {
      setBusy(null);
    }
  }
  return (
    <div class="card" data-anim>
      <h4><span class="ic" style="background:var(--amber)" />{t("cards.repos.title", "Repa")} <span class="n">{list().length || ""}</span></h4>
      <For each={list()} fallback={<div class="empty">{p.s.projects.some((x) => x.git) ? t("cards.repos.empty", "Všechna repa čistá a srovnaná s remote.") : t("cards.repos.none", "Žádný projekt zatím není git repo.")}</div>}>
        {(pr) => (
          <div class="row">
            <div class="grow">
              <div class="name">{pr.name}</div>
              <div class="tags">
                <span class="pill n">{pr.git!.branch ?? "detached"}</span>
                <Show when={pr.git!.dirty > 0}><span class="pill a">{t("cards.repos.dirty", "●{n} neuloženo", { n: pr.git!.dirty })}</span></Show>
                <Show when={pr.git!.ahead > 0}><span class="pill b">{t("cards.repos.ahead", "↑{n} nepushnuto", { n: pr.git!.ahead })}</span></Show>
                <Show when={pr.git!.behind > 0}><span class="pill n">{t("cards.repos.behind", "↓{n} pozadu", { n: pr.git!.behind })}</span></Show>
              </div>
            </div>
            <span class="r">
              <Show when={pr.git!.behind > 0 && pr.git!.dirty === 0}>
                <button class="mini" disabled={busy() === pr.id} aria-busy={busy() === pr.id} title="git pull --ff-only" onClick={() => pull(pr)}>{busy() === pr.id ? "…" : t("cards.repos.pull", "Pull")}</button>
              </Show>
              <button class="mini" title={t("cards.repos.terminal", "Terminál")} aria-label={t("cards.repos.terminalIn", "Terminál v {name}", { name: pr.name })} onClick={() => api.openIn(pr.id, "terminal")}>›_</button>
            </span>
          </div>
        )}
      </For>
    </div>
  );
}
