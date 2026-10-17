//! Aktualizace bez `gh`: veřejné GitHub REST API (`/repos/<repo>/releases/latest`, bez tokenu,
//! User-Agent, termín) a stažení instalátoru přes HTTPS (jen z `github.com/<repo>/releases/download/…`,
//! přesměrování na CDN GitHubu sleduje reqwest). Volá `update.rs`; `gh` zůstává zálohou (soukromé
//! repo, vyčerpaný limit 60 dotazů/h). Ověření SHA256 dělá dál `update::verify` — tady se jen stahuje.
//! Síť běží na runtime Tauri (`block_on`) ve vlastním vlákně, aby nezáleželo, odkud se volá.

use crate::i18n::tr;
use std::io::Write;
use std::path::Path;
use std::time::Duration;

const API_TIMEOUT: Duration = Duration::from_secs(20);
const DOWNLOAD_TIMEOUT: Duration = Duration::from_secs(300);
/// Strop velikosti instalátoru (dnes ~3 MB) — nic většího se na disk nezapíše.
const MAX_SETUP_BYTES: u64 = 200 * 1024 * 1024;

pub fn api_url(repo: &str) -> String {
    format!("https://api.github.com/repos/{repo}/releases/latest")
}

/// HTTP stav API → lidský důvod. Čistá funkce.
pub fn api_error(status: u16, rate_remaining: Option<&str>) -> String {
    match status {
        404 => tr("repo nemá žádné vydání, nebo je soukromé / neexistuje", "the repo has no release, or it's private / doesn't exist").into(),
        403 | 429 if rate_remaining == Some("0") || status == 429 => tr("vyčerpaný limit GitHub API (60 dotazů/h) — zkusím to později", "GitHub API limit used up (60 requests/h) — I'll try later").into(),
        403 => tr("GitHub API odmítlo dotaz (HTTP 403)", "GitHub API refused the request (HTTP 403)").into(),
        s if s >= 500 => trf!("GitHub má potíže (HTTP {s})", "GitHub is having trouble (HTTP {s})"),
        s => format!("GitHub API: HTTP {s}"),
    }
}

/// Stahovat jen instalátor vydání z repa v configu: `https://github.com/<repo>/releases/download/<tag>/<asset>`
/// (vlastník/repo bez ohledu na velikost písmen). Čistá funkce.
pub fn download_ok(url: &str, repo: &str, tag: &str, asset: &str) -> bool {
    let want = format!("https://github.com/{repo}/releases/download/{tag}/{asset}");
    let (u, w) = (url.trim(), want.as_str());
    let split = "https://github.com/".len() + repo.len();
    match (u.get(..split), u.get(split..), w.get(..split), w.get(split..)) {
        (Some(a), Some(b), Some(wa), Some(wb)) => a.eq_ignore_ascii_case(wa) && b == wb,
        _ => false,
    }
}

fn net_error(e: &reqwest::Error) -> String {
    if e.is_timeout() {
        tr("GitHub neodpověděl včas (offline?)", "GitHub didn't answer in time (offline?)").into()
    } else if e.is_connect() {
        tr("GitHub je nedostupný (offline?)", "GitHub is unreachable (offline?)").into()
    } else {
        trf!("síť: {e}", "network: {e}")
    }
}

fn client(timeout: Duration) -> Result<reqwest::Client, String> {
    reqwest::Client::builder()
        .user_agent(concat!("Ledge/", env!("CARGO_PKG_VERSION"), " (+https://github.com/jiri-hyan/ledge-app)"))
        .timeout(timeout)
        .build()
        .map_err(|e| trf!("HTTP klient: {e}", "HTTP client: {e}"))
}

/// Future na runtime Tauri z vlastního vlákna (volající může být sám vlákno runtime).
fn run<T: Send>(f: impl FnOnce() -> Result<T, String> + Send) -> Result<T, String> {
    std::thread::scope(|s| s.spawn(f).join().unwrap_or_else(|_| Err(tr("vlákno stahování spadlo", "the download thread crashed").into())))
}

