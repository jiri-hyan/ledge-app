//! Stav „co je s configem špatně“: vadné sekce (načteno po sekcích), hodnoty mimo rozsah (varování,
//! nikdy chyba — starý config se musí dál načíst) a kopie rozbitého souboru z posledního zápisu.
//! Čisté funkce (`sections_message`, `range_warnings`, `prune_list`) jsou pod tabulkovými testy;
//! stav je jedna globální položka čtená snapshotem (UI banner) a desk widgetem.

use super::Config;
use crate::i18n::tr;
#[cfg(not(test))]
use parking_lot::Mutex;
use serde::Serialize;

/// Kolik kopií `config.json.broken-*` zůstává (nejnovější); starší se mažou až po vytvoření nové.
pub const KEEP_BROKEN: usize = 5;

/// Vadná top-level sekce: klíč a první chyba.
#[derive(Debug, Clone, PartialEq)]
pub struct SectionIssue {
    pub key: String,
    pub error: String,
}

/// Hodnota mimo rozsah, který Ledge doopravdy použije. Nikdy nebrání načtení ani uložení.
#[derive(Debug, Clone, PartialEq)]
pub struct RangeWarning {
    pub path: &'static str,
    pub value: u64,
    pub min: u64,
    pub max: u64,
    /// co Ledge použije místo toho (`None` = nic se neořízne, ale hodnota nefunguje)
    pub effective: Option<u64>,
}

struct State {
    parse_error: Option<String>,
    bad: Vec<SectionIssue>,
    warnings: Vec<RangeWarning>,
    backup: Option<std::path::PathBuf>,
    /// kopii jde obnovit (je to JSON, který se dá načíst); zjištěno jednou při `set_backup`
    restorable: bool,
}

const EMPTY: State = State { parse_error: None, bad: Vec::new(), warnings: Vec::new(), backup: None, restorable: false };

#[cfg(not(test))]
static STATE: Mutex<State> = Mutex::new(EMPTY);

// v testech má každé vlákno (= test) svůj stav, takže se paralelní testy o globální banner neperou
#[cfg(test)]
thread_local! {
    static STATE_TL: std::cell::RefCell<State> = const { std::cell::RefCell::new(EMPTY) };
}

#[cfg(not(test))]
fn with_state<R>(f: impl FnOnce(&mut State) -> R) -> R {
    f(&mut STATE.lock())
}

#[cfg(test)]
fn with_state<R>(f: impl FnOnce(&mut State) -> R) -> R {
    STATE_TL.with(|s| f(&mut s.borrow_mut()))
}

/// Pohled pro UI (`Snapshot.configNotice`); prázdný = nic neposílat.
#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct NoticeView {
    /// varování o hodnotách mimo rozsah (hotové věty)
    pub warnings: Vec<String>,
    /// cesta kopie původního configu z posledního zápisu (jde Obnovit)
    pub backup: Option<String>,
    /// kopie je platný JSON a jde obnovit (jinak UI schová tlačítko Obnovit)
    pub restorable: bool,
}

/// Výsledek načtení: chyba celého souboru, vadné sekce a varování. Přepíše vše kromě kopie.
pub fn set_load(parse_error: Option<String>, bad: Vec<SectionIssue>, warnings: Vec<RangeWarning>) {
    with_state(|s| {
        s.parse_error = parse_error;
        s.bad = bad;
        s.warnings = warnings;
    });
}

/// Platný config přijat (uložen / přečten): zastaralá chyba pryč, varování přepočítat.
pub fn set_applied(cfg: &Config) {
    set_load(None, Vec::new(), range_warnings(cfg));
}

pub fn set_backup(p: Option<std::path::PathBuf>) {
    let restorable = p.as_deref().is_some_and(|p| std::fs::read(p).is_ok_and(|b| is_restorable(&b)));
    with_state(|s| {
        s.backup = p;
        s.restorable = restorable;
    });
}

/// Obnova z kopie uspěje jen s JSONem, který se načte aspoň po sekcích (stejná podmínka jako `restore_backup`).
pub fn is_restorable(bytes: &[u8]) -> bool {
    let body = bytes.strip_prefix(&[0xEF, 0xBB, 0xBF]).unwrap_or(bytes);
    serde_json::from_slice(body).is_ok_and(|raw| super::lenient_from_value(&raw).is_ok())
}

pub fn backup() -> Option<std::path::PathBuf> {
    with_state(|s| s.backup.clone())
}

/// Text chyby configu pro banner (celý soubor nejde přečíst / sekce na výchozích), jinak `None`.
pub fn error() -> Option<String> {
    with_state(|s| s.parse_error.clone().or_else(|| sections_message(&s.bad)))
}

