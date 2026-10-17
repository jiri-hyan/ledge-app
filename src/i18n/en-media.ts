// Angličtina pro záložku Osobní — právě hraje, hlasitost, baterie (`src/media/*`; klíče
// `cards.media.*`, `cards.sound.*`, `cards.battery.*`).
import { registerDict } from "../i18n";

registerDict("en", {
  "cards.media.title": "Now playing",
  "cards.media.none": "Nothing is playing",
  "cards.media.untitled": "Untitled",
  "cards.media.prev": "Previous",
  "cards.media.next": "Next",
  "cards.media.play": "Play",
  "cards.media.pause": "Pause",
  "cards.media.paused": "paused",
  "cards.sound.title": "Sound",
  "cards.sound.mute": "Mute",
  "cards.sound.unmute": "Unmute",
  "cards.sound.level": "Volume",
  "cards.sound.pct": "{n}%",
  "cards.battery.pct": "{n}%",
  "cards.battery.charging": "charging",
  "cards.battery.plugged": "plugged in",
  "cards.battery.onBattery": "on battery",
  "cards.battery.leftHM": "{h} h {m} min left",
  "cards.battery.leftH": "{h} h left",
  "cards.battery.leftM": "{m} min left",
  "cards.battery.leftLess": "less than a minute left",
});
