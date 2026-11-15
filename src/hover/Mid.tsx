// Střední panel z docku (L147; Rust `View::Mid`, šířka ~1/3 monitoru `hovermode::mid_width`, výška podle obsahu): až 3 karty
// (`drawer.dockCards`, ověřené `midCards.ts`) kreslené STEJNÝMI komponentami jako hlavní stránka (`MainPage` s `mid`). Nahoře
// „▾ celý šuplík“ (→ celý šuplík, `api.expandDrawer`) a 📌; odjetí myši zavře jako dok (📌 drží). Neodpočítává se žádné
// samo-rozbalení — z docku sem vede klik / setrvání, dál jen výslovné tlačítko. Fokus nebere (psaní až po kliku do pole).
import { api, type Snapshot } from "../api";
import MainPage from "../MainPage";
import type { NotesNav } from "../nav";
import type { NotePreset } from "../notes/store";
import { t } from "../i18n";
import { dockCardsOf } from "./midCards";
import PinButton from "./PinButton";
import "./hover.css";

export default function Mid(p: { s: Snapshot; now: number; cards: readonly string[]; nav: NotesNav; onNote: (preset?: NotePreset) => void; onSettings: () => void }) {
  return (
    <div class="mid content" role="region" aria-label={t("cards.hover.midRegion", "Střední panel")}>
      <div class="mid-head">
        <button class="hov-more" aria-label={t("cards.hover.more", "▾ celý šuplík").replace(/^▾s*/, "")} onClick={() => void api.expandDrawer()}>{t("cards.hover.more", "▾ celý šuplík")}</button>
        <PinButton />
      </div>
      <MainPage s={p.s} now={p.now} mid={dockCardsOf(p.cards)} onNote={p.onNote} nav={p.nav} onSettings={p.onSettings} />
    </div>
  );
}
