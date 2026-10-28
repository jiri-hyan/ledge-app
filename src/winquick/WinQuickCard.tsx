// Karta „Rychlé dlaždice Windows“ (kolo 23, id `winquick`, jen v katalogu karet): řada dlaždic, které si člověk sám
// přidává, odebírá a řadí (Upravit dlaždice). Mikrofon bere stav ze snapshotu (Core Audio, události), zbytek
// (motiv, rádia, poslední snímek) se čte JEN když je karta vidět a šuplík otevřený (`drawerShown`) — žádný časovač.
// Každá akce je výslovný klik; nic se samo nevkládá ani nespouští. Rust: `winquick.rs`, `commands/winquick.rs`.
import { For, Show, createEffect, createSignal, type JSX } from "solid-js";
import { api, type Snapshot } from "../api";
import { t } from "../i18n";
import { drawerShown } from "../media/shown";
import { say, sayError } from "../ui/toast";
import { TILE_ICON, addTile, baseName, cleanTiles, missingTiles, moveTile, pinFolder, radioOf, removeTile, type TileId, type WqState } from "./model";
import "./winquick.css";

const NAMES: Record<TileId, [string, string]> = {
  mic: ["cards.winquick.mic", "Mikrofon"],
  out: ["cards.winquick.out", "Výstup zvuku"],
  term: ["cards.winquick.term", "Terminál ve složce"],
  dir: ["cards.winquick.dir", "Složky"],
  theme: ["cards.winquick.theme", "Motiv Windows"],
  wifi: ["cards.winquick.wifi", "Wi-Fi"],
  bt: ["cards.winquick.bt", "Bluetooth"],
  set: ["cards.winquick.set", "Nastavení"],
  shot: ["cards.winquick.shot", "Poslední snímek"],
};
const name = (id: TileId) => t(NAMES[id][0], NAMES[id][1]);

const SETTINGS: [string, string, string][] = [
  ["network", "cards.winquick.set.network", "Síť"],
  ["sound", "cards.winquick.set.sound", "Zvuk"],
  ["display", "cards.winquick.set.display", "Displej"],
  ["bluetooth", "cards.winquick.set.bluetooth", "Bluetooth"],
];

const DIRS: Record<string, [string, string]> = {
  downloads: ["cards.winquick.dir.downloads", "Stažené"],
  desktop: ["cards.winquick.dir.desktop", "Plocha"],
  documents: ["cards.winquick.dir.documents", "Dokumenty"],
};
const dirName = (key: string, path: string) => (DIRS[key] ? t(DIRS[key][0], DIRS[key][1]) : baseName(path));

