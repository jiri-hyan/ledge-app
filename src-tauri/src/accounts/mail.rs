//! Pošta účtu Microsoft (Graph, `Mail.ReadBasic` — bez těla a příloh): počet nepřečtených v Doručené
//! poště a 5 nejnovějších zpráv (předmět, odesílatel, čas, odkaz do Outlooku na webu).
//!
//! Obnova: s viditelnou kartou Pošta nejvýš 1× za `CARD_EVERY` (2 min); s položkou proužku `mail`
//! (styl Klasický) i se zavřeným šuplíkem nejvýš 1× za `STRIP_EVERY` (5 min) — to je jediné čtení na
//! pozadí a zapíná ho jen výslovně přidaná položka. Ve fullscreen hře nic.

use super::{GRAPH, Provider, session};
use crate::config::{Config, StripItem};
use crate::drawer::View;
use crate::state::{core, mark_dirty, now_ms};
use chrono::DateTime;
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};

pub const CARD_EVERY: i64 = 2 * 60 * 1000;
pub const STRIP_EVERY: i64 = 5 * 60 * 1000;
/// Po neúspěchu aspoň tolik pauza.
pub const RETRY_MS: i64 = 2 * 60 * 1000;
const STUCK_MS: i64 = 60_000;
pub const ROWS: usize = 5;
/// Outlook na webu, když zprávy nemají vlastní odkaz.
pub const OUTLOOK_WEB: &str = "https://outlook.office.com/mail/";

#[derive(Serialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct MailItem {
    pub subject: String,
    pub from: String,
    pub received_ms: i64,
    pub unread: bool,
    /// odkaz na zprávu v Outlooku na webu (`webLink`, jen https)
    pub link: Option<String>,
}

#[derive(Default)]
struct St {
    unread: Option<u32>,
    items: Vec<MailItem>,
    fetched_ms: Option<i64>,
    attempt_ms: Option<i64>,
    loading: bool,
    error: Option<String>,
    account: String,
}

static S: Mutex<Option<St>> = Mutex::new(None);

#[derive(Serialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct MailView {
    /// off (karta skrytá / pošta vypnutá) · unset (účet nepřihlášený) · idle · loading · ok · stale · unavailable
    pub status: &'static str,
    pub account: String,
    pub unread: Option<u32>,
    pub items: Vec<MailItem>,
    pub fetched_ms: Option<i64>,
    pub error: Option<String>,
    pub loading: bool,
    /// kam vede „Otevřít Outlook“
    pub web: String,
}

// ---------- čisté ----------

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Folder {
    unread_item_count: u32,
}

pub fn parse_folder(body: &[u8]) -> Option<u32> {
    serde_json::from_slice::<Folder>(body).ok().map(|f| f.unread_item_count)
}

#[derive(Deserialize, Default)]
struct Addr {
    #[serde(default)]
    name: String,
    #[serde(default)]
    address: String,
}

#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct From {
    #[serde(default)]
    email_address: Addr,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Msg {
    #[serde(default)]
    subject: Option<String>,
    #[serde(default)]
    is_read: bool,
    #[serde(default)]
    received_date_time: String,
    #[serde(default)]
    from: Option<From>,
    #[serde(default)]
    web_link: Option<String>,
}

#[derive(Deserialize)]
struct Msgs {
    #[serde(default)]
    value: Vec<Msg>,
}

pub fn parse_messages(body: &[u8]) -> Option<Vec<MailItem>> {
    let m: Msgs = serde_json::from_slice(body).ok()?;
    Some(
        m.value
            .into_iter()
            .take(ROWS)
            .map(|x| {
                let f = x.from.unwrap_or_default().email_address;
                MailItem {
                    subject: crate::calendar::clip(x.subject.as_deref().unwrap_or(""), 120),
                    from: crate::calendar::clip(if f.name.trim().is_empty() { &f.address } else { &f.name }, 60),
                    received_ms: DateTime::parse_from_rfc3339(x.received_date_time.trim()).map(|d| d.timestamp_millis()).unwrap_or(0),
                    unread: !x.is_read,
                    link: crate::calendar::graph::https_only(x.web_link).filter(|l| l.starts_with("https://outlook.")),
                }
            })
            .collect(),
    )
}

/// Kam vede „Otevřít Outlook“: počátek odkazu první zprávy (osobní účet = outlook.live.com,
/// pracovní = outlook.office365.com), jinak Outlook na webu obecně.
pub fn web_of(items: &[MailItem]) -> String {
    items
        .iter()
        .find_map(|i| i.link.as_deref())
        .and_then(|l| l.strip_prefix("https://").and_then(|r| r.split('/').next()).map(|host| format!("https://{host}/mail/")))
        .unwrap_or_else(|| OUTLOOK_WEB.into())
}

