// Panel aktualizací pod hlavičkou (otevírá chip „Nová verze“ / „Co je nového“): poznámky k vydání → podle
// tichého stažení (`update.fetch`, Rust `updater.rs`) „Restartovat a aktualizovat“ (už ověřený instalátor),
// průběh stahování, „Zkusit znovu“, nebo postaru „Stáhnout a nainstalovat“. Běží-li servery, Rust vrátí
// `confirm` se seznamem (Ledge je drží v Job Objectu, s ním by umřely) a chce druhé potvrzení.
// Nahoře jednorázově „Co je nového ve verzi X“ (poznámky z cache, bez sítě) s „Rozumím“.
import { createSignal, Match, Show, Switch } from "solid-js";
import { api, type UpdateView } from "../api";
import { IconShield } from "../icons";
import { fmtClock } from "../format";
import { say, sayError } from "../ui/toast";
import { t, tp } from "../i18n";
import { bubble } from "../pkgctx/BubbleNotice";
import ReleaseNotes from "../update/ReleaseNotes";
import { action, mb, progressPct } from "../update/model";
import "./head.css";

export default function UpdatePanel(p: { u: UpdateView; onClose: () => void }) {
  const [running, setRunning] = createSignal<string[] | null>(null);
  const [busy, setBusy] = createSignal(false);
  const avail = () => (p.u.state === "dostupna" ? p.u : null);
  const fresh = () => (p.u.whatsNew?.unread ? p.u.whatsNew : null);
  const published = () => {
    const d = new Date(avail()?.publikovano ?? "");
    return Number.isNaN(d.getTime()) ? "" : t("cards.update.date", "{d}. {m}. {time}", { d: d.getDate(), m: d.getMonth() + 1, time: fmtClock(d) });
  };
  async function install(confirmed: string[]) {
    setBusy(true);
    try {
      const out = await api.updateInstall(confirmed);
      if (out?.outcome === "confirm") setRunning(out.running);
      else if (out?.outcome === "started") say(t("cards.update.installing", "Instaluji — Ledge se za chvíli spustí znovu"));
    } catch (e) {
      sayError(e);
    } finally {
      setBusy(false);
    }
  }
  async function read() {
    await api.updateNotesRead().catch(sayError);
    if (!avail()) p.onClose();
  }
  const working = () => busy() || p.u.installing;
  const act = () => action(p.u.fetch);
  const pct = () => progressPct(p.u.fetch);
  const got = () => (p.u.fetch.state === "downloading" ? p.u.fetch.got : 0);
  const failed = () => (p.u.fetch.state === "failed" ? p.u.fetch.duvod : null);
  return (
    <div class="card upd-panel" data-anim>
      <Show when={fresh()}>
        {(w) => (
          <div class="upd-whatsnew">
            <h4>{t("cards.update.whatsNew", "Co je nového ve verzi {v}", { v: w().verze })}</h4>
            <ReleaseNotes md={w().poznamky} />
            <div class="upd-bar">
              <span class="msg">{t("cards.update.whatsNewHint", "Poznámky najdeš i v Nastavení → Aktualizace.")}</span>
              <button class="btn pri" onClick={read}>{t("cards.update.gotIt", "Rozumím")}</button>
            </div>
          </div>
        )}
      </Show>
      <Show when={avail()}>
        {(a) => (
          <>
            <h4>Ledge {a().verze}<Show when={published()}><span class="n">{t("cards.update.published", "vydáno {date} · teď {current}", { date: published(), current: p.u.current })}</span></Show></h4>
            <ReleaseNotes md={a().poznamky} />
            <Show when={bubble()}>{(b) => <p class="pkg-upd">{t("cards.update.outside", "Ledge běží v bublině {app} — instalátor se spustí mimo ni (Plánovač úloh), nainstaluje Ledge naostro a spustí ho odtamtud.", { app: b().app })}</p>}</Show>
            <Show when={p.u.installError}><div class="banner"><IconShield /> <span>{p.u.installError}</span></div></Show>
            <Show when={failed()}>{(why) => <div class="banner"><IconShield /> <span>{t("cards.update.failed", "Novou verzi se nepodařilo stáhnout: {why}", { why: why() })}</span></div>}</Show>
            <Show when={act() === "downloading"}>
              <div class="upd-prog" classList={{ "upd-indet": pct() === null }} role="progressbar" aria-valuenow={pct() ?? undefined} aria-valuemin={0} aria-valuemax={100}>
                <i style={pct() === null ? undefined : { width: `${pct()}%` }} />
              </div>
            </Show>
            <Show when={running()} fallback={
              <div class="upd-bar">
                <span class="msg">
                  <Switch fallback={t("cards.update.how", "Stáhne se z GitHubu, ověří SHA256 a tiše nainstaluje; Ledge se restartuje.")}>
                    <Match when={act() === "downloading"}>
                      {pct() === null ? t("cards.update.dlUnknown", "Stahuji na pozadí… {mb} MB", { mb: mb(got()) }) : t("cards.update.dlPct", "Stahuji na pozadí… {pct} % ({mb} MB)", { pct: pct() ?? 0, mb: mb(got()) })}
                    </Match>
                    <Match when={working()}>{t("cards.update.downloading", "Ověřuji SHA256 a spouštím instalátor…")}</Match>
                    <Match when={act() === "restart"}>{t("cards.update.ready", "Staženo a ověřeno (SHA256). Restart trvá pár vteřin.")}</Match>
                    <Match when={act() === "retry"}>{t("cards.update.retryHint", "Zkus to znovu, až budeš online.")}</Match>
                  </Switch>
                </span>
                <button class="btn" onClick={p.onClose} disabled={working()}>{t("cards.update.later", "Později")}</button>
                <Switch fallback={<button class="btn pri" onClick={() => install([])} disabled={working() || act() === "downloading"}>{t("cards.update.restart", "Restartovat a aktualizovat")}</button>}>
                  <Match when={act() === "retry"}>
                    <button class="btn pri" onClick={() => api.updateRetry().catch(sayError)} disabled={working()}>{t("cards.update.retry", "Zkusit znovu")}</button>
                  </Match>
                  <Match when={act() === "install"}>
                    <button class="btn pri" onClick={() => install([])} disabled={working()}>{t("cards.update.install", "Stáhnout a nainstalovat")}</button>
                  </Match>
                </Switch>
              </div>
            }>
              {(r) => (
                <div class="upd-bar">
                  <span class="msg warn">{tp("cards.update.stops", r().length, ["Aktualizace zastaví {n} běžící server: {list}", "Aktualizace zastaví {n} běžící servery: {list}", "Aktualizace zastaví {n} běžících serverů: {list}"], { list: r().join(", ") })}</span>
                  <button class="btn" onClick={() => setRunning(null)} disabled={working()}>{t("cards.update.back", "Zpět")}</button>
                  <button class="btn pri" onClick={() => install(r())} disabled={working()}>{t("cards.update.stopInstall", "Zastavit a instalovat")}</button>
                </div>
              )}
            </Show>
          </>
        )}
      </Show>
    </div>
  );
}
