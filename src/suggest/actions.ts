// Akce chipů pruhu návrhů (s vedlejším účinkem — čistá logika je v `model.ts`): uložit adresu jako odkaz
// (akce typu web v configu), adresu do poznámky a poznámku ze schůzky. Vše na výslovný klik.
import { createSignal } from "solid-js";
import { api } from "../api";
import { slugId } from "../actions/model";
import { t } from "../i18n";
import { say, sayError } from "../ui/toast";
import { dismiss, hostOf, meetingNoteText, type Dismissed, type Meeting } from "./model";

/** Zavřené návrhy (klíč → do kdy), jen v paměti — po restartu Ledge se zeptá znovu. */
const [dismissed, setDismissed] = createSignal<Dismissed>(new Map());
export { dismissed };

export const closeSuggestion = (key: string) => setDismissed((d) => dismiss(d, key, Date.now()));

/** Nejvíc akcí v configu (Rust `actions::MAX_ACTIONS`). */
const MAX_ACTIONS = 64;

/** Uloží adresu jako odkaz (akce `url` pro obě záložky). Rozbitý config neukládá — přepsal by uživatelův. */
export async function saveLink(url: string, cfgError: string | null): Promise<boolean> {
  if (cfgError) {
    say(t("cards.suggest.cfgError", "Nastavení obsahuje chybu — odkaz teď přidáš jen v Nastavení."), true);
    return false;
  }
  try {
    const cfg = await api.getConfig();
    if (!cfg) throw new Error(t("cards.suggest.noConfig", "Nastavení se nepodařilo načíst."));
    const list: { id: string }[] = cfg.actions ?? [];
    if (list.length >= MAX_ACTIONS) throw new Error(t("cards.suggest.tooMany", "Víc než 64 akcí a odkazů nejde — nějaký nejdřív smaž."));
    const label = (hostOf(url) || url).slice(0, 40);
    const id = slugId(label, list.map((a) => a.id));
    await api.saveConfig({ ...cfg, actions: [...list, { id, label, kind: "url", target: url, showOn: "both" }] });
    say(t("cards.suggest.linkSaved", "Odkaz „{label}“ uložen.", { label }), false);
    return true;
  } catch (e) {
    sayError(e);
    return false;
  }
}

/** Uloží adresu jako novou obecnou poznámku. */
export async function urlToNote(url: string): Promise<boolean> {
  try {
    await api.noteCreate({ text: url, project: null, remind: null, images: [], kind: null });
    say(t("cards.suggest.noteSaved", "Uloženo do poznámky."), false);
    return true;
  } catch (e) {
    sayError(e);
    return false;
  }
}

/** Nová poznámka ze schůzky (titulek „{název} — {datum}“, kategorie výchozí = Poznámka); vrací cestu, aby ji pruh otevřel v editoru. */
export async function meetingToNote(m: Pick<Meeting, "title" | "start">): Promise<string | false> {
  try {
    return (await api.noteCreate({ text: meetingNoteText(m), project: null, remind: null, images: [], kind: null })) ?? false;
  } catch (e) {
    sayError(e);
    return false;
  }
}