/// Stáhnout? Ne během pokusu, ne dřív než `RETRY_MS` po pokusu, jinak když jsou data starší než `every`.
pub fn due(now: i64, fetched: Option<i64>, attempt: Option<i64>, loading: bool, every: i64) -> bool {
    if loading || attempt.is_some_and(|a| now - a < RETRY_MS.min(every)) {
        return false;
    }
    fetched.is_none_or(|f| now - f >= every)
}

/// Jak často obnovovat: karta vidět → 2 min, jen položka proužku → 5 min, jinak vůbec.
pub fn cadence(card_visible: bool, strip_item: bool) -> Option<i64> {
    if card_visible {
        Some(CARD_EVERY)
    } else if strip_item {
        Some(STRIP_EVERY)
    } else {
        None
    }
}

/// Je položka `mail` v proužku vidět (styl Klasický, proužek zapnutý)?
pub fn strip_wants(cfg: &Config) -> bool {
    cfg.strip.enabled && cfg.strip.look == crate::looks::Look::Classic && cfg.strip.items.contains(&StripItem::Mail)
}

// ---------- za běhu ----------

fn every_now(cfg: &Config, opening: bool) -> Option<i64> {
    let c = core();
    if !cfg.accounts.microsoft.mail || c.dead() {
        return None;
    }
    let card = crate::layout::page_has(&cfg.layout, &c.mode.lock(), "mail") && (opening || c.drawer.view() == View::Full);
    cadence(card, strip_wants(cfg))
}

pub(crate) fn maybe_refresh(opening: bool) {
    let every = {
        let cfg = core().cfg.read();
        every_now(&cfg, opening)
    };
    let Some(every) = every else { return };
    if !session::connected(Provider::Microsoft) {
        return;
    }
    let now = now_ms();
    let go = {
        let mut g = S.lock();
        let st = g.get_or_insert_with(St::default);
        if st.loading && st.attempt_ms.is_some_and(|a| now - a >= STUCK_MS) {
            st.loading = false;
        }
        let go = due(now, st.fetched_ms, st.attempt_ms, st.loading, every);
        if go {
            (st.loading, st.attempt_ms) = (true, Some(now));
        }
        go
    };
    if !go {
        return;
    }
    mark_dirty();
    tauri::async_runtime::spawn(async move {
        let res = fetch().await;
        let account = session::account(Provider::Microsoft);
        let mut g = S.lock();
        let Some(st) = g.as_mut() else { return };
        if st.attempt_ms != Some(now) {
            return;
        }
        st.loading = false;
        st.account = account;
        match res {
            Ok((n, items)) => {
                (st.unread, st.items, st.fetched_ms, st.error) = (Some(n), items, Some(now_ms()), None);
            }
            Err(e) => st.error = Some(e),
        }
        drop(g);
        mark_dirty();
    });
}

async fn fetch() -> Result<(u32, Vec<MailItem>), String> {
    let bad = || crate::i18n::tr("Pošta Microsoft vrátila nečitelná data.", "Microsoft mail returned unreadable data.").to_string();
    let f = session::api_get(Provider::Microsoft, &format!("{GRAPH}/me/mailFolders/inbox?$select=unreadItemCount"), &[]).await?;
    let n = parse_folder(&f.body).ok_or_else(bad)?;
    let url = format!("{GRAPH}/me/mailFolders/inbox/messages?$top={ROWS}&$orderby=receivedDateTime%20desc&$select=subject,from,receivedDateTime,isRead,webLink");
    let m = session::api_get(Provider::Microsoft, &url, &[]).await?;
    Ok((n, parse_messages(&m.body).ok_or_else(bad)?))
}

pub fn forget() {
    *S.lock() = None;
}

pub(crate) fn on_config(old: &Config, new: &Config) {
    if old.accounts.microsoft.mail && !new.accounts.microsoft.mail {
        forget();
    }
    if old.accounts.microsoft.mail != new.accounts.microsoft.mail || old.layout != new.layout || old.strip.items != new.strip.items || old.strip.look != new.strip.look {
        maybe_refresh(false);
        mark_dirty();
    }
}

/// Termín pro emit vlákno: příští obnova (jen když je kvůli kartě nebo proužku potřeba) nebo watchdog.
pub(crate) fn next_deadline() -> Option<i64> {
    let every = {
        let cfg = core().cfg.read();
        every_now(&cfg, false)
    }?;
    let g = S.lock();
    let st = g.as_ref()?;
    if st.loading {
        return st.attempt_ms.map(|a| a + STUCK_MS + 1);
    }
    let after_attempt = st.attempt_ms.map(|a| a + RETRY_MS.min(every));
    let after_fetch = st.fetched_ms.map(|f| f + every);
    [after_attempt, after_fetch].into_iter().flatten().max()
}

