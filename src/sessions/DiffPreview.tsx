// Náhled změny Edit/Write v kartě schválení (`PendingRow` i `cards/Peek.tsx`): sbalený řádek
// „+12 −3 · soubor.rs“, klik rozbalí řádkový diff (kontext, čísla řádků ze souboru, strop s hláškou).
// Data spočítal Rust jednou při příchodu žádosti (`claude/preview.rs` + `claude/diff.rs`); tady jen
// vykreslení — text řádků jde jako textový uzel (žádné innerHTML). Logika v `diff.ts`.
import { createSignal, For, Show } from "solid-js";
import type { EditPreview } from "../api";
import { t, tp } from "../i18n";
import { counts, expandable, gutter, gutterWidth, headKind, sign } from "./diff";
import "../sessions.css";

export default function DiffPreview(p: { pv: EditPreview }) {
  const [open, setOpen] = createSignal(false);
  const can = () => expandable(p.pv);
  const head = () => {
    switch (headKind(p.pv)) {
      case "counts": return counts(p.pv);
      case "newFile": return tp("sessions.diff.newFile", p.pv.total, ["nový soubor, {n} řádek", "nový soubor, {n} řádky", "nový soubor, {n} řádků"]);
      case "unread": return t("sessions.diff.unread", "aktuální soubor nejde přečíst — jen nový obsah");
      case "tooBig": return t("sessions.diff.tooBig", "příliš velké na náhled (nad 512 KB)");
      case "binary": return t("sessions.diff.binary", "nelze zobrazit (binární nebo ne-UTF-8)");
    }
  };
  const width = () => gutterWidth(p.pv.lines, p.pv.numbered);
  return (
    <div class="sess-diff">
      <button class="sess-diff-head" aria-expanded={open()} disabled={!can()} onClick={() => setOpen(!open())} title={p.pv.path}>
        <span classList={{ "sess-diff-n": headKind(p.pv) === "counts" }}>{head()}</span> · {p.pv.file}
        <Show when={p.pv.replaceAll}>
          {" · "}{p.pv.occurrences != null
            ? tp("sessions.diff.allN", p.pv.occurrences, "všechny výskyty ({n})")
            : t("sessions.diff.all", "všechny výskyty")}
        </Show>
        <Show when={can()}> {open() ? "▴" : "▾"}</Show>
      </button>
      <Show when={open() && can()}>
        <div class="sess-diff-body" style={{ "--gw": `${width()}ch` }}>
          <For each={p.pv.lines}>
            {(l) => (
              <div class="sess-diff-ln" classList={{ "sess-diff-add": l.kind === "add", "sess-diff-del": l.kind === "del", "sess-diff-gap": l.kind === "gap" }}>
                <Show when={width() > 0}><span class="sess-diff-no">{gutter(l, p.pv.numbered)}</span></Show>
                <span class="sess-diff-sg">{sign(l.kind)}</span>
                <span class="sess-diff-tx">{l.text}</span>
              </div>
            )}
          </For>
        </div>
        <Show when={p.pv.cut > 0}>
          <small class="sess-diff-note">{tp("sessions.diff.cut", p.pv.cut, ["… a {n} další řádek (náhled je zkrácený)", "… a {n} další řádky (náhled je zkrácený)", "… a {n} dalších řádků (náhled je zkrácený)"])}</small>
        </Show>
        <Show when={p.pv.rough}>
          <small class="sess-diff-note">{t("sessions.diff.rough", "Rozdíl je velký — ukázaný jako nahrazení celého bloku.")}</small>
        </Show>
        <Show when={p.pv.shown === "diff" && !p.pv.numbered}>
          <small class="sess-diff-note">{t("sessions.diff.noNums", "Bez čísel řádků — text se v aktuálním souboru nenašel (nebo soubor nejde přečíst).")}</small>
        </Show>
      </Show>
    </div>
  );
}
