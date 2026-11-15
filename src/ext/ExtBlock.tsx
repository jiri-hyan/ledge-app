// Řádky bloku „Rozšíření“ na stránce Spouštění a hry (sekci s kotvou `extensions` kreslí `panels/Startup.tsx`): řádek na každé rozšíření z registru Rustu (`ext::REGISTRY`) —
// stav, ŽIVĚ změřená RAM a CPU celého rozšíření (proces + WebView2) a tlačítko Zapnout / Vypnout (zapisuje
// `extensions`, platí hned jako ostatní volby). Měří jádro jen při volání `ext_status`: tady po 5 s a jen
// dokud je blok na stránce a okno vidět (`document.hidden` = nic). Bez měření „—“.
import { For, Show, createSignal, onCleanup, onMount } from "solid-js";
import { t } from "../i18n";
import { errorText } from "../ui/toast";
import { num } from "../format";
import { useSettings } from "../settings/ctx";
import { extApi } from "./api";
import { extState, fmtCpu, fmtRam, stateTone, withExt, type ExtRow, type ExtState } from "./model";

const EVERY_MS = 5000;

const NAMES: Record<string, () => string> = {
  wall: () => t("settings.ext.name.wall", "Živá tapeta"),
};

function stateText(r: ExtRow, s: ExtState): string {
  switch (s) {
    case "off": return t("settings.ext.state.off", "Vypnuto");
    case "waiting": return t("settings.ext.state.waiting", "Zapnuto, čeká na spuštění");
    case "starting": return t("settings.ext.state.starting", "Spouští se…");
    case "running": return t("settings.ext.state.running", "Běží");
    case "paused": return r.pauseWhy === "game" ? t("settings.ext.state.pausedGame", "Pauza (hra)") : t("settings.ext.state.paused", "Pauza");
    case "backoff": return t("settings.ext.state.backoff", "Spadlo, zkusí se znovu");
    case "failed": return t("settings.ext.state.failed", "Selhalo");
    case "incompatible": return t("settings.ext.state.incompatible", "Nekompatibilní verze");
  }
}

/** Pozadí tapety z vlastní fotky: název vybrané fotky, Vybrat / Odebrat. Soubor vybírá dialog v jádře (UI cestu neposílá). */
function WallBg() {
  const [name, setName] = createSignal<string | null>(null);
  const [err, setErr] = createSignal("");
  const [busy, setBusy] = createSignal(false);
  onMount(() => void extApi.wallBg("get").then(setName).catch(() => {}));
  const run = async (op: "pick" | "clear") => {
    setBusy(true);
    setErr("");
    try {
      setName(await extApi.wallBg(op));
    } catch (e) {
      setErr(errorText(e));
    } finally {
      setBusy(false);
    }
  };
  return (
    <div class="st-row">
      <div class="st-lbl">
        <span class="st-name">{t("settings.ext.bg", "Pozadí tapety")}</span>
        <small title={t("settings.ext.bg.tip", "JPG, PNG nebo WebP z místního disku. Ledge uloží zmenšenou kopii a ztmaví ji, aby byl text čitelný.")}>
          {name() ?? t("settings.ext.bg.none", "Přechod (bez fotky)")}
          <Show when={err()}><span role="alert">{" — "}{err()}</span></Show>
        </small>
      </div>
      <div class="st-ctl">
        <span class="st-status">
          <button class="mini" disabled={busy()} onClick={() => void run("pick")}>{t("settings.ext.bg.pick", "Vybrat fotku…")}</button>
          <Show when={name()}>
            <button class="mini" disabled={busy()} onClick={() => void run("clear")}>{t("settings.ext.bg.clear", "Odebrat")}</button>
          </Show>
        </span>
      </div>
    </div>
  );
}

export default function ExtBlock() {
  const c = useSettings();
  const [rows, setRows] = createSignal<ExtRow[] | null>(null);
  onMount(() => {
    let alive = true;
    let timer: number | undefined;
    const tick = async () => {
      if (!document.hidden) {
        try {
          const v = await extApi.status();
          if (alive) setRows(v);
        } catch {
          /* jedno měření navíc nic neřeší — příště */
        }
      }
      if (alive) timer = window.setTimeout(tick, EVERY_MS);
    };
    void tick();
    onCleanup(() => {
      alive = false;
      if (timer !== undefined) window.clearTimeout(timer);
    });
  });
  const on = (id: string) => ((c.cfg.extensions as string[] | undefined) ?? []).includes(id);
  const flip = (id: string) => c.set("extensions", withExt((c.cfg.extensions as string[] | undefined) ?? [], id, !on(id)));
  return (
    <>
      <Show when={rows()?.length} fallback={<div class="empty">{t("settings.ext.none", "Žádná rozšíření.")}</div>}>
        <For each={rows()}>
          {(r) => {
            const s = () => extState({ enabled: on(r.id), phase: on(r.id) === r.enabled ? r.phase : "idle" });
            return (
              <>
              <div class="st-row">
                <div class="st-lbl">
                  <span class="st-name">{(NAMES[r.id] ?? (() => r.id))()}</span>
                  <small>
                    <Show when={r.cost} fallback={<>{t("settings.ext.cost.none", "RAM — · CPU —")}</>}>
                      {(k) => (
                        <span title={t("settings.ext.cost.tip", "Součet přes všechny procesy rozšíření (i WebView2): RAM = privátní paměť, CPU = % celého počítače za posledních pár sekund")}>
                          {t("settings.ext.cost", "RAM {ram} · CPU {cpu}", { ram: fmtRam(k(), num), cpu: fmtCpu(k(), num) })}
                          <Show when={k().procs > 1}>{t("settings.ext.cost.procs", " · {n} procesů", { n: k().procs })}</Show>
                        </span>
                      )}
                    </Show>
                    <Show when={r.error && (s() === "failed" || s() === "backoff" || s() === "incompatible")}>
                      {" — "}{r.error}
                    </Show>
                  </small>
                </div>
                <div class="st-ctl">
                  <span class="st-status">
                    <span class="st-dot" classList={{ ok: stateTone(s()) === "ok", warn: stateTone(s()) === "warn", bad: stateTone(s()) === "bad" }} />
                    {stateText(r, s())}
                    <button class="mini" classList={{ ok: on(r.id) }} onClick={() => flip(r.id)}>
                      {on(r.id) ? t("settings.ext.off", "Vypnout") : t("settings.ext.on", "Zapnout")}
                    </button>
                  </span>
                </div>
              </div>
              <Show when={r.id === "wall" && on("wall")}>
                <WallBg />
              </Show>
              </>
            );
          }}
        </For>
      </Show>
    </>
  );
}
