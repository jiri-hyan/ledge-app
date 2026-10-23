// Karta „Přihrádka“: soubory přetažené na proužek (jen odkazy — cesty, nic se nekopíruje). Řádek:
// ikona ze shellu, název, složka, „chybí“; akce Cesta / @ (zmínka pro Claude Code) do schránky,
// Otevřít, Ve složce, ✕. Hlavička: kopírovat vše (oba formáty), vyprázdnit (s potvrzením `ui/Confirm`). Řádek
// jde táhnout ven (Průzkumník, terminál s Claude) — tah spustí jádro (`shelf/dragout.rs`), nic se
// neposílá samo. Data si karta bere sama (`shelf_list`) při otevření šuplíku a po události `shelf`.
import { createSignal, For, Show, type JSX } from "solid-js";
import { on } from "../api";
import type { ViewMsg } from "../types";
import { say, sayError } from "../ui/toast";
import { Confirm } from "../ui/Confirm";
import { t, tp } from "../i18n";
import { shelfApi } from "./api";
import { cardVisible, carriesFiles, isDrag, type ClipFormat, type ShelfItem, type ShelfList } from "./model";
import "./shelf.css";

const [list, setList] = createSignal<ShelfList>();
const [icons, setIcons] = createSignal<Record<string, string>>({});
const asked = new Set<string>();
let wired = false;

async function refresh() {
  try {
    setList(await shelfApi.list());
  } catch {
    /* staré jádro bez přihrádky — karta se neukáže */
  }
}

/** Jednou za běh UI: seznam teď, po změně (`shelf`) a při každém otevření šuplíku (`missing`). */
function wire() {
  if (wired) return;
  wired = true;
  void refresh();
  void on("shelf", () => void refresh());
  void on<ViewMsg>("view", (m) => m.view !== "hidden" && void refresh());
  // soubor puštěný do šuplíku (i položka přihrádky tažená zpátky) nesmí WebView „otevřít“ jako stránku
  const guard = (e: DragEvent) => {
    if (!carriesFiles(e.dataTransfer?.types)) return;
    e.preventDefault();
    if (e.dataTransfer) e.dataTransfer.dropEffect = "none";
  };
  window.addEventListener("dragover", guard);
  window.addEventListener("drop", guard);
}

function iconSrc(path: string): string | undefined {
  if (!asked.has(path)) {
    asked.add(path);
    shelfApi.icon(path).then((url) => url && setIcons((m) => ({ ...m, [path]: url })), () => asked.delete(path));
  }
  return icons()[path];
}

async function copy(paths: string[], format: ClipFormat) {
  try {
    const n = (await shelfApi.copy(paths, format)) ?? 0;
    say(format === "claude"
      ? tp("cards.shelf.copiedClaude", n, ["{n} zmínka pro Claude ve schránce", "{n} zmínky pro Claude ve schránce", "{n} zmínek pro Claude ve schránce"])
      : tp("cards.shelf.copied", n, ["{n} cesta ve schránce", "{n} cesty ve schránce", "{n} cest ve schránce"]));
  } catch (e) {
    sayError(e);
  }
}

const run = (f: () => Promise<unknown>) => () => f().catch(sayError);

