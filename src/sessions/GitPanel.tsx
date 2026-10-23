// Panel Git pod řádkem session v kartě Claude (rozbalí ho „Git“ v `SessionRow`): větev, upstream
// ahead/behind, worktree a hlavní checkout, změněné soubory (klik = diff přes `DiffPreview`), Commit…,
// Push… (potvrzení), Pull (jen fast-forward), Zahodit… a Odstranit worktree… (druhé potvrzení s textem
// z Rustu, co přesně se ztratí). Stav se čte jen při rozbalení, po akci, po ↻ a při změně stavu session —
// žádné pollování. Pravidla a blokace počítá Rust (`sessiongit/`), tady jen vykreslení; logika v `git.ts`.
import { createEffect, createSignal, For, on, onCleanup, Show } from "solid-js";
import { api, type EditPreview, type GitFileView, type GitView, type SessionView } from "../api";
import { t, tp } from "../i18n";
import { say } from "../ui/toast";
import { Confirm } from "../ui/Confirm";
import DiffPreview from "./DiffPreview";
import { badge, canPush, commitReady, defaultPicked, diffModes, hiddenFiles, messageKey, togglePicked } from "./git";
import "./git.css";

type Ask = { kind: "push" } | { kind: "discard"; path: string; text: string } | { kind: "remove"; text: string };
type Diff = { path: string; staged: boolean; pv: EditPreview | null };
const confirmText = (c: Ask, v: GitView) => (c.kind === "push" ? v.pushText : c.text);

