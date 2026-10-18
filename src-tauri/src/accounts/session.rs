//! Přihlášení za běhu: připojení účtu (prohlížeč + loopback + výměna kódu), access token v paměti
//! s automatickou obnovou, odhlášení (smazání ze Správce pověření + revokace u Google), stav pro
//! Nastavení. Záznam se ze Správce pověření čte líně — až když ho něco potřebuje (karta, Nastavení).
//!
//! Zámky: `ACC` (parking_lot) se nikdy nedrží přes `await`; obnovu tokenu serializuje asynchronní
//! zámek na poskytovatele (`refresh_lock`), ať dvě karty neobnovují najednou.

use super::oauth::{self, TokenFail};
use super::vault::{self, Stored, WinCred};
use super::{GCAL, GRAPH, Provider, http, loopback, pkce};
use crate::i18n::tr;
use crate::state::{core, log, mark_dirty, now_ms};
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use std::sync::LazyLock;
use std::sync::atomic::{AtomicBool, Ordering};

#[derive(Default)]
struct Acc {
    loaded: bool,
    stored: Option<Stored>,
    access: Option<(String, i64)>,
    connecting: bool,
    /// přihlášení vypršelo / bylo odvoláno → jen znovu přihlásit
    expired: bool,
    error: Option<String>,
    /// po přechodné chybě obnovy: nejdřív v tuhle chvíli znovu
    retry_at: Option<i64>,
}

static ACC: LazyLock<Mutex<[Acc; 2]>> = LazyLock::new(|| Mutex::new([Acc::default(), Acc::default()]));
static CANCEL: [AtomicBool; 2] = [AtomicBool::new(false), AtomicBool::new(false)];
static REFRESH: LazyLock<[tauri::async_runtime::Mutex<()>; 2]> = LazyLock::new(|| [tauri::async_runtime::Mutex::new(()), tauri::async_runtime::Mutex::new(())]);

fn ix(p: Provider) -> usize {
    match p {
        Provider::Microsoft => 0,
        Provider::Google => 1,
    }
}

fn with<R>(p: Provider, f: impl FnOnce(&mut Acc) -> R) -> R {
    let mut g = ACC.lock();
    let a = &mut g[ix(p)];
    if !a.loaded {
        a.loaded = true;
        match vault::load(&WinCred, p) {
            Ok(s) => a.stored = s,
            Err(e) => a.error = Some(e),
        }
    }
    f(a)
}

/// Je účet přihlášený (a přihlášení nevypršelo)? Levné — paměť (první volání přečte Správce pověření).
pub fn connected(p: Provider) -> bool {
    with(p, |a| a.stored.is_some() && !a.expired)
}

/// Jméno účtu pro hlášky a karty (e-mail; prázdné = nepřihlášeno).
pub fn account(p: Provider) -> String {
    with(p, |a| a.stored.as_ref().map(|s| s.account.clone()).unwrap_or_default())
}

/// Stav účtu pro Nastavení → Účty. Žádné tajemství (ani refresh token, ani client secret).
#[derive(Serialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AccountView {
    pub provider: &'static str,
    pub connected: bool,
    pub connecting: bool,
    pub expired: bool,
    pub account: String,
    pub name: String,
    /// udělená oprávnění (jak je vrátil poskytovatel)
    pub scopes: Vec<String>,
    /// Client ID, se kterým je účet přihlášený (není tajné)
    pub client_id: String,
    pub error: Option<String>,
}

fn view_of(p: Provider, a: &Acc) -> AccountView {
    let s = a.stored.as_ref();
    AccountView {
        provider: p.id(),
        connected: s.is_some(),
        connecting: a.connecting,
        expired: a.expired,
        account: s.map(|s| s.account.clone()).unwrap_or_default(),
        name: s.map(|s| s.name.clone()).unwrap_or_default(),
        scopes: s.map(|s| s.scope.split_whitespace().map(String::from).collect()).unwrap_or_default(),
        client_id: s.map(|s| s.client_id.clone()).unwrap_or_default(),
        error: a.error.clone(),
    }
}

pub fn status() -> Vec<AccountView> {
    Provider::ALL.into_iter().map(|p| with(p, |a| view_of(p, a))).collect()
}

/// Údaje účtu z profilu: (účet, jméno).
#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct MsMe {
    #[serde(default)]
    display_name: Option<String>,
    #[serde(default)]
    mail: Option<String>,
    #[serde(default)]
    user_principal_name: Option<String>,
}

pub fn parse_ms_me(body: &[u8]) -> Option<(String, String)> {
    let m: MsMe = serde_json::from_slice(body).ok()?;
    let acc = m.mail.filter(|s| !s.is_empty()).or(m.user_principal_name).filter(|s| !s.is_empty())?;
    Some((acc, m.display_name.unwrap_or_default()))
}

