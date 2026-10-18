//! Čistá logika OAuth 2.0 (bez sítě): adresy přihlášení, tělo formuláře, odpověď token endpointu,
//! chyby lidskou řečí, politika obnovy access tokenu a maskování tajemství v textu chyb.

use super::Provider;
use crate::i18n::tr;
use serde::Deserialize;

/// Access token se obnoví, když do konce platnosti zbývá méně než tohle.
pub const SKEW_MS: i64 = 2 * 60 * 1000;
/// Kratší platnost od poskytovatele se nebere (ochrana před smyčkou obnov).
const MIN_LIFE_S: i64 = 60;

pub struct Endpoints {
    pub authorize: String,
    pub token: String,
    /// revokace refresh tokenu (Microsoft pro veřejné klienty žádnou nemá)
    pub revoke: Option<&'static str>,
}

pub fn endpoints(p: Provider, tenant: &str) -> Endpoints {
    match p {
        Provider::Microsoft => {
            let t = if tenant.trim().is_empty() { "common" } else { tenant.trim() };
            Endpoints {
                authorize: format!("https://login.microsoftonline.com/{t}/oauth2/v2.0/authorize"),
                token: format!("https://login.microsoftonline.com/{t}/oauth2/v2.0/token"),
                revoke: None,
            }
        }
        Provider::Google => Endpoints {
            authorize: "https://accounts.google.com/o/oauth2/v2/auth".into(),
            token: "https://oauth2.googleapis.com/token".into(),
            revoke: Some("https://oauth2.googleapis.com/revoke"),
        },
    }
}

/// Adresa přesměrování pro port loopback serveru. Microsoft: registruje se `http://localhost`
/// (port se u localhost při porovnání ignoruje; `http://127.0.0.1` jde v Azure Portal přidat jen
/// v manifestu). Google (klient „Desktop app“): `http://127.0.0.1:<port>` bez registrace.
pub fn redirect_uri(p: Provider, port: u16) -> String {
    match p {
        Provider::Microsoft => format!("http://localhost:{port}"),
        Provider::Google => format!("http://127.0.0.1:{port}"),
    }
}

/// Procentní kódování pro `application/x-www-form-urlencoded` i dotaz (RFC 3986 neomezené znaky
/// zůstanou, mezera = `%20`).
pub fn enc(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        if b.is_ascii_alphanumeric() || matches!(b, b'-' | b'.' | b'_' | b'~') {
            out.push(b as char);
        } else {
            out.push_str(&format!("%{b:02X}"));
        }
    }
    out
}

pub fn form(pairs: &[(&str, &str)]) -> String {
    pairs.iter().map(|(k, v)| format!("{}={}", enc(k), enc(v))).collect::<Vec<_>>().join("&")
}

/// Adresa přihlášení v prohlížeči (Authorization Code + PKCE S256).
pub fn authorize_url(p: Provider, tenant: &str, client_id: &str, redirect: &str, state: &str, challenge: &str) -> String {
    let e = endpoints(p, tenant);
    let mut q: Vec<(&str, &str)> = vec![
        ("client_id", client_id),
        ("response_type", "code"),
        ("redirect_uri", redirect),
        ("state", state),
        ("code_challenge", challenge),
        ("code_challenge_method", "S256"),
    ];
    match p {
        Provider::Microsoft => q.extend([("scope", super::MS_SCOPES), ("response_mode", "query"), ("prompt", "select_account")]),
        // offline_access u Google = access_type=offline; prompt=consent, ať refresh token přijde i při
        // opakovaném přihlášení
        Provider::Google => q.extend([("scope", super::GOOGLE_SCOPES), ("access_type", "offline"), ("prompt", "consent"), ("include_granted_scopes", "false")]),
    }
    format!("{}?{}", e.authorize, form(&q))
}

/// Tělo výměny kódu za tokeny.
pub fn code_form(p: Provider, client_id: &str, secret: &str, code: &str, redirect: &str, verifier: &str) -> String {
    let mut f = vec![("client_id", client_id), ("grant_type", "authorization_code"), ("code", code), ("redirect_uri", redirect), ("code_verifier", verifier)];
    if p == Provider::Google {
        f.push(("client_secret", secret));
    } else {
        f.push(("scope", super::MS_SCOPES));
    }
    form(&f)
}

