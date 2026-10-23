// Rozepsaná úprava poznámky (`editdraft.ts`). Chyba, kterou test hlídá: zastaralá / prázdná úprava
// na disku přebila skutečný obsah poznámky (lístek ji mezitím doplnil) a editor v šuplíku byl prázdný;
// úprava jiné poznámky navíc přesměrovala každé otevření.
import { test } from "node:test";
import assert from "node:assert/strict";
import { draftIsJunk, planOther, planSame, trimBody } from "./editdraft.ts";

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