/// JSON nejnovějšího vydání (bez předběžných a konceptů — to zaručuje `/releases/latest`).
pub fn latest(repo: &str) -> Result<String, String> {
    let url = api_url(repo);
    run(|| {
        tauri::async_runtime::block_on(async {
            let resp = client(API_TIMEOUT)?
                .get(&url)
                .header("Accept", "application/vnd.github+json")
                .header("X-GitHub-Api-Version", "2022-11-28")
                .send()
                .await
                .map_err(|e| net_error(&e))?;
            let status = resp.status().as_u16();
            if status != 200 {
                let rem = resp.headers().get("x-ratelimit-remaining").and_then(|v| v.to_str().ok()).map(str::to_string);
                return Err(api_error(status, rem.as_deref()));
            }
            resp.text().await.map_err(|e| net_error(&e))
        })
    })
}

/// Stáhne `url` do `dest` (přepíše). Při chybě nebo přetečení stropu soubor smaže.
pub fn download(url: &str, dest: &Path) -> Result<(), String> {
    let res = run(|| {
        tauri::async_runtime::block_on(async {
            let mut resp = client(DOWNLOAD_TIMEOUT)?.get(url).send().await.map_err(|e| net_error(&e))?;
            if !resp.status().is_success() {
                return Err(trf!("stažení: HTTP {}", "download: HTTP {}", resp.status().as_u16()));
            }
            if resp.content_length().is_some_and(|n| n > MAX_SETUP_BYTES) {
                return Err(tr("instalátor je podezřele velký — nestahuji", "the installer is suspiciously large — not downloading").into());
            }
            let mut f = std::fs::File::create(dest).map_err(|e| format!("{}: {e}", dest.display()))?;
            let mut total = 0u64;
            while let Some(chunk) = resp.chunk().await.map_err(|e| net_error(&e))? {
                total += chunk.len() as u64;
                if total > MAX_SETUP_BYTES {
                    return Err(tr("instalátor je podezřele velký — přerušeno", "the installer is suspiciously large — stopped").into());
                }
                f.write_all(&chunk).map_err(|e| format!("{}: {e}", dest.display()))?;
            }
            f.sync_all().map_err(|e| e.to_string())
        })
    });
    if res.is_err() {
        let _ = std::fs::remove_file(dest);
    }
    res
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn urls_and_errors() {
        assert_eq!(api_url("acme/ledge"), "https://api.github.com/repos/acme/ledge/releases/latest");
        assert!(api_error(404, None).contains("soukromé"));
        assert!(api_error(403, Some("0")).contains("limit"));
        assert!(api_error(429, None).contains("limit"));
        assert!(api_error(403, Some("12")).contains("403"));
        assert!(api_error(502, None).contains("potíže"));
    }

    #[test]
    fn download_only_from_release_of_configured_repo() {
        let ok = "https://github.com/Acme/Ledge/releases/download/v0.2.14/Ledge_0.2.14_x64-setup.exe";
        assert!(download_ok(ok, "acme/ledge", "v0.2.14", "Ledge_0.2.14_x64-setup.exe"));
        #[rustfmt::skip]
        let bad = [
            "http://github.com/acme/ledge/releases/download/v0.2.14/Ledge_0.2.14_x64-setup.exe",
            "https://api.github.com/repos/acme/ledge/releases/assets/1",
            "https://github.com/evil/ledge/releases/download/v0.2.14/Ledge_0.2.14_x64-setup.exe",
            "https://github.com/acme/ledge/releases/download/v0.2.15/Ledge_0.2.14_x64-setup.exe",
            "https://github.com/acme/ledge/releases/download/v0.2.14/ledge_0.2.14_x64-setup.exe",
            "https://github.com/acme/ledge/releases/download/v0.2.14/Ledge_0.2.14_x64-setup.exe?x=1",
            "",
        ];
        for u in bad {
            assert!(!download_ok(u, "acme/ledge", "v0.2.14", "Ledge_0.2.14_x64-setup.exe"), "{u}");
        }
    }
}
