// Schválit / zamítnout Clauda v paletě (Ctrl+K): řádky vzniknou JEN když čeká žádost (`snapshot.pending`) A dotaz začíná
// slovem akce („povolit“, „schválit“, „zamítnout“, „allow“, „approve“, „deny“ — aspoň 2 znaky). Bez psaní se nikdy nenabídnou,
// takže Enter na prvním řádku prázdné palety nic nerozhodne. Rozhodnutí = lidský Enter a stejná cesta jako tlačítka karty
// (`api.decide(id, allow)` — `cards/Peek.tsx` `DecideButtons`, `ClaudeSessions.tsx` `PendingRow`; plán: Schválit = allow bez doplňku).
// Žádný automat, žádná obchvatná cesta. Čistá logika s vloženou akcí — `decide.test.mjs`; zapojení v `live.ts`.
import type { Pending, Snapshot } from "../api";
import { t } from "../i18n.ts";
import { fold } from "./search.ts";
import { planTitle } from "../sessions/plan.ts";
import type { PItem } from "./items";

const ALLOW = ["povolit", "schvalit", "allow", "approve"];
const DENY = ["zamitnout", "zamitni", "deny", "reject"];
const MIN = 2;
const SUMMARY_MAX = 90;

/** Která rozhodnutí dotaz žádá: první slovo je začátek (≥ 2 znaky) slovesa akce. */
export function decideWanted(q: string): { allow: boolean; deny: boolean } {
  const w = fold(q).trim().split(/\s+/)[0] ?? "";
  if (w.length < MIN) return { allow: false, deny: false };
  return { allow: ALLOW.some((v) => v.startsWith(w)), deny: DENY.some((v) => v.startsWith(w)) };
}

const cut = (x: string) => (x.length > SUMMARY_MAX ? `${x.slice(0, SUMMARY_MAX - 1)}…` : x);

/** Odkud žádost je: projekt session, jinak poslední složka cwd (jako `cards/Peek.tsx` `pendingWhere`). */
const whereOf = (s: Pick<Snapshot, "sessions">, pd: Pending): string => s.sessions.find((x) => x.id === pd.sessionId)?.project ?? pd.cwd.split(/[\/]/).pop() ?? "";

export function decideItems(s: Pick<Snapshot, "pending" | "sessions">, q: string, decide: (id: number, allow: boolean) => unknown): PItem[] {
  const want = decideWanted(q);
  if (!want.allow && !want.deny) return [];
  const out: PItem[] = [];
  for (const pd of s.pending ?? []) {
    const what = pd.plan ? planTitle(pd) : `${pd.tool}: ${pd.summary}`;
    const sub = t("palette.decide.sub", "Claude čeká · {where} — rozhodneš Enterem", { where: whereOf(s, pd) });
    if (want.allow) out.push({ id: `decide:${pd.id}:allow`, section: "claude", order: 0, glyph: "✔", title: pd.plan ? t("palette.decide.plan", "Schválit plán: {what}", { what: cut(what) }) : t("palette.decide.allow", "Povolit: {what}", { what: cut(what) }), sub, keywords: "claude schvalit povolit approve allow plan", run: () => decide(pd.id, true) });
    if (want.deny) out.push({ id: `decide:${pd.id}:deny`, section: "claude", order: 0, glyph: "✖", title: t("palette.decide.deny", "Zamítnout: {what}", { what: cut(what) }), sub, keywords: "claude zamitnout deny reject", run: () => decide(pd.id, false) });
  }
  return out;
}
