// Nastavení → Šuplík a karty → sekce „Na ploše“: widgety na ploše (seznam s volbami, připnout další, schovat
// vše), profily rozmístění (uložit / obnovit jedním klikem / smazat) a výchozí chování nových widgetů
// (config `desk.*` — ukládá společná lišta). Seznam a profily jsou v `desk.json` jen pro tento PC, mění se
// hned (štítek „Platí hned“) a stav bere ze snapshotu (`Snapshot.desk`).
import { createSignal, For, Show } from "solid-js";
import { t } from "../i18n";
import type { DeskLayer, DeskWidget, Snapshot } from "../types";
import { LiveBadge, Row, Section } from "../settings/fields/Row";
import { Seg, Slider, Toggle } from "../settings/fields/controls";
import { fieldLabel } from "../settings/labels";
import { tx } from "../layout/text";
import { sayError, say } from "../ui/toast";
import { deskApi, type DeskPatch } from "./api";
import { BUILTIN_KINDS, cleanProfileName, KIND_HINT, LAYERS, OPACITY_MIN, widgetOf } from "./model";
import { deskKindName, pinKind, unpinWidget } from "./pin";
import "./settings.css";

const layerOpt = (l: (typeof LAYERS)[number]) => ({ v: l.v, label: tx(l.name), hint: tx(l.hint) });

function Item(p: { w: DeskWidget }) {
  const set = (x: DeskPatch) => void deskApi.set(x, p.w.id).catch(sayError);
  return (
    <div class="dk-set-item">
      <div class="dk-set-head">
        <span class="grow">{deskKindName(p.w.kind)}</span>
        <button class="mini" onClick={() => void unpinWidget(p.w.id, p.w.kind)}>{t("settings.desk.unpin", "Odepnout")}</button>
      </div>
      <div class="dk-set-opts">
        <Seg value={p.w.layer} onChange={(v: DeskLayer) => set({ layer: v })} options={LAYERS.map(layerOpt)} label={t("settings.desk.layer", "Vrstva")} />
        <label><Toggle value={p.w.locked} onChange={(v) => set({ locked: v })} label={t("settings.desk.lock", "Zamknout")} />{t("settings.desk.lock", "Zamknout")}</label>
        <label title={t("settings.desk.throughHint", "Klik projde na okno pod widgetem — vypneš to tady nebo v Ctrl+K.")}>
          <Toggle value={p.w.through} onChange={(v) => set({ through: v })} label={t("settings.desk.through", "Průchozí")} />
          {t("settings.desk.through", "Průchozí")}
        </label>
        <label><Toggle value={p.w.hideInGame} onChange={(v) => set({ hideInGame: v })} label={t("settings.desk.hideInGame", "Schovat ve hře")} />{t("settings.desk.hideInGame", "Schovat ve hře")}</label>
        <label><Toggle value={p.w.compact} onChange={(v) => set({ compact: v })} label={t("settings.desk.compact", "Kompaktní")} />{t("settings.desk.compact", "Kompaktní")}</label>
        <Slider value={p.w.opacity} onChange={(v) => set({ opacity: v })} min={OPACITY_MIN} max={100} step={5} unit="%" label={t("settings.desk.opacity", "Průhlednost")} />
      </div>
    </div>
  );
}

