// Systémové ikony pro zástupce: výběr ze Segoe Fluent Icons (Windows 11; na Windows 10 stejné kódy v
// Segoe MDL2 Assets). Každý kód je ověřený v obou písmech (cmap `SegoeIcons.ttf` i `segmdl2.ttf`) a tvar
// zkontrolovaný vykreslením. `id` = anglické slovo pro hledání, název = pár pro `t()`.
// Čistá data + hledání bez diakritiky (test `model.test.mjs`).

export type Glyph = { id: string; cp: string; name: readonly [string, string] };

export const GLYPHS: readonly Glyph[] = [
  { id: "home", cp: "E80F", name: ["cards.tile.glyph.home", "Domů"] },
  { id: "document", cp: "E8A5", name: ["cards.tile.glyph.document", "Dokument"] },
  { id: "folder", cp: "E8B7", name: ["cards.tile.glyph.folder", "Složka"] },
  { id: "globe", cp: "E774", name: ["cards.tile.glyph.globe", "Web"] },
  { id: "terminal", cp: "E756", name: ["cards.tile.glyph.terminal", "Terminál"] },
  { id: "code", cp: "E943", name: ["cards.tile.glyph.code", "Kód"] },
  { id: "game", cp: "E7FC", name: ["cards.tile.glyph.game", "Hra"] },
  { id: "music", cp: "E8D6", name: ["cards.tile.glyph.music", "Hudba"] },
  { id: "video", cp: "E714", name: ["cards.tile.glyph.video", "Video"] },
  { id: "camera", cp: "E722", name: ["cards.tile.glyph.camera", "Fotoaparát"] },
  { id: "mail", cp: "E715", name: ["cards.tile.glyph.mail", "Pošta"] },
  { id: "calendar", cp: "E787", name: ["cards.tile.glyph.calendar", "Kalendář"] },
  { id: "chat", cp: "E8F2", name: ["cards.tile.glyph.chat", "Chat"] },
  { id: "person", cp: "E77B", name: ["cards.tile.glyph.person", "Osoba"] },
  { id: "people", cp: "E716", name: ["cards.tile.glyph.people", "Lidé"] },
  { id: "settings", cp: "E713", name: ["cards.tile.glyph.settings", "Nastavení"] },
  { id: "wrench", cp: "E90F", name: ["cards.tile.glyph.wrench", "Nářadí"] },
  { id: "library", cp: "E8F1", name: ["cards.tile.glyph.library", "Knihovna"] },
  { id: "book", cp: "E82D", name: ["cards.tile.glyph.book", "Kniha"] },
  { id: "star", cp: "E734", name: ["cards.tile.glyph.star", "Hvězda"] },
  { id: "heart", cp: "EB51", name: ["cards.tile.glyph.heart", "Srdce"] },
  { id: "download", cp: "E896", name: ["cards.tile.glyph.download", "Stáhnout"] },
  { id: "upload", cp: "E898", name: ["cards.tile.glyph.upload", "Nahrát"] },
  { id: "cloud", cp: "E753", name: ["cards.tile.glyph.cloud", "Cloud"] },
  { id: "lock", cp: "E72E", name: ["cards.tile.glyph.lock", "Zámek"] },
  { id: "copy", cp: "E8C8", name: ["cards.tile.glyph.copy", "Kopírovat"] },
  { id: "edit", cp: "E70F", name: ["cards.tile.glyph.edit", "Upravit"] },
  { id: "save", cp: "E74E", name: ["cards.tile.glyph.save", "Uložit"] },
  { id: "print", cp: "E749", name: ["cards.tile.glyph.print", "Tisk"] },
  { id: "search", cp: "E721", name: ["cards.tile.glyph.search", "Hledat"] },
  { id: "link", cp: "E71B", name: ["cards.tile.glyph.link", "Odkaz"] },
  { id: "calculator", cp: "E8EF", name: ["cards.tile.glyph.calculator", "Kalkulačka"] },
  { id: "lightning", cp: "E945", name: ["cards.tile.glyph.lightning", "Blesk"] },
  { id: "power", cp: "E7E8", name: ["cards.tile.glyph.power", "Napájení"] },
  { id: "wifi", cp: "E701", name: ["cards.tile.glyph.wifi", "Wi-Fi"] },
  { id: "monitor", cp: "E7F4", name: ["cards.tile.glyph.monitor", "Monitor"] },
  { id: "laptop", cp: "E7F8", name: ["cards.tile.glyph.laptop", "Notebook"] },
  { id: "phone", cp: "E8EA", name: ["cards.tile.glyph.phone", "Telefon"] },
  { id: "puzzle", cp: "EA86", name: ["cards.tile.glyph.puzzle", "Puzzle"] },
  { id: "education", cp: "E7BE", name: ["cards.tile.glyph.education", "Škola"] },
  { id: "shop", cp: "E719", name: ["cards.tile.glyph.shop", "Obchod"] },
  { id: "cart", cp: "E7BF", name: ["cards.tile.glyph.cart", "Košík"] },
  { id: "bank", cp: "E825", name: ["cards.tile.glyph.bank", "Banka"] },
  { id: "card", cp: "E8C7", name: ["cards.tile.glyph.card", "Platební karta"] },
  { id: "pin", cp: "E707", name: ["cards.tile.glyph.pin", "Místo"] },
  { id: "world", cp: "E909", name: ["cards.tile.glyph.world", "Svět"] },
  { id: "car", cp: "E804", name: ["cards.tile.glyph.car", "Auto"] },
  { id: "airplane", cp: "E709", name: ["cards.tile.glyph.airplane", "Letadlo"] },
  { id: "emoji", cp: "E76E", name: ["cards.tile.glyph.emoji", "Smajlík"] },
  { id: "photo", cp: "EB9F", name: ["cards.tile.glyph.photo", "Obrázek"] },
  { id: "flag", cp: "E7C1", name: ["cards.tile.glyph.flag", "Vlajka"] },
  { id: "history", cp: "E81C", name: ["cards.tile.glyph.history", "Historie"] },
  { id: "clock", cp: "E823", name: ["cards.tile.glyph.clock", "Hodiny"] },
  { id: "stopwatch", cp: "E916", name: ["cards.tile.glyph.stopwatch", "Stopky"] },
  { id: "bell", cp: "EA8F", name: ["cards.tile.glyph.bell", "Zvonek"] },
  { id: "like", cp: "E8E1", name: ["cards.tile.glyph.like", "Palec nahoru"] },
  { id: "play", cp: "E768", name: ["cards.tile.glyph.play", "Přehrát"] },
  { id: "headphones", cp: "E7F6", name: ["cards.tile.glyph.headphones", "Sluchátka"] },
  { id: "mic", cp: "E720", name: ["cards.tile.glyph.mic", "Mikrofon"] },
  { id: "volume", cp: "E767", name: ["cards.tile.glyph.volume", "Hlasitost"] },
  { id: "palette", cp: "E790", name: ["cards.tile.glyph.palette", "Paleta"] },
  { id: "brush", cp: "E771", name: ["cards.tile.glyph.brush", "Štětec"] },
  { id: "server", cp: "E968", name: ["cards.tile.glyph.server", "Server"] },
  { id: "bug", cp: "EBE8", name: ["cards.tile.glyph.bug", "Brouk"] },
  { id: "key", cp: "E8D7", name: ["cards.tile.glyph.key", "Klíč"] },
  { id: "shield", cp: "EA18", name: ["cards.tile.glyph.shield", "Štít"] },
  { id: "gift", cp: "F133", name: ["cards.tile.glyph.gift", "Dárek"] },
  { id: "coffee", cp: "EC32", name: ["cards.tile.glyph.coffee", "Káva"] },
  { id: "tag", cp: "E8EC", name: ["cards.tile.glyph.tag", "Štítek"] },
  { id: "list", cp: "E8FD", name: ["cards.tile.glyph.list", "Seznam"] },
  { id: "todo", cp: "E9D5", name: ["cards.tile.glyph.todo", "Úkoly"] },
  { id: "chart", cp: "E9D2", name: ["cards.tile.glyph.chart", "Graf"] },
  { id: "health", cp: "E95E", name: ["cards.tile.glyph.health", "Zdraví"] },
  { id: "leaf", cp: "EC0A", name: ["cards.tile.glyph.leaf", "List"] },
  { id: "sun", cp: "E706", name: ["cards.tile.glyph.sun", "Slunce"] },
  { id: "moon", cp: "E708", name: ["cards.tile.glyph.moon", "Měsíc"] },
  { id: "trash", cp: "E74D", name: ["cards.tile.glyph.trash", "Koš"] },
];

/** Znak pro kód `E80F`. */
export const glyphChar = (cp: string): string => String.fromCodePoint(parseInt(cp, 16));

const fold = (s: string) => s.normalize("NFD").replace(/[̀-ͯ]/g, "").toLowerCase();

/** Ikony podle dotazu: podřetězec v anglickém id, v názvu (přeloženém `label`) i v kódu. Prázdný = všechny. */
export function searchGlyphs(q: string, label: (x: Glyph) => string = (x) => x.name[1]): Glyph[] {
  const k = fold(q.trim());
  if (!k) return [...GLYPHS];
  return GLYPHS.filter((x) => x.id.includes(k) || fold(label(x)).includes(k) || fold(x.name[1]).includes(k) || x.cp.toLowerCase() === k);
}