/// Chyba pro banner v UI: nejdřív chyba configu (soubor / sekce), pak chyba kořene poznámek, pak chyba
/// mostu Clauda; neprázdné se spojí mezerou, takže jedna nezakryje druhou. Čistá funkce.
pub fn join_errors(parts: &[Option<String>]) -> Option<String> {
    let v: Vec<&str> = parts
        .iter()
        .flatten()
        .map(|s| s.as_str())
        .filter(|s| !s.trim().is_empty())
        .collect();
    (!v.is_empty()).then(|| v.join(" "))
}

/// Všechny chyby pro UI: config (`error`), `cfg_error` jádra (kořen poznámek) a `bridge_error` (most Clauda).
pub fn shown_error(cfg_error: Option<String>, bridge_error: Option<String>) -> Option<String> {
    join_errors(&[error(), cfg_error, bridge_error])
}

pub fn view() -> Option<NoticeView> {
    with_state(|s| {
        let warnings: Vec<String> = s.warnings.iter().map(warning_message).collect();
        let backup = s.backup.as_ref().map(|p| p.display().to_string());
        (!warnings.is_empty() || backup.is_some()).then_some(NoticeView { warnings, backup, restorable: s.restorable })
    })
}

/// Věta o vadných sekcích; prázdný seznam = `None`.
pub fn sections_message(bad: &[SectionIssue]) -> Option<String> {
    if bad.is_empty() {
        return None;
    }
    let list = bad
        .iter()
        .map(|b| format!("{} ({})", b.key, b.error))
        .collect::<Vec<_>>()
        .join("; ");
    Some(trf!(
        "V config.json je neplatné: {list}. Ostatní sekce jsou načtené, vadné běží na výchozích hodnotách. Soubor zůstává nedotčený; při prvním uložení se nejdřív zazálohuje (config.json.broken-…).",
        "Invalid in config.json: {list}. The other sections are loaded; the bad ones run on defaults. The file stays untouched; the first save backs it up first (config.json.broken-…)."
    ))
}

/// Banner při vadném ručním zápisu / reloadu za běhu: nic z úpravy neplatí, běží poslední platná verze
/// (na rozdíl od `sections_message`, kde vadné sekce běží na výchozích). Bez vadných sekcí je to chyba parseru.
pub fn kept_message(bad: &[SectionIssue], parse_error: &str) -> String {
    match bad.iter().map(|b| format!("{} ({})", b.key, b.error)).collect::<Vec<_>>().join("; ") {
        list if list.is_empty() => trf!(
            "{parse_error} Úprava se nepoužila, běží poslední platná verze. Soubor zůstává nedotčený.",
            "{parse_error} The edit was not applied; the last valid version keeps running. The file stays untouched."
        ),
        list => trf!(
            "V config.json je neplatné: {list}. Úprava se nepoužila, běží poslední platná verze. Soubor zůstává nedotčený; při prvním uložení se nejdřív zazálohuje (config.json.broken-…).",
            "Invalid in config.json: {list}. The edit was not applied; the last valid version keeps running. The file stays untouched; the first save backs it up first (config.json.broken-…)."
        ),
    }
}

/// Rozsahy `claude.*`, které kód doopravdy vynucuje (`claude/mod.rs` `MAX_APPROVAL`, `unseen::clamp_s`,
/// `questions::MAX_HOLD_S`); test níže hlídá shodu.
pub const APPROVAL_S: (u64, u64) = (5, 550);
pub const HOLD_S: (u64, u64) = (0, 550);
pub const UNSEEN_S: (u64, u64) = (5, 550);
pub const PORT: (u64, u64) = (1024, 65535);

/// Hodnoty `claude.*` mimo rozsah. Čistá funkce; starý config se i tak načte (ořez dělá až poslouchač).
pub fn range_warnings(cfg: &Config) -> Vec<RangeWarning> {
    let c = &cfg.claude;
    let mut out = Vec::new();
    let mut check = |path: &'static str, v: u64, (min, max): (u64, u64), effective: Option<u64>| {
        if v < min || v > max {
            out.push(RangeWarning {
                path,
                value: v,
                min,
                max,
                effective,
            });
        }
    };
    check(
        "claude.approvalTimeoutS",
        c.approval_timeout_s,
        APPROVAL_S,
        Some(c.approval_timeout_s.clamp(APPROVAL_S.0, APPROVAL_S.1)),
    );
    check(
        "claude.questionHoldS",
        c.question_hold_s,
        HOLD_S,
        Some(c.question_hold_s.min(HOLD_S.1)),
    );
    // 0 = pojistka vypnutá (platná hodnota)
    if c.question_unseen_s != 0 {
        check(
            "claude.questionUnseenS",
            c.question_unseen_s,
            UNSEEN_S,
            Some(c.question_unseen_s.clamp(UNSEEN_S.0, UNSEEN_S.1)),
        );
    }
    check("claude.port", u64::from(c.port), PORT, None);
    out
}

