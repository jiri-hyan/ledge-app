// ⋯ okna lístku (Lístky 2.0): panel přes tělo lístku (`PopMenu fill`) se skupinami Vzhled · Barva · Štítky ·
// Okno · Schovat do… · Poznámka · Export · Splnit / Archivovat / Smazat. Nahoře křížek, který zavře (odepne)
// celý lístek — poznámka zůstane. Delší části (galerie stylů, Schovat do…, formáty exportu) se rozbalí na
// místě, žádné podstránky. Stav i zápisy drží `PinApp` (props), tady je jen vzhled a volby.
import { createSignal, For, Show } from "solid-js";
import type { NoteDetail, PinPrefs } from "../api";
import { t } from "../i18n";
import { IconX } from "../icons";
import { PopCap, PopItem, PopMenu, PopSep } from "../ui/Popover";
import { copyMarkdown, exportItems } from "../notes/exportActions";
import { lifeActions, type LifeAction } from "../notes/lifeactions";
import { actionText } from "../notes/lifelabels";
import { colorHex, NOTE_COLORS } from "../notes/meta";
import { hidePicks, validHide } from "./hide";
import { remindText } from "../notes/remind";
import StyleGallery, { colorName } from "./StyleGallery";
import type { PinStyleId } from "./styles";
import { TEXT_MIN } from "./model";

const hideName = (id: string): string =>
  ({
    hour: t("notes.pin.hide.hour", "Za hodinu"),
    evening: t("notes.pin.hide.evening", "Dnes v 18:00"),
    tomorrow: t("notes.pin.hide.tomorrow", "Zítra v 8:00"),
    monday: t("notes.pin.hide.monday", "V pondělí v 8:00"),
    week: t("notes.pin.hide.week", "Za týden"),
  })[id] ?? id;

export type PinMenuProps = {
  prefs: PinPrefs;
  note: NoteDetail | null;
  trigger: HTMLElement | undefined;
  /** živý náhled posuvníků (uloží se po puštění) */
  look: { bg: number; text: number } | null;
  setLook: (l: { bg: number; text: number } | null) => void;
  /** barva lístku: vlastní `color:` (id / hex), `null` = výchozí; `tagColor` = barva podle štítku */
  color: string | null;
  tagColor: string | null;
  tags: string[];
  frontTags: string[];
  body: () => string;
  onClose: () => void;
  patch: (x: Partial<PinPrefs>) => Promise<void>;
  setColor: (id: string) => void;
  addTag: (raw: string) => Promise<boolean>;
  removeTag: (tag: string) => void;
  lookAll: () => void;
  hide: (until: string) => void;
  wantThrough: () => void;
  edit: () => void;
  editInLedge: () => void;
  openExternal: () => void;
  unpin: () => void;
  life: (a: LifeAction) => void;
  settle: () => Promise<boolean>;
};

