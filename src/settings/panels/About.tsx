// Sekce „O aplikaci“ a „Aktualizace“ (panel Záloha a aplikace, `AppPanel.tsx`): verze, stav kontroly nové verze (Rust `updater.rs`, jen čte
// snapshot), nastavení kontroly a tichého stažení, „Co je nového“ (poznámky z cache), „Vrátit předchozí verzi“
// (s potvrzením), odkaz na repo, diagnostický log a cesta ke configu.
import { createResource, createSignal, Show } from "solid-js";
import { api, type UpdateView } from "../../api";
import ReleaseNotes from "../../update/ReleaseNotes";
import { ago } from "../../format";
import { t, tp } from "../../i18n";
import { say, sayError } from "../../ui/toast";
import { useSettings } from "../ctx";
import { Slider, Text, Toggle } from "../fields/controls";
import { Row, Section } from "../fields/Row";
import type { PanelProps } from "../types";

/** Stav poslední kontroly nové verze — jen tady (hlavička ukazuje jen „dostupná“). */
function updateState(u: UpdateView): { text: string; tone: "ok" | "warn" | "bad" | "" } {
  const when = u.checkedMs ? ` · ${t("settings.about.checked", "ověřeno {when}", { when: ago(u.checkedMs) })}` : "";
  switch (u.state) {
    case "off": return { text: t("settings.about.st.off", "Kontrola je vypnutá"), tone: "" };
    case "neovereno": return { text: t("settings.about.st.pending", "První kontrola proběhne minutu po startu"), tone: "" };
    case "aktualni": return { text: t("settings.about.st.current", "Máš nejnovější verzi") + when, tone: "ok" };
    case "dostupna": return {
      text: (u.fetch.state === "ready"
        ? t("settings.about.st.ready", "Verze {v} je stažená a ověřená — restartuješ ji z hlavičky šuplíku", { v: u.verze })
        : t("settings.about.st.available", "Dostupná verze {v} — nainstaluješ ji z hlavičky šuplíku", { v: u.verze })) + when,
      tone: "warn",
    };
    case "nelze_overit": return { text: t("settings.about.st.unknown", "Nelze ověřit: {why}", { why: u.duvod }) + when, tone: "bad" };
  }
}

const open = (p: Promise<unknown>) => p.catch(sayError);

/** „Vrátit předchozí verzi X“: první klik ukáže potvrzení, druhý spustí ověřený instalátor (Ledge se restartuje).
 *  Běží-li servery, Rust vrátí jejich seznam a chce třetí, výslovné potvrzení (jako u instalace). */
function Rollback(p: { u: UpdateView }) {
  const [ask, setAsk] = createSignal<string[] | null>(null);
  const [busy, setBusy] = createSignal(false);
  const rb = () => p.u.rollback;
  async function go(confirmed: string[]) {
    setBusy(true);
    try {
      const out = await api.updateRollback(confirmed);
      if (out?.outcome === "confirm") setAsk(out.running);
      else if (out?.outcome === "started") say(t("settings.about.rb.started", "Vracím předchozí verzi — Ledge se za chvíli spustí znovu"));
    } catch (e) {
      sayError(e);
    } finally {
      setBusy(false);
    }
  }
  return (
    <Show when={rb()} fallback={<span class="st-status">{t("settings.about.rb.none", "Není kam se vrátit — instalátor předchozí verze se uloží při příští aktualizaci.")}</span>}>
      {(r) => (
        <Show when={ask()} fallback={<button class="mini" disabled={busy() || p.u.installing} onClick={() => setAsk([])}>{t("settings.about.rb.btn", "Vrátit na {v}", { v: r().verze })}</button>}>
          {(running) => (
            <span class="st-inline">
              <span>
                {running().length
                  ? tp("settings.about.rb.stops", running().length, ["Zastaví se {n} běžící server: {list}.", "Zastaví se {n} běžící servery: {list}.", "Zastaví se {n} běžících serverů: {list}."], { list: running().join(", ") })
                  : r().local
                    ? t("settings.about.rb.askLocal", "Nainstalovat zpět verzi {v}? Instalátor je uložený a ověří se znovu; Ledge se restartuje.", { v: r().verze })
                    : t("settings.about.rb.askRemote", "Nainstalovat zpět verzi {v}? Instalátor se stáhne z GitHubu a ověří SHA256; Ledge se restartuje.", { v: r().verze })}
              </span>
              <button class="mini" disabled={busy()} onClick={() => setAsk(null)}>{t("settings.about.rb.cancel", "Zpět")}</button>
              <button class="mini" disabled={busy() || p.u.installing} onClick={() => go(running())}>{busy() ? t("settings.about.rb.working", "Pracuji…") : t("settings.about.rb.confirm", "Ano, vrátit")}</button>
            </span>
          )}
        </Show>
      )}
    </Show>
  );
}

