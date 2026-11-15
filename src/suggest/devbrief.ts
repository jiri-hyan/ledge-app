// Zadání „dev chyba → poznámka“: čisté složení těla poznámky z balíčku kontextu (projekt, URL, větev + dirty,
// příkaz, uptime, chybové řádky). Bez DOM, `t()` a api — popisky dostane od volajícího (`actions.ts`), test `devbrief.test.mjs`.
// Tajemství maskuje už jádro (`procman/errline.rs`) dřív, než řádky přijdou do snapshotu.

export interface DevBriefInput {
  project: string;
  url: string | null;
  branch: string | null;
  /** počet změněných souborů; null = neznámo (jen s větví) */
  dirty: number | null;
  cmd: string | null;
  /** hotový text („12 min“); null = neznámo */
  uptime: string | null;
  errors: number;
  lines: readonly string[];
}

export interface DevBriefLabels {
  title: string; // „Chyba dev serveru“
  project: string;
  url: string;
  branch: string;
  dirty: (n: number) => string; // „3 neuložené změny“
  cmd: string;
  uptime: string;
  errors: string;
  output: string;
  steps: string;
  expected: string;
  happened: string;
  task: string; // první bod zadání
}

/** Řádek logu uvnitř bloku kódu: ``` by blok ukončilo. */
const fence = (l: string) => l.replace(/`{3,}/g, "'''");

/** Tělo poznámky (druh `zadani`): jen známé údaje (co chybí, se nevypíše), blok výstupu, kostra Chyba/QA a první bod. */
export function devBriefBody(i: DevBriefInput, L: DevBriefLabels): string {
  const row = (k: string, v: string | null | undefined) => (v ? [`**${k}:** ${v}`] : []);
  const branch = i.branch ? i.branch + (i.dirty ? ` (${L.dirty(i.dirty)})` : "") : null;
  const head = [
    `# ${L.title}: ${i.project}`,
    "",
    ...row(L.project, i.project),
    ...row(L.url, i.url),
    ...row(L.branch, branch),
    ...row(L.cmd, i.cmd ? `\`${i.cmd.replace(/`/g, "'")}\`` : null),
    ...row(L.uptime, i.uptime),
    ...row(L.errors, i.errors > 0 ? String(i.errors) : null),
  ];
  const out = i.lines.length ? ["", `**${L.output}:**`, "```", ...i.lines.map(fence), "```"] : [];
  return [
    ...head,
    ...out,
    "",
    `**${L.steps}:**`,
    "1. ",
    "",
    `**${L.expected}:**`,
    "",
    `**${L.happened}:**`,
    "",
    `- [ ] ${L.task}`,
    "",
  ].join("\n");
}
