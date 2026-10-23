// Kontextové návrhy (Rust `suggest/`): zapnuté zdroje a GitHub z paměti — zbytek (schránka, procesy,
// připomínky) bere pruh přímo ze snapshotu a z události `cliphist` (`src/suggest/`).

/** Config `suggest` (Rust `suggest::conf::SuggestConfig`). */
export interface SuggestOn {
  clipboard: boolean;
  devServer: boolean;
  github: boolean;
  reminders: boolean;
  githubPulse: boolean;
}

/** PR, ke kterému se nabízí akce (jen adresy `https://github.com/…`). */
export interface PrRef {
  repo: string;
  number: number;
  title: string;
  url: string;
}

export interface GithubRefs {
  /** čekají na tvoji revizi */
  review: PrRef[];
  /** tvoje PR se spadlým CI */
  failing: PrRef[];
  /** kdy se data naposled načetla (ms); null = nikdy */
  fetchedMs: number | null;
}

/** Snapshot `suggest` (Rust `suggest::SuggestView`); chybí jen ve starém jádře. */
export interface SuggestView {
  on: SuggestOn;
  github: GithubRefs;
}
