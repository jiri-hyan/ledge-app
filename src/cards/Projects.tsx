// Karta Projekty (zástupci): dlaždice projektů, klik = spustit / otevřít (`api.launch`). Je to plnohodnotná
// karta (hlavička s ⋯ — velikost, výška, volby dlaždic `TileOptions`) a jde na libovolnou záložku; volby
// jsou na kartě té záložky (`layout…cards[].tiles`: velikost dlaždic, název, git, skryté projekty).
// Akce projektu (poznámka, připomínka, terminál, složka, Claude na pozadí, klon, GitHub, odkazy) se při
// najetí / fokusu ukážou UVNITŘ dlaždice dole (překryjí jen její název a větev) — nikdy přes kartu pod ní
// a bez posunu rozložení. Stav běhu z `Snapshot.procs` (prstenec, port), loga přes `logos.ts`.
import { createMemo, For, Show, type JSX } from "solid-js";
import { api, type ProjectView, type Snapshot } from "../api";
import { IconBell, IconFolder, IconGithub, IconLink, IconNote, IconTerminal } from "../icons";
import { logoSrc } from "../logos";
import type { NotePreset } from "../notes/store";
import { say, sayError } from "../ui/toast";
import { t } from "../i18n";
import { BgTileButton } from "../sessions/BgLaunch";
import { gotoSettingsPanel } from "../palette/goto";
import { DEFAULT_TILES, visibleProjects, type TileOpts } from "../layout/model";
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

export default function Projects(p: {
  s: Snapshot;
  onNote: (preset: NotePreset) => void;
  onSettings: () => void;
  tiles?: TileOpts;
  menu?: JSX.Element;
  /** „Ukázat všechny“ (všechny projekty na téhle záložce skryté) */
  onShowAll?: () => void;
}) {
  const procOf = (id: string) => p.s.procs.find((x) => x.id === id);
  const tiles = () => p.tiles ?? DEFAULT_TILES;
  const list = () => visibleProjects(p.s.projects, tiles());
  const addProject = () => {
    p.onSettings();
    gotoSettingsPanel("f-projects.add");
  };
  return (
    <div class="card proj-card" data-anim>
      <h4>
        <span class="ic" style="background:var(--accent)" />
        {t("cards.projects.title", "Projekty")}
        <span class="n">{list().length || ""}</span>
        {p.menu}
      </h4>
      <div class="projects" data-tiles={tiles().size}>
        <For each={list()} fallback={
          <Show when={p.s.projects.length} fallback={
            <div class="empty proj-empty" data-anim>
              {t("cards.projects.emptyShort", "Zatím žádné projekty — dlaždice spustí server, otevře složku nebo aplikaci.")}
              <button class="mini ok" onClick={addProject}>{t("cards.projects.add", "Přidat projekt")}</button>
            </div>
          }>
            <div class="empty proj-empty" data-anim>
              {t("cards.projects.allHidden", "Na téhle záložce jsou všechny projekty skryté.")}
              <Show when={p.onShowAll}><button class="mini" onClick={() => p.onShowAll?.()}>{t("cards.projects.showAll", "Ukázat všechny")}</button></Show>
            </div>
          </Show>
        }>
          {(pr) => {
            const proc = createMemo(() => procOf(pr.id));
            const live = () => ["ready", "running", "starting", "preparing"].includes(proc()?.phase ?? "");
            const port = () => (proc()?.phase === "ready" ? proc()?.port : undefined);
            return (
              <div class="proj" data-anim="tile" ref={tilt} classList={{ missing: !pr.pathOk && pr.kind === "dev", live: live() }} style={{ "--a": pr.accent }} title={pr.path ?? t("cards.projects.folderMissing", "Složka nenalezena")}>
                <button class="proj-hit" onClick={() => launch(pr)} aria-label={t("cards.projects.launch", "Spustit {name}", { name: pr.name })}>
                  <div class="logo" classList={{ "has-img": !!logoSrc(pr) }} style={{ background: `linear-gradient(135deg, ${pr.accent}, color-mix(in srgb, ${pr.accent} 45%, #000))` }}>
                    <Show when={logoSrc(pr)} fallback={pr.monogram}>{(src) => <img src={src()} alt="" draggable={false} />}</Show>
                    <Show when={proc()}>{(pc) => <span class={`ring ${pc().phase}`} />}</Show>
                  </div>
                  <Show when={tiles().label}><div class="nm">{pr.name}</div></Show>
                  <Show when={port() || tiles().git}>
                    <div class="br">
                      <Show when={port()} fallback={
                        <Show when={pr.git} fallback={pr.pathOk ? " " : pr.cloning ? t("cards.projects.cloning", "klonuji…") : t("cards.projects.notFound", "nenalezeno")}>
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
                        <span class="live-port">● :{port()}</span>
                      </Show>
                    </div>
                  </Show>
                </button>
                {/* akce uvnitř dlaždice dole (překryjí jen její text), ikony s popiskem v title */}
                <div class="acts" role="toolbar" aria-label={t("cards.projects.actions", "Akce pro {name}", { name: pr.name })}>
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
                    <button
                      title={pr.cloning ? t("cards.projects.cloningBtn", "Klonuji…") : t("cards.projects.cloneTitle", "Naklonovat do první složky z roots (gh repo clone)")}
                      aria-label={t("cards.projects.clone", "Naklonovat")}
                      disabled={pr.cloning}
                      onClick={() => cloneProject(pr)}
                    ><IconFolder /></button>
                  </Show>
                  <Show when={pr.github}>{(u) => <button title="GitHub" aria-label={t("cards.projects.onGithub", "{name} na GitHubu", { name: pr.name })} onClick={() => api.openUrl(u())}><IconGithub /></button>}</Show>
                  <For each={pr.links}>{(l) => <button title={`${l.label} · ${l.url}`} aria-label={l.label} onClick={() => api.openUrl(l.url)}><IconLink /></button>}</For>
                </div>
              </div>
            );
          }}
        </For>
      </div>
    </div>
  );
}
