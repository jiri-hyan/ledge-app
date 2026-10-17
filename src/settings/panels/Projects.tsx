// Panel „Projekty“: seznam dlaždic s logy (řazení přetažením, zapnout/vypnout, upravit, smazat
// s potvrzením a Vrátit), přidání ze složky nebo z naklonovaných rep (návrh z Rust `projectdetect.rs`)
// a `roots` — složky, kde hledat repa. Editor projektu je v `ProjectEditor.tsx`.
import { createEffect, createMemo, createSignal, For, on, Show } from "solid-js";
import { api, type DetectedProject, type RepoCandidate } from "../../api";
import { t } from "../../i18n";
import { logoSrc } from "../../logos";
import { say, sayError } from "../../ui/toast";
import { base, fieldError, offerUndo, useSettings } from "../ctx";
import { Confirm } from "../fields/Confirm";
import { Toggle } from "../fields/controls";
import { PanelHead, Row, Section } from "../fields/Row";
import { SortList } from "../fields/SortList";
import { inRoots, moveItem, norm } from "../model";
import type { PanelProps } from "../types";
import { ProjectEditor, ProjectLogo } from "./ProjectEditor";

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

function Candidates(p: { onPick: (c: RepoCandidate) => void; onClose: () => void }) {
  const [list, setList] = createSignal<RepoCandidate[] | null>(null);
  const [q, setQ] = createSignal("");
  api.projectCandidates().then((l) => setList(l ?? []), (e) => (sayError(e), setList([])));
  const shown = () => (list() ?? []).filter((c) => !q() || norm(`${c.name} ${c.remote ?? ""}`).includes(norm(q())));
  return (
    <div class="st-cands">
      <div class="st-inline">
        <input type="text" class="st-text grow" placeholder={t("settings.proj.cands.filter", "Filtrovat…")} value={q()} onInput={(e) => setQ(e.currentTarget.value)} aria-label={t("settings.proj.cands.filter", "Filtrovat…")} />
        <button class="mini" onClick={p.onClose}>{t("settings.ui.close", "Zavřít")}</button>
      </div>
      <Show when={list()} fallback={<For each={[1, 2, 3]}>{() => <div class="st-skel" />}</For>}>
        <Show when={shown().length} fallback={<p class="st-empty">{list()!.length ? t("settings.proj.cands.none", "Nic neodpovídá filtru.") : t("settings.proj.cands.empty", "V „Kde hledat repa“ není žádné další git repo. Přidej složku, nebo projekt vyber ze složky.")}</p>}>
          <div class="st-cand-list">
            <For each={shown()}>
              {(c) => (
                <button class="st-cand" onClick={() => p.onPick(c)}>
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

function Roots() {
  const c = useSettings();
  const roots = (): string[] => c.cfg.roots ?? [];
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
      <div class="st-actions"><button class="mini" onClick={add}>+ {t("settings.roots.add", "Přidat složku…")}</button></div>
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

  async function addFrom(path: string) {
    const dup = list().findIndex((x) => x.path && x.path.toLowerCase() === path.toLowerCase());
    if (dup >= 0) {
      setOpen(dup);
      return say(t("settings.proj.dup", "„{n}“ už v projektech je.", { n: list()[dup].name }));
    }
    setBusy(true);
    try {
      const d = await api.projectDetect(path);
      if (!d) return;
      const pr = toProject(d, c.cfg.roots ?? []);
      c.update("projects", (all: any[]) => [...all, pr]);
      setNotes({ ...notes(), [pr.id]: d.notes });
      setOpen(list().length - 1);
      setCands(false);
      say(t("settings.proj.added", "Projekt „{n}“ přidán — zkontroluj údaje a ulož.", { n: pr.name }));
    } catch (e) {
      sayError(e);
    } finally {
      setBusy(false);
    }
  }
  async function fromFolder() {
    try {
      const got = await api.pickPath("folder", t("settings.proj.pick", "Složka projektu"), c.cfg.roots?.[0]);
      if (got) await addFrom(got);
    } catch (e) {
      sayError(e);
    }
  }
  function remove(i: number) {
    setAsk(null);
    setOpen(null);
    const before = structuredClone(list());
    c.update("projects", (all: any[]) => all.filter((_, j) => j !== i));
    offerUndo(t("settings.proj.removed", "Projekt „{n}“ odebrán", { n: before[i].name }), () => c.set("projects", before));
  }
  const summary = (x: any) => {
    const what = x.kind === "app" ? (x.exe || x.appName || "—") : (x.run || "—");
    const where = x.path ? x.path : x.remote ? t("settings.proj.byRemote", "podle {r}", { r: x.remote }) : t("settings.proj.noDir", "bez složky");
    return `${what} · ${x.port ? `:${x.port} · ` : ""}${where}`;
  };

  return (
    <>
      <PanelHead id="projects.add" title={t("settings.nav.projekty", "Projekty")} desc={t("settings.proj.desc", "Dlaždice na hlavní stránce šuplíku: spuštění dev serveru nebo aplikace jedním klikem.")}>
        <button class="mini" disabled={busy()} onClick={() => setCands(!cands())} aria-expanded={cands()}>{t("settings.proj.fromRepos", "Z naklonovaných rep")}</button>
        <button class="mini ok" disabled={busy()} onClick={fromFolder}>+ {t("settings.proj.fromFolder", "Přidat ze složky…")}</button>
      </PanelHead>
      <Show when={cands()}>
        <Section title={t("settings.proj.cands", "Naklonovaná repa, která ještě nejsou v projektech")}>
          <Candidates onPick={(x) => addFrom(x.path)} onClose={() => setCands(false)} />
        </Section>
      </Show>
      <Section>
        <Row id="projects" stack>
          <Show when={list().length} fallback={<p class="st-empty">{t("settings.proj.empty", "Zatím žádné projekty. Přidej složku s repem — Ledge sám pozná, jestli je to web nebo aplikace, a navrhne příkaz i port.")}</p>}>
            <SortList items={list()} label={t("settings.f.projects", "Projekty v šuplíku")} onMove={(a, b) => { setOpen(null); c.set("projects", moveItem(list(), a, b)); }}>
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
      </Section>
      <Section>
        <Roots />
      </Section>
    </>
  );
}
