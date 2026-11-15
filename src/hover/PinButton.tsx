// 📌 v docku a ve středním panelu (`drawer.hover = dock`): upevnit okno (neodjede, nezavře blur) / uvolnit. Stav drží
// `dockmode.ts` (`dockPinned`, zrcadlo `Opener::Pinned`); jen výslovný klik → `api.drawerPin`.
import { api } from "../api";
import { sayError } from "../ui/toast";
import { t } from "../i18n";
import { dockPinDo, dockPinned } from "./dockmode";

export async function togglePin(): Promise<void> {
  const next = !dockPinned();
  try {
    await api.drawerPin(next);
    dockPinDo({ type: "set", on: next });
  } catch (e) {
    sayError(e);
  }
}

export default function PinButton() {
  const title = () => (dockPinned() ? t("cards.hover.dock.unpin", "Uvolnit (zavře se odjetím myši)") : t("cards.hover.dock.pin", "Upevnit (zůstane otevřený)"));
  return (
    <button class="mini isl-pin" classList={{ "isl-pin-on": dockPinned() }} data-noexpand aria-pressed={dockPinned()} title={title()} aria-label={t("cards.hover.dock.pinLabel", "Upevnit")} onClick={() => void togglePin()}>
      <span aria-hidden="true">📌</span>
    </button>
  );
}