export default function WinQuickCard(p: { s: Snapshot; menu?: JSX.Element }) {
  const [st, setSt] = createSignal<WqState | null>(null);
  const [err, setErr] = createSignal("");
  const [open, setOpen] = createSignal<TileId | null>(null);
  const [edit, setEdit] = createSignal(false);
  const [proj, setProj] = createSignal("");
  const refresh = () => api.wqState().then((x) => { setSt(x ?? null); setErr(""); }).catch((e) => setErr(String(e)));
  // čte se při zobrazení karty a při každém otevření šuplíku; jinak nic
  createEffect(() => { if (drawerShown()) void refresh(); });

  const v = () => p.s.personal?.volume ?? null;
  const tiles = () => cleanTiles(st()?.tiles ?? []);
  const run = (f: () => Promise<unknown>, ok?: string) => f().then(() => { if (ok) say(ok); return refresh(); }).catch(sayError);
  const save = (tl: string[], folders = st()?.pinned ?? [], sw = st()?.outputSwitch ?? false) =>
    api.wqConfigSet(tl, folders, sw).then(refresh).catch(sayError);
  const pick = (id: TileId) => setOpen(open() === id ? null : id);
  const projects = () => p.s.projects ?? [];
  const cur = () => proj() || projects()[0]?.id || "";

  const click = (id: TileId) => {
    const x = st();
    switch (id) {
      case "mic":
        if (v()?.micMuted != null) void api.volumeMicMute().then((m) => say(m ? t("cards.sound.micMuteNotice", "Mikrofon ztlumen") : t("cards.sound.micUnmuteNotice", "Mikrofon zapnut"))).catch(sayError);
        return;
      case "out":
        if (x?.outputSwitch && (v()?.devices?.length ?? 0) > 0) pick("out");
        else void api.wqSettings("sound").catch(sayError);
        return;
      case "theme":
        void run(() => api.wqThemeToggle());
        return;
      case "wifi":
      case "bt": {
        const r = radioOf(x?.radios ?? [], id);
        if (r) void run(() => api.wqRadioSet(id, !r.on));
        return;
      }
      default:
        pick(id);
    }
  };

  const sub = (id: TileId): string => {
    const x = st();
    switch (id) {
      case "mic": { const m = v()?.micMuted; return m == null ? t("cards.winquick.na", "není k dispozici") : m ? t("cards.board.toggles.micOff", "ztlumený") : t("cards.board.toggles.micOn", "zapnutý"); }
      case "out": return v()?.device || t("cards.winquick.sound", "zvuk");
      case "theme": return x ? (x.light ? t("cards.winquick.light", "světlý") : t("cards.winquick.dark", "tmavý")) : "";
      case "wifi":
      case "bt": { const r = radioOf(x?.radios ?? [], id); return !r ? t("cards.winquick.na", "není k dispozici") : r.on ? t("cards.winquick.on", "zapnuto") : t("cards.winquick.off", "vypnuto"); }
      case "shot": return x?.shot ? t("cards.winquick.shotN", "{n} snímků", { n: x.shot.count }) : t("cards.winquick.none", "nic");
      default: return "";
    }
  };
  const isOn = (id: TileId): boolean => {
    const x = st();
    if (id === "mic") return v()?.micMuted === false;
    if (id === "wifi" || id === "bt") return radioOf(x?.radios ?? [], id)?.on ?? false;
    return false;
  };
  const dead = (id: TileId): boolean => {
    if (id === "mic") return v()?.micMuted == null;
    if (id === "wifi" || id === "bt") return !!st() && !radioOf(st()!.radios, id);
    return false;
  };

  const panel = (id: TileId): JSX.Element => {
    switch (id) {
      case "out":
        return (
          <div class="wq-list">
            <For each={v()?.devices ?? []}>
              {(d) => <button class="wq-chip" classList={{ on: d.active }} onClick={() => void run(() => api.wqOutputSet(d.id))}>{d.name}</button>}
            </For>
          </div>
        );
      case "term":
        return (
          <Show when={projects().length} fallback={<div class="empty">{t("cards.winquick.noProjects", "Žádný projekt.")}</div>}>
            <div class="wq-list">
              <For each={projects()}>{(pr) => <button class="wq-chip" classList={{ on: cur() === pr.id }} onClick={() => setProj(pr.id)}>{pr.name}</button>}</For>
            </div>
            <div class="wq-list">
              <button class="wq-chip" onClick={() => void api.wqTerminal(cur(), "wt").catch(sayError)}>Windows Terminal</button>
              <button class="wq-chip" onClick={() => void api.wqTerminal(cur(), "powershell").catch(sayError)}>PowerShell</button>
              <button class="wq-chip" onClick={() => void api.wqTerminal(cur(), "explorer").catch(sayError)}>{t("cards.winquick.explorer", "Průzkumník")}</button>
            </div>
          </Show>
        );
      case "dir":
        return (
          <div class="wq-list">
            <For each={st()?.folders ?? []}>
              {(f) => <button class="wq-chip" title={f.path} onClick={() => void api.wqFolder(f.key).catch(sayError)}>{dirName(f.key, f.path)}</button>}
            </For>
          </div>
        );
      case "set":
        return (
          <div class="wq-list">
            <For each={SETTINGS}>{([k, key, cs]) => <button class="wq-chip" onClick={() => void api.wqSettings(k).catch(sayError)}>{t(key, cs)}</button>}</For>
          </div>
        );
      case "shot":
        return (
          <Show when={st()?.shot} fallback={<div class="empty">{t("cards.winquick.shotNone", "Ve složce Snímky obrazovky nic není.")}</div>}>
            {(s) => (
              <div class="wq-shot">
                <Show when={s().thumb}><img src={s().thumb!} alt="" /></Show>
                <div class="wq-shot-b">
                  <span class="wq-shot-n" title={s().path}>{s().name}</span>
                  <div class="wq-list">
                    <button class="wq-chip" onClick={() => void api.wqShot("image").then((m) => m && say(m)).catch(sayError)}>{t("cards.winquick.copyImage", "Kopírovat")}</button>
                    <button class="wq-chip" onClick={() => void api.wqShot("path").then((m) => m && say(m)).catch(sayError)}>{t("cards.winquick.copyPath", "Kopírovat cestu")}</button>
                    <button class="wq-chip" title={t("cards.winquick.claudeTip", "Zkopíruje @cestu — vlož ji do chatu s Claudem")} onClick={() => void api.wqShot("claude").then((m) => m && say(m)).catch(sayError)}>{t("cards.winquick.claude", "Poslat Claudovi")}</button>
                    <button class="wq-chip" onClick={() => void api.wqShot("folder").catch(sayError)}>{t("cards.winquick.openFolder", "Otevřít složku")}</button>
                  </div>
                </div>
              </div>
            )}
          </Show>
        );
      default:
        return null;
    }
  };

  const editor = () => (
    <div class="wq-edit">
      <div class="wq-list">
        <For each={missingTiles(tiles())}>{(id) => <button class="wq-chip" onClick={() => void save(addTile(tiles(), id))}>+ {name(id)}</button>}</For>
      </div>
      <label class="wq-opt">
        <input type="checkbox" checked={st()?.outputSwitch ?? false} onChange={(e) => void save(tiles(), st()?.pinned, e.currentTarget.checked)} />
        <span>{t("cards.winquick.outSwitch", "Dlaždice výstupu přepíná zařízení jedním klikem")}</span>
      </label>
      <div class="wq-list">
        <For each={st()?.pinned ?? []}>
          {(f, i) => <button class="wq-chip" title={f} onClick={() => void save(tiles(), (st()?.pinned ?? []).filter((_, j) => j !== i()))}>{baseName(f)} ×</button>}
        </For>
        <button class="wq-chip" onClick={() => void api.pickPath("folder").then((d) => d && save(tiles(), pinFolder(st()?.pinned ?? [], d))).catch(sayError)}>{t("cards.winquick.pin", "+ Připnout složku")}</button>
      </div>
    </div>
  );

  return (
    <div class="card wq-card" data-anim>
      <h4>
        <span class="ic" style="background:var(--blue)" />{t("cards.winquick.title", "Rychlé dlaždice Windows")}
        <button class="mini" aria-pressed={edit()} onClick={() => setEdit(!edit())} title={t("cards.winquick.edit", "Upravit dlaždice")}>{edit() ? t("cards.winquick.done", "Hotovo") : "✎"}</button>
        {p.menu}
      </h4>
      <Show when={err()}><div class="empty">{err()}</div></Show>
      <div class="wq-row">
        <For each={tiles()}>
          {(id) => (
            <div class="wq-tile-w">
              <button class="wq-tile" classList={{ on: isOn(id), dead: dead(id), sel: open() === id }} disabled={dead(id)} aria-pressed={id === "mic" || id === "wifi" || id === "bt" || id === "theme" ? isOn(id) : undefined} title={`${name(id)}${sub(id) ? `: ${sub(id)}` : ""}`} onClick={() => click(id)}>
                <span class="wq-ic" aria-hidden="true">{TILE_ICON[id]}</span>
                <span class="wq-n">{name(id)}</span>
                <span class="wq-v">{sub(id)}</span>
              </button>
              <Show when={edit()}>
                <span class="wq-ed">
                  <button class="mini" aria-label={t("cards.winquick.left", "Doleva")} onClick={() => void save(moveTile(tiles(), id, -1))}>‹</button>
                  <button class="mini" aria-label={t("cards.winquick.right", "Doprava")} onClick={() => void save(moveTile(tiles(), id, 1))}>›</button>
                  <button class="mini" aria-label={t("cards.winquick.remove", "Odebrat")} onClick={() => void save(removeTile(tiles(), id))}>×</button>
                </span>
              </Show>
            </div>
          )}
        </For>
      </div>
      <Show when={!tiles().length && st()}><div class="empty">{t("cards.winquick.empty", "Žádné dlaždice — přidej je přes ✎.")}</div></Show>
      <Show when={open()}>{(id) => <div class="wq-panel">{panel(id())}</div>}</Show>
      <Show when={edit()}>{editor()}</Show>
    </div>
  );
}
