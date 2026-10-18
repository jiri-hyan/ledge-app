//! HTTP pro účty (reqwest + native-tls jako `calendar/live.rs`): termín, strop velikosti odpovědi,
//! žádná přesměrování (API ani token endpointy je nepotřebují — a nesmí odnést hlavičku s tokenem
//! jinam), chyby sítě lidsky a bez adresy. Text odpovědi se do logu nikdy nedává.

use crate::i18n::tr;
use crate::state::log;
use std::time::Duration;

pub const TIMEOUT: Duration = Duration::from_secs(15);
/// Větší odpověď se zahodí (kalendář na 2 týdny i se stránkami je o řád menší).
pub const MAX_BYTES: usize = 2 * 1024 * 1024;

pub struct Resp {
    pub status: u16,
    pub body: Vec<u8>,
}

fn client() -> Result<reqwest::Client, String> {
    reqwest::Client::builder()
        .timeout(TIMEOUT)
        .redirect(reqwest::redirect::Policy::none())
        .user_agent(concat!("Ledge/", env!("CARGO_PKG_VERSION")))
        .build()
        .map_err(|e| trf!("HTTP klient: {}", "HTTP client: {}", e.without_url()))
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
    let resp = client()?
        .post(url)
        .header(reqwest::header::CONTENT_TYPE, "application/x-www-form-urlencoded")
        .header(reqwest::header::ACCEPT, "application/json")
        .body(body)
        .send()
        .await
        .map_err(|e| net_error(what, e))?;
    read(what, resp).await
}

/// GET s Bearer tokenem a volitelnými hlavičkami.
pub async fn get(what: &str, url: &str, token: &str, headers: &[(&str, &str)]) -> Result<Resp, String> {
    let mut req = client()?.get(url).bearer_auth(token).header(reqwest::header::ACCEPT, "application/json");
    for (k, v) in headers {
        req = req.header(*k, *v);
    }
    let resp = req.send().await.map_err(|e| net_error(what, e))?;
    read(what, resp).await
}

/// Chyba API (ne token endpointu) lidsky: 401 = přihlášení neplatí, 403 = chybí oprávnění, 429/5xx = později.
pub fn api_error(what: &str, status: u16) -> String {
    match status {
        401 => trf!("{what} odmítl přihlášení — přihlas se znovu v Nastavení → Účty.", "{what} rejected the sign-in — sign in again in Settings → Accounts."),
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
