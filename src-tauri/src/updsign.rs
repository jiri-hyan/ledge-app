//! Podpis instalátoru aktualizace (minisign / Ed25519). SHA256 v poznámkách vydání (`update::verify`)
//! chrání jen před poškozeným stažením — kdo ovládne vydání na GitHubu, přepíše instalátor i řádek se
//! součtem. Podpis vzniká při vydání soukromým klíčem, který nikdy neopouští stroj vydavatele
//! (`scripts/publish.ps1`, klíč mimo repo, viz `docs/podpis-vydani.md`); veřejný klíč je zapečený v binárce
//! ([`EMBEDDED_KEY`] z `src-tauri/signing/update.pub`). Bez platného podpisu Ledge nic nespustí.
//!
//! **Formát** = `tauri signer sign`: soubor `<instalátor>.sig` je base64 textu minisign podpisu, veřejný
//! klíč je base64 textu `minisign.pub`. Podpis je „prehashed“ (BLAKE2b, `ED`); starý režim `Ed` se
//! odmítá. Důvěryhodný komentář (podepsaný spolu s podpisem) musí jmenovat přesně tenhle soubor
//! (`file:<asset>`) a verzi (`version:<x.y.z>`) — starý platně podepsaný instalátor tak nejde podstrčit pod
//! novější vydání (návrat ke zranitelné verzi).
//!
//! Čisté funkce nad bajty + čtení souboru po kusech; testy s klíči z `tauri signer generate`
//! (`fixtures/sign-*`).

use base64::Engine;
use minisign_verify::{Error as MsError, PublicKey, Signature};
use std::io::Read;
use std::path::Path;

/// Přípona podpisu vedle instalátoru (`Ledge_0.7.0_x64-setup.exe.sig`).
pub const SIG_SUFFIX: &str = ".sig";

/// Veřejný klíč vydání (base64 z `tauri signer generate`, `scripts/signing-setup.ps1`). Prázdný = sestavení
/// bez klíče (vývoj) → žádná aktualizace se nespustí.
pub const EMBEDDED_KEY: &str = include_str!("../signing/update.pub");

/// Jméno assetu s podpisem instalátoru `asset`.
pub fn sig_asset(asset: &str) -> String {
    format!("{asset}{SIG_SUFFIX}")
}

/// Proč podpis neprošel (pro UI přes [`SignError::text`]).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SignError {
    /// tohle sestavení Ledge nemá veřejný klíč
    NoKey,
    /// vydání podpis nemá (prázdný / chybí)
    Missing,
    /// podpis nebo klíč nejde přečíst
    Malformed,
    /// podepsáno jiným klíčem
    OtherKey,
    /// podpis nesedí na obsah (změněný bajt, změněný komentář)
    Invalid,
    /// platný podpis, ale jiného souboru nebo verze
    WrongFile,
}

impl SignError {
    /// Lidská věta s názvem vydání; vždy končí tím, že se nic nespustí.
    pub fn text(&self, tag: &str) -> String {
        match self {
            Self::NoKey => trf!(
                "Tohle sestavení Ledge nemá klíč k ověření podpisu, takže vydání {tag} nejde ověřit — nic nespouštím. Nainstaluj Ledge ze stránky vydání.",
                "This Ledge build has no key to check signatures, so release {tag} can't be verified — not running anything. Install Ledge from the releases page."
            ),
            Self::Missing => trf!(
                "Vydání {tag} není podepsané — nic nespouštím.",
                "Release {tag} isn't signed — not running anything."
            ),
            Self::Malformed => trf!(
                "Podpis vydání {tag} nejde přečíst — nic nespouštím.",
                "The signature of release {tag} can't be read — not running anything."
            ),
            Self::OtherKey => trf!(
                "Vydání {tag} podepsal jiný klíč než vydavatel Ledge — nic nespouštím.",
                "Release {tag} was signed by a different key than the Ledge publisher's — not running anything."
            ),
            Self::Invalid => trf!(
                "Podpis instalátoru vydání {tag} nesedí (soubor je změněný) — nic nespouštím.",
                "The installer signature of release {tag} doesn't match (the file was changed) — not running anything."
            ),
            Self::WrongFile => trf!(
                "Podpis patří jinému souboru nebo verzi než vydání {tag} — nic nespouštím.",
                "The signature belongs to a different file or version than release {tag} — not running anything."
            ),
        }
    }
}

