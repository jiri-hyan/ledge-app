// Karta GitHub: PR, které čekají na tvoji revizi, tvoje otevřené PR se stavem CI a nepřečtené notifikace.
// Data bere Rust (`github/`) z přihlášeného `gh`; karta jen kreslí a poctivě řekne, co chybí (vypnuto /
// `gh` není / nepřihlášeno / offline). Obnova jen když je karta vidět a šuplík otevřený, nejvýš tak často,
// jak dovolí Rust (`nextInMs`, aspoň 2 min); tlačítko Obnovit smí po 10 s. Klik na položku ji otevře v prohlížeči.
import { createEffect, createMemo, createSignal, For, onCleanup, Show, type JSX } from "solid-js";
import { api } from "../api";
import { ago } from "../format";
import { t } from "../i18n";
import { drawerShown } from "../media/shown";
import { say, sayError } from "../ui/toast";
import { intApi, type GithubNotif, type GithubPr, type GithubView } from "./api";
import { CI_TONE, countText, isoMs } from "./format";
import "./integrations.css";

/** Poslední data přežijí přemontování karty (přepnutí záložky). */
const [view, setView] = createSignal<GithubView | null>(null);
const [busy, setBusy] = createSignal(false);

async function load(force: boolean) {
  if (busy()) return;
  setBusy(true);
  try {
    const v = await intApi.githubView(force);
    if (v) setView(v);
  } catch (e) {
    sayError(e);
  } finally {
    setBusy(false);
  }
}

async function open(url: string) {
  if (!url) return;
  try {
    await intApi.githubOpen(url);
  } catch (e) {
    sayError(e);
  }
}

/** Jedním klikem zapnout GitHub (uloží jen `github.enabled`); s chybou v configu neukládá — přepsal by se. */
async function enable(cfgError: string | null) {
  if (cfgError) {
    say(t("cards.github.cfgError", "Nastavení obsahuje chybu — GitHub teď zapneš jen v Nastavení."), true);
    return;
  }
  try {
    const cfg = await api.getConfig();
    if (!cfg) return;
    const g = cfg.github ?? {};
    // zapnuto, ale vše vypnuté by zase ukázalo „vypnuto“ — první zapnutí vrátí všechny tři sekce
    const none = !(g.review || g.mine || g.notifications);
    await api.saveConfig({ ...cfg, github: { ...g, enabled: true, ...(none ? { review: true, mine: true, notifications: true } : {}) } });
    await load(true);
  } catch (e) {
    sayError(e);
  }
}

async function copyLogin() {
  try {
    await intApi.githubCopyLogin();
    say(t("cards.github.copied", "Zkopírováno: gh auth login"));
  } catch (e) {
    sayError(e);
  }
}

function sub(parts: (string | false | undefined)[]): string {
  return parts.filter(Boolean).join(" · ");
}

function PrRow(p: { pr: GithubPr; now: number; showAuthor?: boolean; ci?: boolean }) {
  const when = () => {
    const ms = isoMs(p.pr.updated);
    return ms == null ? "" : ago(ms, p.now);
  };
  const tone = () => CI_TONE[p.pr.ci];
  return (
    <button class="row gh-row" onClick={() => void open(p.pr.url)} disabled={!p.pr.url} title={p.pr.title}>
      <Show when={p.ci}>
        <span
          class="gh-ci"
          classList={{ "gh-ci-green": tone() === "green", "gh-ci-red": tone() === "red", "gh-ci-amber": tone() === "amber" }}
          title={p.pr.ci === "success" ? t("cards.github.ci.success", "Kontroly prošly") : p.pr.ci === "failure" ? t("cards.github.ci.failure", "Kontroly selhaly") : p.pr.ci === "pending" ? t("cards.github.ci.pending", "Kontroly běží") : t("cards.github.ci.none", "Bez kontrol")}
          aria-hidden="true"
        />
      </Show>
      <div class="grow">
        <div class="name">{p.pr.title}</div>
        <div class="sub">{sub([`${p.pr.repo}#${p.pr.number}`, p.showAuthor && p.pr.author, when()])}</div>
      </div>
      <Show when={p.pr.draft}><span class="pill n">{t("cards.github.draft", "koncept")}</span></Show>
      <Show when={!p.showAuthor && p.pr.decision}>
        <span class="pill" classList={{ g: p.pr.decision === "APPROVED", r: p.pr.decision === "CHANGES_REQUESTED", a: p.pr.decision === "REVIEW_REQUIRED" }}>
          {p.pr.decision === "APPROVED" ? t("cards.github.dec.approved", "schváleno") : p.pr.decision === "CHANGES_REQUESTED" ? t("cards.github.dec.changes", "změny") : t("cards.github.dec.required", "čeká na revizi")}
        </span>
      </Show>
    </button>
  );
}

