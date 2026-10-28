// Ukázkový hostitel Ledge Studia JEN pro `settings.html?demo=1` v prohlížeči mimo Tauri (kolo 24): snímky
// a kontrola vzhledu bez běžícího Ledge. V Tauri se nikdy nezapne (`main.tsx` ho načte jen mimo Tauri
// a `api.setDemoHost` tam nic neudělá). Config = výchozí z Rustu (`demo-config.json`, `Config::default()`)
// s pár smyšlenými úpravami; zápis jde jen do paměti stránky. Jména jsou smyšlená (`app`, `app-v2`, example.com).
import { setDemoHost } from "../api";
import { EMPTY_SNAPSHOT } from "../snapshot";
import DEFAULTS from "./demo-config.json";

const clone = <T,>(v: T): T => structuredClone(v);

export function installDemo() {
  const defaults = clone(DEFAULTS) as any;
  let cfg: any = clone(DEFAULTS);
  cfg.strip.look = "island";
  cfg.autostart = true;
  cfg.appearance = { ...(cfg.appearance ?? {}), accent: "violet" };
  cfg.projects = [
    { id: "app", name: "app", enabled: true, kind: "dev", path: "C:/Users/uzivatel/dev/app", run: "npm run dev", port: 5173 },
    { id: "app-v2", name: "app-v2", enabled: true, kind: "dev", path: "C:/Users/uzivatel/dev/app-v2", run: "npm run dev", port: 5174 },
  ];
  const snap: any = clone(EMPTY_SNAPSHOT);
  snap.layout = cfg.layout ?? snap.layout;
  snap.hooks = { state: "full", missing: [], error: null, path: "", hint: null };
  snap.update = { ...snap.update, state: "aktualni", current: "0.9.0" };
  setDemoHost((cmd, args) => {
    switch (cmd) {
      case "get_config": return clone(cfg);
      case "save_config": cfg = clone(args?.cfg); return undefined;
      case "check_config": return null;
      case "config_defaults": return clone(defaults);
      case "get_state": return clone(snap);
      case "app_info": return { version: "0.9.0", configPath: "C:/Users/uzivatel/AppData/Roaming/Ledge/config.json", configDir: "", logPath: "" };
      default: return undefined;
    }
  });
}
