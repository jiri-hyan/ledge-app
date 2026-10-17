// Panel „Aktualizace a o aplikaci“: verze, stav kontroly nové verze (Rust `update.rs`, jen čte
// snapshot), nastavení kontroly, odkaz na repo, diagnostický log a cesta ke configu.
import { createResource, Show } from "solid-js";
import { api, type UpdateView } from "../../api";
import { ago } from "../../format";
import { t } from "../../i18n";
import { sayError } from "../../ui/toast";
import { useSettings } from "../ctx";
import { Slider, Text, Toggle } from "../fields/controls";
import { PanelHead, Row, Section } from "../fields/Row";
import type { PanelProps } from "../types";

/** Stav poslední kontroly nové verze — jen tady (hlavička ukazuje jen „dostupná“). */
function updateState(u: UpdateView): { text: string; tone: "ok" | "warn" | "bad" | "" } {
  const when = u.checkedMs ? ` · ${t("settings.about.checked", "ověřeno {when}", { when: ago(u.checkedMs) })}` : "";
  switch (u.state) {
    case "off": return { text: t("settings.about.st.off", "Kontrola je vypnutá"), tone: "" };
    case "neovereno": return { text: t("settings.about.st.pending", "První kontrola proběhne minutu po startu"), tone: "" };
    case "aktualni": return { text: t("settings.about.st.current", "Máš nejnovější verzi") + when, tone: "ok" };
    case "dostupna": return { text: t("settings.about.st.available", "Dostupná verze {v} — nainstaluješ ji z hlavičky šuplíku", { v: u.verze }) + when, tone: "warn" };
    case "nelze_overit": return { text: t("settings.about.st.unknown", "Nelze ověřit: {why}", { why: u.duvod }) + when, tone: "bad" };
  }
}

const open = (p: Promise<unknown>) => p.catch(sayError);

export default function AboutPanel(_p: PanelProps) {
  const c = useSettings();
  const [info] = createResource(() => api.appInfo());
  const u = () => c.s.update;
  const st = () => updateState(u());
  const repoUrl = () => (/^[\w.-]+\/[\w.-]+$/.test(c.cfg.update.repo ?? "") ? `https://github.com/${c.cfg.update.repo}` : null);
  return (
    <>
      <PanelHead title={t("settings.nav.oaplikaci", "Aktualizace a o aplikaci")} />
      <Section>
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
          <Row id="update.repo">
            <span class="st-inline">
              <Text path="update.repo" label={t("settings.f.update.repo", "Repo s vydáními")} placeholder="owner/name" mono />
              <Show when={repoUrl()}>{(url) => <button class="mini" onClick={() => open(api.openUrl(url()))}>{t("settings.about.openRepo", "Otevřít na GitHubu")}</button>}</Show>
            </span>
          </Row>
        </Show>
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
