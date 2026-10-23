// Poznámky k vydání: bloky z `parseNotes` jako textové uzly Solid (vše escapované, žádné innerHTML ani
// klikací odkazy z vydání). Používá panel „Nová verze“ i „Co je nového“ (šuplík + Nastavení).
import { For, Match, Switch } from "solid-js";
import { t } from "../i18n";
import { parseNotes, type Inline } from "./model";
import "./update.css";

const parts = (ps: Inline[]) => <For each={ps}>{(x) => (x.k === "code" ? <code>{x.s}</code> : x.k === "b" ? <b>{x.s}</b> : x.s)}</For>;

export default function ReleaseNotes(p: { md: string }) {
  const blocks = () => parseNotes(p.md);
  return (
    <div class="upd-md">
      <For each={blocks()} fallback={<p class="upd-dim">{t("cards.update.noNotes", "Bez poznámek k vydání.")}</p>}>
        {(b) => (
          <Switch>
            <Match when={b.k === "h"}><h5>{parts(b.parts)}</h5></Match>
            <Match when={b.k === "li"}><div class="upd-li">{parts(b.parts)}</div></Match>
            <Match when={b.k === "p"}><p>{parts(b.parts)}</p></Match>
          </Switch>
        )}
      </For>
    </div>
  );
}
