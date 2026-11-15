//! HTTP pro účty (reqwest + native-tls jako `calendar/live.rs`): termín, strop velikosti odpovědi,
//! žádná přesměrování (API ani token endpointy je nepotřebují — a nesmí odnést hlavičku s tokenem
//! jinam), chyby sítě lidsky a bez adresy. Text odpovědi se do logu nikdy nedává.

use crate::i18n::tr;
use crate::state::log;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

pub const TIMEOUT: Duration = Duration::from_secs(15);
/// Větší odpověď se zahodí (kalendář na 2 týdny i se stránkami je o řád menší).
pub const MAX_BYTES: usize = 2 * 1024 * 1024;

pub struct Resp {
    pub status: u16,
    pub body: Vec<u8>,
}

/// Slot sdíleného klienta: `static` volajícího (`static C: Slot = Slot::new();`).
pub struct Slot(std::sync::Mutex<Option<(u64, reqwest::Client)>>);

impl Slot {
    pub const fn new() -> Self {
        Slot(std::sync::Mutex::new(None))
    }
}

/// Generace sítě: `send` ji zvedne po transportní chybě, slot se starší generace se postaví znovu.
static GEN: AtomicU64 = AtomicU64::new(0);

/// Zahodit sdíleného klienta po chybě? Jen transportní selhání (spojení vč. TLS, termín): reqwest čte
/// systémovou proxy a certifikáty při buildu, takže po přepnutí VPN/proxy je starý klient k ničemu.
/// Odpověď serveru (4xx/5xx) ani chyba těla klienta nezahazuje.
pub fn should_drop_client(is_connect: bool, is_timeout: bool) -> bool {
    is_connect || is_timeout
}

/// Sdílený HTTP klient podle politiky přesměrování (L210): každý fetch dřív stavěl nový `Client`
/// (SChannel pokaždé otevřel úložiště certifikátů a credential handle → commit jádra). `policy` se zavolá
/// jen při (znovu)sestavení. Timeout se dává per požadavek (`RequestBuilder::timeout`); spojení se nedrží
/// (`pool_max_idle_per_host(0)`), fetche jsou po minutách. Klient je levný klon (`Arc`); po transportní
/// chybě hlášené přes [`send`] se příště postaví nový (L266: přepnutí VPN/proxy za běhu).
pub fn shared(slot: &'static Slot, policy: fn() -> reqwest::redirect::Policy) -> Result<reqwest::Client, String> {
    let gen_now = GEN.load(Ordering::Relaxed);
    let mut g = slot.0.lock().unwrap_or_else(|p| p.into_inner());
    if let Some((gen_c, c)) = g.as_ref()
        && *gen_c == gen_now
    {
        return Ok(c.clone());
    }
    let c = reqwest::Client::builder()
        .redirect(policy())
        .user_agent(concat!("Ledge/", env!("CARGO_PKG_VERSION")))
        .pool_max_idle_per_host(0)
        .build()
        .map_err(|e| trf!("HTTP klient: {}", "HTTP client: {}", e.without_url()))?;
    *g = Some((gen_now, c.clone()));
    Ok(c)
}

/// `send` s hlášením: transportní chyba ([`should_drop_client`]) zneplatní všechny sdílené klienty.
pub async fn send(req: reqwest::RequestBuilder) -> Result<reqwest::Response, reqwest::Error> {
    let r = req.send().await;
    if let Err(e) = &r
        && should_drop_client(e.is_connect(), e.is_timeout())
    {
        GEN.fetch_add(1, Ordering::Relaxed);
    }
    r
}

/// Klient bez přesměrování (účty, limity Claude).
pub fn no_redirect() -> Result<reqwest::Client, String> {
    static C: Slot = Slot::new();
    shared(&C, reqwest::redirect::Policy::none)
}

/// Klient s výchozím přesměrováním reqwestu (aktualizace, počasí).
pub fn default_redirect() -> Result<reqwest::Client, String> {
    static C: Slot = Slot::new();
    shared(&C, reqwest::redirect::Policy::default)
}

/// Síťová chyba (bez odpovědi): uživateli lidsky, příčina bez adresy do logu.
pub fn net_error(what: &str, e: reqwest::Error) -> String {
    let e = e.without_url();
    if e.is_timeout() {
        return trf!("{what} neodpověděl včas — zkusím to znovu.", "{what} didn't answer in time — I'll try again.");
    }
    let mut why = e.to_string();
    let mut src = std::error::Error::source(&e);
    while let Some(s) = src {
        why.push_str(&format!(": {s}"));
        src = s.source();
    }
    log(&format!("účty: síť {what} ({})", super::oauth::scrub(&why, &[])));
    trf!("{what}: nejde se připojit — jsi online? Zkusím to znovu později.", "{what}: can't connect — are you online? I'll try again later.")
}

