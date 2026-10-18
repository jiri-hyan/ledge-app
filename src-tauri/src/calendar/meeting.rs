//! Odkaz „Připojit“ ke schůzce: z textu události (vlastnosti Teams/Google, URL, místo, popis i HTML
//! popis Outlooku) vytáhne první odkaz na známou službu — Teams, Google Meet, Zoom, Webex, Jitsi.
//!
//! Čistá logika bez sítě. Přijme jen `https://` (http se u známých služeb povýší) na přesně známých
//! doménách a cestách — nic jiného se nikdy neotevře. Obalové odkazy (Outlook Safe Links, přesměrování
//! Googlu `google.com/url?q=`) se rozbalí. Teams jde otevřít rovnou v aplikaci (`msteams:`) jen u
//! ověřeného tvaru cesty bez podezřelých znaků (`app_url`); jinak https.

use serde::{Deserialize, Serialize};

/// Nejdelší odkaz, který Ledge vezme (delší = poškozený / podvržený).
pub const MAX_URL: usize = 2048;
/// Kolik odkazů z jednoho textu nejvýš projít (obří popis s patičkami).
const MAX_SCAN: usize = 64;

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Provider {
    Teams,
    Meet,
    Zoom,
    Webex,
    Jitsi,
}

/// Odkaz na schůzku: služba + ověřená `https://` adresa.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct Join {
    pub provider: Provider,
    pub url: String,
}

/// Pořadí zdrojů v události (nižší = spolehlivější): vlastnosti schůzky → URL → místo → popis.
pub const RANK_CONFERENCE: u8 = 0;
pub const RANK_URL: u8 = 1;
pub const RANK_LOCATION: u8 = 2;
pub const RANK_DESCRIPTION: u8 = 3;

/// Vlastnost ICS → pořadí, když může nést odkaz na schůzku.
pub fn rank_of(prop: &str) -> Option<u8> {
    match prop {
        "X-MICROSOFT-SKYPETEAMSMEETINGURL" | "X-GOOGLE-CONFERENCE" | "CONFERENCE" | "X-MICROSOFT-ONLINEMEETINGEXTERNALLINK" => Some(RANK_CONFERENCE),
        "URL" => Some(RANK_URL),
        "LOCATION" => Some(RANK_LOCATION),
        "DESCRIPTION" | "X-ALT-DESC" => Some(RANK_DESCRIPTION),
        _ => None,
    }
}

/// Lepší kandidát vyhrává: `cur` (pořadí, odkaz) se nahradí odkazem z `text`, když má `rank` nižší.
pub fn offer(cur: &mut Option<(u8, Join)>, rank: u8, text: &str) {
    if cur.as_ref().is_some_and(|(r, _)| *r <= rank) {
        return;
    }
    if let Some(j) = find(text) {
        *cur = Some((rank, j));
    }
}

/// První odkaz na známou službu v textu (prostý text, `<https://…>`, HTML `href="…"`).
pub fn find(text: &str) -> Option<Join> {
    links(text).into_iter().find_map(|u| classify(&u))
}

/// Konec odkazu v textu (mezery, HTML, uvozovky, závorky ze zápisu `<https://…>`).
fn stop(c: char) -> bool {
    c.is_whitespace() || c.is_control() || matches!(c, '<' | '>' | '"' | '\'' | '`' | '|' | '\\' | '[' | ']' | '{' | '}' | '^')
}

/// Všechny `http(s)://…` v textu (nejvýš `MAX_SCAN`), s dekódovaným `&amp;` a bez koncové interpunkce.
pub fn links(text: &str) -> Vec<String> {
    let lower = text.to_ascii_lowercase();
    let mut out = Vec::new();
    let mut from = 0;
    while out.len() < MAX_SCAN {
        let Some(i) = ["https://", "http://"].iter().filter_map(|p| lower[from..].find(p)).min() else { break };
        let start = from + i;
        let end = text[start..].char_indices().find(|&(_, c)| stop(c)).map_or(text.len(), |(j, _)| start + j);
        let raw = text[start..end].replace("&amp;", "&").replace("&#38;", "&");
        let url = raw.trim_end_matches(['.', ',', ';', ':', '!', '?', ')', '*']).to_string();
        if url.len() > "https://".len() {
            out.push(url);
        }
        from = end.max(start + 1);
    }
    out
}

