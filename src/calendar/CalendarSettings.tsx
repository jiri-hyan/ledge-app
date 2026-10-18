// Nastavení → Osobní → Kalendáře: přidání = vložit odkaz ICS a „Přidat“ (jedno zkušební stažení, název
// z feedu, barva se přiřadí sama), u každého kalendáře barva, název, zapnutí, „Otestovat“, „Změnit odkaz“
// a Odebrat s Vrátit. Kotva `f-personal.calendars` je `Section` v `settings/panels/Personal.tsx`. Adresa se ukazuje jen jako doména (je to tajemství); chyby lidskou řečí.
import { createSignal, For, Show } from "solid-js";
import { api } from "../api";
import { t } from "../i18n";
import { offerUndo, useSettings } from "../settings/ctx";
import { Text, Toggle } from "../settings/fields/controls";
import { FieldErr } from "../settings/fields/Row";
import { CAL_COLORS, calColor, domainOf, nextColor, normalizeUrl, type UrlProblem } from "./model";
import "./calendar.css";

const MAX_CALENDARS = 8;

type Feed = { name: string; url: string; color: string | null; enabled: boolean };
type Msg = { text: string; err: boolean };

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

export default function CalendarSettings() {
  const c = useSettings();
  const list = (): Feed[] => c.cfg.personal?.calendars ?? [];
  const at = (i: number, f: string) => `personal.calendars.${i}.${f}`;
  const [url, setUrl] = createSignal("");
  const [busy, setBusy] = createSignal(false);
  const [msg, setMsg] = createSignal<Msg | null>(null);
  /** zpráva u řádku (Otestovat, Změnit odkaz) */
  const [rowMsg, setRowMsg] = createSignal<{ i: number; m: Msg } | null>(null);
  const [testing, setTesting] = createSignal<number | null>(null);
  const [changing, setChanging] = createSignal<number | null>(null);
  const has = (u: string) => list().some((f) => {
    const n = normalizeUrl(f.url);
    return "url" in n && n.url === u;
  });

  async function add() {
    const n = normalizeUrl(url());
    if ("problem" in n) return setMsg({ text: problemText(n.problem), err: true });
    if (has(n.url)) return setMsg({ text: t("settings.calendar.dup", "Tenhle kalendář už máš."), err: true });
    setBusy(true);
    setMsg(null);
    try {
      const r = await api.calendarTest(n.url);
      const feed: Feed = { name: r?.name ?? "", url: n.url, color: nextColor(list().map((f) => f.color)), enabled: true };
      c.update("personal.calendars", (l: Feed[] | undefined) => [...(l ?? []), feed]);
      setUrl("");
      setMsg({ text: t("settings.calendar.added", "Přidáno: {name} — nejbližších 14 dní: {n} událostí.", { name: feed.name || domainOf(n.url), n: r?.upcoming ?? 0 }), err: false });
    } catch (e) {
      setMsg({ text: errText(e), err: true });
    } finally {
      setBusy(false);
    }
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

  function changeUrl(i: number, raw: string) {
    setChanging(null);
    if (!raw.trim()) return;
    const n = normalizeUrl(raw);
    if ("problem" in n) return setRowMsg({ i, m: { text: problemText(n.problem), err: true } });
    c.set(at(i, "url"), n.url);
    setRowMsg({ i, m: { text: t("settings.calendar.changed", "Odkaz změněn — stáhne se při otevření záložky Osobní."), err: false } });
  }

  function remove(i: number) {
    const before = structuredClone(list());
    c.update("personal.calendars", (all: Feed[]) => all.filter((_, j) => j !== i));
    setRowMsg(null);
    offerUndo(t("settings.calendar.removed", "Kalendář {n} odebrán", { n: before[i].name || domainOf(before[i].url) }), () => c.set("personal.calendars", before));
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
            <div>
              <div class="cal-set-item">
                <button class="cal-chip" title={t("settings.calendar.color", "Změnit barvu")} aria-label={t("settings.calendar.color", "Změnit barvu")} onClick={() => cycleColor(i())}>
                  <span class="cal-dot" style={{ background: calColor(f.color, i()) }} />
                </button>
                <span class="cal-set-name">
                  <Text path={at(i(), "name")} label={t("settings.calendar.name", "Název")} placeholder={t("settings.calendar.name.ph", "název z kalendáře")} maxLength={60} />
                </span>
                <span class="cal-set-dom" title={t("settings.calendar.masked", "Celá adresa je tajná — Ledge ukazuje jen doménu.")}>{domainOf(f.url) || "—"} ···</span>
                <Toggle path={at(i(), "enabled")} label={t("settings.calendar.enabled", "Zapnutý")} />
                <button class="mini" disabled={testing() !== null} onClick={() => test(i())}>{testing() === i() ? t("settings.calendar.testing", "Zkouším…") : t("settings.calendar.test", "Otestovat")}</button>
                <button class="mini" onClick={() => setChanging(changing() === i() ? null : i())}>{t("settings.calendar.change", "Změnit odkaz")}</button>
                <button class="mini" onClick={() => remove(i())}>{t("settings.ui.remove", "Odebrat")}</button>
              </div>
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
            </div>
          )}
        </For>
        <Show when={list().length < MAX_CALENDARS} fallback={<small>{t("settings.calendar.max", "Nejvýš {n} kalendářů", { n: MAX_CALENDARS })}</small>}>
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
        </Show>
        <Show when={msg()}>{(m) => <div class="cal-set-msg" classList={{ "cal-set-err": m().err }} role={m().err ? "alert" : undefined}>{m().text}</div>}</Show>
      <Help />
    </div>
  );
}
