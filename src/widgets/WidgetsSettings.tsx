// Nastavení widgetů ze skriptů (panel „Projekty, akce a odkazy“, pod skripty — widget je skript, který
// se spouští sám): zapnutí, složka, „Otevřít složku widgetů“ (založí ji), co Ledge ve složce našel
// (stav: povolený / čeká na souhlas / problém / chyba posledního běhu) s povolením a „Obnovit teď“,
// galerie vestavěných příkladů (kopie do složky jedním klikem, nic nespouští) a formát výstupu.
import { createSignal, For, onMount, Show } from "solid-js";
import { ago } from "../format";
import { t } from "../i18n";
import { PathPick, Toggle } from "../settings/fields/controls";
import { LiveBadge, Row, Section } from "../settings/fields/Row";
import { say, sayError } from "../ui/toast";
import type { WidgetTemplate, WidgetView } from "../types";
import { widgetsApi } from "./api";
import { loadWidgets, widgetsListing } from "./registry";
import { intervalText, WidgetConsent } from "./WidgetCard";
import "./widgets.css";

const KIND_NAME: Record<string, string> = { powershell: "PowerShell", cmd: "cmd", python: "Python", node: "Node.js", exe: ".exe" };

function statusText(w: WidgetView, now: number): string {
  switch (w.status) {
    case "off":
      return t("settings.widgets.off", "vypnuté");
    case "problem":
      return w.problem ?? t("settings.widgets.problem", "nejde spustit");
    case "consent":
      return w.consent?.changed ? t("settings.widgets.changed", "změnil se — čeká na nové povolení") : t("settings.widgets.consent", "čeká na povolení");
    case "error":
      return w.error ?? t("settings.widgets.error", "poslední běh selhal");
    case "ok":
      return w.updatedMs ? t("settings.widgets.ok", "povolený · obnoveno {ago}", { ago: ago(w.updatedMs, now) }) : t("settings.widgets.allowed", "povolený");
    case "loading":
      return t("settings.widgets.loading", "běží…");
    default:
      return t("settings.widgets.idle", "povolený · spustí se, až bude vidět");
  }
}