export default function PinMenu(p: PinMenuProps) {
  const [open, setOpen] = createSignal<"look" | "hide" | "export" | null>(null);
  const toggle = (k: "look" | "hide" | "export") => setOpen(open() === k ? null : k);
  const [tag, setTag] = createSignal("");
  const [when, setWhen] = createSignal("");
  const now = () => new Date();
  const run = (f: () => void) => () => {
    p.onClose();
    f();
  };
  async function submitTag() {
    if (await p.addTag(tag())) setTag("");
  }
  return (
    <PopMenu fill anchor={{ left: 0, top: 0, right: 0, bottom: 0 }} trigger={p.trigger} onClose={p.onClose} label={t("notes.pin.more", "Volby lístku")}>
      <div class="pin-menu-head">
        <b>{t("notes.pin.more", "Volby lístku")}</b>
        <button type="button" class="pin-ic" title={t("notes.pin.closeSticky", "Zavřít lístek (poznámka zůstane v Ledge)")} aria-label={t("notes.pin.closeSticky", "Zavřít lístek (poznámka zůstane v Ledge)")} onClick={p.unpin}><IconX /></button>
      </div>

      <PopCap>{t("notes.pin.color", "Barva")}</PopCap>
      <div class="pin-colors" role="radiogroup" aria-label={t("notes.pin.color", "Barva")}>
        <button type="button" class="pin-color pin-color-none" classList={{ on: !p.color }} role="radio" aria-checked={!p.color} title={p.tagColor ? t("notes.pin.colorByTag", "Podle štítku") : t("notes.pin.colorNone", "Výchozí")} style={p.tagColor ? { "--c": p.tagColor } : undefined} onClick={() => p.setColor("")} />
        <For each={NOTE_COLORS}>
          {(c) => <button type="button" class="pin-color" classList={{ on: colorHex(p.color) === c.hex }} role="radio" aria-checked={colorHex(p.color) === c.hex} title={colorName(c.id)} style={{ "--c": c.hex }} onClick={() => p.setColor(c.id)} />}
        </For>
      </div>

      <PopItem onClick={() => toggle("look")}>{open() === "look" ? "▾ " : "▸ "}{t("notes.pin.lookTitle", "Vzhled a průhlednost")}</PopItem>
      <Show when={open() === "look"}>
        <StyleGallery value={p.prefs.style} bg={p.look?.bg ?? p.prefs.bg} text={p.look?.text ?? p.prefs.text} color={colorHex(p.color) ?? p.tagColor} onPick={(s: PinStyleId) => void p.patch({ style: s })} />
        <label class="pin-range">
          <span>{t("notes.pin.bg", "Pozadí")} <b>{p.look?.bg ?? p.prefs.bg} %</b></span>
          <input type="range" min="0" max="100" step="5" value={p.prefs.bg} onInput={(e) => p.setLook({ bg: +e.currentTarget.value, text: p.look?.text ?? p.prefs.text })} onChange={(e) => p.patch({ bg: +e.currentTarget.value }).then(() => p.setLook(null))} />
        </label>
        <label class="pin-range">
          <span>{t("notes.pin.text", "Text")} <b>{p.look?.text ?? p.prefs.text} %</b></span>
          <input type="range" min={TEXT_MIN} max="100" step="5" value={p.prefs.text} onInput={(e) => p.setLook({ bg: p.look?.bg ?? p.prefs.bg, text: +e.currentTarget.value })} onChange={(e) => p.patch({ text: +e.currentTarget.value }).then(() => p.setLook(null))} />
        </label>
        <PopItem onClick={p.lookAll} title={t("notes.pin.lookAllTitle", "Styl, pozadí a text dostanou všechny lístky i každý nový")}>{t("notes.pin.lookAll", "Použít na všechny lístky")}</PopItem>
      </Show>

      <PopCap>{t("notes.pin.tags", "Štítky")}</PopCap>
      <div class="pin-tags-edit">
        <For each={p.tags}>
          {(tg) => (
            <span class="pin-tag">
              #{tg}
              <Show when={p.frontTags.includes(tg)} fallback={<span class="pin-tag-note" title={t("notes.pin.tagInText", "Štítek je v textu poznámky — smaž ho tam")}>·</span>}>
                <button type="button" class="pin-tag-x" title={t("notes.pin.tagRemove", "Odebrat štítek")} aria-label={t("notes.pin.tagRemove", "Odebrat štítek")} onClick={() => p.removeTag(tg)}><IconX /></button>
              </Show>
            </span>
          )}
        </For>
        <input
          class="pin-tag-in"
          value={tag()}
          maxLength={41}
          placeholder={t("notes.pin.tagAdd", "+ štítek")}
          aria-label={t("notes.pin.tagAddLabel", "Přidat štítek")}
          onInput={(e) => setTag(e.currentTarget.value)}
          onKeyDown={(e) => {
            if (e.key === "Enter" || e.key === "," || e.key === " ") {
              e.preventDefault();
              void submitTag();
            }
          }}
          onBlur={() => tag().trim() && void submitTag()}
        />
      </div>
      <PopSep />

      <PopItem checked={p.prefs.onTop} onClick={() => p.patch({ onTop: !p.prefs.onTop })}>{t("notes.pin.onTop", "Vždy navrchu")}</PopItem>
      <PopItem checked={p.prefs.locked} onClick={() => p.patch({ locked: !p.prefs.locked })}>{t("notes.pin.lock", "Zamknout na místě")}</PopItem>
      <PopItem checked={false} onClick={p.wantThrough}>{t("notes.pin.through", "Průchozí — myš projde na stránku pod lístkem")}</PopItem>
      <PopItem onClick={run(() => void p.patch({ collapsed: true }))}>{t("notes.pin.collapse", "Sbalit na lištu (dvojklik na lištu)")}</PopItem>

      <PopItem onClick={() => toggle("hide")}>{open() === "hide" ? "▾ " : "▸ "}{t("notes.pin.hideTitle", "Schovat do…")}</PopItem>
      <Show when={open() === "hide"}>
        <div class="pin-hide">
          <For each={hidePicks(now())}>
            {(h) => <button type="button" class="pin-btn" title={remindText(h.value, now())} onClick={() => p.hide(h.value)}>{hideName(h.id)}</button>}
          </For>
          <div class="pin-hide-own">
            <input type="datetime-local" class="pin-add" value={when()} aria-label={t("notes.pin.hideWhen", "Vlastní čas")} onInput={(e) => setWhen(e.currentTarget.value)} />
            <button type="button" class="pin-btn pri" disabled={!validHide(when(), now())} onClick={() => { const v = validHide(when(), now()); if (v) p.hide(v); }}>{t("notes.pin.hideGo", "Schovat")}</button>
          </div>
          <small class="pin-hide-hint">{t("notes.pin.hideHint", "Lístek zmizí a v ten čas se sám vrátí na stejné místo. Dřív ho vrátíš špendlíkem u poznámky.")}</small>
        </div>
      </Show>
      <PopSep />

      <PopItem onClick={run(p.edit)}>{t("notes.pin.editText", "Upravit text")}</PopItem>
      <PopItem onClick={run(p.editInLedge)}>{t("notes.pin.editInLedge", "Upravit v Ledge")}</PopItem>
      <PopItem onClick={run(p.openExternal)}>{t("notes.pin.openExternal", "Otevřít v externím editoru")}</PopItem>
      <PopSep />

      <Show when={p.note}>
        {(n) => (
          <>
            <PopItem onClick={run(() => void copyMarkdown(p.body()))}>{t("notes.export.copy", "Kopírovat jako Markdown")}</PopItem>
            <PopItem onClick={() => toggle("export")}>{open() === "export" ? "▾ " : "▸ "}{t("notes.pin.export", "Exportovat…")}</PopItem>
            <Show when={open() === "export"}>
              <For each={exportItems(n().path, p.body).filter((it) => !it.copy)}>
                {(it) => <PopItem onClick={run(() => void p.settle().then((ok) => ok && it.run()))}>{it.label}</PopItem>}
              </For>
            </Show>
            <PopSep />
          </>
        )}
      </Show>
      <For each={lifeActions(!!p.note?.done, false)}>
        {(a) => <PopItem danger={a === "delete"} onClick={() => p.life(a)}>{actionText(a)}</PopItem>}
      </For>
    </PopMenu>
  );
}