function NotifRow(p: { n: GithubNotif; now: number }) {
  const when = () => {
    const ms = isoMs(p.n.updated);
    return ms == null ? "" : ago(ms, p.now);
  };
  return (
    <button class="row gh-row" onClick={() => void open(p.n.url)} title={p.n.title}>
      <div class="grow">
        <div class="name">{p.n.title}</div>
        <div class="sub">{sub([p.n.repo, reasonText(p.n.reason), when()])}</div>
      </div>
    </button>
  );
}

/** Důvod notifikace lidsky (neznámý se nezobrazí, ať tu není anglický žargon). */
function reasonText(r: string): string {
  switch (r) {
    case "review_requested":
      return t("cards.github.reason.review", "žádost o revizi");
    case "mention":
    case "team_mention":
      return t("cards.github.reason.mention", "zmínka");
    case "assign":
      return t("cards.github.reason.assign", "přiřazeno tobě");
    case "author":
      return t("cards.github.reason.author", "tvoje vlákno");
    case "ci_activity":
      return t("cards.github.reason.ci", "CI");
    case "comment":
      return t("cards.github.reason.comment", "komentář");
    default:
      return "";
  }
}

function Group(p: { title: string; count: number; more: boolean; children: JSX.Element; empty: string; none: boolean }) {
  return (
    <div class="gh-group">
      <div class="gh-h">{p.title}<span class="gh-n">{countText(p.count, p.more)}</span></div>
      <Show when={!p.none} fallback={<div class="gh-none">{p.empty}</div>}>{p.children}</Show>
    </div>
  );
}

