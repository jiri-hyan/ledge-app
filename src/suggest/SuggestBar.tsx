// Pruh kontextových návrhů nahoře v šuplíku (max 3 chipy). Chip se ukáže jen z reálné události (výběr dělá
// čistá `pick` v `model.ts`): připomínka po termínu, spadlé CI na mém PR, PR čekající na mou revizi, právě
// naběhlý dev server, adresa ve schránce, kterou ještě nemám mezi odkazy. Nic nepolluje: počítá se při změně
// snapshotu a na časových hranicích (`model.ts` `nextChange`, ne každou sekundu); schránku čte jen na událost `cliphist` a jen se
// zapnutým zdrojem. Chip jde zavřít (24 h, v paměti), každý zdroj vypne Nastavení → Obsah karet → Kontextové návrhy.
import { For, Show, createEffect, createSignal, onCleanup, type JSX } from "solid-js";
import { api, on as onEvent, type Snapshot } from "../api";
import { clipApi } from "../cliphist/api";
import { intApi } from "../integrations/api";
import type { NotesNav } from "../nav";
import type { NoteView } from "../types";
import { t } from "../i18n";
import { setTab } from "../notes/store";
import { TASKS_TAB } from "../notes/NotesCard";
import { providerName } from "../calendar/model";
import { sayError } from "../ui/toast";
import { closeSuggestion, devErrorPreset, meetingToNote, saveLink, urlToNote } from "./actions";
import { clipUrl, type Suggestion, type SuggestKind } from "./model";
import { chipLabel, sinceTip } from "./label";
import { sinceTarget } from "./since.ts";
import { useChips } from "./chips";
import "./suggest.css";

const stroke = { viewBox: "0 0 24 24", fill: "none", stroke: "currentColor", "stroke-width": 1.8, "stroke-linecap": "round", "stroke-linejoin": "round", "aria-hidden": true } as const;
const ICON: Record<SuggestKind, () => JSX.Element> = {
  meeting: () => <svg {...stroke}><rect x="3.5" y="6" width="12" height="12" rx="2" /><path d="M15.5 10.5 20.5 7.5v9l-5-3" /></svg>,
  meetingNote: () => <svg {...stroke}><path d="M6 3.5h8l4 4v13H6z" /><path d="M14 3.5v4h4M9 12h6M9 16h4" /></svg>,
  reminder: () => <svg {...stroke}><path d="M6 16V11a6 6 0 0 1 12 0v5l1.5 2h-15z" /><path d="M10 20.5a2 2 0 0 0 4 0" /></svg>,
  ghCi: () => <svg {...stroke}><circle cx="12" cy="12" r="9" /><path d="M12 7.5v5M12 16v.2" /></svg>,
  ghReview: () => <svg {...stroke}><circle cx="7" cy="6" r="2" /><circle cx="7" cy="18" r="2" /><circle cx="17" cy="16" r="2" /><path d="M7 8v8M17 14V9a3 3 0 0 0-3-3h-2" /></svg>,
  dev: () => <svg {...stroke}><rect x="3.5" y="4.5" width="17" height="6" rx="1.5" /><rect x="3.5" y="13.5" width="17" height="6" rx="1.5" /><path d="M7 7.5h.2M7 16.5h.2" /></svg>,
  today: () => <svg {...stroke}><rect x="4" y="5" width="16" height="15" rx="2" /><path d="M4 10h16M9 3.5v3M15 3.5v3M9 15l2 2 4-4" /></svg>,
  claudeSince: () => <svg {...stroke}><rect x="3.5" y="4.5" width="17" height="15" rx="2.5" /><path d="M8 10l2.5 2L8 14M13 14h3" /></svg>,
  briefWaiting: () => <svg {...stroke}><path d="M6 3.5h9l3 3v14H6z" /><path d="M9 11h6M9 15h3M15 3.5v3h3" /><circle cx="17" cy="17" r="3.2" fill="var(--card)" /><path d="M17 15.8v1.5M17 18.4v.1" /></svg>,
  morning: () => <svg {...stroke}><circle cx="12" cy="14" r="4" /><path d="M12 5v2.5M4.5 14H2.5M21.5 14h-2M6.7 8.7 5.3 7.3M17.3 8.7l1.4-1.4M3 19.5h18" /></svg>,
  clip: () => <svg {...stroke}><path d="M10 14a4 4 0 0 0 5.7 0l3-3a4 4 0 0 0-5.7-5.7l-1 1" /><path d="M14 10a4 4 0 0 0-5.7 0l-3 3a4 4 0 0 0 5.7 5.7l1-1" /></svg>,
};

