// Editor jedné rychlé akce (panel Nastavení → Rychlé akce): typ → průvodce polem, „Vybrat…“
// přes systémový dialog (`pick_path`), argumenty a složka u aplikace/příkazu, shell a okno u
// příkazu, velikost okna u okna s webem, ikona, skupina, záložka, potvrzení a výslovný souhlas s příkazem. Okamžitá kontrola
// tvaru (`draftError`) + ověření na disku v jádře (`action_check`, s prodlevou).
import { createEffect, createSignal, For, on, onCleanup, Show } from "solid-js";
import { createStore, unwrap } from "solid-js/store";
import type { ActionCfg, ActionKind, ActionView } from "../types";
import { actionsApi, runAction } from "./api";
import { cleanAction, draftError, kindForTarget, labelFrom, WEB_SIZE } from "./model";
import { ActionIcon, ConfirmAction, LastOutput, askConfirm } from "./ui";
import { say, sayError } from "../ui/toast";
import { t } from "../i18n";

type PickKind = "file" | "folder" | "exe" | "image";
// `ph` je funkce, ať se nápověda přeloží až při vykreslení
const KINDS: { kind: ActionKind; key: string; cs: string; ph: () => string; pick: PickKind | null }[] = [
  { kind: "url", key: "actions.kind.url", cs: "Web", ph: () => "https://…", pick: null },
  { kind: "web", key: "actions.kind.web", cs: "Okno s webem", ph: () => "https://music.youtube.com", pick: null },
  { kind: "folder", key: "actions.kind.folder", cs: "Složka", ph: () => t("actions.kind.folderPh", "C:\\Users\\…\\Dokumenty"), pick: "folder" },
  { kind: "file", key: "actions.kind.file", cs: "Soubor", ph: () => t("actions.kind.filePh", "C:\\…\\dokument.pdf"), pick: "file" },
  { kind: "app", key: "actions.kind.app", cs: "Aplikace", ph: () => t("actions.kind.appPh", "C:\\…\\aplikace.exe  nebo  notepad.exe"), pick: "exe" },
  { kind: "command", key: "actions.kind.command", cs: "Příkaz", ph: () => "npm run build", pick: null },
];
export const kindLabel = (k: ActionKind) => {
  const m = KINDS.find((x) => x.kind === k)!;
  return t(m.key, m.cs);
};

