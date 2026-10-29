//! Limity Claude dotazem na Anthropic (volba `claude.limitsFetch`, výchozí VYPNUTO) — pro práci v aplikaci
//! Claude (karta Code), kde stavový řádek Ledge neběží.
//!
//! Jediné volání sítě: `GET https://api.anthropic.com/api/oauth/usage` (totéž, co dělá `/usage` v Claude
//! Code) s hlavičkami `Authorization: Bearer <token>` a `anthropic-beta: oauth-2025-04-20`. Pevná URL,
//! přesměrování se NEsleduje (3xx = chyba), termín 10 s, odpověď ≤ 64 kB.
//!
//! Token: při každém dotazu se znovu přečte `~/.claude/.credentials.json` a vezme se jen
//! `claudeAiOauth.accessToken` (+ `expiresAt`, `limitsrc::token_of`). Žádný refresh, žádný zápis do souboru,
//! token se nikam neukládá ani neloguje (hlavička je `sensitive`), žije jen po dobu dotazu. Vypršel / 401 =
//! tichý stav „přihlas se v Claude Code“ a další pokus nejdřív za 30 min nebo po změně souboru přihlášení.
//!
//! Kdy: jen se zapnutou volbou, mimo hru / herní režim, když je karta Claude s limity vidět (šuplík naplno)
//! nebo proužek limity ukazuje; nejvýš jednou za `EVERY_MS` a vůbec ne, když je vzorek ze stavového řádku
//! mladší (`limitsrc::due`). Žádný vlastní časovač — volá ho `emit` při probuzení a otevření šuplíku.
//! Výsledek jde do stejné cache jako wrapper (`ratelimits.json`, `"via":"api"`), zbytek Ledge se nemění.

use crate::limitsrc::{self, Outcome, TokenErr};
use crate::state::{core, log, mark_dirty, now_ms};
use parking_lot::Mutex;
use std::time::Duration;

pub(crate) const URL: &str = "https://api.anthropic.com/api/oauth/usage";
const BETA: &str = "oauth-2025-04-20";
/// Běžný odstup dotazů (popis volby v Nastavení říká „jednou za 10 min“).
pub(crate) const EVERY_MS: i64 = 10 * 60_000;
const TIMEOUT: Duration = Duration::from_secs(10);
const MAX_BODY: usize = 64 * 1024;
const MAX_CREDS: u64 = 256 * 1024;

type FileKey = Option<(Option<std::time::SystemTime>, u64)>;

#[derive(Default)]
struct State {
    attempt: Option<i64>,
    /// čas posledního pokusu; přežije vypnutí volby (vypnout+zapnout neobejde odstup)
    last_ms: Option<i64>,
    loading: bool,
    outcome: Option<Outcome>,
    /// odstup dalšího pokusu po posledním výsledku
    wait_ms: i64,
    /// soubor přihlášení při neúspěchu přihlášení: změní se (nové přihlášení) = zkusit hned
    creds_key: FileKey,
}

static STATE: Mutex<State> = Mutex::new(State { attempt: None, last_ms: None, loading: false, outcome: None, wait_ms: EVERY_MS, creds_key: None });

fn creds_path() -> Option<std::path::PathBuf> {
    crate::dirs::claude_dir().map(|d| d.join(".credentials.json"))
}

fn creds_key() -> FileKey {
    let m = std::fs::metadata(creds_path()?).ok()?;
    Some((m.modified().ok(), m.len()))
}

/// Stav pro kartu (`LimitsView.fetch`): "" = volba vypnutá, `pending` = první dotaz běží / ještě nebyl,
/// jinak `Outcome::key` (ok | noLogin | expired | error).
pub(crate) fn view_key(enabled: bool) -> &'static str {
    if !enabled {
        return "";
    }
    STATE.lock().outcome.map_or("pending", Outcome::key)
}

/// Je co aktualizovat? Volba, ne ve hře, karta Claude s limity vidět nebo limity v proužku.
fn wanted(opening: bool) -> bool {
    use crate::drawer::View;
    let c = core();
    let cfg = c.cfg.read();
    if !cfg.claude.limits_fetch || c.dead() {
        return false;
    }
    let card = (opening || c.drawer.view() == View::Full) && cfg.drawer.sections.claude_limits && crate::layout::page_has(&cfg.layout, &c.mode.lock(), "claude");
    let strip = cfg.strip.enabled && cfg.strip.items.contains(&crate::config::StripItem::Limits);
    card || strip
}