/// Tělo obnovy access tokenu.
pub fn refresh_form(p: Provider, client_id: &str, secret: &str, refresh: &str) -> String {
    let mut f = vec![("client_id", client_id), ("grant_type", "refresh_token"), ("refresh_token", refresh)];
    if p == Provider::Google {
        f.push(("client_secret", secret));
    } else {
        f.push(("scope", super::MS_SCOPES));
    }
    form(&f)
}

#[derive(Deserialize, Debug, PartialEq)]
pub struct TokenResponse {
    pub access_token: String,
    #[serde(default)]
    pub refresh_token: Option<String>,
    #[serde(default)]
    pub expires_in: Option<i64>,
    #[serde(default)]
    pub scope: Option<String>,
}

#[derive(Deserialize, Debug, Default)]
struct ErrBody {
    #[serde(default)]
    error: String,
    #[serde(default)]
    error_description: String,
}

/// Proč token endpoint odmítl.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum TokenFail {
    /// refresh token neplatí (odvolaný, vypršelý, změna hesla) → znovu přihlásit
    Revoked,
    /// Client ID / secret nesedí → průvodce registrací
    Client,
    /// Microsoft: chybí adresa přesměrování v registraci
    Redirect,
    /// organizace nepovolila (správce) / uživatel nesouhlasil
    Denied,
    /// síť, 5xx, neznámé → zkusit později
    Transient,
}

/// Rozbor chybové odpovědi token endpointu: (druh, kód pro log — bez tajemství).
pub fn classify(status: u16, body: &[u8]) -> (TokenFail, String) {
    let e: ErrBody = serde_json::from_slice(body).unwrap_or_default();
    // AADSTS kód z popisu (Microsoft), jinak OAuth `error`
    let aad = e.error_description.split(|c: char| !c.is_ascii_alphanumeric()).find(|w| w.starts_with("AADSTS")).unwrap_or("").to_string();
    let code = if aad.is_empty() { e.error.clone() } else { format!("{} {aad}", e.error) };
    let kind = match (e.error.as_str(), aad.as_str()) {
        (_, "AADSTS50011") | ("redirect_uri_mismatch", _) => TokenFail::Redirect,
        (_, a) if ["AADSTS65001", "AADSTS90094", "AADSTS65004", "AADSTS530003", "AADSTS50105"].contains(&a) => TokenFail::Denied,
        ("invalid_grant", _) | ("interaction_required", _) => TokenFail::Revoked,
        ("invalid_client", _) | ("unauthorized_client", _) => TokenFail::Client,
        (_, a) if ["AADSTS700016", "AADSTS7000218", "AADSTS70002"].contains(&a) => TokenFail::Client,
        ("access_denied", _) | ("consent_required", _) => TokenFail::Denied,
        // 5xx, 429 a cokoli neznámého: zkusit později
        _ => TokenFail::Transient,
    };
    (kind, if code.is_empty() { format!("HTTP {status}") } else { code })
}

/// Lidská věta k chybě (bez kódů, adres a tokenů).
pub fn human(p: Provider, f: TokenFail) -> String {
    match (f, p) {
        (TokenFail::Revoked, _) => trf!("Přihlášení k účtu {} vypršelo — přihlas se znovu v Nastavení → Účty.", "Your {} sign-in expired — sign in again in Settings → Accounts.", p.label()),
        (TokenFail::Client, Provider::Microsoft) => tr(
            "Microsoft nezná tohle Client ID nebo nemá povolené veřejné klienty — zkontroluj registraci aplikace (průvodce v Nastavení → Účty).",
            "Microsoft doesn't know this Client ID or public clients aren't allowed — check the app registration (guide in Settings → Accounts).",
        )
        .into(),
        (TokenFail::Client, Provider::Google) => tr(
            "Google nepřijal Client ID nebo client secret — zkopíruj je znovu z Google Cloud Console (klient typu Desktop app).",
            "Google didn't accept the Client ID or client secret — copy them again from Google Cloud Console (Desktop app client).",
        )
        .into(),
        (TokenFail::Redirect, _) => tr(
            "V registraci aplikace chybí adresa přesměrování — přidej ji podle průvodce (krok Platforma) a zkus to znovu.",
            "The app registration is missing the redirect address — add it as the guide says (Platform step) and try again.",
        )
        .into(),
        (TokenFail::Denied, Provider::Microsoft) => tr(
            "Přístup nebyl povolen — buď jsi ho odmítl, nebo správce tvé organizace nepovoluje aplikace mimo firmu. Zkus osobní účet nebo požádej správce.",
            "Access wasn't granted — either you declined, or your organisation's admin doesn't allow outside apps. Try a personal account or ask your admin.",
        )
        .into(),
        (TokenFail::Denied, Provider::Google) => tr(
            "Google přístup nepovolil — u neověřené aplikace musí být tvůj účet mezi testovacími uživateli (průvodce, krok Testovací uživatelé).",
            "Google didn't allow access — for an unverified app your account must be listed as a test user (guide, Test users step).",
        )
        .into(),
        (TokenFail::Transient, _) => trf!("{} teď neodpovídá — zkusím to později sám.", "{} isn't responding right now — I'll try again later.", p.label()),
    }
}