export default function SuggestBar(p: { s: Snapshot; now: number; nav: NotesNav }) {
  const cfgOn = () => p.s.suggest?.on;
  /** nejnovější položka historie schránky, je-li to adresa — jen se zapnutým zdrojem a na událost `cliphist` */
  const [clip, setClip] = createSignal<{ url: string; atMs: number } | null>(null);
  createEffect(() => {
    if (!cfgOn()?.clipboard) return setClip(null);
    const load = async () => {
      const l = await clipApi.list().catch(() => undefined);
      setClip(l?.enabled ? clipUrl(l.items, Date.now()) : null);
    };
    void load();
    let gone = false;
    let off: (() => void) | undefined;
    void onEvent("cliphist", () => void load()).then((u) => (gone ? u() : (off = u)));
    onCleanup(() => {
      gone = true;
      off?.();
    });
  });

  const chips = useChips(() => p.s, () => p.now, clip);

  const run = (c: Suggestion, f: () => unknown) => {
    void Promise.resolve(f()).then((r) => r !== false && closeSuggestion(c.key), sayError);
  };
  const openPr = (c: Suggestion) => run(c, () => intApi.githubOpen(c.url ?? ""));
  /** nová poznámka „{název} — {datum}“ a hned editor (cesta stačí; úprava poznámky bere jen `path`) */
  const openMeetingNote = async (c: Suggestion) => {
    const path = await meetingToNote({ title: c.title, start: c.start ?? Date.now() });
    if (path === false) return false;
    p.nav.onOpen({ path } as NoteView);
    return true;
  };
  const noteOf = (c: Suggestion) => p.s.reminders.find((n) => n.path === c.path);

  const label = chipLabel;
  const tip = (c: Suggestion): string => (c.kind === "claudeSince" ? sinceTip(c.since ?? []) : c.kind === "morning" ? chipLabel(c) : c.kind === "clip" || c.kind === "ghReview" || c.kind === "ghCi" ? [c.detail || c.url, c.url].filter(Boolean).join("\n") : c.title);

  const Actions = (q: { c: Suggestion }) => {
    const c = q.c;
    switch (c.kind) {
      case "reminder":
        return (
          <Show when={noteOf(c)}>
            {(n) => <button class="mini sg-act" onClick={() => p.nav.onOpen(n())}>{t("cards.suggest.openNote", "Otevřít poznámku")}</button>}
          </Show>
        );
      case "meeting":
        return <button class="mini ok sg-act" title={t("cards.calendar.joinTitle", "Připojit se přes {app}", { app: providerName(c.provider) })} onClick={() => run(c, () => api.calendarJoin(c.url ?? ""))}>{t("cards.calendar.join", "Připojit")}</button>;
      case "meetingNote":
        return <button class="mini sg-act" onClick={() => run(c, () => openMeetingNote(c))}>{t("cards.suggest.meetingNote", "Poznámka ze schůzky")}</button>;
      case "today":
        return <button class="mini sg-act" onClick={() => { setTab(TASKS_TAB); p.nav.onAll(); }}>{t("cards.suggest.todayOpen", "Úkoly")}</button>;
      case "briefWaiting":
        return (
          <Show when={c.path}>
            {(path) => <button class="mini sg-act" onClick={() => p.nav.onOpen({ path: path() } as NoteView)}>{t("cards.suggest.openNote", "Otevřít poznámku")}</button>}
          </Show>
        );
      case "morning":
        return null;
      case "claudeSince":
        return (
          <Show when={sinceTarget(c.since ?? [])}>
            {(target) => <button class="mini sg-act" onClick={() => void api.sessionOpen(target().id).catch(sayError)}>{t("cards.suggest.sinceOpen", "Otevřít session")}</button>}
          </Show>
        );
      case "ghCi":
      case "ghReview":
        return <button class="mini sg-act" onClick={() => openPr(c)}>{t("cards.suggest.openPr", "Otevřít PR #{n}", { n: c.num ?? 0 })}</button>;
      case "dev":
        return (
          <>
            <Show when={(c.errors ?? 0) > 0 && p.s.notesEnabled}>
              <button class="mini ok sg-act" title={t("cards.suggest.devBriefTitle", "Nové zadání s chybami z výstupu, projektem, URL a větví — pošleš ho sám v editoru")} onClick={() => run(c, () => { const pre = devErrorPreset(p.s, c); if (pre) p.nav.onNew(pre); return pre !== null; })}>{t("cards.suggest.devBrief", "Chyba → zadání")}</button>
            </Show>
            <button class="mini sg-act" onClick={() => run(c, () => api.openUrl(c.url ?? ""))}>{t("cards.suggest.openLocal", "Otevřít {addr}", { addr: c.title })}</button>
          </>
        );
      case "clip":
        return (
          <>
            <button class="mini sg-act" onClick={() => run(c, () => saveLink(c.url ?? "", p.s.cfgError))}>{t("cards.suggest.saveLink", "Uložit odkaz")}</button>
            <Show when={p.s.notesEnabled}>
              <button class="mini sg-act" onClick={() => run(c, () => urlToNote(c.url ?? ""))}>{t("cards.suggest.toNote", "Do poznámky")}</button>
            </Show>
          </>
        );
    }
  };

  return (
    <Show when={chips().length > 0}>
      <div class="sg-bar" role="region" aria-live="polite" aria-label={t("cards.suggest.region", "Návrhy")}>
        <For each={chips()}>
          {(c) => {
            const Icon = () => ICON[c.kind]();
            return (
              <div class="sg-chip" data-kind={c.kind} title={tip(c)}>
                <span class="sg-ico"><Icon /></span>
                <span class="sg-text">{label(c)}</span>
                <Show when={c.more > 0}><span class="sg-more">+{c.more}</span></Show>
                <Actions c={c} />
                <button class="sg-x" aria-label={t("cards.suggest.dismiss", "Zavřít návrh na 24 hodin")} title={t("cards.suggest.dismiss", "Zavřít návrh na 24 hodin")} onClick={() => closeSuggestion(c.key)}>×</button>
              </div>
            );
          }}
        </For>
      </div>
    </Show>
  );
}
