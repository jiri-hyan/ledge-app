// Jedna dlaždice projektu (zástupce) v kartě Projekty i ve složce: logo nebo vlastní vzhled
// (`tilelook/TileIcon.tsx`, `ProjectView.look`), název / vlastní popisek, větev a git, prstenec běhu, akce
// UVNITŘ dlaždice dole (při najetí). Klik = spustit; v režimu úprav šuplíku klik = „Upravit vzhled…“ a tah
// na jinou dlaždici = složka (obsluhuje `Projects.tsx` přes `onPointerDown`). Pravý klik = malé menu.
import { createMemo, For, Show } from "solid-js";
import { api, type ProjectView, type Snapshot } from "../api";
import { IconBell, IconFolder, IconGithub, IconLink, IconNote, IconTerminal } from "../icons";
import { logoSrc } from "../logos";
import type { NotePreset } from "../notes/store";
import { say, sayError } from "../ui/toast";
import { t } from "../i18n";
import { BgTileButton } from "../sessions/BgLaunch";
import type { TileOpts } from "../layout/model";
import TileIcon from "../tilelook/TileIcon";
import { isPlain, tileLabel } from "../tilelook/model";
import { lookOf } from "../tilelook/state";

export async function launch(pr: ProjectView) {
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
export function tilt(el: HTMLElement) {
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

export const projectAuto = (pr: ProjectView) => ({ src: logoSrc(pr), mono: pr.monogram, color: pr.accent, plate: true });

export default function ProjectTile(p: {
  s: Snapshot;
  pr: ProjectView;
  tiles: TileOpts;
  editing: boolean;
  onNote: (preset: NotePreset) => void;
  /** režim úprav: klik = vzhled; mimo něj ⋯ v akcích */
  onLook: () => void;
  onMenu: (e: MouseEvent) => void;
  onPointerDown?: (e: PointerEvent) => void;
  lifted?: boolean;
  dropAt?: boolean;
}) {
  const pr = () => p.pr;
  const proc = createMemo(() => p.s.procs.find((x) => x.id === p.pr.id));
  const live = () => ["ready", "running", "starting", "preparing"].includes(proc()?.phase ?? "");
  const port = () => (proc()?.phase === "ready" ? proc()?.port : undefined);
  const look = () => lookOf("project", p.pr.id, p.pr.look);
  const label = () => p.tiles.label && !look()?.hideLabel;
  const hit = () => (p.editing ? p.onLook() : void launch(pr()));
  /** běžící projekt: prstenec kolem loga (i kolem ikony s vlastním vzhledem) */
  const ring = () => <Show when={proc()}>{(pc) => <span class={`ring ${pc().phase}`} />}</Show>;
  return (
    <div
      class="proj"
      data-anim="tile"
      data-tl-tile={`p:${pr().id}`}
      ref={tilt}
      classList={{ missing: !pr().pathOk && pr().kind === "dev", live: live(), "proj-lifted": !!p.lifted, "proj-drop": !!p.dropAt }}
      style={{ "--a": look()?.color ?? pr().accent }}
      title={pr().path ?? t("cards.projects.folderMissing", "Složka nenalezena")}
      onContextMenu={(e) => p.onMenu(e)}
      onPointerDown={(e) => p.onPointerDown?.(e)}
    >
      <button
        class="proj-hit"
        onClick={hit}
        aria-label={p.editing ? t("cards.tile.editOf", "Upravit vzhled: {name}", { name: pr().name }) : t("cards.projects.launch", "Spustit {name}", { name: pr().name })}
      >
        <Show when={look() && !isPlain(look()) ? look() : null} fallback={
          <div class="logo" classList={{ "has-img": !!logoSrc(pr()) }} style={{ background: `linear-gradient(135deg, ${pr().accent}, color-mix(in srgb, ${pr().accent} 45%, #000))` }}>
            <Show when={logoSrc(pr())} fallback={pr().monogram}>{(src) => <img src={src()} alt="" draggable={false} />}</Show>
            {ring()}
          </div>
        }>
          {(l) => (
            <TileIcon look={l()} auto={projectAuto(pr())} size="var(--logo)" logo>
              {ring()}
            </TileIcon>
          )}
        </Show>
        <Show when={label()}><div class="nm">{tileLabel(pr().name, look())}</div></Show>
        <Show when={port() || p.tiles.git}>
          <div class="br">
            <Show when={port()} fallback={
              <Show when={pr().git} fallback={pr().pathOk ? " " : pr().cloning ? t("cards.projects.cloning", "klonuji…") : t("cards.projects.notFound", "nenalezeno")}>
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
      <Show when={!p.editing}>
        <div class="acts" role="toolbar" aria-label={t("cards.projects.actions", "Akce pro {name}", { name: pr().name })}>
          <Show when={p.s.notesEnabled}>
            <button title={t("cards.projects.note", "Poznámka")} aria-label={t("cards.projects.noteFor", "Poznámka k {name}", { name: pr().name })} onClick={() => p.onNote({ project: pr().id, remind: false })}><IconNote /></button>
            <button title={t("cards.projects.reminder", "Připomínka")} aria-label={t("cards.projects.reminderFor", "Připomínka k {name}", { name: pr().name })} onClick={() => p.onNote({ project: pr().id, remind: true })}><IconBell /></button>
          </Show>
          <Show when={pr().pathOk}>
            <button title={t("cards.projects.terminal", "Terminál")} aria-label={t("cards.projects.terminalIn", "Terminál v {name}", { name: pr().name })} onClick={() => api.openIn(pr().id, "terminal")}><IconTerminal /></button>
            <button title={t("cards.projects.folder", "Složka")} aria-label={t("cards.projects.folderOf", "Složka {name}", { name: pr().name })} onClick={() => api.openIn(pr().id, "folder")}><IconFolder /></button>
          </Show>
          <BgTileButton s={p.s} pr={pr()} />
          <Show when={pr().clonable}>
            <button
              title={pr().cloning ? t("cards.projects.cloningBtn", "Klonuji…") : t("cards.projects.cloneTitle", "Naklonovat do první složky z roots (gh repo clone)")}
              aria-label={t("cards.projects.clone", "Naklonovat")}
              disabled={pr().cloning}
              onClick={() => cloneProject(pr())}
            ><IconFolder /></button>
          </Show>
          <Show when={pr().github}>{(u) => <button title="GitHub" aria-label={t("cards.projects.onGithub", "{name} na GitHubu", { name: pr().name })} onClick={() => api.openUrl(u())}><IconGithub /></button>}</Show>
          <For each={pr().links}>{(l) => <button title={`${l.label} · ${l.url}`} aria-label={l.label} onClick={() => api.openUrl(l.url)}><IconLink /></button>}</For>
          <button title={t("cards.tile.edit", "Upravit vzhled…")} aria-label={t("cards.tile.editOf", "Upravit vzhled: {name}", { name: pr().name })} onClick={p.onLook}>⋯</button>
        </div>
      </Show>
    </div>
  );
}
