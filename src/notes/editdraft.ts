// Rozhodnutí, co s rozepsanou úpravou z disku (`<root>\.draft\edit.json`), když se otevírá poznámka.
// Čistá funkce bez DOM a Tauri (test `editdraft.test.mjs`); volá ji `draft.ts` `openEdit`.

/** Rozepsaná úprava na disku (Rust `notes::edit::EditDraft`). */
export type EditDraftLike = { path: string; hash: string; text: string; project: string; kind: string; remind: string };
/** Poznámka tak, jak je teď na disku (z `note_read`), v tvaru polí editoru. */
export type DiskNote = { path: string; hash: string; text: string; project: string; kind: string; remind: string };

export type DraftPlan =
  /** rozepsaná úprava je k jiné poznámce a stojí za dokončení → otevřít ji místo požadované */
  | { do: "redirect"; path: string }
  /** obnovit text rozepsané úpravy; `stale` = soubor se od ní změnil (lístek, sync) → ukázat konflikt */
  | { do: "restore"; stale: boolean }
  /** nic nevracet, úpravu na disku smazat (prázdná, shodná s diskem nebo k zmizelé poznámce) */
  | { do: "drop" }
  /** nic nevracet a nic nemazat */
  | { do: "ignore" };

const same = (a: string, b: string) => a.trimEnd() === b.trimEnd();

/** Prázdný text nejde uložit (Rust `check_content`) — taková úprava není co zachraňovat. */
export const draftIsJunk = (d: EditDraftLike) => !d.text.trim();

/** Úprava jiné poznámky, než která se otevírá: přednost má jen ta, v níž něco opravdu je. */
export function planOther(d: EditDraftLike | null, requested: string): DraftPlan {
  if (!d || d.path === requested) return { do: "ignore" };
  return draftIsJunk(d) ? { do: "drop" } : { do: "redirect", path: d.path };
}

/** Úprava téže poznámky proti jejímu aktuálnímu stavu na disku. */
export function planSame(d: EditDraftLike | null, n: DiskNote): DraftPlan {
  if (!d || d.path !== n.path) return { do: "ignore" };
  if (draftIsJunk(d)) return { do: "drop" };
  const noChange = same(d.text, n.text) && d.project === n.project && d.kind === n.kind && d.remind === n.remind;
  if (noChange) return { do: "drop" };
  return { do: "restore", stale: d.hash !== n.hash };
}
