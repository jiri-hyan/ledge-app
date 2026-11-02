// Hlavička šuplíku: hodiny (nebo Zpět + název stránky), stavové chipy, nová verze, záložky (`layout/Tabs.tsx`),
// pilulka „N běží“ (klik = seznam serverů se Stop / Restart / Otevřít / Log — `projects/RunningPill.tsx`),
// chip „N neuloženo“ (klik = Přečteno / Ztišit u repa — `cards/DirtyPill.tsx`),
// ✎ režim úprav šuplíku (`layout/edit.ts`) a nástroje (paleta 🔍, poznámka, nastavení, zavřít). Čte jen snapshot; navigaci dělá skořápka (App) přes callbacky.
// Kolo 27: díly hlavičky, jejich pořadí a styly (hodiny, štítky, záložky) jsou nastavitelné — `appearance.head` / `appearance.tabs`
// (`layout/headModel.ts`); v režimu ✎ je upravíš klikem na hlavičku (`layout/HeadLook.tsx`). Štítky jsou vždy v jedné řadě (`ChipsRow.tsx`).
import { createEffect, For, Match, Show, Switch } from "solid-js";
import { api, type Snapshot } from "../api";
import { IconBack, IconNote, IconSearch, IconSettings, IconX } from "../icons";
import { editPath } from "../notes/store";
import type { Page } from "../nav";
import type { HeadPart } from "../types.ts";
import { t, tp } from "../i18n";
import { rich } from "../ui/rich";
import { bubble } from "../pkgctx/BubbleNotice";
import { gotoSettingsPanel } from "../palette/goto";
import Tabs from "../layout/Tabs";
import { beginEdit, editing, finishEdit } from "../layout/edit";
import { HeadLookPanel } from "../lazyViews";
import { closeHeadLook, headDraft, headLook, openHeadLook, tabsLook } from "../layout/headState";
import { fullOrder, visibleParts } from "../layout/headModel";
import RunningPill from "../projects/RunningPill";
import DirtyPill from "./DirtyPill";
import ChipsRow from "./ChipsRow";
import HeadClock from "./HeadClock";
import "./head.css";

const plain = (s: string) => s.replace(/<[^>]+>/g, "");

