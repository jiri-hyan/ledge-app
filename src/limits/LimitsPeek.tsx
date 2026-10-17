// Peek připomínky resetu kvóty Claude: jádro (`limitslive::tick`) ho ukáže v čase resetu okna, které
// dosáhlo 100 % (ve fullscreen nikdy), a po ~15 s zavře. Bez připomínky vykreslí `fallback`.
import { Show, type JSX } from "solid-js";
import { api, type Snapshot } from "../api";
import { t } from "../i18n";
import "./limits.css";

export default function LimitsPeek(p: { s: Snapshot; fallback: JSX.Element }) {
  const notice = () => p.s.limits?.notice ?? null;
  return (
    <Show when={notice()} fallback={p.fallback}>
      {(k) => (
        <div class="peek" data-anim>
          <div class="lim-ico">
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"><path d="M20 11a8 8 0 1 0-2.3 5.7M20 5v6h-6" /></svg>
          </div>
          <div class="tx">
            <b>{k() === "fiveHour" ? t("sessions.limits.peek.five", "Limit Claude 5h se obnovil") : t("sessions.limits.peek.week", "Týdenní limit Claude se obnovil")}</b>
            <div class="code">{t("sessions.limits.peek.detail", "Kvóta byla vyčerpaná na 100 %, teď začíná nové okno.")}</div>
          </div>
          <button class="btn" onClick={() => api.close()}>{t("sessions.limits.peek.ok", "OK")}</button>
        </div>
      )}
    </Show>
  );
}
