// Angličtina pro blok Rozšíření v Nastavení (`src/ext/ExtBlock.tsx`; klíče `settings.ext.*`).
import { registerDict } from "../i18n";

registerDict("en", {
  "settings.ext.name.wall": "Live wallpaper",
  "settings.ext.none": "No extensions.",
  "settings.ext.state.off": "Off",
  "settings.ext.state.waiting": "On, waiting to start",
  "settings.ext.state.starting": "Starting…",
  "settings.ext.state.running": "Running",
  "settings.ext.state.pausedGame": "Paused (game)",
  "settings.ext.state.paused": "Paused",
  "settings.ext.state.backoff": "Crashed, will retry",
  "settings.ext.state.failed": "Failed",
  "settings.ext.state.incompatible": "Incompatible version",
  "settings.ext.cost.none": "RAM — · CPU —",
  "settings.ext.cost.tip": "Sum over all processes of the extension (incl. WebView2): RAM = private memory, CPU = % of the whole computer over the last few seconds",
  "settings.ext.cost": "RAM {ram} · CPU {cpu}",
  "settings.ext.cost.procs": " · {n} processes",
  "settings.ext.off": "Turn off",
  "settings.ext.on": "Turn on",
  "settings.ext.bg": "Wallpaper background",
  "settings.ext.bg.tip": "JPG, PNG or WebP from a local drive. Ledge keeps a downscaled copy and darkens it so the text stays readable.",
  "settings.ext.bg.none": "Gradient (no photo)",
  "settings.ext.bg.pick": "Choose photo…",
  "settings.ext.bg.clear": "Remove",
});
