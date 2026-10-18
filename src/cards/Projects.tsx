// Karta Projekty (zástupci): dlaždice projektů (`ProjectTile.tsx`), klik = spustit / otevřít. Je to
// plnohodnotná karta (hlavička s ⋯ — velikost, výška, volby dlaždic `TileOptions`) a jde na libovolnou
// záložku; volby jsou na kartě té záložky (`layout…cards[].tiles`: velikost dlaždic, název, git, skryté
// projekty, složky). Varianta zobrazení (vzhled karty): dlaždice / seznam / kompaktní (`data-view`).
// Vlna 8 P: vlastní vzhled každé dlaždice (pravý klik / ⋯ v akcích / v režimu úprav klik → panel
// `tilelook/LookEditor.tsx` pod dlaždicemi) a složky jako na telefonu — v režimu úprav tah dlaždice na
// jinou = složka, na složku = do ní, ze složky ven = vyjmout (pointer události; HTML5 DnD v Tauri nejde).
// Vlna 9 Z: co a odkud se spouští — ⋯ / pravý klik = Upravit…, Změnit složku…, Změnit příkaz, Stop / Restart;
// panel `projects/ProjectPanel.tsx` pod dlaždicemi (i výběr z víc složek s jedním repem) a selhání startu
// (`FailBox`: krátká chyba, poslední řádky logu, Zobrazit log, Změnit složku…).
import { createEffect, createSignal, For, onCleanup, Show, type JSX } from "solid-js";
import type { ProjectView, Snapshot } from "../api";
import type { NotePreset } from "../notes/store";
import { t } from "../i18n";
import { gotoSettingsPanel } from "../palette/goto";
import { DEFAULT_TILES, pageOf, setTiles, tilesOf, visibleProjects, type Layout, type TileOpts } from "../layout/model";
import { addToFolder, arrange, dropOnTile, folderOf, removeFolder, renameFolder, takeOut, MAX_FOLDER_NAME, type TileFolder } from "../layout/folders";
import type { CardView } from "../layout/look";
import { change, draft, editing, offerUndo, saveLayout } from "../layout/edit";
import ProjectTile, { cloneProject, projectAuto } from "./ProjectTile";
import LookEditor from "../tilelook/LookEditor";
import TileIcon from "../tilelook/TileIcon";
import { lookOf, lookTarget, openLook, saveTileLook, wantKeys } from "../tilelook/state";
import { openTileMenu, type MenuItem } from "../tilelook/TileMenu";
import { isPlain } from "../tilelook/model";
import { logoSrc } from "../logos";
import ProjectPanel, { FailBox } from "../projects/ProjectPanel";
import { failInfo, isLive, placeOf } from "../projects/model";
import { changeFolder, closeProjectPanel, openProjectPanel, projectPanel, restartProject, startProject, stopProject, tip, tipLeave } from "../projects/state";
import TileTip from "../projects/TileTip";
import Popover, { boxOf } from "../ui/Popover";
import { tileColumns } from "../ui/place";
import { api } from "../api";
import { hideWithUndo } from "./CardMenu";
import "./projects.css";

type Drag = { id: string; x: number; y: number; over: string | null; moved: boolean };

/** Kotva popoveru = dlaždice projektu v téhle kartě (může být i ve složce). */
const tileBox = (id: string) => boxOf(document.querySelector(`[data-tl-tile="p:${CSS.escape(id)}"]`));

/** Mřížka vyplní celou šířku karty: počet sloupců z šířky (ResizeObserver — jen při změně velikosti) a počtu
 *  dlaždic (`ui/place.ts` `tileColumns`); `key` = velikost / varianta (mění `--tile-min`). */