/// Text minisign z obalu `tauri signer` (base64 celého textu). Holý text (`untrusted comment:` na začátku)
/// projde taky — podpis z `minisign -S` se dá použít beze změny.
fn unwrap_b64(s: &str) -> Option<String> {
    let s = s.trim();
    if s.starts_with("untrusted comment:") {
        return Some(s.to_string());
    }
    let raw = base64::engine::general_purpose::STANDARD.decode(s.as_bytes()).ok()?;
    String::from_utf8(raw).ok()
}

/// Veřejný klíč z base64 obalu (nebo textu `minisign.pub`).
pub fn parse_key(key: &str) -> Result<PublicKey, SignError> {
    if key.trim().is_empty() {
        return Err(SignError::NoKey);
    }
    unwrap_b64(key).and_then(|t| PublicKey::decode(&t).ok()).ok_or(SignError::Malformed)
}

fn parse_sig(sig: &str) -> Result<Signature, SignError> {
    if sig.trim().is_empty() {
        return Err(SignError::Missing);
    }
    unwrap_b64(sig).and_then(|t| Signature::decode(&t).ok()).ok_or(SignError::Malformed)
}

/// Důvěryhodný komentář `timestamp:…\tfile:<asset>\tversion:<x.y.z>` jmenuje přesně `asset` i `version`.
/// Čistá funkce.
pub fn comment_names(comment: &str, asset: &str, version: &str) -> bool {
    let field = |k: &str| comment.split('\t').map(str::trim).find_map(|p| p.strip_prefix(k));
    field("file:") == Some(asset) && field("version:") == Some(version)
}

fn map_err(e: MsError) -> SignError {
    match e {
        MsError::UnexpectedKeyId => SignError::OtherKey,
        MsError::InvalidSignature => SignError::Invalid,
        _ => SignError::Malformed,
    }
}

/// Postupné ověření: klíč, podpis, komentář → `feed` dostane funkci na kusy dat. Společné pro bajty i soubor.
fn verify_with(key: &str, sig: &str, asset: &str, version: &str, feed: impl FnOnce(&mut dyn FnMut(&[u8])) -> Result<(), SignError>) -> Result<(), SignError> {
    let pk = parse_key(key)?;
    let sig = parse_sig(sig)?;
    // legacy (`Ed`, bez předhashe) StreamVerifier odmítne sám
    let mut v = pk.verify_stream(&sig).map_err(map_err)?;
    feed(&mut |chunk| v.update(chunk))?;
    v.finalize().map_err(map_err)?;
    // komentář je pod globálním podpisem — číst ho má smysl až po úspěšném `finalize`
    if !comment_names(sig.trusted_comment(), asset, version) {
        return Err(SignError::WrongFile);
    }
    Ok(())
}

/// Podpis `sig` (obsah `.sig`) platí pro `data` pod klíčem `key` a jmenuje `asset` + `version`. Čistá funkce
/// (jádro testů; produkce čte soubor po kusech přes [`verify_file`] — stejné `verify_with`).
#[cfg(test)]
pub fn verify_bytes(key: &str, data: &[u8], sig: &str, asset: &str, version: &str) -> Result<(), SignError> {
    verify_with(key, sig, asset, version, |f| {
        f(data);
        Ok(())
    })
}

/// Totéž nad souborem (čte po 64 KiB, instalátor se nenačítá celý do paměti).
pub fn verify_file(key: &str, path: &Path, sig: &str, asset: &str, version: &str) -> Result<(), String> {
    let mut io_err = None;
    let res = verify_with(key, sig, asset, version, |f| {
        let mut file = std::fs::File::open(path).map_err(|e| {
            io_err = Some(crate::errors::io(&path.display().to_string(), &e));
            SignError::Invalid
        })?;
        let mut buf = vec![0u8; 64 * 1024];
        loop {
            match file.read(&mut buf) {
                Ok(0) => return Ok(()),
                Ok(n) => f(&buf[..n]),
                Err(e) => {
                    io_err = Some(crate::errors::io(&path.display().to_string(), &e));
                    return Err(SignError::Invalid);
                }
            }
        }
    });
    match (res, io_err) {
        (Ok(()), _) => Ok(()),
        (Err(_), Some(io)) => Err(io),
        (Err(e), None) => Err(e.text(&format!("v{version}"))),
    }
}