export default function GitPanel(p: { id: string; status: SessionView["status"] }) {
  /** undefined = načítá se, null = cwd není v gitu */
  const [view, setView] = createSignal<GitView | null | undefined>(undefined);
  const [loadErr, setLoadErr] = createSignal<string | null>(null);
  /** chyba poslední akce celá (výstup hooku má víc řádků) */
  const [err, setErr] = createSignal<string | null>(null);
  const [busy, setBusy] = createSignal(false);
  const [diff, setDiff] = createSignal<Diff | null>(null);
  /** null = formulář commitu zavřený */
  const [msg, setMsg] = createSignal<string | null>(null);
  const [picked, setPicked] = createSignal<string[]>([]);
  const [confirm, setConfirm] = createSignal<Ask | null>(null);
  const [took, setTook] = createSignal(false);
  let box: HTMLTextAreaElement | undefined;

  async function load() {
    try {
      const v = await api.sessionGit(p.id);
      setView(v ?? null);
      setLoadErr(null);
      if (v) setPicked(picked().filter((x) => v.files.some((f) => f.path === x)));
    } catch (e) {
      setLoadErr(String(e));
    }
  }
  void load();
  // session začala / přestala pracovat → blokace destruktivních akcí se mohla změnit (událost, ne časovač)
  createEffect(on(() => p.status, () => void load(), { defer: true }));

  /** vrátit fokus, jen když ho vzal klik na „Commit…“ (a jen jednou) */
  const release = () => {
    if (!took()) return;
    setTook(false);
    api.releaseFocus();
  };
  onCleanup(release);

  async function run(f: () => Promise<unknown>, done: string, after?: () => void) {
    if (busy()) return;
    setBusy(true);
    setErr(null);
    try {
      const out = await f();
      say(typeof out === "string" && out ? out : done);
      after?.();
    } catch (e) {
      setErr(String(e));
      say(String(e).split("\n")[0], true);
    } finally {
      setBusy(false);
      await load();
    }
  }

  async function openDiff(f: GitFileView, mode?: boolean) {
    if (mode === undefined && diff()?.path === f.path) return setDiff(null);
    const staged = mode ?? diffModes(f)[0] ?? false;
    setDiff({ path: f.path, staged, pv: null });
    try {
      const pv = await api.sessionGitDiff(p.id, f.path, staged);
      const cur = diff();
      if (cur?.path === f.path && cur.staged === staged) setDiff({ path: f.path, staged, pv: pv ?? null });
    } catch (e) {
      setDiff(null);
      say(String(e), true);
    }
  }

  /** Výslovný klik → šuplík dostane klávesnici a fokus jde do pole zprávy (jinak Ledge fokus nebere). */
  function openCommit(v: GitView) {
    setPicked(defaultPicked(v.files));
    setMsg("");
    setTook(true);
    api.focusDrawer();
    queueMicrotask(() => box?.focus());
  }
  function closeCommit() {
    setMsg(null);
    release();
  }
  const sendCommit = () => {
    const m = msg() ?? "";
    if (!commitReady(m, picked())) return;
    void run(() => api.sessionGitCommit(p.id, m, picked()), t("sessions.git.committed", "Zacommitováno"), closeCommit);
  };
  function key(e: KeyboardEvent) {
    const k = messageKey(e.key, e.ctrlKey || e.metaKey);
    if (k) e.preventDefault();
    if (e.key === "Escape") e.stopPropagation(); // Esc jinak zavře šuplík
    if (k === "cancel") closeCommit();
    else if (k === "send") sendCommit();
  }

  function doConfirm(c: Ask) {
    setConfirm(null);
    if (c.kind === "push") void run(() => api.sessionGitPush(p.id), t("sessions.git.pushed", "Pushnuto"));
    else if (c.kind === "discard") void run(() => api.sessionGitDiscard(p.id, c.path), t("sessions.git.discarded", "Změny zahozené"), () => diff()?.path === c.path && setDiff(null));
    else void run(() => api.sessionGitRemoveWorktree(p.id), t("sessions.git.removed", "Worktree odstraněn"));
  }
  const open = (target: "terminal" | "folder") => api.sessionGitOpen(p.id, target).catch((e) => say(String(e), true));

  return (
    <div class="sess-git">
      <Show when={view() !== undefined || loadErr()} fallback={<small class="sess-git-note">{t("sessions.git.loading", "Načítám git…")}</small>}>
        <Show when={!loadErr()} fallback={
          <div class="sess-git-line"><small class="sess-git-err">{loadErr()}</small><button class="sess-git-link" onClick={load}>↻</button></div>
        }>
          <Show when={view()} fallback={<small class="sess-git-note">{t("sessions.git.noRepo", "Pracovní složka session není v gitu.")}</small>}>
            {(v) => (
              <>
                <div class="sess-git-line">
                  <span class="sess-git-branch">{v().branch ?? t("sessions.git.detached", "odpojená HEAD")}</span>
                  <Show when={v().upstream} fallback={<Show when={v().branch}><span class="sess-git-meta">{t("sessions.git.noUpstream", "bez upstreamu")}</span></Show>}>
                    {(u) => <span class="sess-git-meta" title={t("sessions.git.abHint", "Commity navíc / chybějící proti upstreamu (podle posledního fetch)")}>→ {u()} · ↑{v().ahead} ↓{v().behind}</span>}
                  </Show>
                  <button class="sess-git-link" disabled={busy()} onClick={load} title={t("sessions.git.refresh", "Obnovit")}>↻</button>
                </div>
                <Show when={v().worktree}>
                  {(w) => (
                    <div class="sess-git-meta" title={w().path}>
                      {t("sessions.git.worktree", "worktree · hlavní checkout {main} ({branch})", { main: w().main, branch: w().mainBranch ?? "—" })}
                      <Show when={w().locked}> · {t("sessions.git.locked", "zamčený")}</Show>
                    </div>
                  )}
                </Show>
                <Show when={v().block}>{(b) => <small class="sess-git-note">{b()}</small>}</Show>

                <Show when={v().files.length > 0} fallback={<small class="sess-git-note">{t("sessions.git.clean", "Žádné změny.")}</small>}>
                  <div class="sess-git-files">
                    <For each={v().files}>
                      {(f) => (
                        <>
                          <div class="sess-git-file">
                            <Show when={msg() !== null}>
                              <input type="checkbox" class="sess-git-pick" checked={picked().includes(f.path)} disabled={f.kind === "conflict" || busy()}
                                aria-label={t("sessions.git.pick", "Zahrnout {path} do commitu", { path: f.path })}
                                onChange={() => setPicked(togglePicked(picked(), f.path, v().files))} />
                            </Show>
                            <span class="sess-git-badge" classList={{ "sess-git-add": ["?", "A"].includes(badge(f)), "sess-git-del": badge(f) === "D", "sess-git-conf": f.kind === "conflict" }}>{badge(f)}</span>
                            <button class="sess-git-path" aria-expanded={diff()?.path === f.path} onClick={() => openDiff(f)} title={f.orig ? `${f.orig} → ${f.path}` : f.path}>{f.path}</button>
                            <Show when={f.added != null && f.removed != null}><span class="sess-git-n">+{f.added} −{f.removed}</span></Show>
                            <Show when={f.discard}>
                              {(text) => (
                                <button class="sess-git-link" disabled={busy() || !!v().block} title={v().block ?? t("sessions.git.discardHint", "Vrátit soubor do stavu posledního commitu")}
                                  onClick={() => setConfirm({ kind: "discard", path: f.path, text: text() })}>{t("sessions.git.discard", "Zahodit…")}</button>
                              )}
                            </Show>
                          </div>
                          <Show when={diff()?.path === f.path ? diff() : null}>
                            {(d) => (
                              <div class="sess-git-diff">
                                <Show when={diffModes(f).length > 1}>
                                  <div class="sess-git-line">
                                    <button class="sess-git-link" classList={{ on: !d().staged }} aria-pressed={!d().staged} onClick={() => openDiff(f, false)}>{t("sessions.git.unstaged", "nepřipravené")}</button>
                                    <button class="sess-git-link" classList={{ on: d().staged }} aria-pressed={d().staged} onClick={() => openDiff(f, true)}>{t("sessions.git.staged", "připravené (staged)")}</button>
                                  </div>
                                </Show>
                                <Show when={d().pv} fallback={<small class="sess-git-note">{t("sessions.git.diffLoading", "Načítám diff…")}</small>}>
                                  {(pv) => <DiffPreview pv={pv()} open />}
                                </Show>
                              </div>
                            )}
                          </Show>
                        </>
                      )}
                    </For>
                    <Show when={hiddenFiles(v()) > 0}>
                      <small class="sess-git-note">{tp("sessions.git.more", hiddenFiles(v()), ["… a {n} další soubor (v panelu se neukáže)", "… a {n} další soubory (v panelu se neukážou)", "… a {n} dalších souborů (v panelu se neukážou)"])}</small>
                    </Show>
                  </div>
                </Show>

                <Show when={msg() !== null}>
                  <textarea ref={box} class="sess-git-msg" rows={3} value={msg() ?? ""} disabled={busy()}
                    placeholder={t("sessions.git.msgPlaceholder", "Zpráva commitu… (Ctrl+Enter zacommituje, Esc zruší)")}
                    onInput={(e) => setMsg(e.currentTarget.value)} onKeyDown={key} />
                  <div class="sess-git-line">
                    <button class="mini" disabled={busy()} onClick={closeCommit}>{t("sessions.git.cancel", "Zrušit")}</button>
                    <button class="mini ok" disabled={busy() || !commitReady(msg() ?? "", picked())} onClick={sendCommit}>
                      {tp("sessions.git.commitN", picked().length, ["Commit ({n} soubor)", "Commit ({n} soubory)", "Commit ({n} souborů)"])}
                    </button>
                    <small class="sess-git-meta">{t("sessions.git.hooksRun", "hooky gitu proběhnou")}</small>
                  </div>
                </Show>

                <Show when={confirm()}>
                  {(c) => (
                    <Confirm
                      when
                      text={confirmText(c(), v()) ?? ""}
                      yes={c().kind === "push" ? t("sessions.git.pushDo", "Pushnout") : c().kind === "discard" ? t("sessions.git.discardDo", "Zahodit změny") : t("sessions.git.removeDo", "Odstranit worktree")}
                      danger={c().kind !== "push"}
                      busy={busy()}
                      onYes={() => doConfirm(c())}
                      onNo={() => setConfirm(null)}
                    />
                  )}
                </Show>

                <Show when={err()}>{(e) => <pre class="sess-git-err">{e()}</pre>}</Show>

                <div class="sess-git-line">
                  <button class="mini" disabled={busy() || v().files.length === 0 || msg() !== null} onClick={() => openCommit(v())}>{t("sessions.git.commit", "Commit…")}</button>
                  <button class="mini" disabled={busy() || !canPush(v())} title={(() => { const pp = v().push; return pp.kind === "no" ? pp.why : undefined; })()}
                    onClick={() => setConfirm({ kind: "push" })}>{t("sessions.git.push", "Push…")}</button>
                  <button class="mini" disabled={busy() || !v().canPull} title={t("sessions.git.pullHint", "git pull --ff-only — jen rychlé posunutí")}
                    onClick={() => run(() => api.sessionGitPull(p.id), t("sessions.git.pulled", "Staženo"))}>{t("sessions.git.pull", "Pull")}</button>
                  <button class="mini" onClick={() => open("terminal")}>{t("sessions.git.terminal", "Terminál")}</button>
                  <button class="mini" onClick={() => open("folder")}>{t("sessions.git.folder", "Složka")}</button>
                  <Show when={v().worktree?.remove}>
                    {(text) => (
                      <button class="mini" disabled={busy() || !!v().block} title={v().block ?? undefined}
                        onClick={() => setConfirm({ kind: "remove", text: text() })}>{t("sessions.git.remove", "Odstranit worktree…")}</button>
                    )}
                  </Show>
                </div>
              </>
            )}
          </Show>
        </Show>
      </Show>
    </div>
  );
}
