// Šuplík → plocha: volby „Připnout na plochu: …“ v ⋯ karty (`cards/CardMenu.tsx`), lidské názvy druhů a
// připnutí s hláškou. Rust vrátí už připnutý widget (jen se ukáže), plno = věta s důvodem a co udělat.
import { For } from "solid-js";
import { t } from "../i18n";
import { cardName, tx } from "../layout/text";
import type { CardId } from "../layout/model";
import { PopItem } from "../ui/Popover";
import { say, sayError } from "../ui/toast";
import { deskApi } from "./api";
import { isBuiltinKind, KIND_NAME, kindsOfCard } from "./model";

/** Název druhu pro šuplík: vestavěný přeložený, widget ze skriptu jako v katalogu karet. */
export function deskKindName(kind: string): string {
  return isBuiltinKind(kind) ? tx(KIND_NAME[kind]) : cardName(kind as CardId);
}

export async function pinKind(kind: string): Promise<void> {
  try {
    await deskApi.pin(kind);
    say(t("cards.desk.pinned", "Na ploše: {name}. Volby má pod ⋯ v rohu widgetu.", { name: deskKindName(kind) }));
  } catch (e) {
    sayError(e);
  }
}

export async function unpinWidget(id: string, kind: string): Promise<void> {
  try {
    await deskApi.unpin(id);
    say(t("cards.desk.unpinned", "Odepnuto z plochy: {name}", { name: deskKindName(kind) }));
  } catch (e) {
    sayError(e);
  }
}

/** Položky ⋯ karty: karta, kterou jde mít na ploše, nabídne „Připnout na plochu“ (Claude dvě: limity, čekající). */
export function DeskPinItems(p: { card: string; close: () => void }) {
  const kinds = () => kindsOfCard(p.card);
  return (
    <For each={kinds()}>
      {(k) => (
        <PopItem onClick={() => { p.close(); void pinKind(k); }} title={t("cards.desk.pinTip", "Malé okno na ploše nebo druhém monitoru — žije i se zavřeným Ledge")}>
          {kinds().length > 1 ? t("cards.desk.pinNamed", "Připnout na plochu: {name}", { name: deskKindName(k) }) : t("cards.desk.pin", "Připnout na plochu")}
        </PopItem>
      )}
    </For>
  );
}
