// Prázdný stav limitů v kartě Claude: proč data nejsou (`layout/hero.ts` `limitsWhy`) a zapnutí dalšího zdroje
// jedním klikem — aplikace Claude (`claude.limitsDesktop`) nebo dotaz na Anthropic (`claude.limitsFetch`).
// Logika nabídek: `sources.ts`. Rust: `limitsrc.rs`, `limitsfetch.rs`.
import { For, Show, createSignal } from "solid-js";
import type { LimitsView } from "../api";
import { api } from "../api";
import { t } from "../i18n";
import { limitsWhy } from "../layout/hero";
import { tx } from "../layout/text";
import { sayError } from "../ui/toast";
import { offers, withOffer, type Offer } from "./sources";

async function enable(o: Offer, busy: (v: boolean) => void) {
  busy(true);
  try {
    const cfg = await api.getConfig();
    if (cfg) await api.saveConfig(withOffer(cfg, o));
  } catch (e) {
    sayError(e);
  } finally {
    busy(false);
  }
}

export default function LimitsSources(p: { l: LimitsView }) {
  const [busy, setBusy] = createSignal(false);
  const label = (o: Offer) =>
    o === "desktop"
      ? t("sessions.limits.src.desktop", "Číst limity z aplikace Claude")
      : t("sessions.limits.src.fetch", "Ptát se Anthropicu na limity (1× za 10 min)");
  const title = (o: Offer) =>
    o === "desktop"
      ? t("sessions.limits.src.desktop.tip", "Ledge bude číst soubor, do kterého si aplikace Claude ukládá tvé limity (jen procenta, čas obnovy z něj není). Nic se neposílá.")
      : t("sessions.limits.src.fetch.tip", "Ledge se jednou za 10 min zeptá Anthropicu na tvé limity tvým přihlášením Claude Code; token nikam neukládá ani neposílá jinam.");
  // důvod jen když ho nezobrazuje už původní řádek (stavový řádek nezapojený / čeká) — jinak by se opakoval
  const why = () => (p.l.fetch || p.l.desktop === "none" ? tx(limitsWhy(p.l)) : "");
  return (
    <Show when={p.l.at == null}>
      <Show when={why()}>{(w) => <div class="lim-empty">{w()}</div>}</Show>
      <For each={offers(p.l)}>
        {(o) => (
          <button class="hint-link" disabled={busy()} title={title(o)} onClick={() => enable(o, setBusy)}>
            {label(o)}
          </button>
        )}
      </For>
    </Show>
  );
}