/// Úspěšná odpověď token endpointu → (odpověď, kdy access token vyprší v ms epochy).
pub fn parse_token(body: &[u8], now_ms: i64) -> Result<(TokenResponse, i64), String> {
    let t: TokenResponse = serde_json::from_slice(body).map_err(|_| tr("Odpověď přihlášení je nečitelná.", "The sign-in response is unreadable.").to_string())?;
    if t.access_token.is_empty() {
        return Err(tr("Odpověď přihlášení neobsahuje přístup.", "The sign-in response has no access.").into());
    }
    let life = t.expires_in.unwrap_or(3600).max(MIN_LIFE_S);
    Ok((t, now_ms + life * 1000))
}

/// Je access token ještě použitelný (s rezervou `SKEW_MS`)?
pub fn fresh(expires_ms: Option<i64>, now_ms: i64) -> bool {
    expires_ms.is_some_and(|e| now_ms + SKEW_MS < e)
}

/// Nový refresh token po obnově: Microsoft ho rotuje (vrátí nový), Google obvykle ne (platí starý).
pub fn next_refresh(old: &str, got: Option<&str>) -> String {
    got.filter(|s| !s.is_empty()).unwrap_or(old).to_string()
}

/// Smí se po neúspěšné obnově hned zkusit znovu? Jen ne po odvolání, a ne dřív než za minutu.
pub fn retry_after(f: TokenFail, now_ms: i64) -> Option<i64> {
    match f {
        TokenFail::Revoked | TokenFail::Client | TokenFail::Denied | TokenFail::Redirect => None,
        TokenFail::Transient => Some(now_ms + 60_000),
    }
}

/// Z textu (chyby, logu) vymaže známá tajemství, adresy a dlouhé řetězce podobné tokenům.
pub fn scrub(text: &str, secrets: &[&str]) -> String {
    let mut t = text.to_string();
    for s in secrets.iter().filter(|s| s.len() >= 4) {
        t = t.replace(s, "•••");
    }
    let mut out = String::with_capacity(t.len());
    for word in t.split_inclusive(char::is_whitespace) {
        let core = word.trim_end();
        let tail = &word[core.len()..];
        let lower = core.to_ascii_lowercase();
        let tokenish = core.len() >= 24 && core.chars().filter(|c| c.is_ascii_alphanumeric()).count() * 10 >= core.len() * 8 && core.chars().all(|c| c.is_ascii_alphanumeric() || "-._~+/=%_".contains(c));
        if lower.contains("://") || lower.starts_with("www.") {
            out.push_str("[adresa]");
        } else if tokenish {
            out.push_str("•••");
        } else {
            out.push_str(core);
        }
        out.push_str(tail);
    }
    out
}

/// Odpověď loopback serveru po přesměrování z prohlížeče: (`code`, `state`) nebo chyba poskytovatele.
#[derive(Debug, PartialEq)]
pub enum Callback {
    Code { code: String, state: String },
    Error { error: String, state: String },
    /// jiný požadavek (favicon, přednačtení) — ignorovat a čekat dál
    Other,
}