pub fn warning_message(w: &RangeWarning) -> String {
    let (path, value, min, max) = (w.path, w.value, w.min, w.max);
    match w.effective {
        Some(e) => trf!(
            "{path} = {value} je mimo rozsah {min}–{max}, Ledge použije {e}.",
            "{path} = {value} is outside {min}–{max}; Ledge uses {e}."
        ),
        None => trf!(
            "{path} = {value} je mimo rozsah {min}–{max}; hooky Clauda most nenajdou (výchozí 47800).",
            "{path} = {value} is outside {min}–{max}; the Claude hooks won't find the bridge (default 47800)."
        ),
    }
}

/// Které kopie `<jméno>.broken-<unix s>[-<n>]` smazat: nechá `keep` nejnovějších (min. 1) a nikdy
/// kopii `protect` (právě vytvořenou). Řazení podle (s, n) z názvu; cizí názvy se nedotknou.
pub fn prune_list(names: &[String], base: &str, keep: usize, protect: &str) -> Vec<String> {
    let prefix = format!("{base}.broken-");
    let mut found: Vec<((u64, u64), &String)> = names
        .iter()
        .filter_map(|n| {
            let rest = n.strip_prefix(&prefix)?;
            let (secs, idx) = match rest.split_once('-') {
                Some((s, i)) => (s.parse().ok()?, i.parse().ok()?),
                None => (rest.parse().ok()?, 1),
            };
            Some(((secs, idx), n))
        })
        .collect();
    found.sort();
    let keep = keep.max(1);
    let cut = found.len().saturating_sub(keep);
    found
        .into_iter()
        .take(cut)
        .map(|(_, n)| n)
        .filter(|n| n.as_str() != protect)
        .cloned()
        .collect()
}

/// Smaže staré kopie vedle `path` (volat až po úspěšném vytvoření `new`). Chyby maže tiše — kopie navíc nevadí.
pub fn prune_broken(path: &std::path::Path, new: &std::path::Path) {
    let (Some(dir), Some(base)) = (
        path.parent(),
        path.file_name().map(|n| n.to_string_lossy().into_owned()),
    ) else {
        return;
    };
    let Ok(rd) = std::fs::read_dir(dir) else {
        return;
    };
    let names: Vec<String> = rd
        .filter_map(|e| e.ok())
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .collect();
    let protect = new
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    for n in prune_list(&names, &base, KEEP_BROKEN, &protect) {
        let _ = std::fs::remove_file(dir.join(n));
    }
}

