//! Parsování výstupu `gh` (JSON) na malé struktury karty GitHub. Čisté funkce nad textem — žádné I/O,
//! žádný stav; hlídané vzorky v `src/fixtures/gh-*.json`. Parsování je shovívavé: co nejde přečíst
//! (chybějící pole, cizí tvar), se zahodí, nikdy to nespadne. Adresy smí vést jen na `https://github.com/`.

use serde::Serialize;
use serde_json::Value;

/// Kolik položek každého seznamu karta ukáže (zbytek jde jen do počtu).
pub const SHOWN: usize = 5;
const TITLE_MAX: usize = 160;
const WEB: &str = "https://github.com/";

/// Stav kontrol (CI) u PR.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Ci {
    /// PR nemá žádné kontroly
    None,
    Pending,
    Success,
    Failure,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Pr {
    /// `vlastník/repo`
    pub repo: String,
    pub number: u64,
    pub title: String,
    /// adresa na github.com; prázdná = cizí adresa, neotevře se
    pub url: String,
    /// autor (u cizích PR k revizi; u vlastních prázdné)
    pub author: String,
    pub draft: bool,
    /// ISO čas poslední změny
    pub updated: String,
    pub ci: Ci,
    /// `APPROVED`, `CHANGES_REQUESTED`, `REVIEW_REQUIRED` nebo prázdné
    pub decision: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Notif {
    pub title: String,
    pub repo: String,
    /// `PullRequest`, `Issue`, `Release`, `CheckSuite`…
    pub kind: String,
    /// `review_requested`, `mention`, `subscribed`…
    pub reason: String,
    /// webová adresa (z API adresy přepsaná), jinak stránka repa
    pub url: String,
    pub updated: String,
}

fn clean(s: &str) -> String {
    let flat: String = s.chars().map(|c| if c.is_control() { ' ' } else { c }).collect();
    let flat = flat.split_whitespace().collect::<Vec<_>>().join(" ");
    if flat.chars().count() > TITLE_MAX { format!("{}…", flat.chars().take(TITLE_MAX).collect::<String>().trim_end()) } else { flat }
}

fn text(v: &Value, key: &str) -> String {
    v.get(key).and_then(Value::as_str).map(clean).unwrap_or_default()
}

/// Adresa smí vést jen na github.com přes https; jinak prázdná.
pub fn safe_url(u: &str) -> String {
    if u.starts_with(WEB) && !u.chars().any(|c| c.is_control() || c.is_whitespace()) { u.to_string() } else { String::new() }
}

/// `SUCCESS` / `FAILURE` / `ERROR` / `PENDING` / `EXPECTED` z `statusCheckRollup.state` → stav.
pub fn ci_of(state: Option<&str>) -> Ci {
    match state {
        Some("SUCCESS") => Ci::Success,
        Some("FAILURE" | "ERROR") => Ci::Failure,
        Some("PENDING" | "EXPECTED") => Ci::Pending,
        _ => Ci::None,
    }
}

/// `gh search prs --json number,title,url,repository,author,updatedAt,isDraft` → PR k revizi.
pub fn parse_search(json: &str) -> Option<Vec<Pr>> {
    let v: Value = serde_json::from_str(json).ok()?;
    let arr = v.as_array()?;
    Some(
        arr.iter()
            .filter_map(|p| {
                let number = p.get("number")?.as_u64()?;
                let repo = p.pointer("/repository/nameWithOwner")?.as_str().map(clean)?;
                let title = text(p, "title");
                (!repo.is_empty() && !title.is_empty()).then(|| Pr {
                    repo,
                    number,
                    title,
                    url: safe_url(p.get("url").and_then(Value::as_str).unwrap_or("")),
                    author: p.pointer("/author/login").and_then(Value::as_str).map(clean).unwrap_or_default(),
                    draft: p.get("isDraft").and_then(Value::as_bool).unwrap_or(false),
                    updated: text(p, "updatedAt"),
                    ci: Ci::None,
                    decision: String::new(),
                })
            })
            .collect(),
    )
}

/// Odpověď GraphQL `viewer.pullRequests` → (celkový počet, nejnovější PR se stavem CI).
pub fn parse_mine(json: &str) -> Option<(usize, Vec<Pr>)> {
    let v: Value = serde_json::from_str(json).ok()?;
    let prs = v.pointer("/data/viewer/pullRequests")?;
    let total = prs.get("totalCount").and_then(Value::as_u64).unwrap_or(0) as usize;
    let list = prs
        .get("nodes")?
        .as_array()?
        .iter()
        .filter_map(|p| {
            let number = p.get("number")?.as_u64()?;
            let repo = p.pointer("/repository/nameWithOwner")?.as_str().map(clean)?;
            let title = text(p, "title");
            (!repo.is_empty() && !title.is_empty()).then(|| Pr {
                repo,
                number,
                title,
                url: safe_url(p.get("url").and_then(Value::as_str).unwrap_or("")),
                author: String::new(),
                draft: p.get("isDraft").and_then(Value::as_bool).unwrap_or(false),
                updated: text(p, "updatedAt"),
                ci: ci_of(p.pointer("/commits/nodes/0/commit/statusCheckRollup/state").and_then(Value::as_str)),
                decision: text(p, "reviewDecision"),
            })
        })
        .collect::<Vec<_>>();
    Some((total.max(list.len()), list))
}

/// Webová adresa z API adresy předmětu notifikace (`api.github.com/repos/o/r/pulls/5` →
/// `github.com/o/r/pull/5`). Co se nepozná (vydání, kontroly, bez adresy), vede na stránku repa; bez ní
/// na seznam notifikací.
pub fn web_url(subject_api_url: Option<&str>, repo_html: Option<&str>) -> String {
    const API: &str = "https://api.github.com/repos/";
    if let Some(rest) = subject_api_url.and_then(|u| u.strip_prefix(API)) {
        let parts: Vec<&str> = rest.split('/').collect();
        if let [owner, repo, kind, id] = parts[..] {
            let seg = match kind {
                "pulls" => Some("pull"),
                "issues" => Some("issues"),
                "commits" => Some("commit"),
                "discussions" => Some("discussions"),
                _ => None,
            };
            let ok = |s: &str| !s.is_empty() && s.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.'));
            if let Some(seg) = seg.filter(|_| ok(owner) && ok(repo) && ok(id)) {
                return format!("{WEB}{owner}/{repo}/{seg}/{id}");
            }
        }
    }
    let repo = safe_url(repo_html.unwrap_or(""));
    if repo.is_empty() { format!("{WEB}notifications") } else { repo }
}

/// `gh api notifications` → (počet v odpovědi, nejnovější `SHOWN`). Počet je nejvýš tolik, kolik jsme
/// si vyžádali (`per_page`) — volající ho podle toho označí „50+“.
pub fn parse_notifications(json: &str) -> Option<(usize, Vec<Notif>)> {
    let v: Value = serde_json::from_str(json).ok()?;
    let arr = v.as_array()?;
    let list = arr
        .iter()
        .filter_map(|n| {
            let title = n.pointer("/subject/title").and_then(Value::as_str).map(clean)?;
            let repo = n.pointer("/repository/full_name").and_then(Value::as_str).map(clean).unwrap_or_default();
            (!title.is_empty()).then(|| Notif {
                title,
                repo,
                kind: n.pointer("/subject/type").and_then(Value::as_str).map(clean).unwrap_or_default(),
                reason: text(n, "reason"),
                url: web_url(n.pointer("/subject/url").and_then(Value::as_str), n.pointer("/repository/html_url").and_then(Value::as_str)),
                updated: text(n, "updated_at"),
            })
        })
        .collect::<Vec<_>>();
    Some((list.len(), list.into_iter().take(SHOWN).collect()))
}

#[cfg(test)]
mod tests {
    use super::*;

    const SEARCH: &str = include_str!("../fixtures/gh-search-prs.json");
    const MINE: &str = include_str!("../fixtures/gh-graphql-viewer-prs.json");
    const NOTIFS: &str = include_str!("../fixtures/gh-notifications.json");

    #[test]
    fn search_prs_sample() {
        let v = parse_search(SEARCH).unwrap();
        // čtvrtá položka (číslo „x“, bez repa) se zahodí
        assert_eq!(v.len(), 3);
        assert_eq!((v[0].repo.as_str(), v[0].number, v[0].author.as_str(), v[0].draft), ("acme/ledge", 412, "octocat", false));
        assert_eq!(v[0].url, "https://github.com/acme/ledge/pull/412");
        assert_eq!(v[0].updated, "2026-10-17T15:26:56Z");
        assert!(v[1].draft);
        assert_eq!(v[1].author, "dependabot[bot]");
        // řídicí znaky v názvu se slijí do mezer; cizí adresa se nepustí
        assert_eq!(v[2].title, "Řádek s novým řádkem a tabulátorem");
        assert_eq!(v[2].url, "");
        assert!(v.iter().all(|p| p.ci == Ci::None && p.decision.is_empty()));
    }

    #[test]
    fn viewer_prs_sample_with_ci() {
        let (total, v) = parse_mine(MINE).unwrap();
        assert_eq!(total, 23, "celkem otevřených (víc než ukázaných)");
        let ci: Vec<_> = v.iter().map(|p| p.ci).collect();
        assert_eq!(ci, vec![Ci::Success, Ci::Pending, Ci::Failure, Ci::None]);
        assert_eq!(v[0].decision, "REVIEW_REQUIRED");
        assert_eq!(v[1].decision, "", "null → prázdné");
        assert!(v[1].draft);
        assert_eq!(v[2].repo, "acme/tools");
    }

    #[test]
    fn notifications_sample() {
        let (n, v) = parse_notifications(NOTIFS).unwrap();
        assert_eq!(n, 6);
        assert_eq!(v.len(), SHOWN, "ukáže jen nejnovějších pět");
        let urls: Vec<_> = v.iter().map(|x| x.url.as_str()).collect();
        assert_eq!(
            urls,
            vec![
                "https://github.com/acme/ledge/pull/501",
                "https://github.com/acme/ledge/issues/77",
                "https://github.com/acme/ledge", // vydání → stránka repa
                "https://github.com/acme/tools",  // bez adresy předmětu → repo
                "https://github.com/acme/tools/commit/abc123",
            ]
        );
        assert_eq!((v[0].kind.as_str(), v[0].reason.as_str(), v[0].repo.as_str()), ("PullRequest", "review_requested", "acme/ledge"));
    }

    #[test]
    fn web_url_table() {
        let api = |p: &str| Some(format!("https://api.github.com/repos/{p}"));
        let cases: [(&str, Option<String>, Option<&str>, &str); 8] = [
            ("pull", api("o/r/pulls/5"), None, "https://github.com/o/r/pull/5"),
            ("issue", api("o/r/issues/9"), None, "https://github.com/o/r/issues/9"),
            ("diskuse", api("o/r/discussions/3"), None, "https://github.com/o/r/discussions/3"),
            ("vydání → repo", api("o/r/releases/1"), Some("https://github.com/o/r"), "https://github.com/o/r"),
            ("nic → seznam notifikací", None, None, "https://github.com/notifications"),
            ("cizí repo adresa se nepustí", None, Some("https://evil.example/o/r"), "https://github.com/notifications"),
            ("podivné id v adrese", api("o/r/pulls/5%2F..%2Fx"), Some("https://github.com/o/r"), "https://github.com/o/r"),
            ("cizí API host", Some("https://evil.example/repos/o/r/pulls/5".into()), None, "https://github.com/notifications"),
        ];
        for (name, subject, repo, want) in cases {
            assert_eq!(web_url(subject.as_deref(), repo), want, "{name}");
        }
    }

    #[test]
    fn ci_states() {
        let cases = [(Some("SUCCESS"), Ci::Success), (Some("FAILURE"), Ci::Failure), (Some("ERROR"), Ci::Failure), (Some("PENDING"), Ci::Pending), (Some("EXPECTED"), Ci::Pending), (Some("???"), Ci::None), (None, Ci::None)];
        for (s, want) in cases {
            assert_eq!(ci_of(s), want, "{s:?}");
        }
    }

    #[test]
    fn broken_input_is_none_not_a_panic() {
        for bad in ["", "nic", "{}", "[1,2", "null"] {
            assert!(parse_search(bad).is_none(), "{bad}");
            assert!(parse_mine(bad).is_none(), "{bad}");
            assert!(parse_notifications(bad).is_none(), "{bad}");
        }
        assert_eq!(parse_search("[]"), Some(vec![]));
        assert_eq!(parse_search("[1, null, \"x\"]"), Some(vec![]));
        assert_eq!(parse_notifications("[]"), Some((0, vec![])));
        // GraphQL chyba místo dat
        assert!(parse_mine(r#"{"errors":[{"message":"Bad credentials"}]}"#).is_none());
        // příliš dlouhý název se ořízne
        let long = format!(r#"[{{"number":1,"title":"{}","repository":{{"nameWithOwner":"a/b"}}}}]"#, "x".repeat(500));
        assert!(parse_search(&long).unwrap()[0].title.chars().count() <= TITLE_MAX + 1);
    }
}
