// Starší rychlé odkazy (`personal.links`), které Rust NEdokázal převést na akce (`linkmigrate.rs`:
// třeba relativní cesta nebo plný seznam akcí). Ukáže se jen, když nějaké jsou — oprav cíl (celá cesta
// C:\…) a po Uložit se převedou samy; nebo odkaz odeber. Nové odkazy = akce (sekce Akce a odkazy).
import { createSignal, Show } from "solid-js";
import { api, type PickKind } from "../../api";
import { t } from "../../i18n";
import { sayError } from "../../ui/toast";
import { offerUndo, useSettings } from "../ctx";
import { Text } from "../fields/controls";
import { FieldErr, Row, Section } from "../fields/Row";
import { SortList } from "../fields/SortList";
import { moveItem } from "../model";

export default function LegacyLinks() {
  const c = useSettings();
  const links = (): { name: string; target: string }[] => c.cfg.personal?.links ?? [];
  const at = (i: number, f: string) => `personal.links.${i}.${f}`;
  const [picking, setPicking] = createSignal(false);
  async function pick(i: number, kind: PickKind) {
    setPicking(true);
    try {
      const got = await api.pickPath(kind, t("settings.links.pick", "Cíl odkazu"));
      if (!got) return;
      c.set(at(i, "target"), got);
      if (!links()[i].name.trim()) c.set(at(i, "name"), got.replace(/[\\/]+$/, "").split(/[\\/]/).pop()!.replace(/\.exe$/i, ""));
    } catch (e) {
      sayError(e);
    } finally {
      setPicking(false);
    }
  }
  function remove(i: number) {
    const before = structuredClone(links());
    c.update("personal.links", (all: unknown[]) => all.filter((_, j) => j !== i));
    offerUndo(t("settings.links.removed", "Odkaz „{n}“ odebrán", { n: before[i].name || before[i].target }), () => c.set("personal.links", before));
  }
  return (
    <Show when={links().length}>
      <Section title={t("settings.links.legacy", "Starší odkazy, které se nepřevedly")} desc={t("settings.links.legacy.desc", "Tyhle odkazy nejdou uložit jako akce (třeba cesta bez písmene disku). Oprav cíl na celou cestu (C:\\…) a ulož — převedou se samy. Karta Odkazy je ukazuje dál.")}>
        <Row id="personal.links" label={t("settings.links.legacy.list", "Odkazy")} stack nested>
          <SortList items={links()} label={t("settings.links.legacy.list", "Odkazy")} onMove={(a, b) => c.set("personal.links", moveItem(links(), a, b))}>
            {(_l, i, handle) => (
              <div class="st-item-wrap">
                <div class="st-item">
                  {handle()}
                  <Text path={at(i(), "name")} label={t("settings.links.name", "Název")} placeholder={t("settings.links.name", "Název")} maxLength={60} />
                  <Text path={at(i(), "target")} label={t("settings.links.target", "Cíl")} placeholder={t("settings.links.target.ph", "https://… · C:\\cesta · appka.exe")} mono grow />
                  <span class="st-pickers">
                    <button class="mini" disabled={picking()} title={t("settings.links.pick.folder", "Vybrat složku")} onClick={() => pick(i(), "folder")}>{t("settings.links.folder", "Složka")}</button>
                    <button class="mini" disabled={picking()} title={t("settings.links.pick.file", "Vybrat soubor nebo aplikaci")} onClick={() => pick(i(), "file")}>{t("settings.links.file", "Soubor")}</button>
                  </span>
                  <button class="mini" onClick={() => remove(i())}>{t("settings.ui.remove", "Odebrat")}</button>
                </div>
                <FieldErr path={at(i(), "name")} />
                <FieldErr path={at(i(), "target")} />
              </div>
            )}
          </SortList>
        </Row>
      </Section>
    </Show>
  );
}
