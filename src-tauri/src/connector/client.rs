//! Klient MCP: JSON-RPC `tools/call` přes HTTPS POST (Streamable HTTP bez session, odpověď
//! `application/json`; pro jistotu se umí i SSE rámec `data: {…}`). Bearer klíč jde jen v hlavičce;
//! v hláškách, logu ani UI se neobjeví (i kdyby ho server vrátil v textu, vymaže se). Čisté části
//! (`request_body`, `parse_reply`, `task_args`, `note_args`) jsou testované bez sítě; živé volání
//! testy ověřují jen proti lokálnímu falešnému serveru.

use super::plan::{Assignee, Item, Member, Mode, Plan};
use crate::accounts::http;
use crate::i18n::tr;
use serde::Serialize;
use serde_json::{Value, json};
use std::sync::atomic::{AtomicU64, Ordering};

const ACCEPT: &str = "application/json, text/event-stream";
/// Strop délky textu chyby ze serveru do hlášky.
const MAX_ERR: usize = 300;
/// propojená aplikace: `title` úkolu nejvýš 500 znaků, `bodyMarkdown` 100 000.
const MAX_TITLE: usize = 500;
pub const MAX_NOTE: usize = 100_000;

static RPC_ID: AtomicU64 = AtomicU64::new(1);

/// Tělo JSON-RPC požadavku `tools/call`.
pub fn request_body(id: u64, tool: &str, args: &Value) -> String {
    json!({ "jsonrpc": "2.0", "id": id, "method": "tools/call", "params": { "name": tool, "arguments": args } }).to_string()
}

fn scrub(s: &str, key: &str) -> String {
    let s = if key.is_empty() { s.to_string() } else { s.replace(key, "***") };
    let s: String = s.chars().filter(|c| !c.is_control() || *c == '\n').take(MAX_ERR).collect();
    s.trim().to_string()
}

/// Odpověď (`status` + tělo) → výsledek nástroje jako JSON (text výsledku se rozparsuje, jinak zůstane řetězec).
/// Chyby lidsky, bez adresy a klíče; `isError` z nástroje nese hlášku aplikace.
pub fn parse_reply(status: u16, body: &[u8], key: &str) -> Result<Value, String> {
    let text = String::from_utf8_lossy(body);
    let json: Option<Value> = serde_json::from_str(text.trim()).ok().or_else(|| {
        // SSE: poslední řádek `data: {…}`
        text.lines().rev().find_map(|l| l.strip_prefix("data:")).and_then(|d| serde_json::from_str(d.trim()).ok())
    });
    let rpc_error = json.as_ref().and_then(|j| j.pointer("/error/message")).and_then(Value::as_str).map(|m| scrub(m, key));
    if status != 200 {
        return Err(match status {
            401 => tr("Propojená aplikace odmítla klíč — vlož v Nastavení → Claude nový.", "The linked app rejected the key — enter a new one in Settings → Claude.").into(),
            403 => tr("Klíč nemá potřebné oprávnění nebo smí jiné nástroje.", "The key lacks the required permission or allows other tools.").into(),
            429 => tr("Propojená aplikace teď odmítá další požadavky (limit) — zkus to za chvíli.", "The linked app is refusing more requests (limit) — try again in a moment.").into(),
            500..=599 => tr("Propojená aplikace má potíže — zkus to později.", "The linked app is having trouble — try again later.").into(),
            _ => match rpc_error {
                Some(m) if !m.is_empty() => m,
                _ => trf!("Propojená aplikace odpověděla chybou ({status}).", "The linked app answered with an error ({status})."),
            },
        });
    }
    let Some(json) = json else {
        return Err(tr("Propojená aplikace odpověděla nečekaně — je adresa v Nastavení správná?", "The linked app answered unexpectedly — is the address in Settings right?").into());
    };
    if let Some(m) = rpc_error {
        return Err(m);
    }
    let result = json.get("result").ok_or_else(|| tr("Propojená aplikace odpověděla bez výsledku.", "The linked app answered without a result.").to_string())?;
    let text = result.pointer("/content/0/text").and_then(Value::as_str).unwrap_or("");
    if result.get("isError").and_then(Value::as_bool).unwrap_or(false) {
        let m = scrub(text, key);
        return Err(if m.is_empty() { tr("Propojená aplikace úkol odmítla.", "The linked app refused it.").into() } else { m });
    }
    Ok(serde_json::from_str(text).unwrap_or_else(|_| Value::String(text.to_string())))
}

/// Jedno volání nástroje.
pub async fn call(url: &str, key: &str, tool: &str, args: &Value) -> Result<Value, String> {
    if !super::url_ok(url) || url.trim().is_empty() {
        return Err(tr("Adresa propojené aplikace v configu (connector.url) není platná.", "The linked app address in the config (connector.url) isn't valid.").into());
    }
    let id = RPC_ID.fetch_add(1, Ordering::Relaxed);
    let r = http::post_json("Propojená aplikace", url.trim(), key, ACCEPT, request_body(id, tool, args)).await?;
    parse_reply(r.status, &r.body, key)
}

/// `list_team_members` → členové (jen platný e-mail).
pub fn members_of(v: &Value) -> Vec<Member> {
    v.get("members")
        .and_then(Value::as_array)
        .map(|a| a.iter().filter_map(|m| serde_json::from_value::<Member>(m.clone()).ok()).filter(|m| m.email.contains('@')).collect())
        .unwrap_or_default()
}