#[derive(Deserialize)]
struct GPrimary {
    id: String,
    #[serde(default)]
    summary: String,
}

/// Google: hlavní kalendář má jako id e-mail účtu (žádný scope navíc není potřeba).
pub fn parse_google_primary(body: &[u8]) -> Option<(String, String)> {
    let g: GPrimary = serde_json::from_slice(body).ok()?;
    (!g.id.is_empty()).then_some((g.id, g.summary))
}

fn fail(p: Provider, f: TokenFail, code: &str) -> String {
    log(&format!("účty: {} přihlášení selhalo ({})", p.id(), oauth::scrub(code, &[])));
    oauth::human(p, f)
}

/// Připojí účet: otevře přihlášení v systémovém prohlížeči a počká (nejvýš `loopback::WAIT`).
/// `client_id`/`secret`/`tenant` jsou z rozpracovaného Nastavení — záznam ve Správci pověření je pak
/// nese sám (obnova nezávisí na uložení configu).
pub async fn connect(p: Provider, client_id: String, secret: String, tenant: String) -> Result<AccountView, String> {
    let client_id = client_id.trim().to_string();
    let secret = secret.trim().to_string();
    let tenant = if tenant.trim().is_empty() { "common".to_string() } else { tenant.trim().to_string() };
    let ok = match p {
        Provider::Microsoft => super::conf::ms_client_ok(&client_id) && super::conf::tenant_ok(&tenant),
        Provider::Google => super::conf::google_client_ok(&client_id) && super::conf::secret_ok(&secret),
    };
    if !ok {
        return Err(match p {
            Provider::Microsoft => tr("Nejdřív vyplň Application (client) ID z registrace aplikace (průvodce níž).", "First fill in the Application (client) ID from the app registration (guide below)."),
            Provider::Google => tr("Nejdřív vyplň Client ID a client secret z Google Cloud Console (průvodce níž).", "First fill in the Client ID and client secret from Google Cloud Console (guide below)."),
        }
        .into());
    }
    if with(p, |a| std::mem::replace(&mut a.connecting, true)) {
        return Err(tr("Přihlášení už běží — dokonči ho v prohlížeči.", "Sign-in is already running — finish it in the browser.").into());
    }
    CANCEL[ix(p)].store(false, Ordering::SeqCst);
    mark_dirty();
    let res = connect_inner(p, &client_id, &secret, &tenant).await;
    with(p, |a| {
        a.connecting = false;
        match &res {
            Ok(s) => {
                a.stored = Some(s.0.clone());
                a.access = Some(s.1.clone());
                (a.expired, a.error, a.retry_at) = (false, None, None);
            }
            Err(e) => a.error = Some(e.clone()),
        }
    });
    mark_dirty();
    super::calsync::forget(p);
    if p == Provider::Microsoft {
        super::mail::forget();
    }
    res.map(|_| with(p, |a| view_of(p, a)))
}

async fn connect_inner(p: Provider, client_id: &str, secret: &str, tenant: &str) -> Result<(Stored, (String, i64)), String> {
    let (verifier, state) = pkce::fresh()?;
    let listener = loopback::bind()?;
    let redirect = oauth::redirect_uri(p, listener.port);
    let url = oauth::authorize_url(p, tenant, client_id, &redirect, &state, &pkce::challenge(&verifier));
    {
        use tauri_plugin_opener::OpenerExt;
        core().app.opener().open_url(url, None::<&str>).map_err(|_| tr("Prohlížeč se nepodařilo otevřít.", "Couldn't open the browser.").to_string())?;
    }
    let deadline = std::time::Instant::now() + loopback::WAIT;
    let st = state.clone();
    let end = tauri::async_runtime::spawn_blocking(move || listener.wait(&st, deadline, &CANCEL[ix(p)]))
        .await
        .map_err(|_| tr("Přihlášení se přerušilo.", "Sign-in was interrupted.").to_string())?;
    let code = match end {
        loopback::WaitEnd::Code(c) => c,
        loopback::WaitEnd::Provider(e) => return Err(fail(p, oauth::callback_fail(&e), &e)),
        loopback::WaitEnd::Timeout => return Err(tr("Přihlášení v prohlížeči nedoběhlo do 5 minut — zkus to znovu.", "Sign-in in the browser didn't finish within 5 minutes — try again.").into()),
        loopback::WaitEnd::Cancelled => return Err(tr("Přihlášení zrušeno.", "Sign-in cancelled.").into()),
    };
    let ep = oauth::endpoints(p, tenant);
    let r = http::post_form(p.label(), &ep.token, oauth::code_form(p, client_id, secret, &code, &redirect, &verifier)).await?;
    if !(200..300).contains(&r.status) {
        let (f, c) = oauth::classify(r.status, &r.body);
        return Err(fail(p, f, &c));
    }
    let now = now_ms();
    let (t, exp) = oauth::parse_token(&r.body, now)?;
    let Some(refresh) = t.refresh_token.filter(|s| !s.is_empty()) else {
        return Err(tr("Poskytovatel nevydal trvalé přihlášení (refresh token) — zkus to znovu a povol přístup.", "The provider didn't issue a lasting sign-in (refresh token) — try again and allow access.").into());
    };
    let (account, name) = profile(p, &t.access_token).await?;
    let scope = t.scope.unwrap_or_else(|| if p == Provider::Microsoft { super::MS_SCOPES } else { super::GOOGLE_SCOPES }.to_string());
    let stored = Stored {
        v: 1,
        client_id: client_id.into(),
        client_secret: if p == Provider::Google { secret.into() } else { String::new() },
        refresh_token: refresh,
        scope,
        account,
        name,
        tenant: if p == Provider::Microsoft { tenant.into() } else { String::new() },
        saved_ms: now,
    };
    vault::save(&WinCred, p, &stored)?;
    log(&format!("účty: {} přihlášen", p.id()));
    Ok((stored, (t.access_token, exp)))
}

