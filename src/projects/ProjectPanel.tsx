// Panel u dlaždice karty Projekty (vlna 9 Z) — v popoveru nad šuplíkem (`ui/Popover.tsx`), jeden naráz (`projectPanel`):
// - „Upravit…“ = co dlaždice spustí a odkud: příkaz (pole, Enter uloží `run`), složka (Změnit složku… /
//   jiná složka s tímhle repem), port, repo, chyba místa a selhání startu, Spustit / Stop / Restart;
// - „Vybrat složku“ = víc složek s jedním repem: seznam s datem posledního commitu, nejčerstvější
//   předvybraná a označená, volba se uloží do `path` (příště se už neptá).
// `FailBox` = selhání startu přímo v kartě (krátká chyba, poslední řádky logu, Zobrazit log, Změnit složku…).
import { createEffect, createResource, createSignal, For, onCleanup, onMount, Show } from "solid-js";
import { api, type ProcView, type ProjectView, type Snapshot } from "../api";
import { t } from "../i18n";
import { gotoSettingsPanel } from "../palette/goto";
import { commitText, failInfo, freshest, isLive, placeOf, placeProblem, preselect, samePath, whatOf, whereOf } from "./model";
import { changeFolder, closeProjectPanel, launch, openProjectPanel, projectPanel, restartProject, saveProjectFields, stopProject, wantKeys } from "./state";
import "./projpanel.css";

export default function ProjectPanel(p: { s: Snapshot; pr: ProjectView; onSettings: () => void }) {
  let box!: HTMLDivElement;
  const proc = () => p.s.procs.find((x) => x.id === p.pr.id);
  const mode = () => projectPanel()?.mode ?? "edit";
  // klik mimo panel a dlaždice = zavřít, Esc taky (posluchače jen po dobu otevření)
  onMount(() => {
    const down = (e: PointerEvent) => {
      const el = e.target as HTMLElement | null;
      if (!el || box.contains(el) || el.closest("[data-tl-tile], .tl-menu")) return;
      closeProjectPanel();
    };
    const key = (e: KeyboardEvent) => e.key === "Escape" && (e.stopPropagation(), closeProjectPanel());
    document.addEventListener("pointerdown", down, true);
    window.addEventListener("keydown", key, true);
    onCleanup(() => {
      document.removeEventListener("pointerdown", down, true);
      window.removeEventListener("keydown", key, true);
    });
  });
  return (
    <div class="pz-panel" ref={box} role="dialog" aria-label={t("cards.proj.panel.aria", "Projekt {name}: co a odkud se spouští", { name: p.pr.name })}>
      <div class="pz-head">
        <b>{p.pr.name}</b>
        <small>{mode() === "choose" ? t("cards.proj.choose.title", "Ze které složky spouštět?") : t("cards.proj.edit.title", "Co se spustí a odkud")}</small>
        <span class="grow" />
        <button class="mini" aria-label={t("cards.proj.panel.close", "Zavřít")} title={t("cards.proj.panel.close", "Zavřít")} onClick={closeProjectPanel}>×</button>
      </div>
      <Show when={mode() === "choose"} fallback={<Edit s={p.s} pr={p.pr} proc={proc()} onSettings={p.onSettings} />}>
        <Choose pr={p.pr} now={Date.now()} />
      </Show>
    </div>
  );
}

