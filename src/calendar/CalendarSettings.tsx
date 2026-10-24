// Nastavení → Obsah karet → Kalendáře: přidání = výběr zdroje dlaždicí („Odkaz ICS“ — vložit odkaz a Přidat,
// jedno zkušební stažení, s návodem, kde odkaz vzít — PRVNÍ a při prázdném seznamu rovnou rozbalená volba;
// „Outlook v tomto PC“ — klasický Outlook přes COM, bez přihlášení). Přihlášení účtem Microsoft/Google
// (vlastní registrace aplikace) je až v „Pokročilé“ pod kalendáři (`accounts/AccountsSettings.tsx`, skládá
// ho `settings/panels/Personal.tsx`). U každého kalendáře barva, název,
// zapnutí, „Otestovat“ / „Změnit odkaz“ (ICS) nebo „Načíst z Outlooku“, připomínka schůzky (minuty předem
// + oznámení s tlačítkem Připojit) a Odebrat s Vrátit. Kotva `f-personal.calendars` je `Section`
// v `settings/panels/Personal.tsx`. Adresa se ukazuje jen jako doména (je to tajemství); chyby lidskou řečí.
import { createSignal, For, Show } from "solid-js";
import { api } from "../api";
import { gotoSettingsPanel } from "../palette/goto";
import { t } from "../i18n";
import { offerUndo, useSettings } from "../settings/ctx";
import { Select, Text, Toggle } from "../settings/fields/controls";
import { FieldErr } from "../settings/fields/Row";
import { CAL_COLORS, calColor, DEFAULT_REMIND, domainOf, nextColor, normalizeUrl, outlookFeed, remindOf, remindOptions, sourceOf, type UrlProblem } from "./model";
import "./calendar.css";

const MAX_CALENDARS = 8;

type Feed = { name: string; url: string; color: string | null; enabled: boolean; source?: string; remindMin?: number; remindToast?: boolean };
type Msg = { text: string; err: boolean };
type Pick = "ics" | "outlook-local" | null;

function problemText(p: UrlProblem): string {
  switch (p) {
    case "empty":
      return t("settings.calendar.p.empty", "Vlož odkaz na kalendář.");
    case "http":
      return t("settings.calendar.p.http", "Odkaz musí začínat https:// — přes http by tajná adresa šla nešifrovaně.");
    case "scheme":
      return t("settings.calendar.p.scheme", "Tohle nevypadá jako odkaz na kalendář — má začínat https:// nebo webcal://.");
    default:
      return t("settings.calendar.p.invalid", "Odkaz je neúplný nebo poškozený — zkopíruj ho znovu celý.");
  }
}

const errText = (e: unknown) => (typeof e === "string" ? e : e instanceof Error ? e.message : String(e));

/** Rozbalovací návod: kde najít tajnou adresu ve formátu iCal. */
function Help() {
  return (
    <details class="cal-help">
      <summary>{t("settings.calendar.help", "Kde najdu odkaz na kalendář?")}</summary>
      <h5>Google</h5>
      <ol>
        <li>{t("settings.calendar.help.g1", "Na calendar.google.com otevři ozubené kolo → Nastavení.")}</li>
        <li>{t("settings.calendar.help.g2", "Vlevo v části Nastavení mých kalendářů klikni na svůj kalendář.")}</li>
        <li>{t("settings.calendar.help.g3", "Dole v části Integrovat kalendář zkopíruj Tajnou adresu ve formátu iCal.")}</li>
      </ol>
      <h5>Outlook</h5>
      <ol>
        <li>{t("settings.calendar.help.o1", "V Outlooku na webu otevři Nastavení → Kalendář → Sdílené kalendáře.")}</li>
        <li>{t("settings.calendar.help.o2", "V části Publikovat kalendář vyber kalendář, oprávnění Může zobrazit všechny podrobnosti a klikni na Publikovat.")}</li>
        <li>{t("settings.calendar.help.o3", "Zkopíruj odkaz ICS.")}</li>
      </ol>
      <p>{t("settings.calendar.help.secret", "Odkaz je tajný — kdo ho zná, vidí celý kalendář. Ledge ho jen čte, ukazuje z něj jen doménu a nikam ho neposílá.")}</p>
    </details>
  );
}