async fn profile(p: Provider, token: &str) -> Result<(String, String), String> {
    let url = match p {
        Provider::Microsoft => format!("{GRAPH}/me?$select=displayName,mail,userPrincipalName"),
        Provider::Google => format!("{GCAL}/calendars/primary?fields=id,summary"),
    };
    let r = http::get(p.label(), &url, token, &[]).await?;
    if !(200..300).contains(&r.status) {
        return Err(http::api_error(p.label(), r.status));
    }
    let parsed = match p {
        Provider::Microsoft => parse_ms_me(&r.body),
        Provider::Google => parse_google_primary(&r.body),
    };
    parsed.ok_or_else(|| tr("Profil účtu je nečitelný.", "The account profile is unreadable.").into())
}

/// Zruší čekání na přihlášení v prohlížeči.
pub fn cancel(p: Provider) {
    CANCEL[ix(p)].store(true, Ordering::SeqCst);
}

/// Odhlásí: smaže záznam ze Správce pověření a z paměti, u Google refresh token i odvolá.
pub async fn disconnect(p: Provider) -> Result<(), String> {
    let stored = with(p, |a| {
        let s = a.stored.take();
        (a.access, a.expired, a.error, a.retry_at) = (None, false, None, None);
        s
    });
    let removed = vault::remove(&WinCred, p);
    super::calsync::forget(p);
    if p == Provider::Microsoft {
        super::mail::forget();
    }
    mark_dirty();
    if let (Some(s), Some(url)) = (stored, oauth::endpoints(p, "").revoke) {
        // revokace je laskavost navíc: odhlášení platí i bez ní (token je pryč z PC)
        match http::post_form(p.label(), url, oauth::form(&[("token", &s.refresh_token)])).await {
            Ok(r) if (200..300).contains(&r.status) => log(&format!("účty: {} odvoláno", p.id())),
            Ok(r) => log(&format!("účty: {} revokace HTTP {}", p.id(), r.status)),
            Err(_) => {}
        }
    }
    removed
}