function Edit(p: { s: Snapshot; pr: ProjectView; proc: ProcView | undefined; onSettings: () => void }) {
  let input: HTMLInputElement | undefined;
  const [run, setRun] = createSignal(p.pr.run ?? "");
  // jiný projekt / uložená změna zvenku → pole podle configu (rozepsané se nepřepíše)
  createEffect(() => {
    const saved = p.pr.run ?? "";
    if (document.activeElement !== input) setRun(saved);
  });
  onMount(() => {
    if (projectPanel()?.focus === "run" && input) {
      wantKeys();
      input.focus();
      input.select();
    }
  });
  const live = () => isLive(p.proc?.phase);
  const problem = () => placeProblem(p.pr);
  const saveRun = async () => {
    const v = run().trim();
    if (v === (p.pr.run ?? "").trim()) return;
    if (await saveProjectFields(p.pr.id, { run: v })) setRun(v);
  };
  const port = () => p.proc?.port ?? p.pr.port;
  return (
    <>
      <div class="pz-row">
        <span class="pz-cap">{p.pr.kind === "dev" ? t("cards.proj.edit.what", "Spustí") : t("cards.proj.edit.app", "Otevře")}</span>
        <Show when={p.pr.kind === "dev"} fallback={<code class="pz-val">{whatOf(p.pr) ?? "—"}</code>}>
          <input
            ref={input}
            class="pz-input"
            type="text"
            spellcheck={false}
            value={run()}
            placeholder={t("cards.proj.edit.runPh", "např. npm run dev")}
            aria-label={t("cards.proj.edit.runAria", "Příkaz, který dlaždice spustí")}
            onFocus={wantKeys}
            onInput={(e) => setRun(e.currentTarget.value)}
            onKeyDown={(e) => {
              if (e.key === "Enter") void saveRun();
              if (e.key === "Escape") setRun(p.pr.run ?? "");
            }}
            onBlur={() => void saveRun()}
          />
        </Show>
      </div>
      <Show when={p.pr.kind === "dev" && p.proc?.cmd && p.proc.cmd !== p.pr.run}>
        <div class="pz-row pz-note">{t("cards.proj.edit.ranAs", "Naposledy spuštěno jako: {cmd}", { cmd: p.proc!.cmd! })}</div>
      </Show>
      <div class="pz-row">
        <span class="pz-cap">{t("cards.proj.edit.where", "Ze složky")}</span>
        <code class="pz-val" classList={{ "pz-bad": !!problem() }}>{whereOf(p.pr, p.proc)}</code>
      </div>
      <div class="pz-row pz-btns">
        <span class="pz-cap" />
        <button class="mini" onClick={() => void changeFolder(p.pr)}>{t("cards.proj.folder.change", "Změnit složku…")}</button>
        <Show when={p.pr.remote}>
          <button class="mini" onClick={() => openProjectPanel(p.pr.id, "choose")}>{t("cards.proj.folder.found", "Najít složky s tímhle repem")}</button>
        </Show>
        <Show when={p.pr.path && placeOf(p.pr).kind !== "choose" && p.pr.remote}>
          <button class="mini" title={t("cards.proj.folder.autoTitle", "Smaže pevnou složku z configu — Ledge ji dohledá podle repa (při víc složkách se zeptá)")} onClick={() => void saveProjectFields(p.pr.id, { path: null })}>{t("cards.proj.folder.auto", "Hledat podle repa")}</button>
        </Show>
      </div>
      <Show when={p.pr.kind === "dev"}>
        <div class="pz-row">
          <span class="pz-cap">{t("cards.proj.edit.port", "Port")}</span>
          <span class="pz-val">{port() ? String(port()) : t("cards.proj.edit.portAuto", "vybere se volný sám")}</span>
        </div>
      </Show>
      <Show when={p.pr.remote}>
        <div class="pz-row">
          <span class="pz-cap">{t("cards.proj.edit.repo", "Repo")}</span>
          <span class="pz-val">{p.pr.remote}</span>
        </div>
      </Show>
      <Show when={problem()}>{(m) => <div class="pz-err" role="alert">{m()}</div>}</Show>
      <Show when={failInfo(p.proc)} fallback={<Show when={p.proc}>{(pc) => <LogView id={pc().id} open={projectPanel()?.focus === "log"} />}</Show>}>
        <FailBox s={p.s} pr={p.pr} proc={p.proc!} inPanel />
      </Show>
      <div class="pz-row pz-btns pz-foot">
        <Show when={live()} fallback={
          <button class="mini ok" disabled={!!problem() && placeOf(p.pr).kind !== "choose"} onClick={() => (placeOf(p.pr).kind === "choose" ? openProjectPanel(p.pr.id, "choose") : void launch(p.pr))}>▶ {t("cards.proj.run", "Spustit")}</button>
        }>
          <button class="mini ok" onClick={() => void stopProject(p.pr.id, p.pr.name)}>■ {t("cards.proj.stop", "Stop")}</button>
          <button class="mini" onClick={() => void restartProject(p.pr.id, p.pr.name)}>{t("cards.proj.restart", "Restart")}</button>
          <Show when={p.proc?.url}>{(u) => <button class="mini" onClick={() => api.openUrl(u())}>{t("cards.proj.open", "Otevřít")}</button>}</Show>
        </Show>
        <span class="grow" />
        <button class="mini" onClick={() => { closeProjectPanel(); p.onSettings(); gotoSettingsPanel("f-projects.add"); }}>{t("cards.proj.edit.settings", "Vše v Nastavení")}</button>
      </div>
    </>
  );
}