export default function Head(p: { s: Snapshot; now: number; page: Page; onNote: () => void; onSettings: () => void; onBack: () => void; onUpdate: () => void; onPalette?: () => void }) {
  const failed = () => p.s.procs.filter((x) => x.phase === "failed").length;
  const working = () => p.s.sessions.filter((x) => x.state === "working").length;
  const unfinished = () => p.s.sessions.filter((x) => x.uncommitted.length > 0 || x.unpushed > 0);
  const waiting = () => p.s.pending.length + p.s.questions.length;
  // jen „dostupná“ — nelze_overit (gh chybí, offline) hlavička neukazuje, to patří do Nastavení
  const avail = () => (p.page === "main" && p.s.update.state === "dostupna" ? p.s.update : null);
  // jednorázově po první spuštění nové verze (poznámky z cache); zmizí po „Rozumím“ v panelu
  const fresh = () => (p.page === "main" && p.s.update.whatsNew?.unread && !avail() ? p.s.update.whatsNew : null);
  const main = () => p.page === "main";
  // mimo hlavní stránku zůstává hlavička po starém (Zpět + název, nástroje); díly se řídí jen na hlavní
  const look = () => headLook();
  const order = () => (main() ? fullOrder(look()) : fullOrder(undefined).filter((x) => x !== "clock"));
  const shown = () => (main() ? visibleParts(look()) : fullOrder(undefined));
  const on = (part: HeadPart) => shown().includes(part);
  // v režimu ✎ otevře klik do volného místa hlavičky nebo přepínače záložek panel „Vzhled hlavičky“
  const blank = (e: MouseEvent) => {
    if (!editing() || !main()) return;
    const el = e.target as HTMLElement;
    if (el.classList.contains("lay-tabs") || !el.closest("button, input, a, .lay-tab, .hl-chips .chip")) openHeadLook();
  };
  // konec úprav šuplíku uloží i rozpracovaný vzhled hlavičky
  createEffect(() => !editing() && headDraft() && void closeHeadLook(true));
  return (
    <>
      <div class="head" data-anim onClick={blank}>
        <Show when={!main()}>
          <button class="icon-btn" onClick={p.onBack} title={t("cards.head.backTitle", "Zpět (Esc)")} aria-label={t("cards.head.back", "Zpět")}><IconBack /></button>
          <span class="title">{p.page === "settings" ? t("cards.head.settings", "Nastavení") : p.page === "notes" ? t("cards.head.notes", "Poznámky") : editPath() ? t("cards.head.note", "Poznámka") : t("cards.head.quickNote", "Rychlá poznámka")}</span>
        </Show>
        <For each={order()}>
          {(part) => (
            <Switch>
              <Match when={part === "clock" && main() && on("clock")}>
                <HeadClock now={p.now} style={look().clock} showDate={look().showDate !== false} />
              </Match>
              <Match when={part === "chips"}>
                <div class="head-fill">
                  <Show when={main() && on("chips")}>
                    <ChipsRow style={look().chips}>
                      <Show when={waiting() > 0}>
                        <span class="chip amber" data-prio="0" title={t("cards.head.claudeWaiting", "Claude čeká")}>
                          <span class="d" /><span class="ic">⏳</span><b class="lbl"><span class="lng">{t("cards.head.claudeWaiting", "Claude čeká")}</span><span class="shr">{t("cards.head.waitingShort", "čeká")}</span></b>{waiting() > 1 ? ` (${waiting()})` : ""}<b class="cnt">{waiting()}</b>
                        </span>
                      </Show>
                      <Show when={failed() > 0}>
                        <span class="chip red" data-prio="1" title={tp("cards.head.crashes", failed(), "{n}× pád")}><span class="d" /><span class="ic">⚠</span><b class="lbl">{tp("cards.head.crashes", failed(), "{n}× pád")}</b><b class="cnt">{failed()}</b></span>
                      </Show>
                      <RunningPill s={p.s} now={p.now} showZero={look().showZero} />
                      <Show when={working() > 0}>
                        <span class="chip violet" data-prio="2" title={plain(t("cards.head.working", "Claude <b>{n}×</b> pracuje", { n: working() }))}><span class="d" /><span class="ic">✦</span><span class="lng">{rich(t("cards.head.working", "Claude <b>{n}×</b> pracuje", { n: working() }))}</span><span class="shr">{rich(t("cards.head.workingShort", "<b>{n}×</b> pracuje", { n: working() }))}</span></span>
                      </Show>
                      <Show when={unfinished().length > 0}>
                        <span class="chip amber" data-prio="3" title={unfinished().map((x) => x.title ?? x.project ?? x.cwd).join("\n")}><span class="d" /><span class="ic">⏸</span><span class="lng">{rich(tp("cards.head.unfinished", unfinished().length, "<b>{n}</b> session nedokončily"))}</span><span class="shr">{rich(t("cards.head.unfinishedShort", "<b>{n}</b> nedokonč.", { n: unfinished().length }))}</span></span>
                      </Show>
                      <DirtyPill s={p.s} />
                      <Show when={bubble()}>
                        {(b) => (
                          <button class="chip dim" data-prio="6" title={t("cards.pkg.chipTitle", "Ledge běží uvnitř aplikace {app} (balíček MSIX): nastavení, data a spouštění po startu jdou do její izolované kopie. Klik = Nastavení → Restartovat mimo.", { app: b().app })} onClick={() => { p.onSettings(); gotoSettingsPanel("f-autostart"); }}>
                            <span class="d" /><span class="ic">◌</span>{t("cards.pkg.chip", "V bublině {app}", { app: b().app })}
                          </button>
                        )}
                      </Show>
                    </ChipsRow>
                  </Show>
                </div>
                <Show when={avail()}>
                  {(u) => <button class="chip green upd" onClick={p.onUpdate} title={t("cards.head.updateTitle", "Poznámky k vydání a instalace")}><span class="d" />{p.s.update.fetch.state === "ready" ? t("cards.head.readyVersion", "Připraveno") : t("cards.head.newVersion", "Nová verze")} <b>{u().verze}</b></button>}
                </Show>
                <Show when={fresh()}>
                  {(w) => <button class="chip dim upd" onClick={p.onUpdate} title={t("cards.head.whatsNewTitle", "Co se změnilo v této verzi")}>{t("cards.head.whatsNew", "Co je nového v")} <b>{w().verze}</b></button>}
                </Show>
              </Match>
              <Match when={part === "tabs" && main() && on("tabs")}>
                <div class="head-tabs" data-tabstyle={tabsLook().style} data-pencil={on("edit") ? "off" : "on"}><Tabs s={p.s} /></div>
              </Match>
              <Match when={part === "edit" && main() && on("edit")}>
                <button class="icon-btn" data-tip="edit" classList={{ on: editing() }} aria-pressed={editing()} onClick={() => (editing() ? void finishEdit() : beginEdit(p.s))} title={editing() ? t("cards.layout.doneTitle", "Hotovo — uložit rozložení") : t("cards.layout.editTitle", "Upravit šuplík: přesunout, zvětšit, přidat karty a záložky")} aria-label={t("cards.layout.editDrawer", "Upravit šuplík")}>✎</button>
                <Show when={editing()}>
                  <button class="icon-btn" classList={{ on: !!headDraft() }} aria-pressed={!!headDraft()} onClick={() => (headDraft() ? void closeHeadLook(true) : openHeadLook())} title={t("cards.headlook.open", "Vzhled hlavičky: díly, hodiny, štítky, záložky")} aria-label={t("cards.headlook.title", "Vzhled hlavičky")}>Aa</button>
                </Show>
              </Match>
              <Match when={part === "note" && p.s.notesEnabled && main() && on("note")}>
                <button class="icon-btn wide" onClick={p.onNote} title={t("cards.head.newNote", "Nová poznámka")}><IconNote /><span class="hl-lbl">{t("cards.head.note", "Poznámka")}</span></button>
              </Match>
              <Match when={part === "search" && p.onPalette && p.page !== "note" && on("search")}>
                <button class="icon-btn" data-tip="palette" onClick={() => p.onPalette?.()} title={t("cards.head.paletteTitle", "Hledat a spouštět (Ctrl+K)")} aria-label={t("cards.head.palette", "Paleta příkazů")}><IconSearch /></button>
              </Match>
              <Match when={part === "settings"}>
                <button class="icon-btn" onClick={p.onSettings} title={t("cards.head.settings", "Nastavení")} aria-label={t("cards.head.settings", "Nastavení")} aria-pressed={p.page === "settings"}><IconSettings /></button>
              </Match>
              <Match when={part === "close"}>
                <button class="icon-btn" onClick={() => api.close()} title={t("cards.head.closeTitle", "Zavřít (Esc)")} aria-label={t("cards.head.close", "Zavřít šuplík")}><IconX /></button>
              </Match>
            </Switch>
          )}
        </For>
      </div>
      <Show when={main() && editing() && headDraft()}><HeadLookPanel /></Show>
    </>
  );
}
