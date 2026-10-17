// Most k Rustu pro rychlé akce (`commands/actions.rs`) a cache ikon podle `iconKey` (vzor `logos.ts`):
// snapshot nese jen klíč, data URL se vyžádá jednou na klíč. Spuštění s potvrzením a hláškou je
// `runAction` — používá ho karta i paleta.
import { createSignal } from "solid-js";
import { invoke } from "@tauri-apps/api/core";
import { inTauri } from "../api";
import type { ActionCfg, ActionCheck, ActionOutcome, ActionView } from "../types";
import { errorText, say } from "../ui/toast";
import { t } from "../i18n";

const call = <T>(cmd: string, args?: Record<string, unknown>): Promise<T | undefined> => (inTauri ? invoke<T>(cmd, args) : Promise.resolve(undefined));

export const actionsApi = {
  run: (id: string, target: string, confirmed: string | null) => call<ActionOutcome>("run_action", { id, target, confirmed }),
  icon: (id: string) => call<string | null>("action_icon", { id }),
  iconRefresh: (id: string) => call("action_icon_refresh", { id }),
  /** výslovný souhlas se spouštěním příkazu bez ptaní — jen pro uloženou akci v přesně tomhle tvaru */
  consent: (action: ActionCfg) => call("action_consent", { action }),
  check: (action: ActionCfg) => call<ActionCheck>("action_check", { action }),
  /** systémový dialog Otevřít; `null` = zrušeno */
  pickPath: (kind: "file" | "folder" | "exe" | "image") => call<string | null>("pick_path", { kind }),
};

const [iconUrls, setIconUrls] = createSignal<Record<string, string>>({});
const asked = new Set<string>();
/** Data URL ikony akce podle `iconKey` — z jádra jednou na změnu obsahu. */
export function actionIconSrc(a: Pick<ActionView, "id" | "iconKey">): string | undefined {
  const key = a.iconKey;
  if (!key) return undefined;
  if (!asked.has(key)) {
    asked.add(key);
    actionsApi.icon(a.id).then((url) => url && setIconUrls((m) => ({ ...m, [key]: url })), () => asked.delete(key));
  }
  return iconUrls()[key];
}

/** Akce, které právě spouští tohle UI (do odpovědi jádra) — dlaždice ukáže „běží“. */
const [busy, setBusy] = createSignal<ReadonlySet<string>>(new Set());
export const isBusy = (id: string) => busy().has(id);

const NL = "\n";

/** Poslední výsledek skrytého příkazu (karta ho ukáže i s výstupem; hláška je krátká). */
export type LastResult = { id: string; ok: boolean; title: string; output: string };
const [lastResult, setLastResult] = createSignal<LastResult | null>(null);
export { lastResult, setLastResult };

/** Spustí akci: potvrzení (`needsConfirm`) přes `ask`, pak jádro, výsledek do hlášky. Vrací true = hotovo.
 *  `run` = jiný příkaz jádra se stejným tokem (skripty: `actions/scripts.ts`); výchozí `run_action`. */
export async function runAction(
  a: ActionView,
  ask: (a: ActionView) => Promise<boolean>,
  run: (confirmed: string | null) => Promise<ActionOutcome | undefined> = (confirmed) => actionsApi.run(a.id, a.target, confirmed),
): Promise<boolean> {
  if (a.problem) {
    say(t("actions.run.problem", "{label}: {problem}", { label: a.label, problem: a.problem }), true);
    return false;
  }
  if (a.running || isBusy(a.id)) {
    say(t("actions.run.already", "{label} už běží.", { label: a.label }));
    return false;
  }
  if (a.needsConfirm && !(await ask(a))) return false;
  setBusy((s) => new Set(s).add(a.id));
  try {
    const out = await run(a.needsConfirm ? a.confirmText : null);
    if (out && !out.hide) {
      say(out.message);
      setLastResult(out.output ? { id: a.id, ok: true, title: out.message, output: out.output } : null);
    }
    return true;
  } catch (e) {
    const text = errorText(e);
    const [first, ...rest] = text.split(NL);
    say(first, true);
    setLastResult(rest.length ? { id: a.id, ok: false, title: first, output: rest.join(NL) } : null);
    return false;
  } finally {
    setBusy((s) => {
      const n = new Set(s);
      n.delete(a.id);
      return n;
    });
  }
}