export default function ActionEditor(p: {
  initial: ActionCfg;
  isNew: boolean;
  groups: string[];
  /** uložená verze ve snapshotu (ikona, „Spustit teď“) */
  saved?: ActionView;
  /** akce je v configu uložená přesně v tvaru `initial` (jinak by „Spustit teď“ pustilo starou verzi) */
  runnable?: boolean;
  onSave: (a: ActionCfg, consent: boolean) => Promise<void>;
  onCancel: () => void;
}) {
  const [d, setD] = createStore<ActionCfg>(structuredClone(unwrap(p.initial)));
  const [consent, setConsent] = createSignal(false);
  const [warn, setWarn] = createSignal<string | null>(null);
  const [saving, setSaving] = createSignal(false);
  const meta = () => KINDS.find((k) => k.kind === d.kind)!;
  const runs = () => d.kind === "app" || d.kind === "command";
  const risky = () => d.kind === "command" || (d.kind === "app" && (d.args ?? []).some((x) => x.trim()));
  /** prázdný popisek se při uložení doplní z cíle — kontrola tedy s ním */
  const err = () => draftError({ ...d, label: d.label.trim() || labelFrom(d.target) });
  const dirty = () => JSON.stringify(cleanAction(d)) !== JSON.stringify(cleanAction(p.initial));
  const isWeb = () => d.kind === "url" || d.kind === "web";
  const iconMode = () => (!d.icon || d.icon === "auto" ? "auto" : d.icon === "monogram" ? "monogram" : "image");

  // ověření na disku v jádře (cesta existuje, typ sedí) — s prodlevou, starší odpovědi se zahodí
  let seq = 0;
  createEffect(
    on(
      () => JSON.stringify(cleanAction(d)),
      () => {
        const n = ++seq;
        setWarn(null);
        if (err() || !d.target.trim()) return;
        const timer = window.setTimeout(async () => {
          const r = await actionsApi.check(cleanAction({ ...unwrap(d), id: d.id || "novy" })).catch(() => undefined);
          if (n === seq) setWarn(r?.error ?? r?.warning ?? null);
        }, 350);
        onCleanup(() => clearTimeout(timer));
      },
    ),
  );

  async function pick(kind: PickKind, field: "target" | "cwd" | "icon") {
    try {
      const path = await actionsApi.pickPath(kind);
      if (!path) return;
      setD(field, path);
      if (field === "target" && !d.label.trim()) setD("label", labelFrom(path));
    } catch (e) {
      sayError(e);
    }
  }

  async function save() {
    if (err()) return;
    setSaving(true);
    try {
      await p.onSave(cleanAction({ ...unwrap(d), label: d.label.trim() || labelFrom(d.target) }), risky() && consent());
    } catch (e) {
      sayError(e);
    } finally {
      setSaving(false);
    }
  }

  const fieldErr = (f: string) => (err()?.field === f ? err() : null);
  const ErrLine = (q: { f: string }) => <Show when={fieldErr(q.f)}>{(e) => <small class="err">{t(e().key, e().cs)}</small>}</Show>;

  return (
    <div class="act-edit">
      <div class="act-field">
        <label>{t("actions.edit.kind", "Typ")}</label>
        <div class="ctl">
          {/* 6 typů = galerie dlaždic, ne přepínač (víc než 4 volby přetékají) */}
          <div class="act-kinds" role="radiogroup" aria-label={t("actions.edit.kind", "Typ")}>
            <For each={KINDS}>{(k) => <button class="act-kind" classList={{ on: d.kind === k.kind }} role="radio" aria-checked={d.kind === k.kind} onClick={() => setD("kind", k.kind)}>{t(k.key, k.cs)}</button>}</For>
          </div>
        </div>
      </div>
      <div class="act-field">
        <label>{isWeb() ? t("actions.edit.url", "Adresa") : d.kind === "command" ? t("actions.edit.command", "Příkaz") : t("actions.edit.path", "Cesta")}</label>
        <div class="ctl">
          <div class="line">
            <input type="text" value={d.target} placeholder={meta().ph()} spellcheck={false} onInput={(e) => {
              const v = e.currentTarget.value;
              setD("target", v);
              // cesta ve Webu / adresa ve Složce: typ se přepne sám (místo chyby)
              const k = kindForTarget(d.kind, v);
              if (k) setD("kind", k);
            }} />
            <Show when={meta().pick}>{(k) => <button class="mini" onClick={() => pick(k(), "target")}>{t("actions.edit.pick", "Vybrat…")}</button>}</Show>
          </div>
          <ErrLine f="target" />
          <Show when={!fieldErr("target") && warn()}><small class="warn">{warn()}</small></Show>
          <Show when={d.kind === "web"}><small>{t("actions.edit.webHint", "Otevře se v malém okně Ledge (přihlášení zůstane). Zavřením okna se paměť uvolní; odkazy do nového okna jdou do prohlížeče.")}</small></Show>
          <Show when={d.kind === "command"}><small>{t("actions.edit.commandHint", "Spustí se v cmd nebo PowerShellu na tvém PC — jako bys ho napsal do terminálu.")}</small></Show>
        </div>
      </div>
      <div class="act-field">
        <label>{t("actions.edit.label", "Popisek")}</label>
        <div class="ctl">
          <input type="text" value={d.label} placeholder={labelFrom(d.target) || t("actions.edit.labelPh", "Jak se zástupce jmenuje")} onInput={(e) => setD("label", e.currentTarget.value)} />
          <ErrLine f="label" />
        </div>
      </div>
      <Show when={runs()}>
        <div class="act-field">
          <label>{t("actions.edit.args", "Argumenty")}</label>
          <div class="ctl">
            <textarea value={(d.args ?? []).join("\n")} spellcheck={false} placeholder={t("actions.edit.argsPh", "každý argument na vlastní řádek")} onInput={(e) => setD("args", e.currentTarget.value.split("\n"))} />
            <ErrLine f="args" />
          </div>
        </div>
        <div class="act-field">
          <label>{t("actions.edit.cwd", "Pracovní složka")}</label>
          <div class="ctl">
            <div class="line">
              <input type="text" value={d.cwd ?? ""} placeholder={t("actions.edit.cwdPh", "výchozí: domovská složka")} spellcheck={false} onInput={(e) => setD("cwd", e.currentTarget.value)} />
              <button class="mini" onClick={() => pick("folder", "cwd")}>{t("actions.edit.pick", "Vybrat…")}</button>
            </div>
            <ErrLine f="cwd" />
          </div>
        </div>
      </Show>
      <Show when={d.kind === "web"}>
        <div class="act-field">
          <label>{t("actions.edit.size", "Velikost okna")}</label>
          <div class="ctl">
            <div class="line">
              <input type="number" min={WEB_SIZE.minW} max={WEB_SIZE.maxW} value={d.width ?? ""} placeholder={String(WEB_SIZE.w)} onInput={(e) => setD("width", e.currentTarget.value ? Number(e.currentTarget.value) : undefined)} />
              <small>×</small>
              <input type="number" min={WEB_SIZE.minH} max={WEB_SIZE.maxH} value={d.height ?? ""} placeholder={String(WEB_SIZE.h)} onInput={(e) => setD("height", e.currentTarget.value ? Number(e.currentTarget.value) : undefined)} />
              <small>{t("actions.edit.sizeUnit", "px (prázdné = 480 × 720)")}</small>
            </div>
            <ErrLine f="size" />
            <small>{t("actions.edit.sizeHint", "Posunuté nebo zvětšené okno si Ledge pamatuje; změna velikosti tady ho vrátí na výchozí místo.")}</small>
          </div>
        </div>
      </Show>
      <Show when={d.kind === "command"}>
        <div class="act-field">
          <label>{t("actions.edit.shell", "Kde a jak")}</label>
          <div class="ctl">
            <div class="line">
              <div class="seg" role="group">
                <button classList={{ on: d.shell !== "powershell" }} onClick={() => setD("shell", "cmd")}>cmd</button>
                <button classList={{ on: d.shell === "powershell" }} onClick={() => setD("shell", "powershell")}>PowerShell</button>
              </div>
              <div class="seg" role="group">
                <button classList={{ on: !d.terminal }} onClick={() => setD("terminal", false)}>{t("actions.edit.hidden", "Bez okna")}</button>
                <button classList={{ on: !!d.terminal }} onClick={() => setD("terminal", true)}>{t("actions.edit.terminal", "V terminálu")}</button>
              </div>
            </div>
            <Show when={!d.terminal} fallback={<small>{t("actions.edit.terminalHint", "Otevře se okno terminálu a zůstane otevřené.")}</small>}>
              <div class="line">
                <small>{t("actions.edit.timeout", "Výstup přijde do hlášky; po")}</small>
                <input type="number" min="1" max="3600" value={d.timeoutS ?? 60} onInput={(e) => setD("timeoutS", Math.min(3600, Math.max(1, Number(e.currentTarget.value) || 60)))} />
                <small>{t("actions.edit.timeoutTail", "s se příkaz ukončí.")}</small>
              </div>
            </Show>
          </div>
        </div>
      </Show>
      <div class="act-field">
        <label>{t("actions.edit.icon", "Ikona")}</label>
        <div class="ctl">
          <div class="line">
            <Show when={p.saved}>{(v) => <ActionIcon a={v()} size={26} />}</Show>
            <div class="seg" role="group">
              <button classList={{ on: iconMode() === "auto" }} onClick={() => setD("icon", "auto")}>{t("actions.edit.iconAuto", "Automaticky")}</button>
              <button classList={{ on: iconMode() === "monogram" }} onClick={() => setD("icon", "monogram")}>{t("actions.edit.iconMono", "Písmena")}</button>
              <button classList={{ on: iconMode() === "image" }} onClick={() => pick("image", "icon")}>{t("actions.edit.iconImage", "Obrázek…")}</button>
            </div>
            <Show when={p.saved && isWeb() && iconMode() === "auto"}>
              <button class="mini" onClick={() => actionsApi.iconRefresh(d.id).then(() => say(t("actions.edit.iconRefreshing", "Ikona webu se stáhne znovu…")))}>{t("actions.edit.iconRefresh", "Obnovit")}</button>
            </Show>
          </div>
          <Show when={iconMode() === "image"}><small class="code">{d.icon}</small></Show>
          <ErrLine f="icon" />
          <Show when={iconMode() === "auto"}>
            <small>{isWeb() ? t("actions.edit.iconFav", "Ikona webu se stáhne jednou a uloží; jindy Ledge na síť nesahá.") : d.kind === "command" ? t("actions.edit.iconCmd", "U příkazu písmena v barvě.") : t("actions.edit.iconFile", "Ikona, jakou má soubor ve Windows.")}</small>
          </Show>
        </div>
      </div>
      <div class="act-field">
        <label>{t("actions.edit.where", "Zobrazit")}</label>
        <div class="ctl">
          <div class="line">
            <div class="seg" role="group">
              <button classList={{ on: (d.showOn ?? "both") === "both" }} onClick={() => setD("showOn", "both")}>{t("actions.edit.both", "Na záložkách Práce a Osobní")}</button>
              <button classList={{ on: d.showOn === "work" }} onClick={() => setD("showOn", "work")}>{t("actions.edit.work", "Práce")}</button>
              <button classList={{ on: d.showOn === "personal" }} onClick={() => setD("showOn", "personal")}>{t("actions.edit.personal", "Osobní")}</button>
            </div>
            <input type="text" list="act-groups" value={d.group ?? ""} placeholder={t("actions.edit.group", "skupina (nepovinné)")} onInput={(e) => setD("group", e.currentTarget.value)} />
            <datalist id="act-groups"><For each={p.groups}>{(g) => <option value={g} />}</For></datalist>
          </div>
        </div>
      </div>
      <div class="act-field">
        <label>{t("actions.edit.safety", "Potvrzení")}</label>
        <div class="ctl">
          <label class="act-check">
            <input type="checkbox" checked={!!d.confirm} onChange={(e) => setD("confirm", e.currentTarget.checked)} />
            <span>{t("actions.edit.confirm", "Před každým spuštěním se zeptat")}</span>
          </label>
          <Show when={risky() && !d.confirm}>
            <label class="act-check">
              <input type="checkbox" checked={consent()} onChange={(e) => setConsent(e.currentTarget.checked)} />
              <span>{t("actions.edit.consent", "Povoluji spouštět tenhle příkaz bez ptaní. Bez souhlasu se Ledge zeptá před každým spuštěním; souhlas propadne, když se příkaz změní (i synchronizací nebo importem).")}</span>
            </label>
          </Show>
        </div>
      </div>
      <div class="act-actions">
        <span class="msg">{p.isNew ? "" : dirty() ? t("actions.edit.dirty", "Neuložené změny") : ""}</span>
        <Show when={p.isNew ? undefined : p.saved}>
          {(v) => (
            <button class="mini" disabled={dirty() || p.runnable === false} title={dirty() || p.runnable === false ? t("actions.edit.saveFirst", "Nejdřív ulož") : v().target} onClick={() => runAction(v(), askConfirm("panel"))}>
              {t("actions.edit.runNow", "Spustit teď")}
            </button>
          )}
        </Show>
        <button class="btn" onClick={p.onCancel}>{t("actions.edit.cancel", "Zrušit")}</button>
        <button class="btn pri" disabled={!!err() || saving()} onClick={save}>{p.isNew ? t("actions.edit.add", "Přidat") : t("actions.edit.apply", "Hotovo")}</button>
      </div>
      <ConfirmAction where="panel" />
      <LastOutput ids={p.saved ? [p.saved.id] : []} />
    </div>
  );
}
