// Vzhledy karty Claude „Při práci“ (`working`) a „Hotovo“ (`done`), kolo 28. Kreslí je `ClaudeDesign.tsx`.
// Jen skutečná data ze snapshotu (`SessionView` + aktivita z hooků, logika `claudeLive.ts`); čas tiká na `now`
// šuplíku (žádný vlastní časovač), „dýchání“ podkladu je CSS a stojí, když nic nepracuje / při omezeném pohybu.
import { createSignal, For, Show } from "solid-js";
import { api, type SessionView } from "../api";
import { ago } from "../format";
import { t } from "../i18n";
import { say } from "../ui/toast";
import { clipMessage, doneAt, doneList, elapsed, fileParts, openWork, pickWorking, stripIndex, STRIP, workingList } from "./claudeLive";

const PHASE: Record<string, () => string> = {
  reading: () => t("cards.cl.phase.reading", "Čte"),
  editing: () => t("cards.cl.phase.editing", "Upravuje"),
  testing: () => t("cards.cl.phase.testing", "Testuje"),
  done: () => t("cards.cl.phase.done", "Hotovo"),
};

const projectOf = (ss: SessionView): string => ss.project ?? ss.name;

async function openSession(ss: SessionView) {
  if (!ss.canOpen) return say(ss.openHint, true);
  try {
    await api.sessionOpen(ss.id);
  } catch (e) {
    say(String(e), true);
  }
}

export function WorkingBody(p: { s: { sessions: SessionView[] }; now: number }) {
  const [sel, setSel] = createSignal<string | null>(null);
  const list = () => workingList(p.s.sessions);
  const cur = () => pickWorking(p.s.sessions, sel());
  const last = () => doneList(p.s.sessions)[0];
  return (
    <Show
      when={cur()}
      fallback={
        <div class="cd-empty cd-live-empty" role="status">
          <b>{t("cards.cl.idle", "Claude teď nepracuje")}</b>
          <Show when={last()}>
            {(l) => <small>{t("cards.cl.lastDone", "Poslední hotová: {name} · {when}", { name: projectOf(l()), when: ago(doneAt(l()), p.now) })}</small>}
          </Show>
        </div>
      }
    >
      {(ss) => {
        const idx = () => stripIndex(ss());
        const file = () => fileParts(ss().activity?.file);
        const tool = () => ss().activity?.tool || null;
        const subs = () => ss().subagents.running.length;
        return (
          <div class="cd-live" data-on>
            <div class="cd-live-head">
              <b class="cd-live-proj" title={ss().cwd}>{projectOf(ss())}</b>
              <Show when={list().length > 1}>
                <span class="cd-live-dots" role="group" aria-label={t("cards.cl.pick", "Pracující session")}>
                  <For each={list()}>
                    {(o) => <button class="cd-live-dot" classList={{ on: o.id === ss().id }} title={projectOf(o)} aria-label={projectOf(o)} aria-pressed={o.id === ss().id} onClick={() => setSel(o.id)} />}
                  </For>
                </span>
              </Show>
            </div>
            <div class="cd-live-time" title={t("cards.cl.since", "Jak dlouho už pracuje")}>{elapsed(ss().since, p.now) ?? "—"}</div>
            <div class="cd-strip" title={t("cards.cl.phase.tip", "Fáze je odhad podle posledního nástroje, ne krok plánu.")}>
              <For each={STRIP}>{(ph, i) => <span class="cd-seg" classList={{ on: i() === idx(), past: i() < idx() }}><i /><small>{PHASE[ph]()}</small></span>}</For>
            </div>
            <small class="cd-cap" title={t("cards.cl.phase.tip", "Fáze je odhad podle posledního nástroje, ne krok plánu.")}>
              {idx() >= 0 ? t("cards.cl.phase.guess", "{phase} · odhad podle posledního nástroje", { phase: PHASE[STRIP[idx()]]() }) : t("cards.cl.phase.unknown", "Pracuje · fázi neodhadneme")}
            </small>
            <Show when={file() || tool() || subs() > 0}>
              <div class="cd-live-file">
                <Show when={file()}>
                  {(f) => (
                    <span class="cd-live-fname" title={f().dir ? `${f().dir}/${f().name}` : f().name}>
                      <b>{f().name}</b>
                      <Show when={f().dir}><small>{f().dir}</small></Show>
                    </span>
                  )}
                </Show>
                <Show when={tool()}>{(x) => <small class="cd-live-tool">{x()}</small>}</Show>
                <Show when={subs() > 0}>
                  <small class="cd-live-tool">{t("cards.cl.subagents", "{n} subagent(ů) běží", { n: subs() })}</small>
                </Show>
              </div>
            </Show>
          </div>
        );
      }}
    </Show>
  );
}

export function DoneBody(p: { s: { sessions: SessionView[] }; now: number }) {
  const rows = () => doneList(p.s.sessions);
  const shown = () => rows().slice(0, 6);
  return (
    <div class="cd-done">
      <Show when={rows().length} fallback={<div class="cd-empty" role="status"><b>{t("cards.cl.doneNone", "Zatím nic dokončeno")}</b></div>}>
        <b class="cd-done-head">{t("cards.cl.doneHead", "Hotovo {n}", { n: rows().length })}</b>
        <For each={shown()}>
          {(ss) => {
            const msg = () => clipMessage(ss.activity?.message);
            const work = () => openWork(ss);
            return (
              <div class="cd-done-row" data-st={ss.status}>
                <span class="cd-done-main">
                  <span class="cd-done-line">
                    <b class="cd-done-proj" title={ss.cwd}>{projectOf(ss)}</b>
                    <small class="cd-done-when">{ago(doneAt(ss), p.now)}</small>
                    <Show when={work()}>
                      {(w) => (
                        <small class="cd-done-tag" title={t("cards.cl.work.tip", "{f} souborů bez commitu, {u} commitů bez pushe", { f: w().files, u: w().unpushed })}>
                          {w().files ? t("cards.cl.work.files", "{n} souborů", { n: w().files }) : t("cards.cl.work.unpushed", "{n} nepushnuto", { n: w().unpushed })}
                        </small>
                      )}
                    </Show>
                  </span>
                  <Show when={msg()}>{(m) => <small class="cd-done-msg" title={ss.activity?.message ?? ""}>{m()}</small>}</Show>
                </span>
                <button class="cd-done-open" disabled={!ss.canOpen} title={ss.openHint} onClick={() => void openSession(ss)}>{t("cards.cl.open", "Otevřít")}</button>
              </div>
            );
          }}
        </For>
        <Show when={rows().length > shown().length}>
          <small class="cd-cap">{t("cards.cd.sessions.more", "+{n} dalších", { n: rows().length - shown().length })}</small>
        </Show>
      </Show>
    </div>
  );
}
