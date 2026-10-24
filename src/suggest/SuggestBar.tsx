// Pruh kontextových návrhů nahoře v šuplíku (max 3 chipy). Chip se ukáže jen z reálné události (výběr dělá
// čistá `pick` v `model.ts`): připomínka po termínu, spadlé CI na mém PR, PR čekající na mou revizi, právě
// naběhlý dev server, adresa ve schránce, kterou ještě nemám mezi odkazy. Nic nepolluje: počítá se při změně
// snapshotu a `now` (sekundový tik šuplíku, když je vidět); schránku čte jen na událost `cliphist` a jen se
// zapnutým zdrojem. Chip jde zavřít (24 h, v paměti), každý zdroj vypne Nastavení → Obsah karet → Kontextové návrhy.
import { For, Show, createEffect, createMemo, createSignal, onCleanup, type JSX } from "solid-js";
import { api, on as onEvent, type Snapshot } from "../api";
import { clipApi } from "../cliphist/api";
import { intApi } from "../integrations/api";
import type { NotesNav } from "../nav";
import type { NoteView } from "../types";
import { t } from "../i18n";
import { providerName } from "../calendar/model";
import { sayError } from "../ui/toast";
import { closeSuggestion, dismissed, meetingToNote, saveLink, urlToNote } from "./actions";
import { clipUrl, currentMeetings, knownLinks, pick, sameSuggestions, trackMeetings, type Meeting, type Suggestion, type SuggestKind } from "./model";
import "./suggest.css";

const stroke = { viewBox: "0 0 24 24", fill: "none", stroke: "currentColor", "stroke-width": 1.8, "stroke-linecap": "round", "stroke-linejoin": "round", "aria-hidden": true } as const;
const ICON: Record<SuggestKind, () => JSX.Element> = {
  meeting: () => <svg {...stroke}><rect x="3.5" y="6" width="12" height="12" rx="2" /><path d="M15.5 10.5 20.5 7.5v9l-5-3" /></svg>,
  meetingNote: () => <svg {...stroke}><path d="M6 3.5h8l4 4v13H6z" /><path d="M14 3.5v4h4M9 12h6M9 16h4" /></svg>,
  reminder: () => <svg {...stroke}><path d="M6 16V11a6 6 0 0 1 12 0v5l1.5 2h-15z" /><path d="M10 20.5a2 2 0 0 0 4 0" /></svg>,
  ghCi: () => <svg {...stroke}><circle cx="12" cy="12" r="9" /><path d="M12 7.5v5M12 16v.2" /></svg>,
  ghReview: () => <svg {...stroke}><circle cx="7" cy="6" r="2" /><circle cx="7" cy="18" r="2" /><circle cx="17" cy="16" r="2" /><path d="M7 8v8M17 14V9a3 3 0 0 0-3-3h-2" /></svg>,
  dev: () => <svg {...stroke}><rect x="3.5" y="4.5" width="17" height="6" rx="1.5" /><rect x="3.5" y="13.5" width="17" height="6" rx="1.5" /><path d="M7 7.5h.2M7 16.5h.2" /></svg>,
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

  /** schůzky s odkazem pamatované jen v paměti, i když je kalendář po konci skryl (poznámka ze schůzky) */
  let meetMem = new Map<string, Meeting>();
  const known = createMemo(() => knownLinks(p.s.actions, p.s.personal.links));
  // stejný obsah = stejné pole (tik `now` chipy nepřekreslí)
  const chips = createMemo<Suggestion[]>(() => {
    const on = cfgOn();
    if (!on) return [];
    meetMem = trackMeetings(meetMem, currentMeetings(p.s.personal.calendar), p.now);
    return pick(
      {
        on,
        notes: p.s.notesEnabled,
        meetings: [...meetMem.values()],
        reminders: p.s.reminders.map((n) => ({ path: n.path, title: n.title, remindMs: n.remindMs })),
        github: p.s.suggest?.github ?? { review: [], failing: [], fetchedMs: null },
        procs: p.s.procs,
        projects: p.s.projects,
        clip: clip(),
        known: known(),
      },
      dismissed(),
      p.now,
    );
  }, [], { equals: sameSuggestions });

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

  const label = (c: Suggestion): string => {
    switch (c.kind) {
      case "reminder": return t("cards.suggest.reminder", "Připomínka: {title}", { title: c.title || t("cards.suggest.imageOnly", "(jen obrázek)") });
      case "meeting": return t("cards.suggest.meeting", "Schůzka: {title}", { title: c.title || t("cards.calendar.untitled", "(bez názvu)") });
      case "meetingNote": return t("cards.suggest.meetingEnded", "Schůzka skončila: {title}", { title: c.title || t("cards.calendar.untitled", "(bez názvu)") });
      case "ghCi": return t("cards.suggest.ghCi", "Spadlo CI na tvém PR {pr}", { pr: c.title });
      case "ghReview": return t("cards.suggest.ghReview", "Čeká na tvou revizi: {pr}", { pr: c.title });
      case "dev": return t("cards.suggest.dev", "Dev server běží: {project}", { project: c.detail });
      case "clip": return t("cards.suggest.clip", "Zkopíroval jsi odkaz: {host}", { host: c.title });
    }
  };
  const tip = (c: Suggestion): string => (c.kind === "clip" || c.kind === "ghReview" || c.kind === "ghCi" ? [c.detail || c.url, c.url].filter(Boolean).join("\n") : c.title);

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
      case "ghCi":
      case "ghReview":
        return <button class="mini sg-act" onClick={() => openPr(c)}>{t("cards.suggest.openPr", "Otevřít PR #{n}", { n: c.num ?? 0 })}</button>;
      case "dev":
        return <button class="mini sg-act" onClick={() => run(c, () => api.openUrl(c.url ?? ""))}>{t("cards.suggest.openLocal", "Otevřít {addr}", { addr: c.title })}</button>;
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
      <div class="sg-bar" role="region" aria-label={t("cards.suggest.region", "Návrhy")}>
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
