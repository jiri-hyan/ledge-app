// Projekty (první část stránky „Projekty, odkazy a akce“, `ProjectsActions.tsx`): hlavička stránky s přidáním
// (ze složky, z naklonovaných rep — návrh z Rust `projectdetect.rs` — a z katalogu Launcheru) a skupina
// Projekty se seznamem dlaždic s logy (řazení přetažením, zapnout/vypnout, upravit, smazat s potvrzením
// a Vrátit). `GitAlerts` (upozornění na neuloženou práci) a `Roots` (kde hledat repa) skládá stránka do
// vlastních skupin. Editor projektu je v `ProjectEditor.tsx`.
import { createEffect, createMemo, createSignal, For, on, Show } from "solid-js";
import { api, type DetectedProject, type RepoCandidate } from "../../api";
import { t } from "../../i18n";
import { logoSrc } from "../../logos";
import { say, sayError } from "../../ui/toast";
import { base, fieldError, offerUndo, useSettings } from "../ctx";
import { Confirm } from "../../ui/Confirm";
import { Toggle } from "../fields/controls";
import { Group } from "../fields/Group";
import { PanelHead, Row, Section } from "../fields/Row";
import { fieldLabel } from "../labels";
import { SortList } from "../fields/SortList";
import { inRoots, moveItem, norm } from "../model";
import { addTarget, draftIds, draftKnown, findDuplicate } from "../projectAdd";
import { cleanAction, labelFrom, slugId } from "../../actions/model";
import type { PanelProps } from "../types";
import { ProjectEditor, ProjectLogo } from "./ProjectEditor";
import { kindLabel } from "../../actions/ActionEditor";
import { commandLines, commandsOf, importLauncherCatalog, importMessage, withVerdicts } from "../launcherImport";

/** Pole, na kterých závisí logo — dokud se nezmění proti uloženému, platí logo z dlaždice. */
const LOGO_FIELDS = ["path", "remote", "subdir", "logo", "exe", "appName", "kind"];

function toProject(d: DetectedProject, roots: string[]) {
  return {
    id: d.id,
    name: d.name,
    enabled: true,
    kind: d.kind,
    // repo přímo v roots se dohledá podle remote i na jiném PC → cestu nezapisovat
    path: d.remote && inRoots(d.path, roots) ? null : d.path,
    remote: d.remote ?? null,
    subdir: d.subdir ?? null,
    run: d.run ?? null,
    port: d.port ?? null,
    passPort: true,
    openBrowser: true,
    exe: d.exe ?? null,
  };
}

function Candidates(p: { known: () => { paths: string[]; remotes: string[] }; onPick: (cs: RepoCandidate[]) => void; onClose: () => void }) {
  const [list, setList] = createSignal<RepoCandidate[] | null>(null);
  const [q, setQ] = createSignal("");
  const [sel, setSel] = createSignal<string[]>([]);
  const [found, setFound] = createSignal<number | null>(null);
  const load = () => {
    const k = p.known();
    return api.projectCandidates(k.paths, k.remotes).then((l) => setList(l ?? []), (e) => (sayError(e), setList([])));
  };
  load();
  /** „Prohledat znovu“: repa naklonovaná mimo Ledge (v terminálu) se do indexu dostanou až takhle */
  async function rescan() {
    setList(null);
    try {
      setFound((await api.reposRescan()) ?? 0);
    } catch (e) {
      sayError(e);
    }
    await load();
  }
  const shown = () => (list() ?? []).filter((c) => !q() || norm(`${c.name} ${c.remote ?? ""}`).includes(norm(q())));
  const toggle = (path: string) => setSel(sel().includes(path) ? sel().filter((x) => x !== path) : [...sel(), path]);
  const chosen = () => (list() ?? []).filter((c) => sel().includes(c.path));
  return (
    <div class="st-cands">
      <div class="st-inline">
        <input type="text" class="st-text grow" placeholder={t("settings.proj.cands.filter", "Filtrovat…")} value={q()} onInput={(e) => setQ(e.currentTarget.value)} aria-label={t("settings.proj.cands.filter", "Filtrovat…")} />
        <button class="mini" onClick={rescan} title={t("settings.proj.rescan.title", "Znovu projde složky z „Kde hledat repa“ — najde i repa naklonovaná v terminálu")}>{t("settings.proj.rescan", "Prohledat znovu")}</button>
        <Show when={found() !== null}><small role="status">{t("settings.proj.rescan.found", "Nalezeno rep: {n}", { n: found()! })}</small></Show>
        <button class="mini ok" disabled={!chosen().length} onClick={() => p.onPick(chosen())}>{t("settings.proj.cands.addSel", "Přidat vybrané ({n})", { n: chosen().length })}</button>
        <button class="mini" onClick={p.onClose}>{t("settings.ui.close", "Zavřít")}</button>
      </div>
      <Show when={list()} fallback={<For each={[1, 2, 3]}>{() => <div class="st-skel" />}</For>}>
        <Show when={shown().length} fallback={<p class="st-empty">{list()!.length ? t("settings.proj.cands.none", "Nic neodpovídá filtru.") : t("settings.proj.cands.empty", "V „Kde hledat repa“ není žádné další git repo. Přidej složku, nebo projekt vyber ze složky.")}</p>}>
          <div class="st-cand-list">
            <For each={shown()}>
              {(c) => (
                <button class="st-cand" classList={{ on: sel().includes(c.path) }} aria-pressed={sel().includes(c.path)} onClick={() => toggle(c.path)}>
                  <strong>{c.name}</strong>
                  <small class="mono">{c.remote ?? c.path}</small>
                </button>
              )}
            </For>
          </div>
        </Show>
      </Show>
    </div>
  );
}