async fn read(what: &str, mut resp: reqwest::Response) -> Result<Resp, String> {
    let status = resp.status().as_u16();
    if resp.content_length().is_some_and(|n| n > MAX_BYTES as u64) {
        return Err(too_big());
    }
    let mut body = Vec::new();
    while let Some(chunk) = resp.chunk().await.map_err(|e| net_error(what, e))? {
        if body.len() + chunk.len() > MAX_BYTES {
            return Err(too_big());
        }
        body.extend_from_slice(&chunk);
    }
    Ok(Resp { status, body })
}

fn too_big() -> String {
    tr("Odpověď je nečekaně velká — Ledge ji zahodil.", "The response is unexpectedly large — Ledge dropped it.").into()
}

/// POST formuláře (`application/x-www-form-urlencoded`); `what` = komu (do hlášek).
pub async fn post_form(what: &str, url: &str, body: String) -> Result<Resp, String> {
    let req = no_redirect()?
        .post(url)
        .timeout(TIMEOUT)
        .header(reqwest::header::CONTENT_TYPE, "application/x-www-form-urlencoded")
        .header(reqwest::header::ACCEPT, "application/json")
        .body(body);
    let resp = send(req)
        .await
        .map_err(|e| net_error(what, e))?;
    read(what, resp).await
}

/// POST JSON (`application/json`) s Bearer tokenem; `accept` dává volající (MCP chce `application/json, text/event-stream`).
pub async fn post_json(what: &str, url: &str, token: &str, accept: &str, body: String) -> Result<Resp, String> {
    let req = no_redirect()?
        .post(url)
        .timeout(TIMEOUT)
        .bearer_auth(token)
        .header(reqwest::header::CONTENT_TYPE, "application/json")
        .header(reqwest::header::ACCEPT, accept)
        .body(body);
    let resp = send(req)
        .await
        .map_err(|e| net_error(what, e))?;
    read(what, resp).await
}

/// GET s Bearer tokenem a volitelnými hlavičkami.
pub async fn get(what: &str, url: &str, token: &str, headers: &[(&str, &str)]) -> Result<Resp, String> {
    let mut req = no_redirect()?.get(url).timeout(TIMEOUT).bearer_auth(token).header(reqwest::header::ACCEPT, "application/json");
    for (k, v) in headers {
        req = req.header(*k, *v);
    }
    let resp = send(req).await.map_err(|e| net_error(what, e))?;
    read(what, resp).await
}

/// Chyba API (ne token endpointu) lidsky: 401 = přihlášení neplatí, 403 = chybí oprávnění, 429/5xx = později.
pub fn api_error(what: &str, status: u16) -> String {
    match status {
        401 => trf!("{what} odmítl přihlášení — přihlas se znovu v Nastavení → Obsah karet → Kalendáře → Pokročilé.", "{what} rejected the sign-in — sign in again in Settings → Card contents → Calendars → Advanced."),
        403 => trf!(
            "{what} nedal Ledge oprávnění ke čtení — odhlas se a přihlas znovu (a povol přístup).",
            "{what} didn't give Ledge permission to read — sign out and in again (and allow access)."
        ),
        404 => trf!("{what}: tahle data u účtu nejsou (nemá poštovní schránku nebo kalendář?).", "{what}: this account doesn't have that data (no mailbox or calendar?)."),
        429 => trf!("{what} teď odmítá tolik dotazů — zkusím to později sám.", "{what} is refusing requests right now — I'll try again later."),
        500..=599 => trf!("{what} má potíže — zkusím to později sám.", "{what} is having trouble — I'll try again later."),
        _ => trf!("{what} odpověděl chybou — zkusím to později.", "{what} answered with an error — I'll try again later."),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shared_client_is_reused_until_network_error() {
        static SLOT: Slot = Slot::new();
        shared(&SLOT, reqwest::redirect::Policy::none).unwrap();
        let g0 = SLOT.0.lock().unwrap().as_ref().map(|(g, _)| *g);
        shared(&SLOT, || unreachable!("politika se při platné generaci nevolá")).unwrap();
        assert_eq!(g0, SLOT.0.lock().unwrap().as_ref().map(|(g, _)| *g));
    }

    #[test]
    fn drop_client_only_on_transport_errors() {
        // (connect, timeout, zahodit)
        for (c, t, want) in [(true, false, true), (false, true, true), (true, true, true), (false, false, false)] {
            assert_eq!(should_drop_client(c, t), want, "connect={c} timeout={t}");
        }
    }

    #[test]
    fn api_errors_human_without_codes() {
        for code in [400, 401, 403, 404, 429, 500, 503] {
            for l in [crate::uiconf::Lang::Cs, crate::uiconf::Lang::En] {
                let s = crate::i18n::with_lang(l, || api_error("Microsoft", code));
                assert!(!s.contains(&code.to_string()) && !s.contains("://"), "{s}");
                assert!(s.contains("Microsoft"));
            }
        }
    }
}
