// Rozepsaná úprava poznámky (`editdraft.ts`). Chyba, kterou test hlídá: zastaralá / prázdná úprava
// na disku přebila skutečný obsah poznámky (lístek ji mezitím doplnil) a editor v šuplíku byl prázdný;
// úprava jiné poznámky navíc přesměrovala každé otevření.
import { test } from "node:test";
import assert from "node:assert/strict";
import { draftIsJunk, parkInto, planOther, planSame, toDiskEdit, trimBody } from "./editdraft.ts";

const note = { path: "C:\\n\\a.md", hash: "h2", text: "Dlouhý text z lístku\n\n![](./a-1.png)", project: "", kind: "info", remind: "" };
const draft = (o = {}) => ({ path: note.path, hash: "h1", text: "rozepsáno", project: "", kind: "info", remind: "", ...o });

test("prázdná úprava není co zachraňovat (nejde ani uložit)", () => {
  assert.equal(draftIsJunk(draft({ text: "" })), true);
  assert.equal(draftIsJunk(draft({ text: " \n " })), true);
  assert.equal(draftIsJunk(draft()), false);
});

test("úprava jiné poznámky: přesměruje jen ta, v níž něco je", () => {
  const cases = [
    [null, { do: "ignore" }],
    [draft(), { do: "ignore" }], // táž poznámka — řeší planSame
    [draft({ path: "C:\\n\\b.md", text: "" }), { do: "drop" }],
    [draft({ path: "C:\\n\\b.md" }), { do: "redirect", path: "C:\\n\\b.md" }],
  ];
  for (const [d, want] of cases) assert.deepEqual(planOther(d, note.path), want, JSON.stringify(d));
});

test("úprava téže poznámky proti disku", () => {
  const cases = [
    ["žádná", null, { do: "ignore" }],
    ["jiná poznámka", draft({ path: "C:\\n\\b.md" }), { do: "ignore" }],
    // chyba: prázdná úprava se starým otiskem se vracela a editor ukázal prázdno místo obsahu z disku
    ["prázdná", draft({ text: "" }), { do: "drop" }],
    ["shodná s diskem", draft({ text: note.text + "\n" }), { do: "drop" }],
    ["čerstvá (otisk sedí)", draft({ hash: "h2" }), { do: "restore", stale: false }],
    // soubor se od úpravy změnil (lístek, sync) → úprava se vrátí, ale s varováním
    ["zastaralá (otisk nesedí)", draft(), { do: "restore", stale: true }],
    ["změněná kategorie", draft({ text: note.text, kind: "napad", hash: "h2" }), { do: "restore", stale: false }],
  ];
  for (const [name, d, want] of cases) assert.deepEqual(planSame(d, note), want, name);
});

test("cesty se porovnávají jako Windows cesty (velikost písmen, lomítka)", () => {
  const other = "c:/n/A.md";
  assert.deepEqual(planOther(draft({ path: other }), "C:\\n\\a.md"), { do: "ignore" });
  assert.deepEqual(planSame(draft({ path: other, text: "rozepsáno" }), note), { do: "restore", stale: true });
});

test("trimBody: koncová zalomení pryč (Rust je dřív vracel), vnitřní zůstanou", () => {
  const cases = [["a\n", "a"], ["a\r\n\r\n", "a"], ["a\n\nb\n", "a\n\nb"], ["a  ", "a  "], ["", ""], ["\n", ""]];
  for (const [inp, want] of cases) assert.equal(trimBody(inp), want, JSON.stringify(inp));
});

// ---- autosave konceptu NOVÉ poznámky (audit J8: `draft.ts` bez testu; rozhodnutí jsou v čistých funkcích) ----
import { canRestoreDraft, draftFields, planDraftPersist } from "./editdraft.ts";

const meta = (o = {}) => ({ text: "", projectTouched: false, remind: "", kind: "", title: "", ...o });

test("planDraftPersist: prázdný koncept se maže, cokoli rozepsaného se ukládá", () => {
  const cases = [
    [meta(), 0, "clear"],
    [meta({ text: " \n " }), 0, "clear"],
    [meta({ title: "  " }), 0, "clear"],
    [meta({ text: "a" }), 0, "save"],
    [meta(), 1, "save"], // jen obrázek ze schránky
    [meta({ remind: "2026-11-20 09:00" }), 0, "save"],
    [meta({ projectTouched: true }), 0, "save"],
    [meta({ kind: "task" }), 0, "save"],
    [meta({ title: "Nadpis" }), 0, "save"],
  ];
  for (const [m, n, want] of cases) assert.equal(planDraftPersist(m, n), want, JSON.stringify([m, n]));
});

test("draftFields: koncept z disku → pole editoru, chybějící kind/title dostanou výchozí", () => {
  const f = draftFields({ text: "t", project: "p", projectTouched: true, remind: "", images: ["QUJD"] }, "info");
  assert.deepEqual(f, { text: "t", project: "p", projectTouched: true, remind: "", kind: "info", title: "", imgs: ["data:image/png;base64,QUJD"] });
  assert.equal(draftFields({ text: "", project: "", projectTouched: false, remind: "", kind: "task", title: "N", images: [] }, "info").kind, "task");
});

test("canRestoreDraft: obnova jen do prázdného konceptu a jen když na disku něco je", () => {
  const d = { text: "z disku" };
  assert.equal(canRestoreDraft("", 0, d), true);
  assert.equal(canRestoreDraft("už psáno", 0, d), false);
  assert.equal(canRestoreDraft("", 2, d), false);
  assert.equal(canRestoreDraft("", 0, null), false);
});

test("změněný jen nadpis nebo jen obrázky se nezahodí (P6)", () => {
  const same = draft({ text: note.text, hash: "h2" });
  const cases = [
    ["nadpis nezměněn", { ...same, title: "" }, {}, { do: "drop" }],
    ["starší edit.json bez nadpisu", same, {}, { do: "drop" }],
    ["null = nadpis z disku", { ...same, title: null }, {}, { do: "drop" }],
    ["změněný jen nadpis", { ...same, title: "Nový" }, {}, { do: "restore", stale: false }],
    ["nadpis shodný s diskem", { ...same, title: "Z disku" }, {}, { do: "drop", heading: "Z disku" }],
    ["jen obrázky", same, { imgs: true }, { do: "restore", stale: false }],
    ["prázdný text i s obrázky je odpad", { ...same, text: "" }, { imgs: true }, { do: "drop" }],
  ];
  for (const [name, d, extra, want] of cases) {
    const n = want.heading ? { ...note, title: want.heading } : note;
    assert.deepEqual(planSame(d, n, extra), { do: want.do, ...(want.stale === undefined ? {} : { stale: want.stale }) }, name);
  }
});

test("toDiskEdit nese nadpis jen když je znám; parkInto drží jednu úpravu na poznámku", () => {
  const snap = (path, text, title) => ({ base: { path }, hash: "h", text, project: "", kind: "info", remind: "", ...(title === undefined ? {} : { title }) });
  assert.equal(toDiskEdit(null), null);
  assert.deepEqual(toDiskEdit(snap("a", "x", "T")), { path: "a", hash: "h", text: "x", project: "", kind: "info", remind: "", title: "T" });
  assert.equal("title" in toDiskEdit(snap("a", "x")), false);
  const a1 = snap("C:/n/a.md", "1"), a2 = snap("c:/n/A.md", "2"), b = snap("C:/n/b.md", "3");
  assert.deepEqual(parkInto(parkInto([], a1), b), [a1, b]);
  assert.deepEqual(parkInto([a1, b], a2), [b, a2], "nová verze téže poznámky nahradí starou a je nejnovější");
});
