// Okno widgetu na ploše (`desk.html`, label `desk-<id>`, Rust `desk/win.rs`): obsah podle druhu (karta s
// nadpisem) a přes její nadpis průhledná lišta (tah = posun, ⋯ = volby); obsah podle druhu (`DeskBody` ve `views.tsx` — znovupoužité karty šuplíku). Data jen z událostí:
// `desk-data` (Rust `desk/live.rs` je pošle, jen když je widget vidět a něco se změnilo), `desk-prefs`
// (volby změnil někdo jiný), `desk-lang`, `appearance` (motiv). Žádný časovač: odpočty se přepočítají s daty
// (Rust je pošle nejvýš jednou za minutu). Widget nic neschvaluje — „Čeká na tebe“ jen otevře šuplík.
import { createSignal, For, onCleanup, onMount, Show } from "solid-js";
import { on } from "../api";
import { setLang, t, type Lang } from "../i18n";
import { initAppearance } from "../theme/apply";
import { errorText, sayError, Toast } from "../ui/toast";
import { PopCap, PopItem, PopMenu, PopSep, createMenu } from "../ui/Popover";
import type { DeskData, DeskLayer, DeskWidget } from "../types";
import { deskApi, type DeskPatch } from "./api";
import { clampOpacity, kindName, LAYERS, OPACITY_MIN } from "./model";
import { tx } from "../layout/text";
import DeskBody from "./views";

/** Název widgetu v liště: vestavěný přeložený, skript = titulek z dat nebo jméno souboru. */
function title(w: DeskWidget, d: DeskData | null): string {
  const n = kindName(w.kind);
  if (typeof n !== "string") return tx(n);
  const v = d?.widgets?.find((x) => x.id === w.kind);
  return v?.data?.title ?? v?.title ?? n;
}