/** Log běžícího / skončeného serveru (posledních 60 řádků, načte se až na rozbalení). */
function LogView(p: { id: string; open: boolean }) {
  const [lines, setLines] = createSignal<string[] | null>(null);
  const load = async () => setLines(((await api.logs(p.id)) ?? []).slice(-60));
  onMount(() => p.open && void load());
  return (
    <div class="pz-row pz-btns">
      <span class="pz-cap">{t("cards.proj.edit.log", "Výstup")}</span>
      <button class="mini" aria-expanded={!!lines()} onClick={() => (lines() ? setLines(null) : void load())}>{lines() ? t("cards.proj.fail.hideLog", "Skrýt log") : t("cards.proj.fail.log", "Zobrazit log")}</button>
      <Show when={lines()}>{(l) => <pre class="log pz-log" ref={(el) => queueMicrotask(() => (el.scrollTop = el.scrollHeight))}>{l().join("\n") || t("cards.proj.edit.logEmpty", "Zatím bez výstupu.")}</pre>}</Show>
    </div>
  );
}

function Choose(p: { pr: ProjectView; now: number }) {
  const [data] = createResource(() => p.pr.id, (id) => api.projectFolders(id));
  const [pick, setPick] = createSignal<string | null>(null);
  createEffect(() => {
    const d = data();
    if (d && pick() == null) setPick(preselect(d.items, d.current));
  });
  const fresh = () => freshest(data()?.items ?? []);
  const use = async (start: boolean) => {
    const dir = pick();
    if (!dir) return;
    if (!(await saveProjectFields(p.pr.id, { path: dir }))) return;
    closeProjectPanel();
    if (start) await launch({ ...p.pr, path: dir, place: { kind: "ok" }, pathOk: true });
  };
  return (
    <>
      <p class="pz-lead">
        {placeOf(p.pr).kind === "choose" ? placeProblem(p.pr) : t("cards.proj.choose.lead", "Složky s tímhle repem na disku.")}{" "}
        {t("cards.proj.choose.remember", "Volbu si zapamatuju — příště se spustí rovnou odsud.")}
      </p>
      <Show when={!data.loading} fallback={<div class="pz-note">{t("cards.proj.choose.loading", "Zjišťuji, kde se naposledy commitovalo…")}</div>}>
        <Show when={data()?.items.length} fallback={<div class="pz-note">{t("cards.proj.choose.none", "Žádnou složku s tímhle repem jsem v kořenech projektů nenašel. Vyber ji ručně.")}</div>}>
          <div class="pz-list" role="radiogroup" aria-label={t("cards.proj.choose.aria", "Složky s repem projektu")}>
            <For each={data()!.items}>
              {(c) => (
                <label class="pz-opt" classList={{ "pz-on": pick() === c.path }}>
                  <input type="radio" name={`pz-${p.pr.id}`} checked={pick() === c.path} onChange={() => setPick(c.path)} />
                  <span class="grow">
                    <code>{c.path}</code>
                    <small>
                      {commitText(c.lastCommit, p.now)}
                      <Show when={c.worktree}> · {t("cards.proj.choose.worktree", "pracovní kopie (worktree)")}</Show>
                    </small>
                  </span>
                  <Show when={fresh() === c.path}><span class="pill g">{t("cards.proj.choose.freshest", "nejčerstvější")}</span></Show>
                  <Show when={data()?.current && samePath(data()!.current!, c.path)}><span class="pill b">{t("cards.proj.choose.current", "teď")}</span></Show>
                </label>
              )}
            </For>
          </div>
        </Show>
      </Show>
      <div class="pz-row pz-btns pz-foot">
        <button class="mini ok" disabled={!pick()} onClick={() => void use(true)}>{t("cards.proj.choose.useRun", "Použít a spustit")}</button>
        <button class="mini" disabled={!pick()} onClick={() => void use(false)}>{t("cards.proj.choose.use", "Jen uložit")}</button>
        <button class="mini" onClick={() => void changeFolder(p.pr).then((d) => d && closeProjectPanel())}>{t("cards.proj.folder.other", "Jiná složka…")}</button>
        <span class="grow" />
        <button class="mini" onClick={closeProjectPanel}>{t("cards.proj.choose.cancel", "Zrušit")}</button>
      </div>
    </>
  );
}