export default function AboutPanel(_p: PanelProps) {
  const c = useSettings();
  const [info] = createResource(() => api.appInfo());
  const u = () => c.s.update;
  const st = () => updateState(u());
  return (
    <>
      <Section title={t("settings.about.title", "O aplikaci")}>
        <Row id="about.version" hint={t("settings.about.version.hint", "Ledge — šuplík u horního okraje obrazovky")}>
          <strong class="st-version">{info()?.version ?? u().current ?? "—"}</strong>
        </Row>
        <Row id="about.status" label={t("settings.about.status", "Nová verze")}>
          <span class="st-status">
            <span class="st-dot" classList={{ ok: st().tone === "ok", warn: st().tone === "warn", bad: st().tone === "bad" }} />
            <span>{u().checking ? t("settings.about.checking", "Kontroluji…") : st().text}</span>
            <button class="mini" disabled={u().checking || !c.cfg.update.check} onClick={() => open(api.updateCheckNow())}>{t("settings.about.checkNow", "Zkontrolovat teď")}</button>
          </span>
        </Row>
      </Section>
      <Section title={t("settings.about.updates", "Aktualizace")}>
        <Row id="update.check"><Toggle path="update.check" label={t("settings.f.update.check", "Hlídat nové verze")} /></Row>
        <Show when={c.cfg.update.check}>
          <Row id="update.intervalH">
            <Slider path="update.intervalH" min={1} max={48} label={t("settings.f.update.intervalH", "Jak často kontrolovat")} fmt={(v) => t("settings.about.every", "každých {v} h", { v })} />
          </Row>
          <Row id="update.prefetch"><Toggle path="update.prefetch" label={t("settings.f.update.prefetch", "Stahovat nové verze předem")} /></Row>
        </Show>
        <Show when={u().whatsNew}>
          {(w) => (
            <Row id="update.whatsNew" label={t("settings.about.whatsNew", "Co je nového ve verzi {v}", { v: w().verze })} stack>
              <ReleaseNotes md={w().poznamky} />
            </Row>
          )}
        </Show>
        <Row id="update.rollback"><Rollback u={u()} /></Row>
      </Section>
    </>
  );
}

/** Technické volby aplikace (v „Pokročilé“ panelu `AppPanel.tsx`): repo s vydáními, soubory a log. */
export function AboutAdvanced() {
  const c = useSettings();
  const [info] = createResource(() => api.appInfo());
  const repoUrl = () => (/^[\w.-]+\/[\w.-]+$/.test(c.cfg.update.repo ?? "") ? `https://github.com/${c.cfg.update.repo}` : null);
  return (
    <>
      <Section title={t("settings.about.updates", "Aktualizace")}>
        <Row id="update.repo" dim={!c.cfg.update.check}>
          <span class="st-inline">
            <Text path="update.repo" label={t("settings.f.update.repo", "Repo s vydáními")} placeholder="owner/name" mono />
            <Show when={repoUrl()}>{(url) => <button class="mini" onClick={() => open(api.openUrl(url()))}>{t("settings.about.openRepo", "Otevřít na GitHubu")}</button>}</Show>
          </span>
        </Row>
      </Section>
      <Section title={t("settings.about.files", "Soubory")}>
        <Row id="about.files" stack>
          <Show when={info()} fallback={<div class="st-skel" style={{ width: "70%" }} />}>
            {(i) => (
              <div class="st-files">
                <div><span>{t("settings.about.config", "Nastavení")}</span><code>{i().configPath}</code><button class="mini" onClick={() => open(api.openAppFile("dir"))}>{t("settings.about.openDir", "Otevřít složku")}</button></div>
                <div><span>{t("settings.about.log", "Diagnostický log")}</span><code>{i().logPath}</code><button class="mini" onClick={() => open(api.openAppFile("log"))}>{t("settings.about.openLog", "Otevřít log")}</button></div>
              </div>
            )}
          </Show>
        </Row>
      </Section>
    </>
  );
}