/** Dlaždice zdrojů v pořadí; účty Microsoft/Google (přihlášení) jsou v Pokročilé pod kalendáři. */
const SOURCES = () => [
  { id: "ics" as const, icon: "🔗", title: t("settings.calendar.src.ics", "Odkaz ICS"), desc: t("settings.calendar.src.icsDesc", "Google nebo Outlook na webu — tajný odkaz, bez přihlášení") },
  { id: "outlook-local" as const, icon: "🖥", title: t("settings.calendar.src.outlook", "Outlook v tomto PC"), desc: t("settings.calendar.src.outlookDesc", "klasický Outlook nainstalovaný tady — bez přihlášení, bez internetu") },
];

/** Připomínka schůzky u kalendáře `i`: minuty předem + oznámení Windows s Připojit. */
function Remind(p: { i: number; feed: Feed }) {
  const c = useSettings();
  const at = (f: string) => `personal.calendars.${p.i}.${f}`;
  const opts = () =>
    remindOptions(remindOf(p.feed)).map((m) => ({
      v: String(m),
      label: m === 0 ? t("settings.calendar.remind.off", "Nepřipomínat") : t("settings.calendar.remind.min", "{n} min předem", { n: m }),
    }));
  const on = () => remindOf(p.feed) > 0;
  return (
    <div class="cal-set-remind">
      <span class="cal-set-lbl">{t("settings.calendar.remind", "Připomenout schůzku")}</span>
      <Select
        label={t("settings.calendar.remind", "Připomenout schůzku")}
        options={opts()}
        value={String(remindOf(p.feed))}
        onChange={(v) => c.set(at("remindMin"), Number(v))}
      />
      <Show when={on()}>
        <span class="cal-set-lbl" title={t("settings.calendar.remind.toastHint", "Kromě proužku i oznámení Windows s tlačítkem Připojit (ne ve hře ani při Nerušit).")}>
          {t("settings.calendar.remind.toast", "i oznámením")}
        </span>
        <Toggle path={at("remindToast")} label={t("settings.calendar.remind.toast", "i oznámením")} />
      </Show>
      <FieldErr path={at("remindMin")} />
    </div>
  );
}