export function Roots() {
  const c = useSettings();
  const roots = (): string[] => c.cfg.roots ?? [];
  const [found, setFound] = createSignal<number | null>(null);
  /** hledá v ULOŽENÝCH složkách (nové přidané ulož před hledáním) */
  async function rescan() {
    try {
      setFound((await api.reposRescan()) ?? 0);
    } catch (e) {
      sayError(e);
    }
  }
  async function add() {
    try {
      const got = await api.pickPath("folder", t("settings.roots.pick", "Složka s repy"));
      if (!got) return;
      if (roots().some((r) => r.toLowerCase() === got.toLowerCase())) return say(t("settings.roots.dup", "Tahle složka už v seznamu je."));
      c.set("roots", [...roots(), got]);
    } catch (e) {
      sayError(e);
    }
  }
  return (
    <Row id="roots" stack>
      <Show when={roots().length} fallback={<p class="st-empty">{t("settings.roots.empty", "Žádná složka — projekty pak musí mít vyplněnou složku a „Naklonovat“ nemá kam.")}</p>}>
        <SortList items={roots()} label={t("settings.f.roots", "Kde hledat repa")} onMove={(a, b) => c.set("roots", moveItem(roots(), a, b))}>
          {(r, i, handle) => (
            <div class="st-item">
              {handle()}
              <span class="grow mono">{r}</span>
              <Show when={i() === 0}><span class="st-tag">{t("settings.roots.cloneHere", "sem se klonuje")}</span></Show>
              <button class="mini" onClick={() => c.set("roots", roots().filter((_, j) => j !== i()))}>{t("settings.ui.remove", "Odebrat")}</button>
            </div>
          )}
        </SortList>
      </Show>
      <div class="st-actions">
        <button class="mini" onClick={add}>+ {t("settings.roots.add", "Přidat složku…")}</button>
        <button class="mini" onClick={rescan} title={t("settings.proj.rescan.title", "Znovu projde složky z „Kde hledat repa“ — najde i repa naklonovaná v terminálu")}>{t("settings.proj.rescan", "Prohledat znovu")}</button>
        <Show when={found() !== null}><small role="status">{t("settings.proj.rescan.found", "Nalezeno rep: {n}", { n: found()! })}</small></Show>
      </div>
    </Row>
  );
}