/// Klíč, proti kterému se ověřují vydání: v binárce zapečený, v testech testovací (`fixtures/sign-test.pub`),
/// aby testy prošly celou cestou stažení → cache → spuštění.
#[cfg(not(test))]
fn release_key() -> &'static str {
    EMBEDDED_KEY
}
#[cfg(test)]
fn release_key() -> &'static str {
    include_str!("fixtures/sign-test.pub")
}

/// Instalátor `path` vydání (`asset`, `version`) proti klíči vydání.
pub fn verify_release_file(path: &Path, sig: &str, asset: &str, version: &str) -> Result<(), String> {
    verify_file(release_key(), path, sig, asset, version)
}

#[cfg(test)]
mod tests {
    use super::*;

    // klíče a podpis z `tauri signer generate --ci -p test` / `tauri signer sign --app-version 0.7.0`
    // (soukromé klíče zahozené — jen pro test, nic s nimi nejde podepsat)
    const KEY: &str = include_str!("fixtures/sign-test.pub");
    const OTHER: &str = include_str!("fixtures/sign-other.pub");
    const DATA: &[u8] = include_bytes!("fixtures/sign-payload.bin");
    const SIG: &str = include_str!("fixtures/sign-payload.bin.sig");
    const ASSET: &str = "App_0.7.0_x64-setup.exe";

    fn decoded(s: &str) -> String {
        unwrap_b64(s).unwrap()
    }
    fn wrap(t: &str) -> String {
        base64::engine::general_purpose::STANDARD.encode(t.as_bytes())
    }

    #[test]
    fn verify_table() {
        let mut flipped = DATA.to_vec();
        flipped[3] ^= 0x01;
        let plain_sig = decoded(SIG);
        let plain_key = decoded(KEY);
        // změněný důvěryhodný komentář (jiná verze) — globální podpis ho kryje
        let forged_comment = wrap(&plain_sig.replace("version:0.7.0", "version:0.9.0"));
        // podpis přeložený na jiný soubor: první řádek bajtů jiný, ale platná délka
        let lines: Vec<&str> = plain_sig.lines().collect();
        let mut bin = base64::engine::general_purpose::STANDARD.decode(lines[1]).unwrap();
        bin[20] ^= 0xff;
        let forged_sig = wrap(&format!("{}\n{}\n{}\n{}\n", lines[0], base64::engine::general_purpose::STANDARD.encode(&bin), lines[2], lines[3]));
        // legacy algoritmus `Ed` (bez předhashe) se odmítá
        bin[1] = b'd';
        let legacy = wrap(&format!("{}\n{}\n{}\n{}\n", lines[0], base64::engine::general_purpose::STANDARD.encode(&bin), lines[2], lines[3]));
        #[rustfmt::skip]
        // (název, klíč, data, podpis, soubor, verze, výsledek)
        type Case<'a> = (&'a str, &'a str, &'a [u8], &'a str, &'a str, &'a str, Result<(), SignError>);
        let cases: Vec<Case> = vec![
            ("platný",                 KEY,   DATA,     SIG,             ASSET,                     "0.7.0", Ok(())),
            ("platný holý text",       KEY,   DATA,     &plain_sig,      ASSET,                     "0.7.0", Ok(())),
            ("holý klíč minisign.pub", &plain_key, DATA, SIG,         ASSET,                     "0.7.0", Ok(())),
            ("změněný bajt",           KEY,   &flipped, SIG,             ASSET,                     "0.7.0", Err(SignError::Invalid)),
            ("zkrácený soubor",        KEY,   &DATA[..DATA.len() - 1], SIG, ASSET,                  "0.7.0", Err(SignError::Invalid)),
            ("jiný klíč",              OTHER, DATA,     SIG,             ASSET,                     "0.7.0", Err(SignError::OtherKey)),
            ("chybí podpis",           KEY,   DATA,     "",              ASSET,                     "0.7.0", Err(SignError::Missing)),
            ("jen mezery",             KEY,   DATA,     "  \n",          ASSET,                     "0.7.0", Err(SignError::Missing)),
            ("nečitelný podpis",       KEY,   DATA,     "nesmysl!!",     ASSET,                     "0.7.0", Err(SignError::Malformed)),
            ("sestavení bez klíče",    "",    DATA,     SIG,             ASSET,                     "0.7.0", Err(SignError::NoKey)),
            ("nečitelný klíč",         "xx",  DATA,     SIG,             ASSET,                     "0.7.0", Err(SignError::Malformed)),
            ("změněný komentář",       KEY,   DATA,     &forged_comment, ASSET,                     "0.9.0", Err(SignError::Invalid)),
            ("změněný podpis",         KEY,   DATA,     &forged_sig,     ASSET,                     "0.7.0", Err(SignError::Invalid)),
            ("legacy Ed",              KEY,   DATA,     &legacy,         ASSET,                     "0.7.0", Err(SignError::Malformed)),
            ("jiný soubor",            KEY,   DATA,     SIG,             "App_0.8.0_x64-setup.exe", "0.7.0", Err(SignError::WrongFile)),
            ("jiná verze",             KEY,   DATA,     SIG,             ASSET,                     "0.8.0", Err(SignError::WrongFile)),
        ];
        for (name, key, data, sig, asset, ver, want) in cases {
            assert_eq!(verify_bytes(key, data, sig, asset, ver), want, "{name}");
        }
    }