/// Rozdělení `scheme://host/path?query#frag` — host malými písmeny, bez portu; `None` = ne http(s)
/// nebo přihlašovací údaje v adrese.
struct Parts<'a> {
    /// doména i s portem, jak je v odkazu
    auth: &'a str,
    host: String,
    /// cesta + dotaz + fragment (to, co jde za doménu)
    tail: &'a str,
    path: String,
}

fn split(url: &str) -> Option<Parts<'_>> {
    let lower = url.get(..8)?.to_ascii_lowercase();
    let rest = if lower.starts_with("https://") {
        &url[8..]
    } else if lower.starts_with("http://") {
        &url[7..]
    } else {
        return None;
    };
    let cut = rest.find(['/', '?', '#']).unwrap_or(rest.len());
    let auth = &rest[..cut];
    if auth.is_empty() || auth.contains('@') {
        return None;
    }
    let host = auth.split(':').next().unwrap_or("").trim_end_matches('.').to_ascii_lowercase();
    let tail = &rest[cut..];
    let path = tail.split(['?', '#']).next().unwrap_or("").to_ascii_lowercase();
    Some(Parts { auth, host, tail, path })
}

/// Hodnota parametru dotazu (procentové kódování rozbalené).
fn query_param(tail: &str, key: &str) -> Option<String> {
    let q = tail.split_once('?')?.1.split('#').next().unwrap_or("");
    q.split('&').find_map(|kv| kv.split_once('=').filter(|(k, _)| k.eq_ignore_ascii_case(key)).map(|(_, v)| percent_decode(v)))
}