export default function ProjectsPanel(p: PanelProps) {
  const c = useSettings();
  const list = (): any[] => c.cfg.projects ?? [];
  const [open, setOpen] = createSignal<number | null>(null);
  const [ask, setAsk] = createSignal<number | null>(null);
  const [cands, setCands] = createSignal(false);
  const [busy, setBusy] = createSignal(false);
  /** co zjistil návrh u právě přidaného projektu (podle id) */
  const [notes, setNotes] = createSignal<Record<string, string[]>>({});

  // chyba validace v projektu → rozbalit jeho editor
  createEffect(on(fieldError, (e) => {
    const m = e?.field?.match(/^projects\.(\d+)\./);
    if (m) setOpen(Number(m[1]));
  }));

  const saved = createMemo(() => {
    try {
      return (JSON.parse(base()).projects ?? []) as any[];
    } catch {
      return [];
    }
  });
  /** logo z dlaždice, když se pole loga od uložení nezměnila */
  const savedLogo = (pr: any) => {
    const old = saved().find((x) => x.id === pr.id);
    if (!old || LOGO_FIELDS.some((f) => (old[f] ?? null) !== (pr[f] ?? null))) return undefined;
    const view = p.s.projects.find((v) => v.id === pr.id);
    return view ? logoSrc(view) : undefined;
  };

  /** Přidá projekt ze složky; `quiet` = bez rozbalení a hlášky o úspěchu (hromadné přidání). Vrací, zda přibyl. */
  async function addFrom(path: string, quiet = false): Promise<boolean> {
    const dup = findDuplicate(list(), { path });
    if (dup >= 0) {
      if (!quiet) setOpen(dup);
      say(t("settings.proj.dup", "„{n}“ už v projektech je.", { n: list()[dup].name }));
      return false;
    }
    setBusy(true);
    try {
      // id z rozpracovaného návrhu, ne jen z uloženého configu (dva projekty se stejným názvem složky)
      const d = await api.projectDetect(path, draftIds(list()));
      if (!d) return false;
      // stejné repo v jiné složce (jiný klon) = stejný projekt
      const same = findDuplicate(list(), { remote: d.remote });
      if (same >= 0) {
        if (!quiet) setOpen(same);
        say(t("settings.proj.dup", "„{n}“ už v projektech je.", { n: list()[same].name }));
        return false;
      }
      const pr = toProject(d, c.cfg.roots ?? []);
      c.update("projects", (all: any[]) => [...all, pr]);
      setNotes({ ...notes(), [pr.id]: d.notes });
      if (!quiet) {
        setOpen(list().length - 1);
        setCands(false);
        say(t("settings.proj.added", "Projekt „{n}“ přidán — zkontroluj údaje a ulož.", { n: pr.name }));
      }
      return true;
    } catch (e) {
      sayError(e);
      return false;
    } finally {
      setBusy(false);
    }
  }
  /** Víc rep naráz z nabídky; jedna souhrnná hláška. */
  async function addMany(picked: RepoCandidate[]) {
    let n = 0;
    for (const x of picked) if (await addFrom(x.path, true)) n++;
    if (n) {
      setCands(false);
      setOpen(null);
      say(t("settings.proj.addedMany", "Přidáno projektů: {n} — zkontroluj údaje a ulož.", { n }));
    }
  }

  // jedno „+ Přidat“: vložená adresa / cesta → typ podle `guessKind` (složka = projekt, jinak odkaz nebo akce)
  const [adding, setAdding] = createSignal(false);
  const [addText, setAddText] = createSignal("");
  async function addPicked(kind: "folder" | "exe") {
    try {
      const got = await api.pickPath(kind, t("settings.proj.pick", "Složka projektu"), c.cfg.roots?.[0]);
      if (got) setAddText(got);
    } catch (e) {
      sayError(e);
    }
  }
  async function addSmart() {
    const text = addText().trim();
    const what = addTarget(text);
    if (!what) return;
    if (what.as === "project") {
      if (await addFrom(text)) {
        setAddText("");
        setAdding(false);
      }
      return;
    }
    const acts: any[] = c.cfg.actions ?? [];
    const label = labelFrom(text);
    const a = cleanAction({ id: slugId(label, acts.map((x) => x.id)), label, kind: what.kind, target: text, showOn: "both" });
    c.set("actions", [...acts, a]);
    setAddText("");
    setAdding(false);
    say(t("actions.panel.added", "Akce „{label}“ přidána — ulož ji dole v liště.", { label: a.label }));
  }
  /** Launcher → Ledge (zadání týden 43 bod 4): zástupci z `catalog.json`. Příkazy projdou pojistkou v jádře
   *  (`launcher_import_check`); co neprojde, je vypnuté. Před převzetím potvrzení se seznamem příkazů; pak jde vše
   *  do návrhu (Uložit / Vrátit). Ledge z katalogu nic nespouští. */
  const [pendingImport, setPendingImport] = createSignal<{ cfg: any; lines: ReturnType<typeof commandLines>; added: number; duplicates: number; noCommand: number } | null>(null);
  async function importCatalog() {
    setBusy(true);
    try {
      const got = await api.pickPath("json", t("settings.proj.importPick", "Vyber catalog.json z Launcheru"));
      if (!got) return;
      const raw = await api.launcherCatalog(got);
      const res = importLauncherCatalog(raw, c.cfg);
      if (!res.added.length) return say(t("settings.proj.importNone", "Nic nového — všechno z katalogu už v projektech je (nebo nemá příkaz)."));
      const verdicts = (await api.launcherImportCheck(commandsOf(res.nextCfg, res.addedIds))) ?? [];
      const next = withVerdicts(res.nextCfg, res.addedIds, verdicts);
      setPendingImport({ cfg: next, lines: commandLines(next, res.addedIds, verdicts), added: res.added.length, duplicates: res.duplicates.length, noCommand: res.noCommand.length });
    } catch (e) {
      sayError(e);
    } finally {
      setBusy(false);
    }
  }
  function applyImport() {
    const imp = pendingImport();
    setPendingImport(null);
    if (!imp) return;
    const before = { projects: structuredClone(c.cfg.projects ?? []), layout: structuredClone(c.cfg.layout) };
    c.set("projects", imp.cfg.projects);
    if (imp.cfg.layout) c.set("layout", imp.cfg.layout);
    offerUndo(importMessage(imp.added, imp.duplicates, imp.noCommand), () => {
      c.set("projects", before.projects);
      if (before.layout !== undefined) c.set("layout", before.layout);
    });
  }
  function remove(i: number) {
    setAsk(null);
    setOpen(null);
    const before = structuredClone(list());
    c.update("projects", (all: any[]) => all.filter((_, j) => j !== i));
    offerUndo(t("settings.proj.removed", "Projekt „{n}“ odebrán", { n: before[i].name }), () => c.set("projects", before));
  }
  const summary = (x: any) => {
    const what = x.kind === "app" ? (x.exe || x.appName || x.aumid || "—") : (x.run || "—");
    const where = x.path ? x.path : x.remote ? t("settings.proj.byRemote", "podle {r}", { r: x.remote }) : t("settings.proj.noDir", "bez složky");
    return `${what} · ${x.port ? `:${x.port} · ` : ""}${where}`;
  };

  return (
    <>
      <PanelHead id="projects.add" title={t("settings.nav.akce", "Projekty, odkazy a akce")} desc={t("settings.proj.desc", "Všechno, co se otevírá jedním klikem: projekty (dev server nebo aplikace), rychlé akce a odkazy na weby, složky a soubory.")}>
        <button class="mini" disabled={busy()} onClick={importCatalog} title={t("settings.proj.importTitle", "Zástupce a složky z katalogu Launcheru (catalog.json)")}>{t("settings.proj.importCatalog", "Z Launcheru…")}</button>
        <button class="mini" disabled={busy()} onClick={() => setCands(!cands())} aria-expanded={cands()}>{t("settings.proj.fromRepos", "Z naklonovaných rep")}</button>
        <button class="mini ok" disabled={busy()} onClick={() => setAdding(!adding())} aria-expanded={adding()}>+ {t("settings.proj.add", "Přidat")}</button>
      </PanelHead>
      <Confirm
        when={!!pendingImport()}
        text={t("settings.proj.importAsk", "Převzít {n} zástupců z Launcheru? Klik na zástupce pak spustí tyto příkazy:", { n: pendingImport()?.added ?? 0 })}
        detail={
          <ul class="st-import-cmds">
            <For each={pendingImport()?.lines ?? []}>
              {(l) => (
                <li>
                  <b>{l.name}</b>: <code>{l.command}</code>
                  <Show when={l.problem}>{(why) => <em> — {t("settings.proj.importOff", "importuji vypnutý: {why}", { why: why() })}</em>}</Show>
                </li>
              )}
            </For>
          </ul>
        }
        yes={t("settings.proj.importYes", "Převzít")}
        onYes={applyImport}
        onNo={() => setPendingImport(null)}
      />
      <Show when={adding()}>
        <div class="st-inline st-addrow">
          <input type="text" class="st-text grow mono" value={addText()} placeholder={t("settings.proj.add.ph", "Cesta ke složce, souboru nebo programu, nebo adresa webu")} aria-label={t("settings.proj.add.ph", "Cesta ke složce, souboru nebo programu, nebo adresa webu")} onInput={(e) => setAddText(e.currentTarget.value)} onKeyDown={(e) => e.key === "Enter" && addSmart()} />
          <button class="mini" onClick={() => addPicked("folder")}>{t("settings.proj.add.folder", "Vybrat složku…")}</button>
          <button class="mini" onClick={() => addPicked("exe")}>{t("settings.proj.add.exe", "Vybrat program…")}</button>
          <button class="mini ok" disabled={busy() || !addText().trim()} onClick={addSmart}>{t("actions.edit.add", "Přidat")}</button>
        </div>
        <Show when={addTarget(addText())}>{(w) => <small class="st-note" role="status">{w().as === "project" ? t("settings.proj.add.asProject", "Přidá se jako projekt (složka).") : t("settings.proj.add.asAction", "Přidá se jako odkaz nebo akce: {k}.", { k: kindLabel(w().kind) })}</small>}</Show>
      </Show>
      <Show when={cands()}>
        <Section>
          <p class="st-note">{t("settings.proj.cands", "Naklonovaná repa, která ještě nejsou v projektech — vyber jedno nebo víc.")}</p>
          <Candidates known={() => draftKnown(list())} onPick={addMany} onClose={() => setCands(false)} />
        </Section>
      </Show>
      <Group gid="akce.projects" title={t("settings.proj.title", "Projekty")} desc={t("settings.proj.gDesc", "Dev servery a aplikace, které spustíš jedním klikem z hlavní stránky šuplíku.")} badge={list().length ? String(list().length) : undefined}>
        <Row id="projects" stack>
          <Show when={list().length} fallback={<p class="st-empty">{t("settings.proj.empty", "Zatím žádné projekty. Přidej složku s repem — Ledge sám pozná, jestli je to web nebo aplikace, a navrhne příkaz i port.")}</p>}>
            <SortList items={list()} label={fieldLabel("projects")} onMove={(a, b) => { setOpen(null); c.set("projects", moveItem(list(), a, b)); }}>
              {(pr, i, handle) => (
                <div class="st-item-wrap" classList={{ open: open() === i() }}>
                  <div class="st-item st-projrow" classList={{ dim: !pr.enabled }}>
                    {handle()}
                    <ProjectLogo project={pr} savedLogo={savedLogo(pr)} />
                    <span class="grow">
                      <strong>{pr.name || t("settings.proj.unnamed", "Bez názvu")}</strong>
                      <span class={`st-tag ${pr.kind}`}>{pr.kind === "app" ? t("settings.proj.kind.appShort", "aplikace") : t("settings.proj.kind.devShort", "web")}</span>
                      <small class="mono">{summary(pr)}</small>
                    </span>
                    <Toggle path={`projects.${i()}.enabled`} label={t("settings.proj.enabled", "Zobrazit v šuplíku")} />
                    <button class="mini" aria-expanded={open() === i()} onClick={() => setOpen(open() === i() ? null : i())}>{open() === i() ? t("settings.ui.done", "Hotovo") : t("settings.ui.edit", "Upravit")}</button>
                    <button class="mini" onClick={() => setAsk(i())}>{t("settings.ui.delete", "Smazat")}</button>
                  </div>
                  <Confirm
                    when={ask() === i()}
                    danger
                    text={t("settings.proj.ask", "Smazat projekt „{n}“ z šuplíku?", { n: pr.name })}
                    detail={t("settings.proj.ask.detail", "Složka na disku zůstane. Smazání jde vrátit, dokud neuložíš.")}
                    yes={t("settings.ui.delete", "Smazat")}
                    onYes={() => remove(i())}
                    onNo={() => setAsk(null)}
                  />
                  <Show when={open() === i()}>
                    <ProjectEditor i={i()} notes={notes()[pr.id]} />
                  </Show>
                </div>
              )}
            </SortList>
          </Show>
        </Row>
      </Group>
    </>
  );
}