    /// Zapečený klíč je buď prázdný (sestavení bez klíče, vše se odmítne), nebo platný veřejný klíč —
    /// rozbitý `signing/update.pub` nesmí projít do vydání.
    #[test]
    fn embedded_key_empty_or_valid() {
        if EMBEDDED_KEY.trim().is_empty() {
            assert_eq!(parse_key(EMBEDDED_KEY).unwrap_err(), SignError::NoKey);
        } else {
            assert!(parse_key(EMBEDDED_KEY).is_ok(), "signing/update.pub není platný veřejný klíč");
            assert_ne!(EMBEDDED_KEY.trim(), KEY.trim(), "do vydání nesmí testovací klíč");
        }
    }

    #[test]
    fn comment_fields_exact() {
        let c = "timestamp:1792846124\tfile:App_0.7.0_x64-setup.exe\tversion:0.7.0";
        assert!(comment_names(c, ASSET, "0.7.0"));
        assert!(!comment_names(c, "App_0.7.0_x64-setup.exe.sig", "0.7.0"));
        assert!(!comment_names(c, ASSET, "0.7"));
        assert!(!comment_names("timestamp:1\tfile:App_0.7.0_x64-setup.exe", ASSET, "0.7.0"), "bez verze neprojde");
        assert!(!comment_names("file:x/App_0.7.0_x64-setup.exe\tversion:0.7.0", ASSET, "0.7.0"));
    }

    #[test]
    fn file_verify_streams_and_reports() {
        let dir = tempfile::tempdir().unwrap();
        let f = dir.path().join(ASSET);
        std::fs::write(&f, DATA).unwrap();
        assert!(verify_file(KEY, &f, SIG, ASSET, "0.7.0").is_ok());
        // větší než jeden kus (64 KiB): stejný výsledek jako nad bajty
        let big = vec![7u8; 200 * 1024];
        std::fs::write(&f, &big).unwrap();
        let err = verify_file(KEY, &f, SIG, ASSET, "0.7.0").unwrap_err();
        assert!(err.contains("nic nespouštím") && err.contains("v0.7.0"), "{err}");
        assert!(verify_file(KEY, &dir.path().join("nic.exe"), SIG, ASSET, "0.7.0").is_err());
        assert!(verify_file(KEY, &f, "", ASSET, "0.7.0").unwrap_err().contains("není podepsané"));
    }

    #[test]
    fn texts_in_both_languages() {
        let all = [SignError::NoKey, SignError::Missing, SignError::Malformed, SignError::OtherKey, SignError::Invalid, SignError::WrongFile];
        for e in &all {
            assert!(e.text("v1.2.3").contains("v1.2.3") && e.text("v1.2.3").contains("nic nespouštím"), "{e:?}");
        }
        crate::i18n::with_lang(crate::uiconf::Lang::En, || {
            for e in &all {
                assert!(e.text("v1.2.3").contains("not running anything"), "{e:?}");
            }
        });
        assert_eq!(sig_asset(ASSET), "App_0.7.0_x64-setup.exe.sig");
    }
}
