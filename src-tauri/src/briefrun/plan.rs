//! Čisté rozhodnutí, zda a jak zadání spustit: projekt, MCP, klíč, stav (pokračovat jen z
//! `waiting` / `review`), model a zbývající rozpočet. Každé „ne“ je lidská věta pro hlášku.

use super::args::{clean_model, valid_session};
use super::note::Info;
use crate::i18n::tr;

/// Co z Ledge vstupuje do rozhodnutí (mimo poznámku).
#[derive(Debug, Clone, PartialEq)]
pub struct Env {
    pub mcp_enabled: bool,
    pub has_key: bool,
    pub default_model: String,
    pub default_budget: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Plan {
    pub model: String,
    pub budget: f64,
    /// `budget − cost`
    pub left: f64,
    /// `Some` při pokračování (uložená session)
    pub resume_session: Option<String>,
}

/// Nejmenší zbytek rozpočtu, se kterým má běh smysl.
pub const MIN_LEFT: f64 = 0.01;

pub fn plan(info: &Info, resume: bool, env: &Env) -> Result<Plan, String> {
    if info.project.is_none() {
        return Err(tr("Zadání nemá projekt — vyber ho v poznámce (session poběží v jeho složce).", "The brief has no project — pick one in the note (the session runs in its folder).").into());
    }
    if !env.mcp_enabled {
        return Err(tr("Zapni MCP v Nastavení → Claude → MCP — session přes něj čte zadání a zapisuje otázky.", "Turn on MCP in Settings → Claude → MCP — the session reads the brief and writes questions through it.").into());
    }
    if !env.has_key {
        return Err(tr("Chybí API klíč — ulož ho v Nastavení → Claude → API klíč.", "No API key — save it in Settings → Claude → API key.").into());
    }
    let status = info.status.as_deref().unwrap_or("draft");
    let resume_session = if resume {
        if !matches!(status, "waiting" | "review") {
            return Err(tr("Pokračovat jde jen u zadání, které čeká na odpověď nebo kontrolu.", "You can only continue a brief that waits for an answer or a review.").into());
        }
        match info.session.as_deref().filter(|s| valid_session(s)) {
            Some(s) => Some(s.to_string()),
            None => return Err(tr("Zadání nemá platnou session — pošli ho znovu.", "The brief has no valid session — send it again.").into()),
        }
    } else {
        if status == "done" {
            return Err(tr("Zadání je hotové.", "The brief is done.").into());
        }
        None
    };
    let model = clean_model(info.model.as_deref().unwrap_or(&env.default_model))?;
    let budget = info.budget.unwrap_or(env.default_budget);
    let left = budget - info.cost;
    if left < MIN_LEFT {
        return Err(trf!(
            "Rozpočet zadání je vyčerpaný (${:.2} z ${:.2}) — zvyš `budget` v poznámce.",
            "The brief's budget is used up (${:.2} of ${:.2}) — raise `budget` in the note.",
            info.cost,
            budget
        ));
    }
    Ok(Plan { model, budget, left, resume_session })
}

#[cfg(test)]
mod tests {
    use super::*;

    const S: &str = "0f8fad5b-d9cb-469f-a165-70867728950e";

    fn env() -> Env {
        Env { mcp_enabled: true, has_key: true, default_model: "sonnet".into(), default_budget: 2.0 }
    }
    fn info(status: Option<&str>) -> Info {
        Info { project: Some("app".into()), status: status.map(str::to_string), session: Some(S.into()), ..Default::default() }
    }

    #[test]
    fn plan_table() {
        let ok = plan(&info(None), false, &env()).unwrap();
        assert_eq!(ok, Plan { model: "sonnet".into(), budget: 2.0, left: 2.0, resume_session: None });
        let r = plan(&info(Some("waiting")), true, &env()).unwrap();
        assert_eq!(r.resume_session.as_deref(), Some(S));
        assert!(plan(&info(Some("review")), true, &env()).is_ok());

        let err = |i: &Info, resume: bool, e: &Env, word: &str| {
            let m = plan(i, resume, e).unwrap_err();
            assert!(m.contains(word), "{m} ∌ {word}");
        };
        err(&Info { project: None, ..info(None) }, false, &env(), "projekt");
        err(&info(None), false, &Env { mcp_enabled: false, ..env() }, "MCP");
        err(&info(None), false, &Env { has_key: false, ..env() }, "API klíč");
        for st in [None, Some("draft"), Some("running"), Some("done")] {
            err(&info(st), true, &env(), "Pokračovat");
        }
        err(&Info { session: Some("x".into()), ..info(Some("waiting")) }, true, &env(), "session");
        err(&info(Some("done")), false, &env(), "hotové");
        err(&Info { cost: 2.0, ..info(None) }, false, &env(), "vyčerpaný");
        err(&Info { model: Some("-x".into()), ..info(None) }, false, &env(), "model");
    }

    #[test]
    fn note_overrides_defaults() {
        let i = Info { model: Some("opus".into()), budget: Some(5.0), cost: 1.25, ..info(None) };
        let p = plan(&i, false, &env()).unwrap();
        assert_eq!((p.model.as_str(), p.budget, p.left), ("opus", 5.0, 3.75));
    }
}