export default function GithubCard(p: { now: number; cfgError: string | null; menu?: JSX.Element }) {
  // vypnutý GitHub se neobnovuje (Rust na síť nesáhne); po zapnutí se smyčka rozběhne znovu, protože
  // se změní `off`. Jedno `load` proběhne při každém zobrazení — zapnutí v Nastavení se tak projeví hned.
  const off = createMemo(() => view()?.status === "off");
  createEffect(() => {
    if (!drawerShown()) return;
    void off();
    let alive = true;
    let timer: number | undefined;
    const loop = async () => {
      await load(false);
      if (!alive) return;
      const v = view();
      // vypnuto = není co obnovovat (zapnutí tlačítkem načte samo); běží cizí dotaz = krátce počkat
      if (!v || v.status === "off") return;
      timer = window.setTimeout(loop, v.loading ? 3000 : Math.max(v.nextInMs, 2000) + 300);
    };
    void loop();
    onCleanup(() => {
      alive = false;
      if (timer !== undefined) window.clearTimeout(timer);
    });
  });
  const v = () => view();
  const status = () => v()?.status ?? "off";
  return (
    <div class="card" data-anim>
      <h4><span class="ic" style="background:var(--blue)" />{t("cards.github.title", "GitHub")}{p.menu}</h4>
      <Show when={v()} fallback={<div class="empty">{t("cards.github.loading", "Načítám z GitHubu…")}</div>}>
        {(d) => (
          <>
            <Show when={status() === "off"}>
              <div class="empty gh-empty">
                {t("cards.github.off", "GitHub je vypnutý. Po zapnutí Ledge přes tvoje přihlášené gh (GitHub CLI) přečte PR, které čekají na tvoji revizi, tvoje otevřené PR a nepřečtené notifikace, a jen je tady ukáže. Nic neodesílá a heslo ani token nevidí.")}
                <button class="mini ok" onClick={() => void enable(p.cfgError)}>{t("cards.github.enable", "Zapnout GitHub")}</button>
              </div>
            </Show>
            <Show when={status() === "noGh"}>
              <div class="empty gh-empty">
                {t("cards.github.noGh", "GitHub CLI (gh) není nainstalované. Nainstaluj ho a přihlas se příkazem gh auth login.")}
                <span class="gh-act">
                  <button class="mini" onClick={() => void api.openUrl("https://cli.github.com")}>{t("cards.github.getGh", "Stáhnout gh")}</button>
                  <button class="mini" onClick={() => void copyLogin()}>{t("cards.github.copyLogin", "Kopírovat příkaz")}</button>
                </span>
              </div>
            </Show>
            <Show when={status() === "login"}>
              <div class="empty gh-empty">
                {t("cards.github.login", "gh není přihlášené. Přihlas se v terminálu příkazem gh auth login.")}
                <span class="gh-act">
                  <button class="mini ok" onClick={() => void copyLogin()}>{t("cards.github.copyLogin", "Kopírovat příkaz")}</button>
                  <button class="mini" disabled={busy()} onClick={() => void load(true)}>{t("cards.github.retry", "Zkusit znovu")}</button>
                </span>
              </div>
            </Show>
            <Show when={status() === "ok" || status() === "error"}>
              <Show when={status() === "error" && !d().fetchedMs}>
                <div class="empty gh-empty">{d().error ?? t("cards.github.failed", "GitHub se nepodařilo načíst.")}</div>
              </Show>
              <Show when={d().fetchedMs || status() === "ok"}>
                <Show when={d().reviewOn}>
                  <Group title={t("cards.github.review", "Čeká na tvoji revizi")} count={d().reviewCount} more={d().reviewMore} none={d().review.length === 0} empty={t("cards.github.reviewNone", "Nikdo od tebe revizi nečeká.")}>
                    <For each={d().review}>{(pr) => <PrRow pr={pr} now={p.now} showAuthor />}</For>
                  </Group>
                </Show>
                <Show when={d().mineOn}>
                  <Group title={t("cards.github.mine", "Tvoje otevřená PR")} count={d().mineCount} more={false} none={d().mine.length === 0} empty={t("cards.github.mineNone", "Žádné otevřené PR.")}>
                    <For each={d().mine}>{(pr) => <PrRow pr={pr} now={p.now} ci />}</For>
                  </Group>
                </Show>
                <Show when={d().notifsOn}>
                  <Group title={t("cards.github.notifs", "Nepřečtené notifikace")} count={d().notifCount} more={d().notifMore} none={d().notifs.length === 0} empty={t("cards.github.notifsNone", "Vše přečteno.")}>
                    <For each={d().notifs}>{(n) => <NotifRow n={n} now={p.now} />}</For>
                  </Group>
                </Show>
              </Show>
              <Show when={d().error && (d().fetchedMs || status() === "ok")}>
                <div class="sub err">{d().error}</div>
              </Show>
            </Show>
            <Show when={status() !== "off"}>
              <div class="gh-foot">
                <Show when={d().fetchedMs}>{(ms) => <span class="gh-meta">{d().loading || busy() ? t("cards.github.refreshing", "obnovuji…") : t("cards.github.fetched", "obnoveno {ago}", { ago: ago(ms(), p.now) })}</span>}</Show>
                <Show when={status() === "ok" || status() === "error"}>
                  <button class="mini" disabled={busy() || d().loading} onClick={() => void load(true)} title={t("cards.github.refreshTitle", "Znovu načíst z GitHubu")}>{t("cards.github.refresh", "Obnovit")}</button>
                </Show>
              </div>
            </Show>
          </>
        )}
      </Show>
    </div>
  );
}