/// Zeptat se, když je čas (volá `emit` při probuzení, otevření šuplíku a změně záložky). Jinak nic.
pub(crate) fn maybe_refresh(opening: bool) {
    if !wanted(opening) {
        return;
    }
    let now = now_ms();
    let newest = crate::limitslive::newest_full_at();
    {
        let mut st = STATE.lock();
        // po neúspěchu přihlášení: nový soubor přihlášení = zkusit hned (ale ne častěji než 1× za minutu)
        if matches!(st.outcome, Some(Outcome::Expired | Outcome::NoLogin)) && st.last_ms.is_none_or(|l| now - l >= crate::incidents::MIN_RETRY_MS) && creds_key() != st.creds_key {
            st.attempt = None;
        }
        if !limitsrc::due(now, st.attempt, st.loading, st.wait_ms, newest, EVERY_MS) || !crate::incidents::retry_floor_ok(now, st.last_ms) {
            return;
        }
        st.attempt = Some(now);
        st.last_ms = Some(now);
        st.loading = true;
    }
    tauri::async_runtime::spawn(async move {
        let out = fetch(now).await;
        {
            let mut st = STATE.lock();
            if st.attempt != Some(now) {
                return; // novější pokus nebo vypnuto
            }
            st.loading = false;
            st.wait_ms = out.next_after_ms(EVERY_MS);
            st.creds_key = if matches!(out, Outcome::Expired | Outcome::NoLogin) { creds_key() } else { None };
            if st.outcome != Some(out) && out != Outcome::Ok {
                log(&format!("limity (Anthropic): {}", out.key()));
            }
            st.outcome = Some(out);
        }
        mark_dirty();
    });
}

/// Volba se změnila (i `limitsDesktop` → přečíst zdroje znovu): vypnuto = zapomenout stav (ne čas pokusu); zapnuto = zeptat se hned (s mezí 1 min).
pub(crate) fn on_config(old: &crate::config::Config, new: &crate::config::Config) {
    if old.claude.limits_desktop != new.claude.limits_desktop {
        crate::limitslive::invalidate(); // zdroj z aplikace Claude přečíst / zahodit hned
    }
    if old.claude.limits_fetch == new.claude.limits_fetch {
        return;
    }
    let last_ms = STATE.lock().last_ms;
    *STATE.lock() = State { last_ms, wait_ms: EVERY_MS, ..State::default() };
    if new.claude.limits_fetch {
        maybe_refresh(false);
    }
    mark_dirty();
}

/// Jeden dotaz: token → GET → vzorek do `ratelimits.json`. Token žije jen v téhle funkci.
async fn fetch(now: i64) -> Outcome {
    let Some(p) = creds_path() else { return Outcome::NoLogin };
    let text = match std::fs::metadata(&p) {
        Ok(m) if m.len() <= MAX_CREDS => match std::fs::read_to_string(&p) {
            Ok(t) => t,
            Err(_) => return Outcome::NoLogin,
        },
        _ => return Outcome::NoLogin,
    };
    let token = limitsrc::token_of(&text, now);
    drop(text);
    let token = match token {
        Ok(t) => t,
        Err(TokenErr::Missing) => return Outcome::NoLogin,
        Err(TokenErr::Expired) => return Outcome::Expired,
    };
    let body = match get(token).await {
        Ok(b) => b,
        Err(o) => return o,
    };
    let at = now_ms();
    let Some(s) = limitsrc::from_usage_api(&body, at) else { return Outcome::Error };
    let path = crate::statusline::sample_path();
    // novější vzorek (wrapper mezitím zapsal) se nepřepisuje
    let old = std::fs::read_to_string(&path).ok().and_then(|t| crate::limits::from_file(&t, crate::limits::Source::StatusLine));
    if old.is_none_or(|o| o.at < s.at) {
        if let Some(d) = path.parent() {
            let _ = std::fs::create_dir_all(d);
        }
        if crate::statuslinesetup::write_atomic(&path, limitsrc::to_file_api(&s).as_bytes()).is_err() {
            return Outcome::Error;
        }
        crate::limitslive::invalidate();
    }
    Outcome::Ok
}

async fn get(token: String) -> Result<String, Outcome> {
    let client = reqwest::Client::builder()
        .timeout(TIMEOUT)
        .redirect(reqwest::redirect::Policy::none())
        .user_agent(concat!("Ledge/", env!("CARGO_PKG_VERSION")))
        .build()
        .map_err(|_| Outcome::Error)?;
    let mut auth = reqwest::header::HeaderValue::from_str(&format!("Bearer {token}")).map_err(|_| Outcome::NoLogin)?;
    drop(token);
    auth.set_sensitive(true);
    let req = client
        .get(URL)
        .header(reqwest::header::AUTHORIZATION, auth)
        .header("anthropic-beta", BETA)
        .header(reqwest::header::ACCEPT, "application/json");
    let mut resp = req.send().await.map_err(|_| Outcome::Error)?;
    let out = limitsrc::outcome_of_status(resp.status().as_u16());
    if out != Outcome::Ok {
        return Err(out);
    }
    let mut buf = Vec::new();
    while let Some(chunk) = resp.chunk().await.map_err(|_| Outcome::Error)? {
        buf.extend_from_slice(&chunk);
        if buf.len() > MAX_BODY {
            return Err(Outcome::Error);
        }
    }
    Ok(String::from_utf8_lossy(&buf).into_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn url_is_fixed_anthropic_https() {
        assert!(URL.starts_with("https://api.anthropic.com/"));
        assert_eq!(reqwest::Url::parse(URL).unwrap().host_str(), Some("api.anthropic.com"));
        const { assert!(EVERY_MS >= 5 * 60_000, "nejvýš jednou za 5 min") };
    }

    #[test]
    fn view_key_off_and_pending() {
        assert_eq!(view_key(false), "");
        // čerstvý stav (žádný dotaz v testu neběží)
        assert!(matches!(view_key(true), "pending" | "ok" | "noLogin" | "expired" | "error"));
    }
}
