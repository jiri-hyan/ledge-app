// Jedna dlaždice projektu (zástupce) v kartě Projekty i ve složce: logo nebo vlastní vzhled
// (`tilelook/TileIcon.tsx`, `ProjectView.look`), název / vlastní popisek, větev a git, prstenec běhu, akce
// UVNITŘ dlaždice dole (při najetí / fokusu: ▶ / ■ Stop, u větších dlaždic i poznámka, terminál, složka, …;
// vždy ⋯). Klik = spustit (víc složek s repem = nejdřív výběr, chybí složka = panel s chybou —
// `projects/state.ts` `startProject`). JEDNO menu dlaždice ve všech velikostech: pravý klik, ⋯, Shift+F10 /
// klávesa menu (`Projects.tsx` `menuFor`). Najetí = jen náhled (jméno, stav, čím a odkud — `projects/TileTip.tsx`).
// V režimu úprav šuplíku klik = „Upravit vzhled…“ a tah na jinou dlaždici = složka (`Projects.tsx` `onPointerDown`).
import { createMemo, For, Show } from "solid-js";
import { api, type ProjectView, type Snapshot } from "../api";
import { IconBell, IconFolder, IconGithub, IconLink, IconNote, IconTerminal } from "../icons";
import { logoSrc } from "../logos";
import type { NotePreset } from "../notes/store";
import { say, sayError } from "../ui/toast";
import { failInfo, healthText, healthTone, isLive, placeOf, tileHealth, tileTip } from "../projects/model";
import { startProject, stopProject, tipEnter, tipLeave } from "../projects/state";
import { t } from "../i18n";
import { BgTileButton } from "../sessions/BgLaunch";
import type { TileOpts } from "../layout/model";
import TileIcon from "../tilelook/TileIcon";
import { isPlain, tileLabel } from "../tilelook/model";
import { lookOf } from "../tilelook/state";

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

export async function cloneProject(pr: ProjectView) {
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
  const live = () => isLive(proc()?.phase);
  const failed = () => !!failInfo(proc());
  const place = () => placeOf(p.pr);
  const port = () => (proc()?.phase === "ready" ? proc()?.port : undefined);
  const look = () => lookOf("project", p.pr.id, p.pr.look);
  const label = () => p.tiles.label && !look()?.hideLabel;
  /** odznaky zdraví (neuložené změny, nepushnuté commity, Claude pracuje) — jen ze snapshotu */
  const health = createMemo(() => tileHealth(p.pr, p.s.sessions), null, { equals: (a, b) => a?.dirty === b?.dirty && a?.ahead === b?.ahead && a?.claude === b?.claude });
  const hit = () => (p.editing ? p.onLook() : startProject(pr(), proc()));
  /** běžící projekt: prstenec kolem loga (i kolem ikony s vlastním vzhledem) */
  const ring = () => <Show when={proc()}>{(pc) => <span class={`ring ${pc().phase}`} />}</Show>;
  return (
    <div
      class="proj"
      data-anim="tile"
      data-tl-tile={`p:${pr().id}`}
      ref={tilt}
      classList={{ missing: (place().kind === "missing" || place().kind === "none") && pr().kind === "dev", live: live(), failed: failed() && !live(), "proj-lifted": !!p.lifted, "proj-drop": !!p.dropAt }}
      style={{ "--a": look()?.color ?? pr().accent }}
      onContextMenu={(e) => p.onMenu(e)}
      onPointerDown={(e) => (tipLeave(), p.onPointerDown?.(e))}
      onPointerEnter={(e) => !p.editing && tipEnter(pr().id, e.currentTarget)}
      onPointerLeave={tipLeave}
      onFocusIn={(e) => !p.editing && tipEnter(pr().id, e.currentTarget)}
      onFocusOut={tipLeave}
    >
      <button
        class="proj-hit"
        onClick={hit}
        aria-description={[tileTip(pr(), proc()), health() ? healthText(health()!) : ""].filter(Boolean).join("\n")}
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
              <Show when={!failed() && place().kind !== "choose" && pr().git} fallback={
                failed() && !live() ? <span class="proj-bad">{t("cards.proj.tile.failed", "nenaběhl")}</span>
                  : place().kind === "choose" ? <span class="proj-ask">{t("cards.proj.tile.choose", "vyber složku")}</span>
                  : place().kind === "missing" ? t("cards.proj.tile.missing", "složka chybí")
                  : pr().pathOk ? " " : pr().cloning ? t("cards.projects.cloning", "klonuji…") : t("cards.projects.notFound", "nenalezeno")
              }>
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
      {/* malé odznaky v rohu (nejsou klikací; věta je v náhledu po najetí `TileTip`); kde je git řádek pod názvem, dirty a ahead nezdvojuje */}
      <Show when={health()}>
        {(h) => (
          <span class="proj-hl" data-tone={healthTone(h())} data-br={p.tiles.git ? "" : undefined} aria-hidden="true">
            <Show when={h().claude}><i class="proj-hl-c" /></Show>
            <Show when={h().dirty > 0}><i class="proj-hl-g proj-hl-d" /></Show>
            <Show when={h().ahead > 0}><i class="proj-hl-g proj-hl-a" /></Show>
          </span>
        )}
      </Show>
      {/* akce uvnitř dlaždice dole (překryjí jen její text), ikony s popiskem v title */}
      <Show when={!p.editing}>
        <div class="acts" role="toolbar" aria-label={t("cards.projects.actions", "Akce pro {name}", { name: pr().name })}>
          <Show when={live()} fallback={
            <Show when={pr().kind === "dev" || pr().pathOk}>
              <button title={t("cards.proj.run", "Spustit")} aria-label={t("cards.projects.launch", "Spustit {name}", { name: pr().name })} onClick={() => startProject(pr(), proc())}>▶</button>
            </Show>
          }>
            <button class="proj-stop" title={t("cards.proj.stopTitle", "Zastavit server (celý strom procesů)")} aria-label={t("cards.proj.stopOf", "Zastavit {name}", { name: pr().name })} onClick={() => void stopProject(pr().id, pr().name)}>■</button>
          </Show>
          <span class="proj-xa">
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
          </span>
          <button title={t("cards.proj.more", "Upravit, změnit složku nebo příkaz, vzhled…")} aria-label={t("cards.proj.moreOf", "Další volby: {name}", { name: pr().name })} aria-haspopup="menu" onClick={(e) => p.onMenu(e)}>⋯</button>
        </div>
      </Show>
    </div>
  );
}