function Profiles(p: { s: Snapshot }) {
  const v = () => p.s.desk;
  const [name, setName] = createSignal("");
  const save = async () => {
    const n = cleanProfileName(name());
    if (!n) return say(t("settings.desk.profileNoName", "Napiš jméno profilu, třeba Práce nebo Hraní."), true);
    try {
      await deskApi.profileSave(n);
      setName("");
      say(t("settings.desk.profileSaved", "Rozmístění uložené jako „{name}“.", { name: n }));
    } catch (e) {
      sayError(e);
    }
  };
  return (
    <Row id="desk.profiles" label={t("settings.desk.profiles", "Profily rozmístění")} hint={t("settings.desk.profilesHint", "Ulož, kde widgety jsou (třeba Práce a Hraní), a obnov jedním klikem. Poloha platí jen pro tento počítač.")} stack live>
      <div class="dk-set-prof">
        <input type="text" maxLength={40} value={name()} placeholder={t("settings.desk.profilePh", "Jméno, např. Práce")} aria-label={t("settings.desk.profileName", "Jméno profilu")} onInput={(e) => setName(e.currentTarget.value)} onKeyDown={(e) => e.key === "Enter" && void save()} />
        <button class="mini ok" disabled={!v()?.widgets.length} onClick={() => void save()}>{t("settings.desk.profileSave", "Uložit rozmístění")}</button>
      </div>
      <For each={v()?.profiles ?? []}>
        {(n) => (
          <div class="dk-set-prof">
            <span class="grow">{n}</span>
            <Show when={v()?.active === n}><span class="dk-set-active">{t("settings.desk.profileActive", "naposledy použitý")}</span></Show>
            <button class="mini" onClick={() => void deskApi.profileApply(n).catch(sayError)}>{t("settings.desk.profileApply", "Obnovit")}</button>
            <button class="mini" onClick={() => void deskApi.profileDelete(n).catch(sayError)}>{t("settings.desk.profileDelete", "Smazat")}</button>
          </div>
        )}
      </For>
    </Row>
  );
}

export default function DeskSettings(p: { s: Snapshot }) {
  const v = () => p.s.desk;
  const free = () => BUILTIN_KINDS.filter((k) => !widgetOf(v(), k) && ((k !== "limits" && k !== "waiting") || p.s.claudeEnabled));
  return (
    <Section id="desk" title={t("settings.desk.title", "Na ploše")} desc={t("settings.desk.desc", "Karty Ledge jako malá okna na ploše nebo na druhém monitoru — žijí i se zavřeným šuplíkem. Připneš je i přes ⋯ na kartě → Připnout na plochu.")}>
      <Row id="desk.widgets" label={t("settings.desk.widgets", "Widgety na ploše")} hint={t("settings.desk.widgetsHint", "Nejvýš {n} — každý widget je samostatné okno a bere trochu paměti.", { n: v()?.max ?? 6 })} stack live>
        <div class="dk-set-list">
          <For each={v()?.widgets ?? []} fallback={<div class="empty">{t("settings.desk.none", "Zatím žádný widget na ploše.")}</div>}>
            {(w) => <Item w={w} />}
          </For>
          <Show when={free().length && (v()?.widgets.length ?? 0) < (v()?.max ?? 6)}>
            <div class="dk-set-add">
              <For each={free()}>
                {(k) => <button class="mini" title={tx(KIND_HINT[k])} onClick={() => void pinKind(k)}>{t("settings.desk.add", "+ {name}", { name: deskKindName(k) })}</button>}
              </For>
            </div>
          </Show>
          <Show when={v()?.widgets.length}>
            <div class="dk-set-add">
              <LiveBadge />
              <button class="mini" onClick={() => void deskApi.hideAll(!v()?.hiddenAll).catch(sayError)}>
                {v()?.hiddenAll ? t("settings.desk.showAll", "Ukázat všechny") : t("settings.desk.hideAll", "Schovat všechny")}
              </button>
            </div>
          </Show>
        </div>
      </Row>
      <Profiles s={p.s} />
      <Row id="desk.layer"><Seg path="desk.layer" options={LAYERS.map(layerOpt)} label={fieldLabel("desk.layer")} /></Row>
      <Row id="desk.hideInGame"><Toggle path="desk.hideInGame" label={fieldLabel("desk.hideInGame")} /></Row>
      <Row id="desk.opacity"><Slider path="desk.opacity" min={OPACITY_MIN} max={100} step={5} unit="%" label={fieldLabel("desk.opacity")} /></Row>
      <Row id="desk.snap"><Toggle path="desk.snap" label={fieldLabel("desk.snap")} /></Row>
    </Section>
  );
}