pub fn percent_decode(s: &str) -> String {
    let b = s.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        let hex = |c: u8| (c as char).to_digit(16);
        match b[i] {
            b'%' if i + 2 < b.len() => {
                if let (Some(h), Some(l)) = (hex(b[i + 1]), hex(b[i + 2])) {
                    out.push((h * 16 + l) as u8);
                    i += 3;
                    continue;
                }
                out.push(b'%');
            }
            b'+' => out.push(b' '),
            c => out.push(c),
        }
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// Obalový odkaz (Outlook Safe Links, přesměrování Googlu) → vnitřní adresa.
fn unwrap_redirect(url: &str) -> Option<String> {
    let p = split(url)?;
    let inner = if p.host.ends_with("safelinks.protection.outlook.com") {
        query_param(p.tail, "url")
    } else if matches!(p.host.as_str(), "www.google.com" | "google.com") && p.path == "/url" {
        query_param(p.tail, "q")
    } else {
        None
    }?;
    inner.to_ascii_lowercase().starts_with("http").then_some(inner)
}

fn is_or_sub(host: &str, domain: &str) -> bool {
    host == domain || host.strip_suffix(domain).is_some_and(|h| h.ends_with('.'))
}

/// `abc-defg-hij` (kód Google Meet).
fn meet_code(seg: &str) -> bool {
    let p: Vec<&str> = seg.split('-').collect();
    p.len() == 3 && [3, 4, 3].iter().zip(&p).all(|(n, s)| s.len() == *n && s.bytes().all(|c| c.is_ascii_lowercase()))
}

/// Odkaz → známá služba s https adresou; cokoli jiného (nápověda, nastavení schůzky, web služby) `None`.
pub fn classify(url: &str) -> Option<Join> {
    let mut url = url.trim().to_string();
    for _ in 0..2 {
        match unwrap_redirect(&url) {
            Some(inner) => url = inner,
            None => break,
        }
    }
    if url.len() > MAX_URL || url.chars().any(|c| c.is_whitespace() || c.is_control() || matches!(c, '"' | '<' | '>' | '\\' | '`')) {
        return None;
    }
    let p = split(&url)?;
    let path = p.path.as_str();
    let (h, has) = (p.host.as_str(), |pre: &str| path.starts_with(pre) && path.len() > pre.len());
    let provider = if (is_or_sub(h, "teams.microsoft.com") || h == "teams.live.com" || is_or_sub(h, "teams.microsoft.us")) && (has("/l/meetup-join/") || has("/meet/")) {
        Provider::Teams
    } else if h == "meet.google.com" && (meet_code(path.trim_start_matches('/').split('/').next().unwrap_or("")) || has("/lookup/")) {
        Provider::Meet
    } else if (is_or_sub(h, "zoom.us") || is_or_sub(h, "zoomgov.com") || is_or_sub(h, "zoom.com")) && ["/j/", "/w/", "/my/", "/s/", "/wc/join/"].iter().any(|x| has(x)) {
        Provider::Zoom
    } else if h.ends_with(".webex.com") && !matches!(h, "www.webex.com" | "help.webex.com") && (path.contains("/j.php") || has("/meet/") || has("/join/") || path.contains("/joinservice/")) {
        Provider::Webex
    } else if matches!(h, "meet.jit.si" | "8x8.vc") && path.trim_matches('/').split('/').next().is_some_and(|room| !room.is_empty() && !matches!(room, "static" | "libs" | "images" | "sounds")) {
        Provider::Jitsi
    } else {
        return None;
    };
    Some(Join { provider, url: format!("https://{}{}", p.auth, p.tail) })
}

/// Teams rovnou v aplikaci: `msteams:/l/meetup-join/…` — jen `teams.microsoft.com` s ověřenou cestou
/// a znaky, které v odkazu Teams bývají (žádné uvozovky, mezery, `\`). Jinak `None` = otevřít https.
pub fn app_url(j: &Join) -> Option<String> {
    if j.provider != Provider::Teams {
        return None;
    }
    let p = split(&j.url)?;
    let ok_path = p.path.starts_with("/l/meetup-join/") || p.path.starts_with("/meet/");
    let ok_chars = p.tail.bytes().all(|c| c.is_ascii_alphanumeric() || b"-._~%/?=&:+@!$,;*".contains(&c));
    (p.host == "teams.microsoft.com" && ok_path && ok_chars).then(|| format!("msteams:{}", p.tail))
}

/// Má Windows obsluhu protokolu `msteams:` (nainstalované Teams)? Jen dotaz na asociaci, nic nespouští.
fn teams_handler() -> bool {
    use windows::Win32::UI::Shell::{ASSOCF_IS_PROTOCOL, ASSOCSTR_FRIENDLYAPPNAME, AssocQueryStringW};
    let mut len = 0u32;
    unsafe { AssocQueryStringW(ASSOCF_IS_PROTOCOL, ASSOCSTR_FRIENDLYAPPNAME, windows::core::w!("msteams"), windows::core::PCWSTR::null(), None, &mut len) }.is_ok() && len > 1
}

/// Co otevřít po kliku na Připojit: Teams v aplikaci, když je bezpečné (`app_url`) a Teams jsou
/// nainstalované, jinak ověřená https adresa (prohlížeč / aplikace služby si ji převezme sama).
pub fn launch_uri(j: &Join) -> String {
    app_url(j).filter(|_| teams_handler()).unwrap_or_else(|| j.url.clone())
}

#[cfg(test)]
mod tests {
    use super::*;
    use Provider::*;

    const TEAMS: &str = "https://teams.microsoft.com/l/meetup-join/19%3ameeting_NjM4ZTk1YjQtZmQ0Ni00%40thread.v2/0?context=%7b%22Tid%22%3a%2272f988bf%22%2c%22Oid%22%3a%22a1b2%22%7d";

    #[test]
    fn find_table_real_shapes() {
        type Case<'a> = (&'a str, String, Option<(Provider, &'a str)>);
        let cases: Vec<Case> = vec![
            ("prostý odkaz", format!("Připoj se: {TEAMS}"), Some((Teams, TEAMS))),
            (
                "Outlook prostý text: <https://…> a nápověda před ním",
                format!("________________\r\nMicrosoft Teams Need help?<https://aka.ms/JoinTeamsMeeting>\r\nJoin the meeting now<{TEAMS}>\r\nMeeting ID: 123 456 789\r\nMeeting options<https://teams.microsoft.com/meetingOptions/?organizerId=a1b2&tenantId=72f9>"),
                Some((Teams, TEAMS)),
            ),
            (
                "Outlook HTML (X-ALT-DESC) s &amp;",
                r#"<html><body><a href="https://teams.microsoft.com/l/meetup-join/19%3ameeting_X%40thread.v2/0?context=%7b%7d&amp;tenant=1" target="_blank">Join the meeting now</a></body></html>"#.to_string(),
                Some((Teams, "https://teams.microsoft.com/l/meetup-join/19%3ameeting_X%40thread.v2/0?context=%7b%7d&tenant=1")),
            ),
            ("nový tvar Teams /meet/", "Join: https://teams.microsoft.com/meet/34567890123?p=AbCdEf123".into(), Some((Teams, "https://teams.microsoft.com/meet/34567890123?p=AbCdEf123"))),
            ("Teams pro domácnosti", "https://teams.live.com/meet/9876543210?p=xyz".into(), Some((Teams, "https://teams.live.com/meet/9876543210?p=xyz"))),
            ("jen nastavení schůzky Teams = nic", "Meeting options<https://teams.microsoft.com/meetingOptions/?x=1>".into(), None),
            (
                "Google popis: nápověda, pak Meet",
                "Připojit se přes Google Meet: https://meet.google.com/abc-defg-hij\nDalší informace o Meet: https://support.google.com/a/users/answer/9282720".into(),
                Some((Meet, "https://meet.google.com/abc-defg-hij")),
            ),
            ("Meet se zbytkem dotazu", "https://meet.google.com/abc-defg-hij?authuser=0&hs=122.".into(), Some((Meet, "https://meet.google.com/abc-defg-hij?authuser=0&hs=122"))),
            ("Meet bez kódu = nic", "https://meet.google.com/".into(), None),
            ("Meet telefon = nic", "https://tel.meet/abc-defg-hij?pin=123".into(), None),
            ("Zoom v místě", "https://us02web.zoom.us/j/81234567890?pwd=QWErty12".into(), Some((Zoom, "https://us02web.zoom.us/j/81234567890?pwd=QWErty12"))),
            ("Zoom osobní místnost, http → https", "(http://firma.zoom.us/my/jana.novak)".into(), Some((Zoom, "https://firma.zoom.us/my/jana.novak"))),
            ("Zoom web = nic", "https://zoom.us/download".into(), None),
            ("Webex", "Join: https://firma.webex.com/firma/j.php?MTID=m0123abcd".into(), Some((Webex, "https://firma.webex.com/firma/j.php?MTID=m0123abcd"))),
            ("Webex osobní", "https://firma.webex.com/meet/jana.novak".into(), Some((Webex, "https://firma.webex.com/meet/jana.novak"))),
            ("Webex web = nic", "https://www.webex.com/downloads.html".into(), None),
            ("Jitsi", "Místnost: https://meet.jit.si/LedgeTym2026, heslo u Jany".into(), Some((Jitsi, "https://meet.jit.si/LedgeTym2026"))),
            ("Jitsi bez místnosti = nic", "https://meet.jit.si/".into(), None),
            (
                "Outlook Safe Links obal",
                "<https://eur01.safelinks.protection.outlook.com/?url=https%3A%2F%2Fus02web.zoom.us%2Fj%2F81234567890%3Fpwd%3Dabc&data=05%7C01&reserved=0>".into(),
                Some((Zoom, "https://us02web.zoom.us/j/81234567890?pwd=abc")),
            ),
            (
                "přesměrování Googlu v HTML popisu",
                r#"<a href="https://www.google.com/url?q=https%3A%2F%2Fmeet.jit.si%2FPorada&amp;sa=D&amp;source=calendar">Porada</a>"#.into(),
                Some((Jitsi, "https://meet.jit.si/Porada")),
            ),
            ("podvržená doména = nic", "https://teams.microsoft.com.evil.cz/l/meetup-join/x".into(), None),
            ("přihlášení v adrese = nic", "https://teams.microsoft.com@evil.cz/l/meetup-join/x".into(), None),
            ("jiné schéma = nic", "zoommtg://zoom.us/join?confno=1 ftp://meet.jit.si/x".into(), None),
            ("Microsoft Teams Meeting (bez odkazu)", "Microsoft Teams Meeting".into(), None),
            ("prázdno", String::new(), None),
        ];
        for (name, text, want) in cases {
            let got = find(&text);
            assert_eq!(got.as_ref().map(|j| (j.provider, j.url.as_str())), want, "{name}");
        }
    }

    #[test]
    fn rank_prefers_conference_property() {
        let mut cur = None;
        offer(&mut cur, RANK_DESCRIPTION, "https://meet.jit.si/Popis");
        assert_eq!(cur.as_ref().map(|(r, j)| (*r, j.provider)), Some((RANK_DESCRIPTION, Jitsi)));
        offer(&mut cur, RANK_LOCATION, "Kancelář 4.12");
        assert_eq!(cur.as_ref().map(|(r, _)| *r), Some(RANK_DESCRIPTION), "místo bez odkazu nic nemění");
        offer(&mut cur, RANK_CONFERENCE, TEAMS);
        assert_eq!(cur.as_ref().map(|(r, j)| (*r, j.provider)), Some((RANK_CONFERENCE, Teams)));
        offer(&mut cur, RANK_DESCRIPTION, "https://meet.google.com/abc-defg-hij");
        assert_eq!(cur.as_ref().map(|(_, j)| j.provider), Some(Teams), "horší zdroj nepřepíše lepší");
        assert_eq!(rank_of("X-MICROSOFT-SKYPETEAMSMEETINGURL"), Some(RANK_CONFERENCE));
        assert_eq!(rank_of("X-ALT-DESC"), Some(RANK_DESCRIPTION));
        assert_eq!(rank_of("SUMMARY"), None);
    }

    #[test]
    fn app_url_only_when_safe() {
        let j = |p, u: &str| Join { provider: p, url: u.into() };
        let cases = [
            (j(Teams, TEAMS), Some(format!("msteams:{}", &TEAMS["https://teams.microsoft.com".len()..]))),
            (j(Teams, "https://teams.microsoft.com/meet/345?p=Ab"), Some("msteams:/meet/345?p=Ab".to_string())),
            // teams.live.com a gov nemají ověřený tvar pro aplikaci → https
            (j(Teams, "https://teams.live.com/meet/987?p=x"), None),
            (j(Teams, "https://teams.microsoft.com/l/meetup-join/x'y"), None),
            (j(Teams, "https://teams.microsoft.com/l/chat/0/0"), None),
            (j(Zoom, "https://zoom.us/j/1"), None),
        ];
        for (join, want) in cases {
            assert_eq!(app_url(&join), want, "{}", join.url);
        }
    }

    #[test]
    fn percent_decode_table() {
        let cases = [("a%2Fb", "a/b"), ("%3a%3A", "::"), ("100%", "100%"), ("%zz", "%zz"), ("a+b", "a b"), ("%C5%AF", "ů"), ("%4", "%4")];
        for (raw, want) in cases {
            assert_eq!(percent_decode(raw), want, "{raw}");
        }
    }
}