pub async fn list_members(url: &str, key: &str) -> Result<Vec<Member>, String> {
    Ok(members_of(&call(url, key, "list_team_members", &json!({ "limit": 200 })).await?))
}

fn clip(s: &str, max: usize) -> String {
    s.chars().take(max).collect()
}

/// Argumenty `create_task`: bez řešitele a priority se pole vynechají (propojená aplikace doplní vlastníka klíče a 3).
/// `Ambiguous` / `Unknown` řešitel se nikdy neposílá.
pub fn task_args(item: &Item, note_title: &str) -> Value {
    let mut a = json!({
        "title": clip(&item.title, MAX_TITLE),
        "description": format!("{}{}", tr("Z poznámky v Ledge: ", "From a note in Ledge: "), clip(note_title.trim(), 200)),
        "clientRequestId": item.request_id,
    });
    if let Assignee::Resolved { email, .. } = &item.assignee {
        a["assignee"] = json!(email);
    }
    if let Some(p) = item.priority {
        a["priority"] = json!(p);
    }
    a
}

/// Argumenty `create_note` (`clientRequestId` ≤ 64 znaků).
pub fn note_args(plan: &Plan, body: &str) -> Value {
    json!({ "title": clip(&plan.note_title, 200), "bodyMarkdown": body, "clientRequestId": plan.note_request_id })
}

/// Výsledek jednoho odeslaného bodu / poznámky.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Outcome {
    /// řádek bodu (u poznámky 0)
    pub line: usize,
    pub ok: bool,
    /// id úkolu / poznámky v propojené aplikaci
    pub id: Option<String>,
    /// absolutní odkaz na úkol (`appUrl` + `url` z odpovědi), je-li
    pub url: Option<String>,
    pub error: Option<String>,
    /// propojená aplikace ho už znal (opakovaný klik) — nezdvojilo se
    pub duplicate: bool,
    /// text bodu (bez `⏰`) z plánu — odkaz se do poznámky píše podle něj, řádek je jen nápověda
    #[serde(skip)]
    pub text: String,
}

/// Odkaz z `url` odpovědi (`?tab=tasks&task=ID`) a adresy webu; jiný tvar než bezpečný dotaz se zahodí.
pub fn task_link(app_base: &str, rel: &str) -> Option<String> {
    let ok = rel.starts_with('?') && rel.len() <= 200 && rel.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '?' | '&' | '=' | '_' | '-' | '.' | '%'));
    (ok && super::url_ok(app_base) && !app_base.is_empty()).then(|| format!("{}/{rel}", app_base.trim_end_matches('/')))
}

/// Z odpovědi `create_task` / `create_note` → (id, odkaz, duplicita).
pub fn outcome_of(v: &Value, app_base: &str) -> (Option<String>, Option<String>, bool) {
    let id = ["taskId", "noteId"].iter().find_map(|k| v.get(k).and_then(Value::as_str)).map(str::to_string);
    let url = v.get("url").and_then(Value::as_str).and_then(|u| task_link(app_base, u));
    (id, url, v.get("deduplicated").and_then(Value::as_bool).unwrap_or(false))
}

fn fail(line: usize, text: &str, error: String) -> Outcome {
    Outcome { line, ok: false, id: None, url: None, error: Some(error), duplicate: false, text: text.to_string() }
}

/// Odešle vybrané body (`lines`) jako úkoly, nebo poznámku (`Mode::Note`). Po jednom, chyba jednoho
/// bodu nezastaví ostatní; chyba přihlášení (401/403) a limit (429) zastaví zbytek (další pokusy by selhaly stejně).
pub async fn send(url: &str, key: &str, plan: &Plan, body: &str, lines: &[usize], app_base: &str) -> Vec<Outcome> {
    let mut out = Vec::new();
    if plan.mode == Mode::Note {
        let o = match call(url, key, "create_note", &note_args(plan, body)).await {
            Ok(v) => {
                let (id, url, duplicate) = outcome_of(&v, app_base);
                Outcome { line: 0, ok: id.is_some(), id, url, error: None, duplicate, text: String::new() }
            }
            Err(e) => fail(0, "", e),
        };
        return vec![o];
    }
    for it in plan.items.iter().filter(|i| !i.sent && lines.contains(&i.line)) {
        match call(url, key, "create_task", &task_args(it, &plan.note_title)).await {
            Ok(v) => {
                let (id, url, duplicate) = outcome_of(&v, app_base);
                out.push(Outcome { line: it.line, ok: id.is_some(), id, url, error: None, duplicate, text: it.text.clone() });
            }
            Err(e) => {
                let stop = fatal(&e);
                out.push(fail(it.line, &it.text, e));
                if stop {
                    break;
                }
            }
        }
    }
    out
}

/// Chyba přihlášení / oprávnění / limitu: další body by selhaly stejně, smyčka se zastaví.
fn fatal(e: &str) -> bool {
    [
        tr("Propojená aplikace odmítla klíč", "The linked app rejected the key"),
        tr("Klíč nemá potřebné oprávnění", "The key lacks the required"),
        tr("Propojená aplikace teď odmítá", "The linked app is refusing"),
    ]
    .iter()
    .any(|p| e.starts_with(p))
}
