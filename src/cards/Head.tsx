// Hlavička šuplíku: hodiny (nebo Zpět + název stránky), stavové chipy, nová verze, záložky (`layout/Tabs.tsx`),
// ✎ režim úprav šuplíku (`layout/edit.ts`) a nástroje (paleta 🔍, poznámka, nastavení, zavřít). Čte jen snapshot; navigaci dělá skořápka (App) přes callbacky.
import { Show } from "solid-js";
import { api, type Snapshot } from "../api";
import { IconBack, IconNote, IconSettings, IconX } from "../icons";
import { editPath } from "../notes/store";
import { fmtDate, fmtTime } from "../format";
import type { Page } from "../nav";
import { IconSearch } from "../palette/Palette";
import { t, tp } from "../i18n";
import { rich } from "../ui/rich";
import { bubble } from "../pkgctx/BubbleNotice";
import { gotoSettingsPanel } from "../palette/goto";
import { setActive } from "../settings/ctx";
import Tabs from "../layout/Tabs";
import { beginEdit, editing, finishEdit } from "../layout/edit";
import "./head.css";

export default function Head(p: { s: Snapshot; now: number; page: Page; onNote: () => void; onSettings: () => void; onBack: () => void; onUpdate: () => void; onPalette?: () => void }) {
  const d = () => new Date(p.now);
  const hhmm = () => fmtTime(d());
  const date = () => fmtDate(d());
  const live = () => p.s.procs.filter((x) => ["ready", "running", "starting", "preparing"].includes(x.phase));
  const failed = () => p.s.procs.filter((x) => x.phase === "failed").length;
  const working = () => p.s.sessions.filter((x) => x.state === "working").length;
  const dirty = () => p.s.projects.filter((x) => (x.git?.dirty ?? 0) > 0).length;
  const unpushed = () => p.s.projects.filter((x) => (x.git?.ahead ?? 0) > 0).length;
  const unfinished = () => p.s.sessions.filter((x) => x.uncommitted.length > 0 || x.unpushed > 0);
  // jen „dostupná“ — nelze_overit (gh chybí, offline) hlavička neukazuje, to patří do Nastavení
  const avail = () => (p.page === "main" && p.s.update.state === "dostupna" ? p.s.update : null);
  return (
    <div class="head" data-anim>
      <Show when={p.page !== "main"} fallback={
        <div class="clock">
          <span class="time">{hhmm()}</span>
          <span class="date">{date()}</span>
        </div>
      }>
        <button class="icon-btn" onClick={p.onBack} title={t("cards.head.backTitle", "Zpět (Esc)")} aria-label={t("cards.head.back", "Zpět")}><IconBack /></button>
        <span class="title">{p.page === "settings" ? t("cards.head.settings", "Nastavení") : p.page === "notes" ? t("cards.head.notes", "Poznámky") : editPath() ? t("cards.head.note", "Poznámka") : t("cards.head.quickNote", "Rychlá poznámka")}</span>
      </Show>
      <div class="chips">
        <Show when={p.page === "main"}>
          <Show when={p.s.pending.length + p.s.questions.length > 0}>
            <span class="chip amber"><span class="d" /><b>{t("cards.head.claudeWaiting", "Claude čeká")}</b>{p.s.pending.length + p.s.questions.length > 1 ? ` (${p.s.pending.length + p.s.questions.length})` : ""}</span>
          </Show>
          <Show when={failed() > 0}><span class="chip red"><span class="d" /><b>{tp("cards.head.crashes", failed(), "{n}× pád")}</b></span></Show>
          <span class={`chip ${live().length ? "green" : "dim"}`}><span class="d" />{rich(t("cards.head.live", "<b>{n}</b> běží", { n: live().length }))}</span>
          <Show when={working() > 0}><span class="chip violet"><span class="d" />{rich(t("cards.head.working", "Claude <b>{n}×</b> pracuje", { n: working() }))}</span></Show>
          <Show when={unfinished().length > 0}>
            <span class="chip amber" title={unfinished().map((x) => x.title ?? x.project ?? x.cwd).join("\n")}><span class="d" />{rich(tp("cards.head.unfinished", unfinished().length, "<b>{n}</b> session nedokončily"))}</span>
          </Show>
          <Show when={dirty() + unpushed() > 0}>
            <span class="chip amber"><span class="d" />{rich(t("cards.head.dirty", "<b>{n}</b> neuloženo", { n: dirty() }))}{unpushed() ? t("cards.head.unpushed", " · {n} nepushnuto", { n: unpushed() }) : ""}</span>
          </Show>
          <Show when={bubble()}>
            {(b) => (
              <button class="chip dim" title={t("cards.pkg.chipTitle", "Ledge běží uvnitř aplikace {app} (balíček MSIX): nastavení, data a spouštění po startu jdou do její izolované kopie. Klik = Nastavení → Restartovat mimo.", { app: b().app })} onClick={() => { setActive("suplik"); p.onSettings(); gotoSettingsPanel("f-autostart"); }}>
                <span class="d" />{t("cards.pkg.chip", "V bublině {app}", { app: b().app })}
              </button>
            )}
          </Show>
        </Show>
      </div>
      <div class="tools">
        <Show when={avail()}>
          {(u) => <button class="chip green upd" onClick={p.onUpdate} title={t("cards.head.updateTitle", "Poznámky k vydání a instalace")}><span class="d" />{t("cards.head.newVersion", "Nová verze")} <b>{u().verze}</b></button>}
        </Show>
        <Show when={p.page === "main"}>
          <Tabs s={p.s} />
          <button class="icon-btn" classList={{ on: editing() }} aria-pressed={editing()} onClick={() => (editing() ? void finishEdit() : beginEdit(p.s))} title={editing() ? t("cards.layout.doneTitle", "Hotovo — uložit rozložení") : t("cards.layout.editTitle", "Upravit šuplík: přesunout, zvětšit, přidat karty a záložky")} aria-label={t("cards.layout.editDrawer", "Upravit šuplík")}>✎</button>
        </Show>
        <Show when={p.s.notesEnabled && p.page === "main"}>
          <button class="icon-btn wide" onClick={p.onNote} title={t("cards.head.newNote", "Nová poznámka")}><IconNote />{t("cards.head.note", "Poznámka")}</button>
        </Show>
        <Show when={p.onPalette && p.page !== "note"}>
          <button class="icon-btn" onClick={() => p.onPalette?.()} title={t("cards.head.paletteTitle", "Hledat a spouštět (Ctrl+K)")} aria-label={t("cards.head.palette", "Paleta příkazů")}><IconSearch /></button>
        </Show>
        <button class="icon-btn" onClick={p.onSettings} title={t("cards.head.settings", "Nastavení")} aria-label={t("cards.head.settings", "Nastavení")} aria-pressed={p.page === "settings"}><IconSettings /></button>
        <button class="icon-btn" onClick={() => api.close()} title={t("cards.head.closeTitle", "Zavřít (Esc)")} aria-label={t("cards.head.close", "Zavřít šuplík")}><IconX /></button>
      </div>
    </div>
  );
}
