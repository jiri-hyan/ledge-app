// Angličtina pro bublinu MSIX (`src/pkgctx/*`, chip v hlavičce, řádek v panelu aktualizace; klíče
// `settings.pkg.*`, `cards.pkg.*`, `cards.update.outside`).
import { registerDict } from "../i18n";

registerDict("en", {
  "settings.pkg.notice": "Ledge is running inside the {app} app (an MSIX package). Settings, data and start with Windows are saved into its isolated copy.",
  "settings.pkg.where": "Copy inside the bubble:",
  "settings.pkg.after": "Outside the bubble Ledge uses",
  "settings.pkg.noCopy": "— settings from the bubble aren't carried over.",
  "settings.pkg.restart": "Restart outside",
  "settings.pkg.busy": "Restarting…",
  "settings.pkg.restarting": "Restarting outside the package — Ledge will be back in a moment",
  "settings.pkg.stops.one": "The restart will stop {n} running server: {list}",
  "settings.pkg.stops.other": "The restart will stop {n} running servers: {list}",
  "settings.pkg.back": "Back",
  "settings.pkg.stopRestart": "Stop and restart",
  "settings.pkg.autostartBlocked": "Not possible right now — Ledge is running inside the {family} package; Windows wouldn't see the entry at sign-in. It's written automatically after a restart outside.",
  "cards.pkg.chip": "In the {app} bubble",
  "cards.pkg.chipTitle": "Ledge is running inside the {app} app (an MSIX package): settings, data and start with Windows go into its isolated copy. Click = Settings → Restart outside.",
  "cards.update.outside": "Ledge is running in the {app} bubble — the installer will run outside it (Task Scheduler), install Ledge for real and start it from there.",
});