function Row(p: { it: ShelfItem }) {
  let start: { x: number; y: number } | null = null;
  const down = (e: PointerEvent) => {
    if (e.button !== 0 || p.it.missing || (e.target as Element).closest("button")) return;
    start = { x: e.clientX, y: e.clientY };
    (e.currentTarget as Element).setPointerCapture(e.pointerId);
  };
  const move = (e: PointerEvent) => {
    if (!start || !(e.buttons & 1) || !isDrag(e.clientX - start.x, e.clientY - start.y)) return;
    start = null;
    shelfApi.dragOut([p.it.path]).catch(sayError);
  };
  const up = () => (start = null);
  const src = () => iconSrc(p.it.path);
  return (
    <div
      class="row shelf-row"
      classList={{ missing: p.it.missing }}
      title={p.it.missing ? p.it.path : `${p.it.path}\n${t("cards.shelf.dragTip", "Přetáhni ven: do Průzkumníka, terminálu s Claude Code, editoru…")}`}
      onPointerDown={down}
      onPointerMove={move}
      onPointerUp={up}
      onPointerCancel={up}
    >
      <span class="shelf-ico">
        <Show when={src()} fallback={<span class="shelf-blank" />}>{(s) => <img src={s()} alt="" draggable={false} />}</Show>
      </span>
      <div class="grow">
        <div class="name">{p.it.name}</div>
        <div class="sub">{p.it.dir || "—"}</div>
      </div>
      <Show when={p.it.missing}><span class="pill r">{t("cards.shelf.missing", "chybí")}</span></Show>
      <span class="r">
        <button class="mini" title={t("cards.shelf.copyPathTip", "Kopírovat cestu do schránky")} onClick={() => copy([p.it.path], "plain")}>{t("cards.shelf.copyPath", "Cesta")}</button>
        <button class="mini" title={t("cards.shelf.copyClaudeTip", "Kopírovat jako zmínku pro Claude Code (@cesta)")} onClick={() => copy([p.it.path], "claude")}>@</button>
        <button class="mini" disabled={p.it.missing} onClick={run(() => shelfApi.open(p.it.path))}>{t("cards.shelf.open", "Otevřít")}</button>
        <button class="mini" disabled={p.it.missing} title={t("cards.shelf.revealTip", "Ukázat ve složce")} onClick={run(() => shelfApi.reveal(p.it.path))}>{t("cards.shelf.reveal", "Ve složce")}</button>
        <button class="mini" title={t("cards.shelf.remove", "Odebrat z přihrádky (soubor zůstane)")} aria-label={t("cards.shelf.remove", "Odebrat z přihrádky (soubor zůstane)")} onClick={run(() => shelfApi.remove(p.it.path))}>✕</button>
      </span>
    </div>
  );
}

/** Karta je teď vidět (přihrádka zapnutá a má položky) — pro prázdný stav hlavní stránky. */
export const shelfShown = () => cardVisible(list());

export default function ShelfCard(p: { wide?: boolean; menu?: JSX.Element }) {
  wire();
  const [confirm, setConfirm] = createSignal(false);
  const items = () => list()?.items ?? [];
  const clear = async () => {
    setConfirm(false);
    await shelfApi.clear().catch(sayError);
  };
  return (
    <Show when={cardVisible(list())}>
      <div class="card shelf-card" classList={{ wide: !!p.wide }} data-anim>
        <h4>
          <span class="ic" style="background:var(--teal)" />
          {t("cards.shelf.title", "Přihrádka")}
          <span class="n">{items().length}</span>
          <button class="mini" title={t("cards.shelf.copyAllTip", "Všechny cesty do schránky, každá na řádek")} onClick={() => copy([], "plain")}>{t("cards.shelf.copyAll", "Kopírovat vše")}</button>
          <button class="mini" title={t("cards.shelf.copyAllClaudeTip", "Všechny jako zmínky pro Claude Code na jednom řádku")} onClick={() => copy([], "claude")}>{t("cards.shelf.copyAllClaude", "@ vše")}</button>
          <button class="mini" onClick={() => setConfirm(true)}>{t("cards.shelf.clear", "Vyprázdnit")}</button>
          {p.menu}
        </h4>
        <Confirm
          when={confirm()}
          text={t("cards.shelf.clearConfirm", "Vyprázdnit přihrádku?")}
          detail={t("cards.shelf.clearDetail", "Odebere všechny odkazy z přihrádky. Soubory na disku zůstanou.")}
          yes={t("cards.shelf.clear", "Vyprázdnit")}
          danger
          onYes={() => void clear()}
          onNo={() => setConfirm(false)}
        />
        <div class="shelf-list">
          <For each={items()}>{(it) => <Row it={it} />}</For>
        </div>
      </div>
    </Show>
  );
}