/// Pro snapshot — jen paměť (nesahá na Správce pověření ani `core().cfg`).
pub(crate) fn view(cfg: &Config, now: i64) -> MailView {
    let off = |status| MailView { status, account: String::new(), unread: None, items: vec![], fetched_ms: None, error: None, loading: false, web: OUTLOOK_WEB.into() };
    if !cfg.drawer.sections.mail || !cfg.accounts.microsoft.mail {
        return off("off");
    }
    let g = S.lock();
    let Some(st) = g.as_ref() else { return off("unset") };
    let loading = st.loading && st.attempt_ms.is_some_and(|a| now - a < STUCK_MS);
    let old = st.fetched_ms.is_none_or(|f| now - f >= 3 * CARD_EVERY);
    MailView {
        status: crate::calendar::live::feed_status(st.fetched_ms.is_some(), st.error.is_some(), loading, old),
        account: st.account.clone(),
        unread: st.unread,
        items: st.items.clone(),
        fetched_ms: st.fetched_ms,
        error: st.error.clone(),
        loading,
        web: web_of(&st.items),
    }
}

/// Segment proužku: „✉ 3“ jen při nepřečtených > 0 (jen paměť).
pub(crate) fn strip_segment(cfg: &Config) -> Option<crate::strip::Segment> {
    if !cfg.accounts.microsoft.mail {
        return None;
    }
    let n = S.lock().as_ref()?.unread?;
    segment(n)
}

pub fn segment(n: u32) -> Option<crate::strip::Segment> {
    use crate::strip::{Rgba, Segment};
    (n > 0).then(|| Segment { text: format!("✉ {}", if n > 999 { "999+".to_string() } else { n.to_string() }), color: Rgba::DIM, dot: Some(Rgba::BLUE), strong: false, pill: false })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_saved_samples() {
        assert_eq!(parse_folder(br#"{"@odata.context":"x","unreadItemCount":7}"#), Some(7));
        assert_eq!(parse_folder(b"{}"), None);
        let items = parse_messages(include_bytes!("../fixtures/graph-messages.json")).unwrap();
        let t: Vec<(&str, &str, bool, bool)> = items.iter().map(|i| (i.subject.as_str(), i.from.as_str(), i.unread, i.link.is_some())).collect();
        assert_eq!(
            t,
            vec![
                ("Faktura za říjen", "Účtárna", true, true),
                ("Re: oběd", "petr@example.com", false, true),
                ("(bez předmětu na řádku)", "Jana", true, false),
                ("", "", false, false),
                ("Pátá", "Pět", false, true),
            ],
            "nejvýš 5, tělo se nečte"
        );
        assert_eq!(items[0].received_ms, 1_792_306_800_000);
        assert_eq!(web_of(&items), "https://outlook.office365.com/mail/");
        assert_eq!(web_of(&[]), OUTLOOK_WEB);
    }

    #[test]
    fn refresh_policy_table() {
        let now = 10_000_000_000;
        // (data, pokus, běží, každých, stáhnout?)
        let cases = [
            (None, None, false, CARD_EVERY, true),
            (Some(now - CARD_EVERY + 1), Some(now - CARD_EVERY + 1), false, CARD_EVERY, false),
            (Some(now - CARD_EVERY), Some(now - CARD_EVERY), false, CARD_EVERY, true),
            (Some(now - CARD_EVERY), Some(now - CARD_EVERY), false, STRIP_EVERY, false),
            (None, Some(now - 1000), false, CARD_EVERY, false),
            (None, None, true, CARD_EVERY, false),
        ];
        for (f, a, l, every, want) in cases {
            assert_eq!(due(now, f, a, l, every), want, "{f:?} {a:?} {l} {every}");
        }
        assert_eq!(cadence(true, true), Some(CARD_EVERY));
        assert_eq!(cadence(false, true), Some(STRIP_EVERY));
        assert_eq!(cadence(false, false), None, "bez karty a položky proužku se nic nečte");
    }

    #[test]
    fn strip_segment_only_when_unread() {
        assert!(segment(0).is_none());
        assert_eq!(segment(3).unwrap().text, "✉ 3");
        assert_eq!(segment(5000).unwrap().text, "✉ 999+");
        let mut cfg = Config::default();
        assert!(!strip_wants(&cfg), "výchozí proužek poštu nemá");
        cfg.strip.items.push(StripItem::Mail);
        assert!(strip_wants(&cfg));
        cfg.strip.look = crate::looks::Look::Dots;
        assert!(!strip_wants(&cfg), "jiný styl než Klasický položky neukazuje");
    }
}