/** Upozornění na neuloženou / nepushnutou práci (`gitAlerts.enabled`, výchozí ano) a ztišená repa s Odztišit
 *  (`projects[].muteDirty`). Mění draft — uloží společná lišta. */
export function GitAlerts() {
  const c = useSettings();
  const muted = () => (Array.isArray(c.cfg.projects) ? c.cfg.projects : []).map((p: any, i: number) => ({ p, i })).filter((x: any) => x.p?.muteDirty);
  return (
    <>
      <Row id="gitAlerts.enabled"><Toggle path="gitAlerts.enabled" label={fieldLabel("gitAlerts.enabled")} /></Row>
      <Show when={muted().length}>
        <Row id="gitAlerts.muted" stack label={t("settings.proj.mutedList", "Ztišená repa")} hint={t("settings.proj.mutedHint", "U těchhle se neuložená práce nepočítá do hlavičky ani proužku.")}>
          <div class="st-list">
            <For each={muted()}>
              {(x) => (
                <div class="st-item">
                  <span class="grow">{x.p.name ?? x.p.id}</span>
                  <button class="mini" onClick={() => c.set(`projects.${x.i}.muteDirty`, false)}>{t("settings.proj.unmute", "Odztišit")}</button>
                </div>
              )}
            </For>
          </div>
        </Row>
      </Show>
    </>
  );
}
