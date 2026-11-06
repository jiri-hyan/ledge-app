// Pád vykreslení jedné části UI (`ErrorBoundary`): místo prázdného / rozbitého okna lidská věta a „Zkusit znovu“,
// zbytek aplikace jede dál. Chyba jde do `ledge.log` (`uiErrors.ts`). `resetOn` = signál, jehož změna (přepnutí
// stránky) chybu sama zahodí. Žádné časovače; bez chyby nevykresluje nic navíc.
import { ErrorBoundary, createEffect, on, type JSX } from "solid-js";
import { t } from "../i18n";
import { logUiError } from "../uiErrors";
import "./boundary.css";

function Fallback(p: { err: unknown; reset: () => void; resetOn?: () => unknown }) {
  logUiError("render", p.err);
  if (p.resetOn) createEffect(on(p.resetOn, () => p.reset(), { defer: true }));
  return (
    <div class="ui-boundary" role="alert">
      <div>{t("ui.boundary.msg", "Tahle část se nenačetla.")}</div>
      <div class="ui-boundary-sub">{t("ui.boundary.hint", "Zbytek Ledge funguje dál, podrobnosti jsou v logu.")}</div>
      <button class="mini" onClick={p.reset}>{t("ui.boundary.retry", "Zkusit znovu")}</button>
    </div>
  );
}

export default function Boundary(p: { children: JSX.Element; resetOn?: () => unknown }) {
  return <ErrorBoundary fallback={(err, reset) => <Fallback err={err} reset={reset} resetOn={p.resetOn} />}>{p.children}</ErrorBoundary>;
}
