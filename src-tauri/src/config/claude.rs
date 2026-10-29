//! Claude (port, hlídání commitů), kontrola aktualizací a ověření `owner/name`.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(default, rename_all = "camelCase")]
pub struct ClaudeConfig {
    /// Lokální most pro hooky Claude Code (jen 127.0.0.1).
    pub enabled: bool,
    pub port: u16,
    /// Jak dlouho (s) Ledge drží žádost o schválení. Pak ji vrátí terminálu (nikdy neschválí sám).
    pub approval_timeout_s: u64,
    /// Hlídač nedokončených session: soubory, které session sama upravila (Edit/Write/…) a nezacommitovala,
    /// a její commity, které nejsou na žádném remote. `off` = nehlídat, `notify` = jen ukázat v Ledge,
    /// `block` = při Stop vrátit session do práce s důvodem (nikdy při `stop_hook_active`).
    #[serde(default)]
    pub enforce_commit: EnforceCommit,
    /// Otázky Clauda (AskUserQuestion): jak dlouho (s) Ledge drží otázku a čeká na odpověď kliknutím;
    /// pak ji pustí do okna Claude. 0 = Ledge otázky jen ukazuje. Nejvýš 550 (hook má 600 s).
    pub question_hold_s: u64,
    /// Pojistka otázek Clauda: když drženou otázku do tolika sekund nikdo v Ledge neviděl (šuplík
    /// zavřený, hra přes celou obrazovku), Ledge ji pustí do okna Claude Code a zapíše do deníku.
    /// 0 = vypnuto (drží celé `questionHoldS`), jinak 5–550.
    #[serde(default = "default_question_unseen_s")]
    pub question_unseen_s: u64,
    /// Připomínka resetu kvóty: když 5h nebo týdenní limit dosáhl 100 %, v čase resetu krátký peek
    /// a bliknutí proužku (ve fullscreen nic). Data ze stavového řádku Ledge (`limitslive.rs`).
    pub limits_reminder: bool,
    /// Oznámení Windows (toast), když Claude čeká na schválení / otázku nebo dokončí turn. Bez fokusu,
    /// nikdy ve fullscreen hře, v herním režimu ani při „nerušit“; stejná session + událost max 1× za 30 s.
    pub toast: bool,
    /// Sekce „Útrata Claude (odhad)“ v kartě Claude: tokeny a odhad ceny v cenách API (ne účet) podle
    /// projektu a dne z lokálních transcriptů `~/.claude/projects` (`costslive.rs`). Počítá se jen při
    /// rozbalení sekce, nejvýš 1× za minutu. `false` = sekce zmizí a nic se nečte.
    pub costs: bool,
    /// Pruh velikosti kontextu pod každou živou session v kartě Claude (rysky 250 k / 350 k, „každý tah čte ~N k
    /// tokenů“). Čte konec transcriptu session jen při otevřeném šuplíku (`ctxsize.rs`). `false` = nic se nečte.
    pub context_bar: bool,
    /// Tempo čerpání limitů: v pruzích 5h / týden ryska plánu (rovnoměrné čerpání) a výstražná barva nad plánem.
    /// Výchozí vypnuto; zapíná sbírání vzorků 5h okna v paměti (`limitslive.rs`).
    pub limits_pace: bool,
    /// Upozornění (toast), když 5h nebo týdenní limit dosáhne 80 % a 90 %; jednou za okno. Výchozí vypnuto.
    pub limits_alert: bool,
    /// Limity 5h / týden i z aplikace Claude (desktop): Ledge čte její `plan-usage-history.json` (jen procenta,
    /// bez času resetu; nic neposílá). Výchozí vypnuto (`limitsrc.rs`, `limitslive.rs`).
    pub limits_desktop: bool,
    /// Ledge se jednou za 10 min zeptá Anthropicu (`api.anthropic.com/api/oauth/usage`) na limity 5h / týden
    /// přihlášením Claude Code z `~/.claude/.credentials.json`: token jen čte (žádný refresh, žádný zápis),
    /// drží ho jen po dobu dotazu a nikam jinam neposílá. Jen když je karta s limity vidět nebo proužek limity
    /// ukazuje, nikdy ve hře. Výchozí vypnuto (`limitsfetch.rs`).
    pub limits_fetch: bool,
}

impl Default for ClaudeConfig {
    fn default() -> Self {
        Self { enabled: true, port: 47800, approval_timeout_s: 45, enforce_commit: EnforceCommit::Notify, question_hold_s: 540, question_unseen_s: default_question_unseen_s(), limits_reminder: true, toast: false, costs: true, context_bar: false, limits_pace: false, limits_alert: false, limits_desktop: false, limits_fetch: false }
    }
}

/// Výchozí pojistka neviděné otázky (s).
fn default_question_unseen_s() -> u64 {
    crate::claude::unseen::DEFAULT_S
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum EnforceCommit {
    /// nehlídat (žádné volání gitu)
    Off,
    /// jen ukázat štítky v šuplíku a proužku
    #[default]
    Notify,
    /// Stop hook vrátí `{"decision":"block"}` → Claude musí commitnout / pushnout
    Block,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(default, rename_all = "camelCase")]
pub struct UpdateConfig {
    /// Kontrolovat nové verze (60 s po startu, pak každých `intervalH` h). Jen upozorní chipem
    /// v hlavičce šuplíku — nainstaluje se až na klik. Dotazy jdou přes veřejné GitHub API bez tokenu,
    /// záložně přes přihlášené `gh` (soukromé repo, limit API).
    pub check: bool,
    /// GitHub repo s vydáními, tvar `owner/name`.
    pub repo: String,
    /// Jak často kontrolovat (hodiny, 1–48).
    pub interval_h: u32,
    /// Novou verzi hned po kontrole tiše stáhnout a ověřit (SHA256) na pozadí — v šuplíku pak stačí
    /// „Restartovat a aktualizovat“. Platí jen se zapnutým `check`; nic se samo neinstaluje. Chybí = zapnuto.
    pub prefetch: bool,
}

impl Default for UpdateConfig {
    fn default() -> Self {
        Self { check: true, repo: "jiri-hyan/ledge-app".into(), interval_h: 6, prefetch: true }
    }
}

/// `owner/name` z GitHubu: dvě neprázdné části z `[A-Za-z0-9._-]`, ne `.`/`..`.
pub fn valid_repo(s: &str) -> bool {
    let part = |p: &str| !p.is_empty() && p != "." && p != ".." && p.bytes().all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'_' | b'-'));
    let mut it = s.split('/');
    matches!((it.next(), it.next(), it.next()), (Some(o), Some(n), None) if part(o) && part(n))
}