function fitColumns(el: HTMLElement, key: () => string) {
  const [w, setW] = createSignal(0);
  const [n, setN] = createSignal(el.childElementCount);
  const ro = new ResizeObserver((es) => setW(es[0]?.contentRect.width ?? 0));
  const mo = new MutationObserver(() => setN(el.childElementCount));
  ro.observe(el);
  mo.observe(el, { childList: true });
  onCleanup(() => (ro.disconnect(), mo.disconnect()));
  createEffect(() => {
    void key();
    const cs = getComputedStyle(el);
    const min = parseFloat(cs.getPropertyValue("--tile-min")) || 118;
    const gap = parseFloat(cs.columnGap) || 0;
    el.style.setProperty("--cols", String(tileColumns(w(), min, gap, n())));
  });
}

export default function Projects(p: {
  s: Snapshot;
  /** záložka, na které karta je (složky a volby jsou její) */
  page: string;
  onNote: (preset: NotePreset) => void;
  onSettings: () => void;
  tiles?: TileOpts;
  view?: CardView;
  menu?: JSX.Element;
  /** „Ukázat všechny“ (všechny projekty na téhle záložce skryté) */
  onShowAll?: () => void;
}) {
  const tiles = () => p.tiles ?? DEFAULT_TILES;
  const list = () => visibleProjects(p.s.projects, tiles());
  const entries = () => arrange(list(), tiles());
  const [open, setOpen] = createSignal<string | null>(null);
  const [drag, setDrag] = createSignal<Drag | null>(null);
  const folder = (): { id: string; name: string; items: ProjectView[] } | null => {
    const e = entries().find((x) => x.kind === "folder" && x.f.id === open());
    return e && e.kind === "folder" ? { id: e.f.id, name: e.f.name, items: e.items } : null;
  };
  const addProject = () => {
    p.onSettings();
    gotoSettingsPanel("f-projects.add");
  };
  const byId = (id: string) => p.s.projects.find((x) => x.id === id);

  /** změna složek na téhle záložce: v režimu úprav do draftu (s Vrátit), jinak hned uložit */
  function applyTiles(f: (t: TileOpts) => TileOpts, undoText?: string) {
    const pageId = p.page;
    const upd = (l: Layout) => setTiles(l, pageId, { folders: f(tilesOf(pageOf(l, pageId).cards.find((c) => c.id === "projects"))).folders ?? [] });
    if (editing()) {
      const prev = draft();
      change(upd);
      if (undoText && prev) offerUndo(undoText, () => change(() => prev));
    } else void saveLayout(upd, p.s.cfgError);
  }

  const look = (pr: ProjectView) => (closeProjectPanel(), openLook({ kind: "project", id: pr.id, name: pr.name, initial: pr.look }, p.s.cfgError));
  function menuFor(e: MouseEvent, pr: ProjectView) {
    const proc = p.s.procs.find((x) => x.id === pr.id);
    const items: MenuItem[] = [];
    tipLeave();
    // JEDNO menu dlaždice (pravý klik, ⋯, Shift+F10) — stejné v každé velikosti a variantě karty
    if (isLive(proc?.phase)) {
      items.push({ text: "■ " + t("cards.proj.menu.stop", "Zastavit server"), run: () => void stopProject(pr.id, pr.name) });
      items.push({ text: t("cards.proj.menu.restart", "Restartovat"), run: () => void restartProject(pr.id, pr.name) });
    } else if (placeOf(pr).kind === "choose") items.push({ text: "▶ " + t("cards.proj.menu.choose", "Vybrat složku…"), run: () => openProjectPanel(pr.id, "choose") });
    else items.push({ text: "▶ " + t("cards.proj.run", "Spustit"), run: () => startProject(pr, proc) });
    if (proc?.url) items.push({ text: t("cards.proj.menu.browser", "Otevřít v prohlížeči"), run: () => void api.openUrl(proc.url!) });
    if (pr.pathOk) items.push({ text: t("cards.proj.menu.folder", "Otevřít složku"), run: () => void api.openIn(pr.id, "folder") });
    if (proc) items.push({ text: t("cards.proj.menu.log", "Zobrazit log"), run: () => openProjectPanel(pr.id, "edit", "log") });
    if (pr.clonable && !pr.cloning) items.push({ text: t("cards.projects.clone", "Naklonovat"), run: () => void cloneProject(pr) });
    items.push({ text: t("cards.proj.folder.change", "Změnit složku…"), run: () => void changeFolder(pr) });
    if (pr.kind === "dev") items.push({ text: t("cards.proj.menu.run", "Změnit příkaz…"), run: () => openProjectPanel(pr.id, "edit", "run") });
    items.push({ text: t("cards.proj.menu.look", "Vzhled…"), run: () => look(pr) });
    if (!isPlain(pr.look)) items.push({ text: t("cards.tile.resetLook", "Obnovit vzhled"), run: () => void saveTileLook("project", pr.id, undefined) });
    items.push({ text: t("cards.proj.edit.open", "Upravit…"), run: () => openProjectPanel(pr.id, "edit") });
    if (folderOf(tiles(), pr.id)) items.push({ text: t("cards.folder.takeOut", "Vyjmout ze složky"), run: () => applyTiles((x) => takeOut(x, pr.id)) });
    items.push({ text: t("cards.proj.menu.remove", "Odebrat z téhle záložky"), danger: true, run: () => removeHere(pr) });
    openTileMenu(e, items);
  }
  /** „Odebrat“ = skrýt dlaždici na téhle záložce (`tiles.hidden`), s Vrátit; projekt v configu zůstává. */
  function removeHere(pr: ProjectView) {
    const pageId = p.page;
    const hide = (l: Layout, on: boolean) => {
      const cur = tilesOf(pageOf(l, pageId).cards.find((c) => c.id === "projects")).hidden;
      return setTiles(l, pageId, { hidden: on ? [...new Set([...cur, pr.id])] : cur.filter((x) => x !== pr.id) });
    };
    const text = t("cards.proj.removed", "„{name}“ odebrán z téhle záložky (projekt zůstává v Nastavení)", { name: pr.name });
    if (editing()) {
      change((l) => hide(l, true));
      offerUndo(text, () => change((l) => hide(l, false)));
    } else void hideWithUndo(text, (l) => ({ layout: hide(l, true), undo: (x) => hide(x, false) }), p.s.cfgError);
  }
  function folderMenu(e: MouseEvent, f: TileFolder) {
    openTileMenu(e, [
      { text: t("cards.folder.open", "Otevřít složku"), run: () => setOpen(f.id) },
      { text: t("cards.folder.remove", "Zrušit složku (projekty zůstanou)"), run: () => removeOne(f) },
    ]);
  }
  function removeOne(f: Pick<TileFolder, "id" | "name">) {
    if (open() === f.id) setOpen(null);
    applyTiles((x) => removeFolder(x, f.id), t("cards.folder.removed", "Složka „{name}“ zrušena", { name: folderName(f) }));
  }

  /** režim úprav: tah dlaždice (práh 6 px), pustit na dlaždici = složka, na složku = do ní, ze složky ven = vyjmout;
   *  bez pohybu = klik (vzhled) */
  function startDrag(e: PointerEvent, id: string) {
    if (!editing() || e.button !== 0) return;
    const x0 = e.clientX;
    const y0 = e.clientY;
    const inFolder = !!folderOf(tiles(), id);
    let d: Drag = { id, x: x0, y: y0, over: null, moved: false };
    const at = (x: number, y: number) => {
      const hit = document.elementFromPoint(x, y) as HTMLElement | null;
      const tile = hit?.closest<HTMLElement>("[data-tl-tile]")?.dataset.tlTile ?? null;
      const inPanel = !!hit?.closest("[data-tl-fpanel]");
      return tile && tile !== `p:${id}` ? tile : inPanel ? "panel" : null;
    };
    // bez pointer capture: klik bez pohybu musí dojít na tlačítko dlaždice (vzhled)
    const move = (ev: PointerEvent) => {
      if (!d.moved && Math.hypot(ev.clientX - x0, ev.clientY - y0) < 6) return;
      d = { id, x: ev.clientX, y: ev.clientY, over: at(ev.clientX, ev.clientY), moved: true };
      setDrag(d);
    };
    const stop = () => {
      window.removeEventListener("pointermove", move);
      window.removeEventListener("pointerup", end);
      window.removeEventListener("pointercancel", stop);
      setDrag(null);
    };
    const end = () => {
      stop();
      if (!d.moved) return;
      // tah není klik: případný click po puštění nad dlaždicí spolknout
      const eat = (c: MouseEvent) => (c.stopPropagation(), c.preventDefault());
      window.addEventListener("click", eat, { capture: true, once: true });
      setTimeout(() => window.removeEventListener("click", eat, { capture: true }), 0);
      drop(id, d.over, inFolder);
    };
    window.addEventListener("pointermove", move);
    window.addEventListener("pointerup", end);
    window.addEventListener("pointercancel", stop);
  }
  function drop(id: string, over: string | null, inFolder: boolean) {
    const name = byId(id)?.name ?? id;
    if (over?.startsWith("p:")) {
      const target = over.slice(2);
      let made: string | null = null;
      applyTiles((x) => {
        const r = dropOnTile(x, id, target);
        made = r.folder;
        return r.tiles;
      }, t("cards.folder.made", "„{name}“ ve složce", { name }));
      if (made) setOpen(made);
    } else if (over?.startsWith("f:")) {
      applyTiles((x) => addToFolder(x, over.slice(2), id), t("cards.folder.made", "„{name}“ ve složce", { name }));
    } else if (inFolder && over !== "panel") {
      applyTiles((x) => takeOut(x, id), t("cards.folder.out", "„{name}“ vyjmut ze složky", { name }));
    }
  }

  const folderName = (f: Pick<TileFolder, "name">) => f.name || t("cards.folder.name", "Složka");
  const tile = (pr: ProjectView) => (
    <ProjectTile
      s={p.s}
      pr={pr}
      tiles={tiles()}
      editing={editing()}
      onNote={p.onNote}
      onLook={() => look(pr)}
      onMenu={(e) => menuFor(e, pr)}
      onPointerDown={(e) => startDrag(e, pr.id)}
      lifted={drag()?.id === pr.id}
      dropAt={drag()?.over === `p:${pr.id}`}
    />
  );

  return (
    <div class="card proj-card" data-anim>
      <h4>
        <span class="ic" style="background:var(--accent)" />
        {t("cards.projects.title", "Projekty")}
        <span class="n">{list().length || ""}</span>
        {p.menu}
      </h4>
      <div class="projects proj-fit" ref={(el) => fitColumns(el, () => `${tiles().size}|${p.view ?? "tiles"}`)} data-tiles={tiles().size} data-view={p.view ?? "tiles"} data-edit={editing() ? "" : undefined}>
        <For each={entries()} fallback={
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
          {(e) => (
            <Show when={e.kind === "folder" ? e : null} fallback={e.kind === "project" ? tile(e.p) : null}>
              {(f) => (
                <div
                  class="proj proj-folder"
                  data-tl-tile={`f:${f().f.id}`}
                  classList={{ "proj-drop": drag()?.over === `f:${f().f.id}`, "proj-open": open() === f().f.id }}
                  onContextMenu={(ev) => folderMenu(ev, f().f)}
                >
                  <button class="proj-hit" aria-expanded={open() === f().f.id} onClick={() => setOpen(open() === f().f.id ? null : f().f.id)} aria-label={t("cards.folder.aria", "Složka {name}, projektů: {n}", { name: folderName(f().f), n: f().items.length })}>
                    <div class="proj-fbox">
                      <For each={f().items.slice(0, 4)}>
                        {(pr) => (
                          <Show when={!isPlain(lookOf("project", pr.id, pr.look))} fallback={
                            <span class="proj-fmini" style={{ background: pr.accent }}>
                              <Show when={logoSrc(pr)} fallback={pr.monogram}>{(src) => <img src={src()} alt="" draggable={false} />}</Show>
                            </span>
                          }>
                            <TileIcon look={lookOf("project", pr.id, pr.look)!} auto={projectAuto(pr)} size="calc(var(--logo) * 0.4)" />
                          </Show>
                        )}
                      </For>
                    </div>
                    <Show when={tiles().label}><div class="nm">{folderName(f().f)}</div></Show>
                    <Show when={tiles().git}><div class="br">{f().items.length}×</div></Show>
                  </button>
                </div>
              )}
            </Show>
          )}
        </For>
      </div>
      <Show when={folder()}>
        {(f) => (
          <div class="proj-fpanel" data-tl-fpanel>
            <div class="proj-fhead">
              <input
                type="text"
                class="proj-fname"
                maxLength={MAX_FOLDER_NAME}
                value={f().name}
                placeholder={t("cards.folder.name", "Složka")}
                aria-label={t("cards.folder.rename", "Název složky")}
                onFocus={wantKeys}
                onChange={(ev) => applyTiles((x) => renameFolder(x, f().id, ev.currentTarget.value))}
              />
              <Show when={editing()}><small>{t("cards.folder.dragOut", "Táhni dlaždici ven = vyjmout")}</small></Show>
              <span class="grow" />
              <button class="mini" onClick={() => removeOne(f())}>{t("cards.folder.removeShort", "Zrušit složku")}</button>
              <button class="mini" aria-label={t("cards.folder.close", "Zavřít složku")} title={t("cards.folder.close", "Zavřít složku")} onClick={() => setOpen(null)}>×</button>
            </div>
            <div class="projects proj-fit" ref={(el) => fitColumns(el, () => `${tiles().size}|${p.view ?? "tiles"}`)} data-tiles={tiles().size} data-view={p.view ?? "tiles"} data-edit={editing() ? "" : undefined}>
              <For each={f().items}>{(pr) => tile(pr)}</For>
            </div>
          </div>
        )}
      </Show>
      <For each={p.s.procs.filter((x) => failInfo(x) && projectPanel()?.id !== x.id && list().some((pr) => pr.id === x.id))}>
        {(pc) => <FailBox s={p.s} pr={byId(pc.id)!} proc={pc} />}
      </For>
      {/* Upravit… / výběr složky / Vzhled… = popover nad šuplíkem u dlaždice (ne v toku stránky, karta ho neořízne) */}
      <Show when={projectPanel() && byId(projectPanel()!.id) && list().some((x) => x.id === projectPanel()!.id) ? projectPanel()!.id : null} keyed>
        {(id) => (
          <Popover anchor={() => tileBox(id)} label={byId(id)?.name ?? id}>
            <ProjectPanel s={p.s} pr={byId(id)!} onSettings={p.onSettings} />
          </Popover>
        )}
      </Show>
      <Show when={lookTarget()?.kind === "project" && byId(lookTarget()!.id) ? lookTarget() : null} keyed>
        {(tg) => (
          <Popover anchor={() => tileBox(tg.id)} label={tg.name}>
            <LookEditor auto={projectAuto(byId(tg.id)!)} editing={editing()} />
          </Popover>
        )}
      </Show>
      {/* náhled po najetí: jméno, stav, čím a odkud (jen informace — akce jsou v menu) */}
      <Show when={tip() && byId(tip()!.id) && !projectPanel() && !lookTarget() ? tip() : null} keyed>
        {(tp) => (
          <Popover anchor={() => tileBox(tp.id) ?? boxOf(tp.el)} label={byId(tp.id)!.name} passive>
            <TileTip pr={byId(tp.id)!} proc={p.s.procs.find((x) => x.id === tp.id)} />
          </Popover>
        )}
      </Show>
      <Show when={drag()?.moved ? drag() : null}>
        {(d) => (
          <div class="tl-float" style={{ left: `${d().x}px`, top: `${d().y}px` }} aria-hidden="true">
            {byId(d().id)?.name}
            <Show when={d().over?.startsWith("p:") || d().over?.startsWith("f:")}> → {t("cards.folder.into", "do složky")}</Show>
          </div>
        )}
      </Show>
    </div>
  );
}
