// Pruh velikosti kontextu pod řádkem živé session v kartě Claude (kolo 23, E1): kolik tokenů teď čte KAŽDÝ tah
// (poslední `usage` v transcriptu: vstup + zápis cache + čtení cache), rysky 250 k / 350 k, poměr k začátku
// session a nad 250 k tip o předávce. Data si řádek žádá sám přes `api.sessionContext` — jen při otevřeném šuplíku
// a vykresleném řádku (Rust to ještě omezí na 1× / 15 s na soubor); bez zdroje (session bez hooku, config vypnutý,
// soubor bez usage) se nekreslí nic. Žádný časovač: žádost se váže na změnu session a minutový tik šuplíku.
import { createEffect, createSignal, on, Show } from "solid-js";
import { api, type SessionView, type Snapshot } from "../api";
import { num } from "../format";
import { t } from "../i18n";
import { compact } from "../costs/model";
import { ctxFill, ctxTicks, sameCtx, showRatio, showTip, type CtxView } from "./ctx";
import "./ctx.css";

/** „182 k“ / „1,2 M“ (bez slova „tokenů“) */
const kt = (n: number) => {
  const c = compact(n);
  return `${num(c.v, c.digits)}${c.unit ? ` ${c.unit}` : ""}`;
};

export default function ContextBar(p: { ss: SessionView; s: Snapshot; now: number }) {
  const [ctx, setCtx] = createSignal<CtxView | null>(null);
  const live = () => p.ss.hooked && p.ss.status !== "ended";
  // žádost: změna session (hook event), otevření šuplíku, jinak nejvýš co 30 s (Rust drží cache podle mtime + délky)
  createEffect(on([() => p.s.view, live, () => p.ss.updatedAt, () => Math.floor(p.now / 30_000)], ([view, on_]) => {
    if (view !== "full" || !on_) return;
    void api
      .sessionContext(p.ss.id)
      .then((r) => {
        const c = r ?? null;
        if (!sameCtx(ctx(), c)) setCtx(c);
      })
      .catch(() => {});
  }));
  return (
    <Show when={live() && ctx()}>
      {(c) => (
        <div class="ctx" classList={{ warn: c().level === "warn", high: c().level === "high" }}>
          <div class="ctx-line">
            <span
              class="ctx-track"
              role="img"
              aria-label={t("sessions.ctx.aria", "Kontext session: {n} tokenů", { n: num(c().tokens) })}
              title={t("sessions.ctx.tip", "Kontext posledního volání: {n} tokenů (vstup + zápis cache + čtení cache). Rysky: 250 k a 350 k.", { n: num(c().tokens) })}
            >
              <span class="ctx-fill" style={{ width: `${ctxFill(c().tokens) * 100}%` }} />
              {ctxTicks().map((x) => <span class="ctx-tick" style={{ left: `${x * 100}%` }} />)}
            </span>
            <span class="ctx-text">
              {t("sessions.ctx.each", "každý tah teď čte ~{n} tokenů", { n: kt(c().tokens) })}
              {showRatio(c()) ? ` · ${t("sessions.ctx.ratio", "{r}× víc než na začátku", { r: num(c().ratio!, 1) })}` : ""}
            </span>
          </div>
          <Show when={showTip(c())}>
            <div class="ctx-tip">{t("sessions.ctx.advice", "Velký kontext prodražuje každý tah — zvaž předávku a novou relaci")}</div>
          </Show>
        </div>
      )}
    </Show>
  );
}