fn dec(s: &str) -> String {
    let b = s.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        match b[i] {
            b'+' => out.push(b' '),
            b'%' if i + 2 < b.len() => {
                match u8::from_str_radix(std::str::from_utf8(&b[i + 1..i + 3]).unwrap_or("zz"), 16) {
                    Ok(v) => {
                        out.push(v);
                        i += 2;
                    }
                    Err(_) => out.push(b'%'),
                }
            }
            c => out.push(c),
        }
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// Rozbor cesty požadavku (`/?code=…&state=…`).
pub fn parse_callback(url: &str) -> Callback {
    let Some((_, q)) = url.split_once('?') else { return Callback::Other };
    let mut code = None;
    let mut state = String::new();
    let mut error = None;
    for pair in q.split('&') {
        let (k, v) = pair.split_once('=').unwrap_or((pair, ""));
        match k {
            "code" => code = Some(dec(v)),
            "state" => state = dec(v),
            "error" => error = Some(dec(v)),
            _ => {}
        }
    }
    match (code, error) {
        (_, Some(error)) => Callback::Error { error, state },
        (Some(code), None) if !code.is_empty() => Callback::Code { code, state },
        _ => Callback::Other,
    }
}

/// Chyba z přesměrování (`error=…`) → druh.
pub fn callback_fail(error: &str) -> TokenFail {
    match error {
        "access_denied" | "consent_required" | "interaction_required" => TokenFail::Denied,
        "unauthorized_client" | "invalid_client" | "invalid_request" => TokenFail::Client,
        _ => TokenFail::Transient,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encoding_and_forms() {
        assert_eq!(enc("a b&c=d/é~"), "a%20b%26c%3Dd%2F%C3%A9~");
        let f = refresh_form(Provider::Google, "id.apps.googleusercontent.com", "GOCSPX-x", "1//r+t");
        assert_eq!(f, "client_id=id.apps.googleusercontent.com&grant_type=refresh_token&refresh_token=1%2F%2Fr%2Bt&client_secret=GOCSPX-x");
        let f = refresh_form(Provider::Microsoft, "g", "", "rt");
        assert!(f.contains("scope=offline_access%20User.Read%20Calendars.Read%20Mail.ReadBasic") && !f.contains("client_secret"));
        let f = code_form(Provider::Microsoft, "g", "", "C0DE", "http://localhost:5555", "ver");
        assert!(f.contains("code_verifier=ver") && f.contains("redirect_uri=http%3A%2F%2Flocalhost%3A5555"));
    }

    #[test]
    fn authorize_urls() {
        let u = authorize_url(Provider::Microsoft, "common", "gid", "http://localhost:5000", "st", "ch");
        assert!(u.starts_with("https://login.microsoftonline.com/common/oauth2/v2.0/authorize?client_id=gid&response_type=code&redirect_uri=http%3A%2F%2Flocalhost%3A5000&state=st&code_challenge=ch&code_challenge_method=S256"), "{u}");
        assert!(u.contains("scope=offline_access%20User.Read%20Calendars.Read%20Mail.ReadBasic"));
        let u = authorize_url(Provider::Google, "", "x.apps.googleusercontent.com", "http://127.0.0.1:5000", "st", "ch");
        assert!(u.starts_with("https://accounts.google.com/o/oauth2/v2/auth?"));
        assert!(u.contains("scope=https%3A%2F%2Fwww.googleapis.com%2Fauth%2Fcalendar.readonly") && u.contains("access_type=offline"));
        assert!(!u.contains("gmail"), "žádné Gmail scopes");
        assert_eq!(endpoints(Provider::Microsoft, " ").token, "https://login.microsoftonline.com/common/oauth2/v2.0/token");
        assert_eq!(redirect_uri(Provider::Google, 4242), "http://127.0.0.1:4242");
    }

    #[test]
    fn token_parse_and_refresh_policy() {
        let now = 1_000_000;
        let (t, exp) = parse_token(br#"{"token_type":"Bearer","access_token":"AT","expires_in":3599,"refresh_token":"RT","scope":"User.Read"}"#, now).unwrap();
        assert_eq!((t.access_token.as_str(), t.refresh_token.as_deref(), exp), ("AT", Some("RT"), now + 3_599_000));
        // bez expires_in = hodina; nesmyslně krátká platnost = aspoň minuta
        assert_eq!(parse_token(br#"{"access_token":"A"}"#, 0).unwrap().1, 3_600_000);
        assert_eq!(parse_token(br#"{"access_token":"A","expires_in":1}"#, 0).unwrap().1, 60_000);
        for bad in [&b""[..], b"{}", br#"{"access_token":""}"#, b"<html>"] {
            assert!(parse_token(bad, now).is_err());
        }
        // (vyprší, teď, čerstvý?)
        let cases = [(None, 0, false), (Some(now + SKEW_MS + 1), now, true), (Some(now + SKEW_MS), now, false), (Some(now - 1), now, false)];
        for (exp, n, want) in cases {
            assert_eq!(fresh(exp, n), want, "{exp:?}");
        }
        assert_eq!(next_refresh("old", Some("new")), "new");
        assert_eq!(next_refresh("old", None), "old");
        assert_eq!(next_refresh("old", Some("")), "old");
        assert_eq!(retry_after(TokenFail::Transient, 5), Some(60_005));
        assert_eq!(retry_after(TokenFail::Revoked, 5), None);
    }

    #[test]
    fn classify_table() {
        let cases: [(u16, &str, TokenFail); 9] = [
            (400, r#"{"error":"invalid_grant","error_description":"AADSTS70008: The refresh token has expired. Trace ID: 1"}"#, TokenFail::Revoked),
            (400, r#"{"error":"invalid_grant","error_description":"Token has been expired or revoked."}"#, TokenFail::Revoked),
            (400, r#"{"error":"invalid_request","error_description":"AADSTS50011: The redirect URI 'http://localhost:1' does not match"}"#, TokenFail::Redirect),
            (400, r#"{"error":"invalid_client","error_description":"AADSTS7000218: ... client_assertion or client_secret"}"#, TokenFail::Client),
            (400, r#"{"error":"unauthorized_client","error_description":"AADSTS700016: Application not found"}"#, TokenFail::Client),
            (400, r#"{"error":"invalid_grant","error_description":"AADSTS65001: The user or administrator has not consented"}"#, TokenFail::Denied),
            (401, r#"{"error":"invalid_client","error_description":"Unauthorized"}"#, TokenFail::Client),
            (503, "<html>", TokenFail::Transient),
            (429, "", TokenFail::Transient),
        ];
        for (st, body, want) in cases {
            let (k, code) = classify(st, body.as_bytes());
            assert_eq!(k, want, "{body}");
            assert!(!code.contains("http"), "kód pro log bez adresy: {code}");
        }
        assert_eq!(classify(400, br#"{"error":"invalid_grant","error_description":"AADSTS70008: x"}"#).1, "invalid_grant AADSTS70008");
    }

    #[test]
    fn human_errors_have_no_codes_or_urls() {
        for p in Provider::ALL {
            for f in [TokenFail::Revoked, TokenFail::Client, TokenFail::Redirect, TokenFail::Denied, TokenFail::Transient] {
                for lang in [crate::uiconf::Lang::Cs, crate::uiconf::Lang::En] {
                    let s = crate::i18n::with_lang(lang, || human(p, f));
                    assert!(!s.contains("://") && !s.contains("AADSTS") && !s.contains("invalid_"), "{s}");
                }
            }
        }
    }

    #[test]
    fn scrub_masks_secrets() {
        let rt = "M.C540_BAY.0.U.-Cjz8xZq9uVn3R1aB2cD3eF4gH5iJ6kL7";
        let cases: [(String, &[&str], &str); 5] = [
            (format!("token {rt} odmítnut"), &[], "token ••• odmítnut"),
            ("chyba na https://login.microsoftonline.com/common?code=abc".into(), &[], "chyba na [adresa]"),
            ("secret GOCSPX-krátký".into(), &["GOCSPX-krátký"], "secret •••"),
            ("Bearer eyJ0eXAiOiJKV1QiLCJhbGciOiJSUzI1NiJ9.eyJzdWIiOiIxIn0.sig".into(), &[], "Bearer •••"),
            ("AADSTS70008: The refresh token has expired.".into(), &[], "AADSTS70008: The refresh token has expired."),
        ];
        for (inp, secrets, want) in cases {
            assert_eq!(scrub(&inp, secrets), want, "{inp}");
        }
    }

    #[test]
    fn callback_table() {
        let cases = [
            ("/?code=M.C5%2Fab&state=s1", Callback::Code { code: "M.C5/ab".into(), state: "s1".into() }),
            ("/?state=s2&code=4%2F0Ab&scope=x", Callback::Code { code: "4/0Ab".into(), state: "s2".into() }),
            ("/?error=access_denied&state=s3", Callback::Error { error: "access_denied".into(), state: "s3".into() }),
            ("/favicon.ico", Callback::Other),
            ("/?code=&state=x", Callback::Other),
            ("/?x=1", Callback::Other),
        ];
        for (u, want) in cases {
            assert_eq!(parse_callback(u), want, "{u}");
        }
        assert_eq!(dec("a+b%20c%zz%2"), "a b c%zz%2");
        assert_eq!(callback_fail("access_denied"), TokenFail::Denied);
        assert_eq!(callback_fail("server_error"), TokenFail::Transient);
    }
}