/// Platný access token (obnoví ho, když vyprší). Chyba = lidská věta.
pub async fn access_token(p: Provider) -> Result<String, String> {
    let now = now_ms();
    let quick = with(p, |a| -> Result<Option<String>, String> {
        if a.stored.is_none() {
            return Err(trf!("Účet {} není přihlášený.", "The {} account isn't signed in.", p.label()));
        }
        if a.expired {
            return Err(oauth::human(p, TokenFail::Revoked));
        }
        Ok(a.access.as_ref().filter(|(_, e)| oauth::fresh(Some(*e), now)).map(|(t, _)| t.clone()))
    })?;
    if let Some(t) = quick {
        return Ok(t);
    }
    let _g = REFRESH[ix(p)].lock().await;
    // mezitím mohl obnovit jiný volající
    let (done, stored, retry) = with(p, |a| (a.access.as_ref().filter(|(_, e)| oauth::fresh(Some(*e), now_ms())).map(|(t, _)| t.clone()), a.stored.clone(), a.retry_at));
    if let Some(t) = done {
        return Ok(t);
    }
    let Some(s) = stored else { return Err(trf!("Účet {} není přihlášený.", "The {} account isn't signed in.", p.label())) };
    if retry.is_some_and(|r| now_ms() < r) {
        return Err(oauth::human(p, TokenFail::Transient));
    }
    let ep = oauth::endpoints(p, &s.tenant);
    let r = match http::post_form(p.label(), &ep.token, oauth::refresh_form(p, &s.client_id, &s.client_secret, &s.refresh_token)).await {
        Ok(r) => r,
        Err(e) => {
            with(p, |a| a.retry_at = oauth::retry_after(TokenFail::Transient, now_ms()));
            return Err(e);
        }
    };
    if !(200..300).contains(&r.status) {
        let (f, code) = oauth::classify(r.status, &r.body);
        let msg = fail(p, f, &code);
        with(p, |a| {
            a.retry_at = oauth::retry_after(f, now_ms());
            if a.retry_at.is_none() {
                a.expired = true;
                a.access = None;
            }
            a.error = Some(msg.clone());
        });
        mark_dirty();
        return Err(msg);
    }
    let (t, exp) = oauth::parse_token(&r.body, now_ms())?;
    let next = oauth::next_refresh(&s.refresh_token, t.refresh_token.as_deref());
    if next != s.refresh_token {
        let mut s2 = s.clone();
        s2.refresh_token = next;
        s2.saved_ms = now_ms();
        if let Err(e) = vault::save(&WinCred, p, &s2) {
            log(&format!("účty: {} nový refresh token nejde uložit ({e})", p.id()));
        }
        with(p, |a| a.stored = Some(s2));
    }
    with(p, |a| {
        a.access = Some((t.access_token.clone(), exp));
        (a.error, a.retry_at) = (None, None);
    });
    Ok(t.access_token)
}

/// API odmítlo token (401): zahodit access token, další volání ho obnoví.
pub fn invalidate(p: Provider) {
    with(p, |a| a.access = None);
}

/// GET na API s tokenem účtu; 401 → jednou obnovit a zkusit znovu.
pub async fn api_get(p: Provider, url: &str, headers: &[(&str, &str)]) -> Result<http::Resp, String> {
    let mut token = access_token(p).await?;
    for attempt in 0..2 {
        let r = http::get(p.label(), url, &token, headers).await?;
        if r.status == 401 && attempt == 0 {
            invalidate(p);
            token = access_token(p).await?;
            continue;
        }
        if !(200..300).contains(&r.status) {
            return Err(http::api_error(p.label(), r.status));
        }
        return Ok(r);
    }
    Err(http::api_error(p.label(), 401))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn profiles() {
        type Case<'a> = (&'a [u8], Option<(&'a str, &'a str)>);
        let cases: [Case; 4] = [
            (r#"{"displayName":"Jana Nováková","mail":"jana@contoso.com","userPrincipalName":"jana@contoso.onmicrosoft.com"}"#.as_bytes(), Some(("jana@contoso.com", "Jana Nováková"))),
            // osobní účet: mail bývá null, přihlašovací jméno je v UPN
            (br#"{"displayName":"Jana","mail":null,"userPrincipalName":"jana@outlook.com"}"#, Some(("jana@outlook.com", "Jana"))),
            (br#"{"displayName":"X"}"#, None),
            (b"nesmysl", None),
        ];
        for (b, want) in cases {
            assert_eq!(parse_ms_me(b), want.map(|(a, n)| (a.into(), n.into())), "{}", String::from_utf8_lossy(b));
        }
        assert_eq!(parse_google_primary(br#"{"id":"jana@example.com","summary":"Jana"}"#), Some(("jana@example.com".into(), "Jana".into())));
        assert_eq!(parse_google_primary(br#"{"id":""}"#), None);
    }

    #[test]
    fn view_never_carries_secrets() {
        let a = Acc {
            loaded: true,
            stored: Some(Stored {
                v: 1,
                client_id: "cid".into(),
                client_secret: "GOCSPX-tajne".into(),
                refresh_token: "1//refresh-tajny".into(),
                scope: "https://www.googleapis.com/auth/calendar.readonly".into(),
                account: "jana@example.com".into(),
                name: "Jana".into(),
                tenant: String::new(),
                saved_ms: 1,
            }),
            access: Some(("ya29.access-tajny".into(), 9)),
            ..Default::default()
        };
        let v = view_of(Provider::Google, &a);
        let json = serde_json::to_string(&v).unwrap();
        for secret in ["GOCSPX", "refresh-tajny", "access-tajny"] {
            assert!(!json.contains(secret), "{json}");
        }
        assert_eq!((v.connected, v.account.as_str(), v.scopes.len()), (true, "jana@example.com", 1));
        let none = view_of(Provider::Microsoft, &Acc::default());
        assert!(!none.connected && none.account.is_empty());
    }
}
