// Dlaždice projektů: klik = spustit / otevřít (`api.launch`), pod dlaždicí akce (poznámka, terminál,
// složka, klon, GitHub, odkazy). Stav běhu z `Snapshot.procs` (prstenec, port), loga přes `logos.ts`.
import { createMemo, For, Show } from "solid-js";
import { api, type ProjectView, type Snapshot } from "../api";
import { IconBell, IconFolder, IconGithub, IconLink, IconNote, IconTerminal } from "../icons";
import { logoSrc } from "../logos";
import type { NotePreset } from "../notes/store";
import { say, sayError } from "../ui/toast";
import { t } from "../i18n";
import { BgTileButton } from "../sessions/BgLaunch";
import "./projects.css";

async function launch(pr: ProjectView) {
  try {
    const r = await api.launch(pr.id);
    if (r?.outcome === "alreadyRunning") {
      if (r.url) api.openUrl(r.url);
      else say(t("cards.projects.alreadyRunning", "{name} už běží.", { name: pr.name }));
    } else {
      say(pr.kind === "dev" ? t("cards.projects.starting", "Spouštím {run} · {name}", { run: pr.run ?? "server", name: pr.name }) : t("cards.projects.opening", "Otevírám {name}…", { name: pr.name }));
    }
  } catch (e) {
    sayError(e);
  }
}

/** 3D náklon dlaždice za kurzorem (max ~±6°) — jen transform, žádný layout. */
function tilt(el: HTMLElement) {
  el.addEventListener(
    "pointermove",
    (e: PointerEvent) => {
      const r = el.getBoundingClientRect();
      const x = (e.clientX - r.left) / r.width - 0.5;
      const y = (e.clientY - r.top) / r.height - 0.5;
      el.style.setProperty("--rx", `${(-y * 10).toFixed(2)}deg`);
      el.style.setProperty("--ry", `${(x * 12).toFixed(2)}deg`);
    },
    { passive: true },
  );
  el.addEventListener("pointerleave", () => {
    el.style.setProperty("--rx", "0deg");
    el.style.setProperty("--ry", "0deg");
  });
}


async function cloneProject(pr: ProjectView) {
  say(t("cards.projects.cloningToast", "Klonuji {name}… (velké repo i pár minut)", { name: pr.name }));
  try {
    say((await api.cloneProject(pr.id)) ?? t("cards.projects.done", "Hotovo"));
  } catch (e) {
    sayError(e);
  }
}

export default function Projects(p: { s: Snapshot; onNote: (preset: NotePreset) => void }) {
  const procOf = (id: string) => p.s.procs.find((x) => x.id === id);
  return (
    <div class="projects">
      <For each={p.s.projects} fallback={<div class="empty" data-anim>{t("cards.projects.empty", "Žádné projekty. Přidej je v config.json (Nastavení → Otevřít config).")}</div>}>
        {(pr) => {
          const proc = createMemo(() => procOf(pr.id));
          const live = () => ["ready", "running", "starting", "preparing"].includes(proc()?.phase ?? "");
          return (
            <div class="proj" data-anim="tile" ref={tilt} classList={{ missing: !pr.pathOk && pr.kind === "dev", live: live() }} style={{ "--a": pr.accent }} title={pr.path ?? t("cards.projects.folderMissing", "Složka nenalezena")}>
              <button class="proj-hit" onClick={() => launch(pr)} aria-label={t("cards.projects.launch", "Spustit {name}", { name: pr.name })}>
                <div class="logo" classList={{ "has-img": !!logoSrc(pr) }} style={{ background: `linear-gradient(135deg, ${pr.accent}, color-mix(in srgb, ${pr.accent} 45%, #000))` }}>
                  <Show when={logoSrc(pr)} fallback={pr.monogram}>{(src) => <img src={src()} alt="" draggable={false} />}</Show>
                  <Show when={proc()}>{(pc) => <span class={`ring ${pc().phase}`} />}</Show>
                </div>
                <div class="nm">{pr.name}</div>
                <div class="br">
                  <Show when={proc()?.phase === "ready" && proc()?.port} fallback={
                    <Show when={pr.git} fallback={pr.pathOk ? " " : pr.cloning ? t("cards.projects.cloning", "klonuji…") : t("cards.projects.notFound", "nenalezeno")}>
                      {(g) => (
                        <>
                          {g().branch ?? "detached"}
                          <Show when={g().dirty > 0}> <span class="dirty">●{g().dirty}</span></Show>
                          <Show when={g().ahead > 0}> <span class="ahead">↑{g().ahead}</span></Show>
                          <Show when={g().behind > 0}> ↓{g().behind}</Show>
                        </>
                      )}
                    </Show>
                  }>
                    <span class="live-port">● :{proc()!.port}</span>
                  </Show>
                </div>
              </button>
              <div class="acts">
                <Show when={p.s.notesEnabled}>
                  <button title={t("cards.projects.note", "Poznámka")} aria-label={t("cards.projects.noteFor", "Poznámka k {name}", { name: pr.name })} onClick={() => p.onNote({ project: pr.id, remind: false })}><IconNote /></button>
                  <button title={t("cards.projects.reminder", "Připomínka")} aria-label={t("cards.projects.reminderFor", "Připomínka k {name}", { name: pr.name })} onClick={() => p.onNote({ project: pr.id, remind: true })}><IconBell /></button>
                </Show>
                <Show when={pr.pathOk}>
                  <button title={t("cards.projects.terminal", "Terminál")} aria-label={t("cards.projects.terminalIn", "Terminál v {name}", { name: pr.name })} onClick={() => api.openIn(pr.id, "terminal")}><IconTerminal /></button>
                  <button title={t("cards.projects.folder", "Složka")} aria-label={t("cards.projects.folderOf", "Složka {name}", { name: pr.name })} onClick={() => api.openIn(pr.id, "folder")}><IconFolder /></button>
                </Show>
                <BgTileButton s={p.s} pr={pr} />
                <Show when={pr.clonable}>
                  <button title={t("cards.projects.cloneTitle", "Naklonovat do první složky z roots (gh repo clone)")} disabled={pr.cloning} onClick={() => cloneProject(pr)}><IconFolder />{pr.cloning ? t("cards.projects.cloningBtn", "Klonuji…") : t("cards.projects.clone", "Naklonovat")}</button>
                </Show>
                <Show when={pr.github}>{(u) => <button title="GitHub" aria-label={t("cards.projects.onGithub", "{name} na GitHubu", { name: pr.name })} onClick={() => api.openUrl(u())}><IconGithub /></button>}</Show>
                <For each={pr.links}>{(l) => <button title={l.url} onClick={() => api.openUrl(l.url)}><IconLink />{l.label}</button>}</For>
              </div>
            </div>
          );
        }}
      </For>
    </div>
  );
}
