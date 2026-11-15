// ⋯ menu poznámky v editoru: skupina „Export“ (Markdown / HTML do souboru, kopie do schránky, Otevřít v … registrovaných aplikacích).
// Seznam je společné menu `ui/Popover.tsx` (pevná pozice u tlačítka — editor s omezenou výškou ho neořízne);
// Esc ho zavře a nedojde k editoru (nezahodí rozepsané).

import { createResource, createSignal, For, Show } from "solid-js";
import { t } from "../i18n";
import { createMenu, PopCap, PopItem, PopMenu } from "../ui/Popover";
import type { Box } from "../ui/place";
import { say, sayError } from "../ui/toast";
import { gotoSettingsPanel } from "../palette/goto";
import { connectorState } from "../connector/api";
import SendDialog from "../connector/SendDialog";
import { calendarAdd } from "../calendar/api";
import CalAddDialog from "../calendar/CalAddDialog";
import { CAL_DUR_MIN, CAL_REMIND_MIN, pickCalEvent, type CalDraft } from "../calendar/write";
import { IconMore } from "../pin/icons";
import { exportItems } from "./exportActions";
import ColorDots from "./ColorDots";

export default function NoteMenu(p: { path: string; body: () => string; title?: string; remind?: () => string; color?: string | null; inherited?: string | null; onColor?: (id: string) => void }) {
  const menu = createMenu();
  const label = t("notes.export.more", "Export a další volby");
  const [send, setSend] = createSignal<Box | null>(null);
  const [cal, setCal] = createSignal<{ draft: CalDraft; anchor: Box } | null>(null);
  // stav propojené aplikace (klíč, adresa, název) se zjišťuje až při otevření menu (jedno čtení Správce pověření)
  const [mono] = createResource(() => menu.open() || undefined, () => connectorState().catch(() => null));
  // „Do kalendáře“: událost z připomínky poznámky, jinak z prvního otevřeného bodu s ⏰ (počítá se jen při otevřeném menu)
  const calDraft = () => (p.remind ? pickCalEvent(p.path, p.title ?? "", p.body(), p.remind()) : null);
  // soubor .ics se otevře ve výchozím kalendáři (okno cizí aplikace) — jen na tenhle výslovný klik
  async function toIcs(d: CalDraft) {
    try {
      await calendarAdd({ title: d.title, startIso: d.startIso, allDay: d.allDay, durMin: CAL_DUR_MIN, remindMin: CAL_REMIND_MIN, uid: d.uid, target: "ics" });
    } catch (e) {
      sayError(e);
    }
  }
  // „Poslat do {název}“: bez adresy nebo klíče vede rovnou na pole v Nastavení, jinak otevře náhled (nic se neodešle bez Odeslat)
  async function toConnector(anchor: Box) {
    try {
      const st = await connectorState();
      if (!st.configured) {
        say(t("notes.connector.nourl", "Nejdřív nastav adresu propojené aplikace: Nastavení → Claude."));
        gotoSettingsPanel("f-connector.url");
        return;
      }
      if (!st.hasKey) {
        say(t("notes.connector.nokey", "Nejdřív ulož klíč propojené aplikace v Nastavení → Claude."));
        gotoSettingsPanel("f-connector.key");
        return;
      }
      setSend(anchor);
    } catch (e) {
      sayError(e);
    }
  }
  return (
    <>
      <button class="icon-btn" classList={{ on: menu.open() }} aria-haspopup="menu" aria-expanded={menu.open()} title={label} aria-label={label} onClick={menu.toggle}>
        <IconMore />
      </button>
      <Show when={menu.at()}>
        {(at) => (
          <PopMenu anchor={at().anchor} keyboard={at().keyboard} trigger={menu.trigger()} onClose={menu.close} align="end" label={label}>
            <PopCap>{t("notes.export.title", "Export")}</PopCap>
            <For each={exportItems(p.path, p.body)}>
              {(it) => <PopItem onClick={() => { menu.close(); it.run(); }}>{it.label}</PopItem>}
            </For>
            <PopCap>{mono()?.name.trim() || t("notes.connector.menu.cap", "Propojená aplikace")}</PopCap>
            <PopItem onClick={() => { const a = at().anchor; menu.close(); void toConnector(a); }}>{mono()?.name.trim() ? t("notes.connector.menu", "Poslat do {name}", { name: mono()!.name.trim() }) : t("notes.connector.menu.none", "Poslat do propojené aplikace")}</PopItem>
            <Show when={calDraft()}>
              {(d) => (
                <>
                  <PopCap>{t("notes.cal.cap", "Kalendář")}</PopCap>
                  <PopItem title={t("notes.cal.ics.title", "Otevře soubor ve výchozí aplikaci kalendáře — její okno se přesune do popředí.")} onClick={() => { menu.close(); void toIcs(d()); }}>{t("notes.cal.ics", "Do kalendáře (otevře soubor .ics)")}</PopItem>
                  <Show when={mono()?.hasKey && mono()?.configured}>
                    <PopItem onClick={() => { const a = at().anchor; menu.close(); setCal({ draft: d(), anchor: a }); }}>{mono()?.name.trim() ? t("notes.cal.conn", "Do kalendáře {name}…", { name: mono()!.name.trim() }) : t("notes.cal.conn.none", "Do kalendáře propojené aplikace…")}</PopItem>
                  </Show>
                </>
              )}
            </Show>
            <Show when={p.onColor}>
              {(pick) => (
                <>
                  <PopCap>{t("notes.color.title", "Barva poznámky")}</PopCap>
                  <ColorDots color={p.color} inherited={p.inherited} onPick={(id) => { menu.close(); pick()(id); }} />
                </>
              )}
            </Show>
          </PopMenu>
        )}
      </Show>
      <Show when={cal()}>
        {(c) => <CalAddDialog draft={c().draft} anchor={c().anchor} onClose={() => setCal(null)} />}
      </Show>
      <Show when={send()}>
        <SendDialog path={p.path} anchor={send()} onClose={() => setSend(null)} />
      </Show>
    </>
  );
}
