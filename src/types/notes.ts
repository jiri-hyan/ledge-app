/** Poznámka z `%USERPROFILE%\Ledge\notes` (jeden `.md` s frontmatter). */
export interface NoteView {
  /** absolutní cesta k `.md` — zároveň klíč */
  path: string;
  /** id projektu, null = obecná */
  project: string | null;
  /** ISO 8601 s offsetem */
  created: string;
  createdMs: number;
  remind: string | null;
  remindMs: number | null;
  done: boolean;
  /** zobrazený název: vlastní nadpis (`title:`), jinak první řádek bez Markdown značek */
  title: string;
  /** vlastní nadpis z frontmatteru; chybí = název je první řádek */
  heading?: string;
  images: number;
  /** kategorie (`kind:`), "info" když chybí; neznámé id z druhého PC projde beze změny */
  kind: string;
  /** poslední úprava v Ledge (ISO s offsetem) */
  updated: string | null;
  tasksDone: number;
  tasksTotal: number;
  /** barva lístku (`color:` — id z palety nebo `#rrggbb`), chybí = výchozí */
  color?: string;
  /** štítky: `tags:` ze frontmatteru + `#štítek` v textu; chybí = žádné */
  tags?: string[];
}

/** Kategorie poznámek z `notes.kinds`. */
export interface NoteKind { id: string; label: string; color: string; icon: string }

/** Poznámka k úpravě (`note_read`). */
export interface NoteDetail extends NoteView {
  /** tělo bez frontmatteru; řádky úkolů se počítají v něm */
  body: string;
  /** otisk souboru při čtení — uložení ho chce zpátky */
  hash: string;
  /** relativní obrázky (`./a-1.png`) jako data URL */
  imageData: { src: string; data: string }[];
}

export interface SaveNote {
  path: string;
  expectedHash: string;
  body: string;
  project: string | null;
  kind: string | null;
  /** vlastní nadpis: chybí = beze změny, "" = smazat */
  title?: string;
  /** lokální čas `YYYY-MM-DDTHH:MM` — offset doplní Rust; null = bez připomínky */
  remind: string | null;
  /** nové PNG (base64 bez prefixu), připojí se na konec */
  images: string[];
}
export type SaveOutcome = { outcome: "saved"; note: NoteDetail } | { outcome: "changed" };

/** Volby lístku na plochu (Rust `pins::PinPrefs`). `bg` 0–100 %, `text` 50–100 % neprůhlednost. */
export interface PinPrefs { onTop: boolean; locked: boolean; through: boolean; bg: number; text: number; collapsed: boolean; /** styl (Rust `pins::PinStyle`) */ style: "classic" | "paper" | "glass" | "stripe" | "minimal" | "dark" }
export interface PinsView { open: { path: string; through: boolean }[]; key: string }
/** Start okna lístku (`pin_init`): poznámka, volby, zkratka průchozího režimu (a jestli je zaregistrovaná). */
export interface PinInit { path: string; prefs: PinPrefs; key: string; keyOk: boolean; hintSeen: boolean; lang: "cs" | "en"; fresh: boolean; /** `notes.tagColors` — barva podle štítku */ tagColors: Record<string, string>; /** id projektu → název */ projects?: Record<string, string>; /** id projektu → barva `#rrggbb` */ projectColors?: Record<string, string> }

export interface NoteQuery { kind?: string | null; project?: string | null; includeDone?: boolean; /** jen hotové */ onlyDone?: boolean; /** archivované místo aktivních */ archived?: boolean; limit?: number; /** štítek bez `#` */ tag?: string | null }
/** Výsledek smazání poznámky: id pro „Zpět“ a titulek pro lištu. */
export interface Deleted { id: string; title: string }
export interface QueryResult { notes: NoteView[]; total: number }

/** Rozepsaná úprava existující poznámky (`<root>\.draft\edit.json`). */
export interface EditDraft { path: string; hash: string; text: string; project: string; kind: string; remind: string }

export interface NewNote {
  text: string;
  project: string | null;
  /** hodnota z `datetime-local` (lokální čas) — offset doplní Rust */
  remind: string | null;
  /** PNG v base64 (bez prefixu data:) */
  images: string[];
  /** kategorie; null = výchozí */
  kind: string | null;
  /** vlastní nadpis; chybí / prázdný = název je první řádek */
  title?: string;
}

/** Synchronizace poznámek přes git (`notesync/`). `off` = vypnutá; `syncing` = právě běží (i první běh). */
export interface NotesSyncView {
  state: "off" | "ok" | "syncing" | "offline" | "neprihlaseno" | "konflikt" | "chyba";
  /** poslední úspěšný sync v tomto běhu Ledge (nic se nedopočítává) */
  lastOkMs: number | null;
  message: string | null;
  /** soubory v konfliktu */
  files: string[];
  /** kořen poznámek */
  root: string | null;
}

/** Konflikt propojení se složkou: obě strany se změnily, druhá verze je uložená jako kopie. */
export interface NotesFolderConflict {
  name: string;
  atMs: number;
  /** absolutní cesta kopie v Ledge (poznámka), null = tam nevznikla */
  ledge: string | null;
  /** absolutní cesta kopie ve složce */
  folder: string | null;
  /** čí verze zůstala na původním místě */
  kept: "folder" | "ledge";
}

/** Propojení poznámek se složkou (`threads/notes_folder.rs`); `off` = vypnuté. */
export interface NotesFolderView {
  state: "off" | "idle" | "syncing" | "error";
  /** poslední úspěšný běh v tomto běhu Ledge */
  lastMs: number | null;
  /** věta o posledním běhu, nebo chyba */
  message: string | null;
  conflicts: NotesFolderConflict[];
}

/** Rozepsaná poznámka — leží na disku (`<root>\.draft\`), přežije zavření šuplíku i pád. */
export interface DraftMeta {
  text: string;
  project: string;
  projectTouched: boolean;
  remind: string;
  kind: string;
  /** vlastní nadpis; prázdný = první řádek */
  title?: string;
}
export interface Draft extends DraftMeta {
  /** PNG v base64 (bez prefixu data:) */
  images: string[];
}
