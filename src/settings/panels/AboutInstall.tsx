// „Nainstalovat aktualizaci“ v O aplikaci: totéž, co tlačítko v hlavičce šuplíku (`api.updateInstall`, Rust
// `updater.rs`) — bez nové logiky. Běží-li servery, Rust vrátí jejich seznam a chce druhé, výslovné potvrzení.
import { createSignal, Show } from "solid-js";
import { api, type UpdateView } from "../../api";
import { t, tp } from "../../i18n";
import { say, sayError } from "../../ui/toast";

export default function InstallNow(p: { u: UpdateView }) {
  const [ask, setAsk] = createSignal<string[] | null>(null);
  const [busy, setBusy] = createSignal(false);
  async function go(confirmed: string[]) {
    setBusy(true);
    try {
      const out = await api.updateInstall(confirmed);
      if (out?.outcome === "confirm") setAsk(out.running);
      else if (out?.outcome === "started") {
        setAsk(null);
        say(t("cards.update.installing", "Instaluji — Ledge se za chvíli spustí znovu"));
      }
    } catch (e) {
      sayError(e);
    } finally {
      setBusy(false);
    }
  }
  const off = () => busy() || p.u.installing;
  return (
    <Show when={p.u.state === "dostupna"}>
      <Show
        when={ask()}
        fallback={
          <button class="mini ok" disabled={off()} onClick={() => void go([])}>
            {t("settings.about.install", "Nainstalovat verzi {v}", { v: p.u.state === "dostupna" ? p.u.verze : "" })}
          </button>
        }
      >
        {(running) => (
          <span class="st-inline">
            <span>
              {tp("settings.about.rb.stops", running().length, ["Zastaví se {n} běžící server: {list}.", "Zastaví se {n} běžící servery: {list}.", "Zastaví se {n} běžících serverů: {list}."], { list: running().join(", ") })}
            </span>
            <button class="mini" disabled={busy()} onClick={() => setAsk(null)}>{t("settings.about.rb.cancel", "Zpět")}</button>
            <button class="mini" disabled={off()} onClick={() => void go(running())}>{t("settings.about.install.yes", "Ano, nainstalovat")}</button>
          </span>
        )}
      </Show>
    </Show>
  );
}