/** Selhání startu: krátká zpráva, poslední řádky výstupu, Zobrazit log, Změnit složku…, Zkusit znovu, Zavřít. */
export function FailBox(p: { s: Snapshot; pr: ProjectView; proc: ProcView; inPanel?: boolean }) {
  const info = () => failInfo(p.proc);
  const [log, setLog] = createSignal<string[] | null>(null);
  const toggleLog = async () => {
    if (log()) return setLog(null);
    setLog(((await api.logs(p.proc.id)) ?? []).slice(-60));
  };
  return (
    <Show when={info()}>
      {(f) => (
        <div class="pz-fail" classList={{ "pz-fail-card": !p.inPanel }} role="alert">
          <div class="pz-fail-msg">
            <Show when={!p.inPanel}><b>{p.pr.name}: </b></Show>
            {f().message}
          </div>
          <Show when={p.proc.cwd}><div class="pz-note">{t("cards.proj.fail.where", "Složka: {dir}", { dir: p.proc.cwd! })}</div></Show>
          <Show when={!log() && f().tail.length}><pre class="log">{f().tail.join("\n")}</pre></Show>
          <Show when={log()}>{(l) => <pre class="log" ref={(el) => queueMicrotask(() => (el.scrollTop = el.scrollHeight))}>{l().join("\n")}</pre>}</Show>
          <div class="pz-row pz-btns">
            <button class="mini" aria-expanded={!!log()} onClick={() => void toggleLog()}>{log() ? t("cards.proj.fail.hideLog", "Skrýt log") : t("cards.proj.fail.log", "Zobrazit log")}</button>
            <button class="mini" onClick={() => void changeFolder(p.pr)}>{t("cards.proj.folder.change", "Změnit složku…")}</button>
            <Show when={!p.inPanel}><button class="mini" onClick={() => openProjectPanel(p.pr.id, "edit")}>{t("cards.proj.edit.open", "Upravit…")}</button></Show>
            <button class="mini" onClick={() => void restartProject(p.pr.id, p.pr.name)}>{t("cards.proj.fail.retry", "Zkusit znovu")}</button>
            <span class="grow" />
            <button class="mini" onClick={() => api.dismiss(p.proc.id)}>{t("cards.proj.fail.dismiss", "Zavřít")}</button>
          </div>
        </div>
      )}
    </Show>
  );
}
