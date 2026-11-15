// `pollWhileShown`: Solid obal nad `sharedPoll` — odběr běží jen dokud je šuplík vidět (`media/shown.ts`), zavřený =
// žádný časovač ani IPC. Volej uvnitř komponenty; víc komponent nad stejným `sharedPoll` sdílí jednu smyčku.
import { createEffect, onCleanup } from "solid-js";
import { drawerShown } from "../media/shown";
import type { SharedPoll } from "./poll";

export function pollWhileShown(poll: SharedPoll): void {
  createEffect(() => {
    if (!drawerShown()) return;
    onCleanup(poll.acquire());
  });
}