export default function WidgetsSettings() {
  const [tpls, setTpls] = createSignal<WidgetTemplate[]>([]);
  const [ask, setAsk] = createSignal<string | null>(null);
  const load = async () => {
    try {
      await loadWidgets();
      setTpls((await widgetsApi.templates()) ?? []);
    } catch (e) {
      sayError(e);
    }
  };
  onMount(() => void load());
  const list = () => widgetsListing();

  async function open() {
    try {
      const dir = await widgetsApi.openDir();
      if (dir) say(t("settings.widgets.opened", "Složka widgetů: {dir}", { dir }));
      await load();
    } catch (e) {
      sayError(e);
    }
  }
  async function install(x: WidgetTemplate) {
    try {
      await widgetsApi.install(x.file);
      say(t("settings.widgets.installed", "„{name}“ je ve složce widgetů. Přidej ho na záložku (✎ → Katalog karet) a povol.", { name: x.title }));
      await load();
    } catch (e) {
      sayError(e);
    }
  }

  return (
    <>
      <Section
        id="widgets"
        title={t("settings.widgets.title", "Widgety ze skriptů")}
        desc={t("settings.widgets.desc", "Vlastní karta nebo položka proužku z libovolného skriptu: skript vypíše JSON nebo pár řádků textu a Ledge z toho udělá kartu. Spouští se bez okna, jen když je karta vidět (nebo je widget v proužku), a až po tvém povolení — po každé změně souboru znovu.")}
        aside={
          <>
            <button class="mini" onClick={() => void load()}>{t("settings.widgets.reload", "Načíst znovu")}</button>
            <button class="mini" onClick={() => void open()}>{t("settings.widgets.open", "Otevřít složku widgetů")}</button>
          </>
        }
      >
        <Row id="widgets.enabled">
          <Toggle path="widgets.enabled" label={t("settings.f.widgets.enabled", "Widgety ze skriptů")} />
        </Row>
        <Row id="widgets.dir" stack>
          <PathPick path="widgets.dir" kind="folder" nullable label={t("settings.f.widgets.dir", "Složka widgetů")} placeholder={list()?.dir || "%USERPROFILE%\\Ledge\\widgets"} />
        </Row>
        <Show when={list()}>
          {(l) => (
            <Show
              when={l().exists}
              fallback={<div class="empty">{l().enabled ? t("settings.widgets.noDir", "Složka zatím neexistuje — „Otevřít složku widgetů“ ji založí, nebo přidej příklad z galerie níž.") : t("settings.widgets.offAll", "Widgety jsou vypnuté.")}</div>}
            >
              <For each={l().widgets} fallback={<div class="empty">{t("settings.widgets.none", "Ve složce zatím není žádný soubor s hlavičkou @ledge.widget.")}</div>}>
                {(w) => (
                  <>
                    <div class="wg-st-row">
                      <span class="wg-st-ico" aria-hidden="true">{w.icon ?? "🧩"}</span>
                      <div class="grow">
                        <div class="name">{w.title}</div>
                        <div class="sub" classList={{ err: w.status === "problem" || w.status === "error" }}>
                          {w.file} · {KIND_NAME[w.kind ?? ""] ?? ""} · {w.intervalS ? intervalText(w.intervalS) : ""} · {statusText(w, Date.now())}
                        </div>
                      </div>
                      <Show when={w.status === "consent"}>
                        <LiveBadge />
                        <button class="mini ok" onClick={() => setAsk(ask() === w.id ? null : w.id)}>{t("settings.widgets.review", "Zkontrolovat a povolit")}</button>
                      </Show>
                      <Show when={w.status !== "consent" && w.status !== "problem" && w.status !== "off"}>
                        <button class="mini" disabled={w.running} onClick={() => void widgetsApi.refresh(w.id).then(() => say(t("settings.widgets.refreshing", "Spouštím…")), sayError)}>{t("cards.widget.menu.refresh", "Obnovit teď")}</button>
                      </Show>
                      <button class="mini" onClick={() => void widgetsApi.edit(w.id).catch(sayError)}>{t("cards.widget.menu.edit", "Upravit skript")}</button>
                    </div>
                    <Show when={ask() === w.id}>
                      <WidgetConsent v={w} />
                    </Show>
                  </>
                )}
              </For>
              <Show when={l().skipped > 0}>
                <div class="empty">{t("settings.widgets.skipped", "Dalších {n} souborů se nenačetlo (strop 50).", { n: l().skipped })}</div>
              </Show>
              <Show when={l().error}>{(e) => <div class="sub err">{e()}</div>}</Show>
            </Show>
          )}
        </Show>
        <p class="st-note">{t("settings.widgets.where", "Kartu widgetu přidáš na libovolnou záložku v režimu úprav (✎ → Katalog karet). Do proužku ho dáš v ⋯ na kartě nebo ve Vzhledu → Co proužek ukazuje.")}</p>
      </Section>

      <Section id="widgets.gallery" title={t("settings.widgets.gallery", "Galerie příkladů")} desc={t("settings.widgets.galleryDesc", "Krátké hotové widgety — jedním klikem se zkopírují do složky widgetů. Nic se nespustí, dokud widget nepřidáš a nepovolíš. Uprav si je (město, server, repo) přes „Upravit skript“.")}>
        <div class="wg-gallery">
          <For each={tpls()}>
            {(x) => (
              <div class="wg-tpl">
                <div class="wg-tpl-head"><span aria-hidden="true">{x.icon}</span>{x.title}</div>
                <div class="wg-tpl-desc">{x.description}</div>
                <div class="wg-tpl-foot">
                  <span class="pill n">{KIND_NAME[x.kind]}</span>
                  <span class="pill n" title={x.net ? t("settings.widgets.netTitle", "Kam widget sahá po síti") : ""}>{x.net ? t("settings.widgets.net", "síť: {host}", { host: x.net }) : t("settings.widgets.offline", "bez sítě")}</span>
                  <span class="grow" />
                  <button class="mini" disabled={x.installed} onClick={() => void install(x)}>{x.installed ? t("settings.widgets.inFolder", "Ve složce") : t("settings.widgets.add", "Přidat")}</button>
                </div>
                <details>
                  <summary class="sub">{t("settings.widgets.show", "Ukázat skript")}</summary>
                  <pre class="wg-pre">{x.body}</pre>
                </details>
              </div>
            )}
          </For>
        </div>
        <details class="act-help">
          <summary>{t("settings.widgets.format", "Jak napsat vlastní widget")}</summary>
          <pre class="log">{t("settings.widgets.example", "# @ledge.widget Můj widget\n# @ledge.icon 🧩\n# @ledge.refresh 5m\n# @ledge.show both\n# @ledge.timeout 10\n@{ title = 'Ahoj'; text = 'Text karty'; strip = '🧩 42'; items = @(@{ label = 'Odkaz'; url = 'https://example.com' }) } | ConvertTo-Json -Depth 5")}</pre>
          <div class="empty">
            {t("settings.widgets.help", "Hlavička: @ledge.widget (název, povinný), icon, refresh (30s až 24h, výchozí 5m), show (card, strip, both), timeout (1–120 s, výchozí 10), args, description. Výstup: JSON s title, text, badge, progress (0–1), strip (≤ 24 znaků), color a items [{label, value, icon, color, url nebo action}] — action jen open (složka / dokument), copy (text) nebo run (skript ze složky skriptů + arg). Nebo text: první řádek do proužku, pak ---, pak položky „Popisek | href=… color=green“. Programy (.exe) mají hlavičku v souboru jméno.exe.ledge.")}
          </div>
        </details>
      </Section>
    </>
  );
}