/// Text pro lidi, když obnova nejde.
pub fn no_backup() -> &'static str {
    tr(
        "Kopie původního configu už neexistuje.",
        "The copy of the original config no longer exists.",
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn names(v: &[&str]) -> Vec<String> {
        v.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn prune_table() {
        let b = "config.json";
        let all = names(&[
            "config.json",
            "config.json.1",
            "config.json.broken-10",
            "config.json.broken-20",
            "config.json.broken-20-2",
            "config.json.broken-30",
            "config.json.broken-9",
            "config.json.broken-x",
            "jiny.broken-1",
        ]);
        // keep 3 z pěti platných: nejstarší 9 a 10 pryč
        assert_eq!(
            prune_list(&all, b, 3, ""),
            names(&["config.json.broken-9", "config.json.broken-10"])
        );
        // pod limitem se nic nemaže
        assert!(prune_list(&all, b, 5, "").is_empty());
        assert!(prune_list(&all, b, 9, "").is_empty());
        // keep 0 se bere jako 1: poslední nikdy neodejde
        let p = prune_list(&all, b, 0, "");
        assert_eq!(p.len(), 4);
        assert!(!p.contains(&"config.json.broken-30".to_string()));
        // chráněná (právě vytvořená) se nemaže, ani kdyby byla „stará“
        assert!(
            !prune_list(&all, b, 1, "config.json.broken-9")
                .contains(&"config.json.broken-9".to_string())
        );
        // číselné řazení (9 < 10), ne textové
        assert_eq!(
            prune_list(&names(&["c.broken-9", "c.broken-10"]), "c", 1, ""),
            names(&["c.broken-9"])
        );
        assert!(prune_list(&[], b, 3, "").is_empty());
    }

    #[test]
    fn section_message_lists_all() {
        assert_eq!(sections_message(&[]), None);
        let m = sections_message(&[
            SectionIssue {
                key: "strip".into(),
                error: "chyba 1".into(),
            },
            SectionIssue {
                key: "hotkeys".into(),
                error: "chyba 2".into(),
            },
        ])
        .unwrap();
        assert!(m.contains("strip (chyba 1); hotkeys (chyba 2)"), "{m}");
    }

    #[test]
    fn restorable_table() {
        let cases: [(&str, &[u8], bool); 6] = [
            ("platný", &b"{\"autostart\":false}"[..], true),
            ("s BOM", &[0xEF, 0xBB, 0xBF, b'{', b'}'][..], true),
            ("rozbitý JSON", b"{\"projects\":[", false),
            ("prázdný", b"", false),
            ("kořen není objekt", b"[1]", false),
            ("ne UTF-8", &[0xFF, 0xFE, b'{', b'}'][..], false),
        ];
        for (what, bytes, want) in cases {
            assert_eq!(is_restorable(bytes), want, "{what}");
        }
        let d = tempfile::tempdir().unwrap();
        let ok = d.path().join("a");
        let bad = d.path().join("b");
        std::fs::write(&ok, "{}").unwrap();
        std::fs::write(&bad, "{ rozbité").unwrap();
        set_backup(Some(ok));
        assert!(view().unwrap().restorable);
        set_backup(Some(bad));
        assert!(!view().unwrap().restorable);
        set_backup(Some(d.path().join("chybi")));
        assert!(!view().unwrap().restorable);
        set_backup(None);
    }

    #[test]
    fn kept_message_says_last_good_runs() {
        let bad = [SectionIssue { key: "strip".into(), error: "chyba 1".into() }];
        let m = kept_message(&bad, "x");
        assert!(m.contains("strip (chyba 1)") && m.contains("poslední platná verze"), "{m}");
        assert!(!m.contains("na výchozích"), "{m}");
        let m = kept_message(&[], "Rozbitý JSON.");
        assert!(m.starts_with("Rozbitý JSON.") && m.contains("poslední platná verze"), "{m}");
        assert!(!m.contains("výchozích"), "{m}");
    }

    #[test]
    fn range_table() {
        let mk = |f: &dyn Fn(&mut Config)| {
            let mut c = Config::default();
            f(&mut c);
            range_warnings(&c)
        };
        assert!(mk(&|_| {}).is_empty(), "výchozí hodnoty nehlásí nic");
        let w = mk(&|c| c.claude.approval_timeout_s = 1);
        assert_eq!(
            (w[0].path, w[0].effective),
            ("claude.approvalTimeoutS", Some(5))
        );
        let w = mk(&|c| c.claude.approval_timeout_s = 9000);
        assert_eq!(w[0].effective, Some(550));
        assert!(
            mk(&|c| c.claude.approval_timeout_s = 5).is_empty()
                && mk(&|c| c.claude.approval_timeout_s = 550).is_empty()
        );
        assert_eq!(
            mk(&|c| c.claude.question_hold_s = 551)[0].effective,
            Some(550)
        );
        assert!(mk(&|c| c.claude.question_hold_s = 0).is_empty());
        assert!(
            mk(&|c| c.claude.question_unseen_s = 0).is_empty(),
            "0 = vypnuto"
        );
        assert_eq!(
            mk(&|c| c.claude.question_unseen_s = 2)[0].effective,
            Some(5)
        );
        let w = mk(&|c| c.claude.port = 0);
        assert_eq!((w[0].path, w[0].effective), ("claude.port", None));
        assert_eq!(mk(&|c| c.claude.port = 1023).len(), 1);
        assert!(
            mk(&|c| c.claude.port = 1024).is_empty() && mk(&|c| c.claude.port = 65535).is_empty()
        );
        assert_eq!(
            mk(&|c| {
                c.claude.port = 80;
                c.claude.approval_timeout_s = 1;
            })
            .len(),
            2
        );
        assert!(warning_message(&w[0]).contains("claude.port = 0"));
    }

    #[test]
    fn join_errors_table() {
        let s = |x: &str| Some(x.to_string());
        let table: [(Vec<Option<String>>, Option<&str>); 6] = [
            (vec![], None),
            (vec![None, None, None], None),
            (vec![None, None, s("Most neběží.")], Some("Most neběží.")),
            (
                vec![None, s("Kořen."), s("Most neběží.")],
                Some("Kořen. Most neběží."),
            ),
            (
                vec![s("Config."), None, s("Most neběží.")],
                Some("Config. Most neběží."),
            ),
            (vec![s(""), s("  "), None], None),
        ];
        for (parts, want) in table {
            assert_eq!(join_errors(&parts).as_deref(), want, "{parts:?}");
        }
    }

    #[test]
    fn ranges_match_the_code_that_clamps() {
        assert_eq!(APPROVAL_S.1, crate::claude::MAX_APPROVAL.as_secs());
        assert_eq!(UNSEEN_S.0, crate::claude::unseen::MIN_S);
        assert_eq!(UNSEEN_S.1, HOLD_S.1);
        for (v, want) in [(0, 0), (1, UNSEEN_S.0), (9999, UNSEEN_S.1)] {
            assert_eq!(crate::claude::unseen::clamp_s(v), want);
        }
    }
}