export default function DeskApp() {
  const [w, setW] = createSignal<DeskWidget | null>(null);
  const [data, setData] = createSignal<DeskData | null>(null);
  const [gone, setGone] = createSignal<string | null>(null);
  /** posuvník průhlednosti: náhled během tahu, uloží se po puštění */
  const [look, setLook] = createSignal<number | null>(null);
  const menu = createMenu();
  let menuBtn: HTMLButtonElement | undefined;

  const offs: (() => void)[] = [];
  onCleanup(() => offs.forEach((f) => f()));
  onMount(() => {
    void on<DeskData>("desk-data", setData).then((f) => offs.push(f));
    void on<DeskWidget>("desk-prefs", setW).then((f) => offs.push(f));
    void on<Lang>("desk-lang", (l) => void setLang(l)).then((f) => offs.push(f));
    void initAppearance().then((f) => offs.push(f));
    deskApi.init().then(
      (i) => {
        if (!i) return;
        setLang(i.lang);
        setW(i.widget);
        setData(i.data);
      },
      (e) => setGone(errorText(e)),
    );
  });

  async function patch(x: DeskPatch) {
    try {
      const r = await deskApi.set(x);
      if (r) setW(r);
    } catch (e) {
      sayError(e);
    }
  }
  const openMenu = (e: MouseEvent) => {
    if (menu.open()) return closeMenu();
    void deskApi.menu(true);
    menu.toggle(e);
  };
  const closeMenu = () => {
    menu.close();
    setLook(null);
    void deskApi.menu(false);
  };
  const drag = (e: PointerEvent) => {
    if (e.button !== 0 || (e.target as HTMLElement).closest("button")) return;
    void deskApi.drag();
  };
  const run = (f: () => unknown) => () => {
    closeMenu();
    void Promise.resolve(f()).catch(sayError);
  };
  const opacity = () => clampOpacity(look() ?? w()?.opacity ?? 100);

  return (
    <div class="dk" classList={{ "dk-compact": !!w()?.compact, "dk-through": !!w()?.through, "dk-menu-open": !!menu.open() }} style={{ "--dk-o": String(opacity() / 100) }}>
      <Show when={w()} fallback={<div class="dk-gone">{gone() ?? ""}</div>}>
        {(wd) => (
          <>
            <div class="dk-body">
              <DeskBody w={wd()} d={data()} title={title(wd(), data())} />
            </div>
            <div class="dk-bar" onPointerDown={drag} aria-label={title(wd(), data())} title={wd().locked ? t("cards.desk.lockedTip", "Zamčeno — posun vypneš v ⋯") : t("cards.desk.dragTip", "Táhni = posunout")}>
              <Show when={wd().locked}><span class="dk-lock" aria-label={t("cards.desk.locked", "Zamčeno")}>🔒</span></Show>
              <button ref={menuBtn} class="dk-btn" aria-haspopup="menu" aria-expanded={!!menu.open()} title={t("cards.desk.more", "Volby widgetu")} aria-label={t("cards.desk.more", "Volby widgetu")} onClick={openMenu}>⋯</button>
            </div>
            <Show when={menu.at()}>
              {(at) => (
                <PopMenu fill anchor={at().anchor} keyboard={at().keyboard} trigger={menuBtn} onClose={closeMenu} label={t("cards.desk.more", "Volby widgetu")}>
                  <span class="pop-choice" role="group" aria-label={t("cards.desk.layer", "Vrstva")}>
                    <PopCap>{t("cards.desk.layer", "Vrstva")}</PopCap>
                    <span class="pop-seg">
                      <For each={LAYERS}>
                        {(l) => (
                          <button role="menuitemradio" aria-checked={wd().layer === l.v} classList={{ "pop-on": wd().layer === l.v }} title={tx(l.hint)} onClick={() => void patch({ layer: l.v as DeskLayer })}>
                            {tx(l.name)}
                          </button>
                        )}
                      </For>
                    </span>
                  </span>
                  <PopItem checked={wd().compact} onClick={() => void patch({ compact: !wd().compact })}>{t("cards.desk.compact", "Kompaktní")}</PopItem>
                  <PopItem checked={wd().locked} onClick={() => void patch({ locked: !wd().locked })}>{t("cards.desk.lock", "Zamknout na místě")}</PopItem>
                  <PopItem
                    checked={wd().through}
                    title={t("cards.desk.throughTip", "Klik projde na okno pod widgetem. Vypneš ho v Nastavení → Na ploše.")}
                    onClick={run(() => patch({ through: !wd().through }))}
                  >
                    {t("cards.desk.through", "Průchozí (klik projde)")}
                  </PopItem>
                  <PopItem checked={wd().hideInGame} onClick={() => void patch({ hideInGame: !wd().hideInGame })}>{t("cards.desk.hideInGame", "Schovat ve hře")}</PopItem>
                  <label class="dk-range">
                    <span>{t("cards.desk.opacity", "Průhlednost")}</span>
                    <input
                      type="range"
                      min={OPACITY_MIN}
                      max={100}
                      step={5}
                      value={opacity()}
                      aria-label={t("cards.desk.opacity", "Průhlednost")}
                      onInput={(e) => setLook(Number(e.currentTarget.value))}
                      onChange={(e) => void patch({ opacity: Number(e.currentTarget.value) }).then(() => setLook(null))}
                    />
                    <span class="dk-pct">{opacity()} %</span>
                  </label>
                  <PopSep />
                  <PopItem onClick={run(() => deskApi.openDrawer("drawer"))}>{t("cards.desk.openLedge", "Otevřít Ledge")}</PopItem>
                  <PopItem onClick={run(() => deskApi.openDrawer("settings"))}>{t("cards.desk.settings", "Nastavení widgetů…")}</PopItem>
                  <PopItem danger onClick={run(() => deskApi.unpin())}>{t("cards.desk.unpin", "Odepnout z plochy")}</PopItem>
                </PopMenu>
              )}
            </Show>
          </>
        )}
      </Show>
      <Toast />
    </div>
  );
}