export default function CalendarSettings() {
  const c = useSettings();
  const list = (): Feed[] => c.cfg.personal?.calendars ?? [];
  const at = (i: number, f: string) => `personal.calendars.${i}.${f}`;
  // bez kalendáře rovnou rozbalený Odkaz ICS (první a nejjednodušší cesta)
  const [pick, setPick] = createSignal<Pick>(list().length === 0 ? "ics" : null);
  const [url, setUrl] = createSignal("");
  const [busy, setBusy] = createSignal(false);
  const [msg, setMsg] = createSignal<Msg | null>(null);
  /** klasický Outlook: null = ještě nezjištěno */
  const [outlook, setOutlook] = createSignal<{ installed: boolean; running: boolean } | null>(null);
  /** zpráva u řádku (Otestovat, Změnit odkaz, Načíst z Outlooku) */
  const [rowMsg, setRowMsg] = createSignal<{ i: number; m: Msg } | null>(null);
  const [testing, setTesting] = createSignal<number | null>(null);
  const [changing, setChanging] = createSignal<number | null>(null);
  const has = (u: string) => list().some((f) => {
    const n = normalizeUrl(f.url);
    return sourceOf(f) === "ics" && "url" in n && n.url === u;
  });
  const hasOutlook = () => list().some((f) => sourceOf(f) === "outlook-local");

  function choose(id: Pick) {
    setMsg(null);
    setPick(pick() === id ? null : id);
    if (id === "outlook-local" && outlook() === null) {
      api.calendarOutlookProbe().then((r) => setOutlook(r ?? { installed: false, running: false })).catch(() => setOutlook({ installed: false, running: false }));
    }
  }

  async function add() {
    const n = normalizeUrl(url());
    if ("problem" in n) return setMsg({ text: problemText(n.problem), err: true });
    if (has(n.url)) return setMsg({ text: t("settings.calendar.dup", "Tenhle kalendář už máš."), err: true });
    setBusy(true);
    setMsg(null);
    try {
      const r = await api.calendarTest(n.url);
      const feed: Feed = { name: r?.name ?? "", url: n.url, color: nextColor(list().map((f) => f.color)), enabled: true, remindMin: DEFAULT_REMIND, remindToast: false };
      c.update("personal.calendars", (l: Feed[] | undefined) => [...(l ?? []), feed]);
      setUrl("");
      setPick(null);
      setMsg({ text: t("settings.calendar.added", "Přidáno: {name} — nejbližších 14 dní: {n} událostí.", { name: feed.name || domainOf(n.url), n: r?.upcoming ?? 0 }), err: false });
    } catch (e) {
      setMsg({ text: errText(e), err: true });
    } finally {
      setBusy(false);
    }
  }

  function addOutlook() {
    if (hasOutlook()) return setMsg({ text: t("settings.calendar.outlook.dup", "Outlook v tomto PC už v seznamu máš."), err: true });
    c.update("personal.calendars", (l: Feed[] | undefined) => [...(l ?? []), outlookFeed(nextColor(list().map((f) => f.color)))]);
    setPick(null);
    setMsg({
      text: outlook()?.running
        ? t("settings.calendar.outlook.added", "Přidáno: Outlook v tomto PC — načte se po uložení při otevření záložky Osobní.")
        : t("settings.calendar.outlook.addedClosed", "Přidáno: Outlook v tomto PC — Outlook je teď zavřený, načte se, až ho otevřeš (nebo klikni na Načíst z Outlooku)."),
      err: false,
    });
  }

  async function test(i: number) {
    setTesting(i);
    try {
      const r = await api.calendarTest(list()[i].url);
      setRowMsg({ i, m: { text: t("settings.calendar.tested", "Funguje — {all} událostí, nejbližších 14 dní: {n}.", { all: r?.events ?? 0, n: r?.upcoming ?? 0 }), err: false } });
    } catch (e) {
      setRowMsg({ i, m: { text: errText(e), err: true } });
    } finally {
      setTesting(null);
    }
  }

  async function loadOutlook(i: number) {
    setTesting(i);
    try {
      const n = await api.calendarOutlookLoad();
      setRowMsg({ i, m: { text: t("settings.calendar.outlook.loaded", "Funguje — nejbližších 14 dní: {n} událostí.", { n: n ?? 0 }), err: false } });
    } catch (e) {
      setRowMsg({ i, m: { text: errText(e), err: true } });
    } finally {
      setTesting(null);
    }
  }

  function changeUrl(i: number, raw: string) {
    setChanging(null);
    if (!raw.trim()) return;
    const n = normalizeUrl(raw);
    if ("problem" in n) return setRowMsg({ i, m: { text: problemText(n.problem), err: true } });
    c.set(at(i, "url"), n.url);
    setRowMsg({ i, m: { text: t("settings.calendar.changed", "Odkaz změněn — stáhne se při otevření záložky Osobní."), err: false } });
  }

  const label = (f: Feed) => f.name || (sourceOf(f) === "outlook-local" ? t("settings.calendar.src.outlook", "Outlook v tomto PC") : domainOf(f.url));

  function remove(i: number) {
    const before = structuredClone(list());
    c.update("personal.calendars", (all: Feed[]) => all.filter((_, j) => j !== i));
    setRowMsg(null);
    offerUndo(t("settings.calendar.removed", "Kalendář {n} odebrán", { n: label(before[i]) }), () => c.set("personal.calendars", before));
  }

  const cycleColor = (i: number) => {
    const cur = calColor(list()[i].color, i);
    const k = CAL_COLORS.indexOf(cur as (typeof CAL_COLORS)[number]);
    c.set(at(i, "color"), CAL_COLORS[(k + 1) % CAL_COLORS.length]);
  };

  return (
    <div class="cal-set-list">
      <FieldErr path="personal.calendars" />
      <For each={list()}>
        {(f, i) => (
          <div class="cal-set-feed">
            <div class="cal-set-item">
              <button class="cal-chip" title={t("settings.calendar.color", "Změnit barvu")} aria-label={t("settings.calendar.color", "Změnit barvu")} onClick={() => cycleColor(i())}>
                <span class="cal-dot" style={{ background: calColor(f.color, i()) }} />
              </button>
              <span class="cal-set-name">
                <Text path={at(i(), "name")} label={t("settings.calendar.name", "Název")} placeholder={t("settings.calendar.name.ph", "název z kalendáře")} maxLength={60} />
              </span>
              <Show
                when={sourceOf(f) === "ics"}
                fallback={<span class="cal-set-dom">{sourceOf(f) === "outlook-local" ? t("settings.calendar.src.outlook", "Outlook v tomto PC") : t("settings.calendar.src.account", "účet")}</span>}
              >
                <span class="cal-set-dom" title={t("settings.calendar.masked", "Celá adresa je tajná — Ledge ukazuje jen doménu.")}>{domainOf(f.url) || "—"} ···</span>
              </Show>
              <Toggle path={at(i(), "enabled")} label={t("settings.calendar.enabled", "Zapnutý")} />
              <Show
                when={sourceOf(f) === "ics"}
                fallback={
                  <Show when={sourceOf(f) === "outlook-local"}>
                    <button class="mini" disabled={testing() !== null} onClick={() => loadOutlook(i())}>
                      {testing() === i() ? t("settings.calendar.outlook.loading", "Načítám…") : t("cards.calendar.outlookLoad", "Načíst z Outlooku")}
                    </button>
                  </Show>
                }
              >
                <button class="mini" disabled={testing() !== null} onClick={() => test(i())}>{testing() === i() ? t("settings.calendar.testing", "Zkouším…") : t("settings.calendar.test", "Otestovat")}</button>
                <button class="mini" onClick={() => setChanging(changing() === i() ? null : i())}>{t("settings.calendar.change", "Změnit odkaz")}</button>
              </Show>
              <button class="mini" onClick={() => remove(i())}>{t("settings.ui.remove", "Odebrat")}</button>
            </div>
            <Remind i={i()} feed={f} />
            <Show when={changing() === i()}>
              <div class="cal-set-add">
                <input
                  type="text"
                  class="st-text mono cal-set-url"
                  spellcheck={false}
                  autocomplete="off"
                  aria-label={t("settings.calendar.newUrl", "Nový odkaz ICS")}
                  placeholder={t("settings.calendar.url.ph", "https://… nebo webcal://… (končí .ics)")}
                  onKeyDown={(e) => {
                    if (e.key === "Enter") changeUrl(i(), e.currentTarget.value);
                    if (e.key === "Escape") setChanging(null);
                  }}
                  onChange={(e) => changeUrl(i(), e.currentTarget.value)}
                />
              </div>
            </Show>
            <Show when={rowMsg()?.i === i() ? rowMsg()!.m : null}>{(m) => <div class="cal-set-msg" classList={{ "cal-set-err": m().err }} role={m().err ? "alert" : undefined}>{m().text}</div>}</Show>
            <FieldErr path={at(i(), "name")} />
            <FieldErr path={at(i(), "url")} />
            <FieldErr path={at(i(), "color")} />
            <FieldErr path={at(i(), "source")} />
          </div>
        )}
      </For>
      <Show when={list().length < MAX_CALENDARS} fallback={<small>{t("settings.calendar.max", "Nejvýš {n} kalendářů", { n: MAX_CALENDARS })}</small>}>
        <div class="cal-src" role="radiogroup" aria-label={t("settings.calendar.addFrom", "Přidat kalendář z")}>
          <For each={SOURCES()}>
            {(s) => (
              <button class="cal-src-tile" classList={{ on: pick() === s.id }} role="radio" aria-checked={pick() === s.id} onClick={() => choose(s.id)}>
                <span class="cal-src-ic" aria-hidden="true">{s.icon}</span>
                <span class="cal-src-t">{s.title}</span>
                <span class="cal-src-d">{s.desc}</span>
              </button>
            )}
          </For>
        </div>
        <Show when={pick() === "ics"}>
          <div class="cal-set-add">
            <input
              type="text"
              class="st-text mono cal-set-url"
              spellcheck={false}
              autocomplete="off"
              aria-label={t("settings.calendar.url", "Odkaz na kalendář (ICS)")}
              placeholder={t("settings.calendar.url.ph", "https://… nebo webcal://… (končí .ics)")}
              value={url()}
              onInput={(e) => setUrl(e.currentTarget.value)}
              onKeyDown={(e) => e.key === "Enter" && !busy() && add()}
            />
            <button class="mini ok" disabled={busy() || !url().trim()} onClick={add}>{busy() ? t("settings.calendar.adding", "Načítám…") : t("settings.calendar.add", "Přidat")}</button>
          </div>
          <p class="cal-src-note">{t("settings.calendar.icsHow", "Google: Nastavení kalendáře → Integrovat kalendář → Tajná adresa ve formátu iCal. Outlook na webu: Nastavení → Kalendář → Sdílené kalendáře → Publikovat kalendář → odkaz ICS.")}</p>
          <Help />
        </Show>
        <Show when={pick() === "outlook-local"}>
          <div class="cal-src-info">
            <Show when={outlook()} fallback={<span>{t("settings.calendar.outlook.probing", "Zjišťuji, jestli je tu Outlook…")}</span>}>
              {(o) => (
                <Show
                  when={o().installed}
                  fallback={
                    <span>{t("settings.calendar.outlook.missing", "Klasický Outlook tu není. Nový Outlook jiným aplikacím kalendář nepůjčí — použij Odkaz ICS (Outlook na webu → Nastavení → Kalendář → Sdílené kalendáře).")}</span>
                  }
                >
                  <span>
                    {t(
                      "settings.calendar.outlook.what",
                      "Ledge bude číst jen tvůj výchozí kalendář v Outlooku na tomto PC — předmět, čas, místo a odkaz na schůzku z pozvánky. Nic se nikam neposílá. Zavřený Outlook Ledge sám nespouští, jen po kliku na Načíst z Outlooku.",
                    )}
                  </span>
                  <span class="cal-src-note">{t("settings.calendar.outlook.newNote", "Používáš nový Outlook? Ten jiným aplikacím kalendář nepůjčí — použij Odkaz ICS.")}</span>
                  <button class="mini ok" disabled={hasOutlook()} onClick={addOutlook}>
                    {hasOutlook() ? t("settings.calendar.outlook.already", "Už přidaný") : t("settings.calendar.outlook.add", "Přidat Outlook")}
                  </button>
                </Show>
              )}
            </Show>
          </div>
        </Show>
      </Show>
      <div class="cal-src-info">
        <span>{t("settings.calendar.account.what", "Pracovní účet Microsoft nebo Google s přihlášením místo odkazu najdeš níže v Pokročilé.")}</span>
        <button class="mini" onClick={() => gotoSettingsPanel("f-accounts.microsoft")}>{t("settings.calendar.account.open", "Ukázat")}</button>
      </div>
      <Show when={msg()}>{(m) => <div class="cal-set-msg" classList={{ "cal-set-err": m().err }} role={m().err ? "alert" : undefined}>{m().text}</div>}</Show>
    </div>
  );
}
