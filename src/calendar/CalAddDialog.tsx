// „Do kalendáře {název}“ z ⋯ poznámky: náhled události → Odeslat (jako u úkolů nic nejde ven před kliknutím).
// Plovoucí panel (`ui/Popover`), fokus nekrade. Soubor `.ics` náhled nemá (zůstává u počítače), viz `NoteMenu`.
import { createSignal, Show } from "solid-js";
import { t } from "../i18n";
import { connectorState } from "../connector/api";
import { parseLocal, remindText } from "../notes/remind";
import Popover from "../ui/Popover";
import type { Box } from "../ui/place";
import { say, sayError } from "../ui/toast";
import { calendarAdd } from "./api";
import { CAL_DUR_MIN, CAL_REMIND_MIN, type CalDraft } from "./write";
import "../connector/connector.css";

export const whenText = (d: CalDraft, now = new Date()): string => (d.allDay ? d.startIso : parseLocal(d.startIso) ? remindText(d.startIso, now) : d.startIso);

export default function CalAddDialog(p: { draft: CalDraft; anchor: Box | null; onClose: () => void }) {
  const [busy, setBusy] = createSignal(false);
  const [host, setHost] = createSignal("");
  const [name, setName] = createSignal("");
  const calTitle = () => (name().trim() ? t("notes.cal.title", "Do kalendáře {name}", { name: name().trim() }) : t("notes.cal.title.none", "Do kalendáře propojené aplikace"));
  void connectorState().then((s) => { setHost(s.host); setName(s.name); }).catch(() => {});

  async function send() {
    setBusy(true);
    try {
      const r = await calendarAdd({ title: p.draft.title, startIso: p.draft.startIso, allDay: p.draft.allDay, durMin: CAL_DUR_MIN, remindMin: CAL_REMIND_MIN, uid: p.draft.uid, target: "connector" });
      say(r.duplicate ? (name().trim() ? t("notes.cal.dup", "Událost už v kalendáři {name} je.", { name: name().trim() }) : t("notes.cal.dup.none", "Událost už v kalendáři propojené aplikace je.")) : (name().trim() ? t("notes.cal.sent", "Událost je v kalendáři {name}.", { name: name().trim() }) : t("notes.cal.sent.none", "Událost je v kalendáři propojené aplikace.")));
      p.onClose();
    } catch (e) {
      sayError(e);
      setBusy(false);
    }
  }

  return (
    <Popover anchor={() => p.anchor} align="end" label={calTitle()} onDismiss={p.onClose}>
      <div class="mono-dlg">
        <header>
          <strong>{calTitle()}</strong>
          <Show when={host()}>{(h) => <small>{t("notes.connector.to", "do {host}", { host: h() })}</small>}</Show>
        </header>
        <p class="mono-title">{p.draft.title}</p>
        <p class="mono-note">
          {whenText(p.draft)}
          {p.draft.allDay ? " · " + t("notes.cal.allday", "celý den") : " · " + t("notes.cal.span", "{n} min", { n: CAL_DUR_MIN })}
        </p>
        <p class="mono-note">{t("notes.cal.note", "Pošle se jen název a čas.")}</p>
        <footer>
          <button class="mini" onClick={p.onClose}>{t("ui.confirm.cancel", "Zrušit")}</button>
          <button class="mini ok" disabled={busy()} onClick={() => void send()}>{t("notes.cal.send", "Odeslat")}</button>
        </footer>
      </div>
    </Popover>
  );
}
