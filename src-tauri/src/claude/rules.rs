//! „Vždy povolit“ a „Zamítnout s důvodem“ u schvalování Clauda (hook `PermissionRequest`). Čisté.
//!
//! - **Vždy povolit:** Ledge vrátí `decision.behavior = "allow"` + `updatedPermissions`
//!   (`[{type:"addRules", rules, behavior:"allow", destination}]`) a pravidlo zapíše **Claude Code sám**
//!   — Ledge settings.json nikdy needituje. Pravidlo se vybere při příchodu žádosti (`propose`) a karta
//!   ho ukáže PŘED klikem; IPC posílá jen cíl (`Destination`), pravidlo bere Rust z čekající žádosti —
//!   UI tak nemůže poslat jiné pravidlo, než jaké člověk viděl.
//! - Zdroj pravidla: `permission_suggestions` od Clauda (jen `addRules` s `behavior:"allow"` pro týž
//!   nástroj); bez návrhu konzervativně z nástroje + vstupu (`fallback`: jen Bash/PowerShell, přesný
//!   jednoduchý příkaz bez zástupných znaků a operátorů shellu). Jinak nic → tlačítko se neukáže.
//! - **Zamítnout s důvodem:** `decision.message` = „Zamítnuto v Ledge: <důvod>“, důvod očištěný
//!   (`clean_reason`, strop `REASON_MAX` znaků), escapování dělá serde.
//!
//! Tvar ověřen v dokumentaci hooků (code.claude.com/docs/en/hooks → PermissionRequest input / decision
//! control / Permission update entries). Mapa: `docs/codemap/sessions.md`.

use super::Decision;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

/// Nejdelší důvod zamítnutí (znaky); delší se ořízne.
pub const REASON_MAX: usize = 500;
/// Víc pravidel v jednom návrhu Ledge neukáže (karta by je neuměla poctivě vypsat).
const MAX_RULES: usize = 8;
/// Nejdelší příkaz, ze kterého Ledge sám sestaví přesné pravidlo.
const FALLBACK_MAX: usize = 300;
/// Nástroje, kde pravidlo bez `ruleContent` = povolit úplně všechno → nikdy.
const SHELLS: [&str; 2] = ["Bash", "PowerShell"];
/// Operátory a zástupné znaky shellu: s nimi by přesné pravidlo nebylo přesné (`*` je v pravidle
/// zástupný znak) nebo by ho Claude Code u složeného příkazu stejně nepoužil.
/// Řídicí znaky (nové řádky) kryje zvlášť `char::is_control`.
const SHELL_META: [&str; 8] = ["*", "&", "|", ";", ">", "<", "`", "$("];

/// Jedno pravidlo oprávnění (`{toolName, ruleContent?}` jako v dokumentaci).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Rule {
    pub tool_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rule_content: Option<String>,
}

impl Rule {
    /// Zápis, jak ho uvidí člověk i settings.json: `Bash(npm run lint:*)`, `mcp__x__y`.
    pub fn label(&self) -> String {
        match &self.rule_content {
            Some(c) => format!("{}({c})", self.tool_name),
            None => self.tool_name.clone(),
        }
    }
}

/// Návrh „Vždy povolit“ u čekající žádosti (`Pending.always`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Always {
    pub rules: Vec<Rule>,
    /// přesné pravidlo (víc pravidel spojených „, “) — karta ho ukáže před klikem
    pub label: String,
    /// `true` = návrh poslal Claude (`permission_suggestions`), `false` = sestavil Ledge
    pub from_claude: bool,
}

/// Kam Claude Code pravidlo zapíše (`destination`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub enum Destination {
    /// `<projekt>/.claude/settings.local.json` (výchozí)
    #[default]
    LocalSettings,
    /// `~/.claude/settings.json`
    UserSettings,
    /// jen v paměti této session
    Session,
}

/// Pravidla + cíl, které jdou Claudovi v `updatedPermissions`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Grant {
    pub rules: Vec<Rule>,
    pub destination: Destination,
}

/// Doplněk IPC `decide(id, allow, extra?)` — chybí = chování jako dřív.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Extra {
    /// „Vždy povolit“ do tohoto cíle (jen s `allow`)
    pub always: Option<Destination>,
    /// důvod zamítnutí pro Clauda (jen bez `allow`); u plánu = připomínka
    pub reason: Option<String>,
    /// jen plán (`plan.rs`): „Schválit a povolit úpravy“ (jen s `allow`)
    pub mode: Option<super::plan::Mode>,
    /// jen plán: „Vybrat režim v Claude“ — Ledge nerozhodne, Claude ukáže svůj dialog (jen s `allow`)
    pub in_claude: bool,
}

fn str_of<'a>(v: &'a Value, k: &str) -> Option<&'a str> {
    v.get(k).and_then(Value::as_str)
}

/// Pravidla z `permission_suggestions`: jen `addRules` + `behavior:"allow"`, jen pro týž nástroj, shell
/// nikdy bez `ruleContent`, žádné víceřádkové. Ostatní typy (`setMode`, `addDirectories`…) Ledge ignoruje.
fn from_suggestions(tool: &str, sugg: &Value) -> Vec<Rule> {
    let mut out: Vec<Rule> = vec![];
    for e in sugg.as_array().into_iter().flatten() {
        if str_of(e, "type") != Some("addRules") || str_of(e, "behavior") != Some("allow") {
            continue;
        }
        for r in e.get("rules").and_then(Value::as_array).into_iter().flatten() {
            let Some(name) = str_of(r, "toolName").filter(|n| *n == tool) else { continue };
            let content = match r.get("ruleContent") {
                None | Some(Value::Null) => None,
                Some(Value::String(c)) if !c.trim().is_empty() && !c.chars().any(char::is_control) => Some(c.clone()),
                Some(_) => continue, // prázdný, víceřádkový nebo nečitelný obsah
            };
            if content.is_none() && SHELLS.contains(&tool) {
                continue;
            }
            let rule = Rule { tool_name: name.to_string(), rule_content: content };
            if !out.contains(&rule) {
                out.push(rule);
            }
        }
    }
    if out.len() > MAX_RULES { vec![] } else { out }
}

/// Bez návrhu Clauda: jen shell, přesný jednoduchý příkaz (žádný wildcard, žádné operátory).
/// Edit/Write/Read a ostatní nic — rozsah (složka? vzor?) by Ledge vymýšlel.
pub fn fallback(tool: &str, input: &Value) -> Option<Rule> {
    if !SHELLS.contains(&tool) {
        return None;
    }
    let cmd = str_of(input, "command")?;
    let ok = !cmd.trim().is_empty()
        && cmd.trim() == cmd
        && cmd.chars().count() <= FALLBACK_MAX
        && !cmd.chars().any(char::is_control)
        && !SHELL_META.iter().any(|m| cmd.contains(m));
    ok.then(|| Rule { tool_name: tool.to_string(), rule_content: Some(cmd.to_string()) })
}

/// Návrh „Vždy povolit“ pro žádost; `None` = tlačítko se neukáže.
pub fn propose(tool: &str, input: &Value, suggestions: Option<&Value>) -> Option<Always> {
    let (rules, from_claude) = match suggestions.map(|s| from_suggestions(tool, s)) {
        Some(r) if !r.is_empty() => (r, true),
        _ => (vec![fallback(tool, input)?], false),
    };
    let label = rules.iter().map(Rule::label).collect::<Vec<_>>().join(", ");
    Some(Always { rules, label, from_claude })
}

/// Důvod zamítnutí: bez řídicích znaků (kromě nového řádku), ořízlý, nejvýš `REASON_MAX` znaků.
/// Prázdný → `None` = obyčejné zamítnutí.
pub fn clean_reason(s: &str) -> Option<String> {
    let kept: String = s.chars().filter(|c| *c == '\n' || !c.is_control()).collect();
    let t = kept.trim();
    if t.is_empty() {
        return None;
    }
    Some(t.chars().take(REASON_MAX).collect::<String>().trim_end().to_string())
}

/// Co znamená klik s doplňkem. `None` = nesmyslná kombinace (Vždy povolit bez návrhu, Vždy + zamítnout)
/// → nic se nerozhodne, karta zůstane.
pub fn resolve(allow: bool, extra: &Extra, always: Option<&Always>) -> Option<Decision> {
    match (allow, extra.always) {
        (true, None) => Some(Decision::Allow),
        (true, Some(destination)) => always.map(|a| Decision::AllowAlways(Grant { rules: a.rules.clone(), destination })),
        (false, Some(_)) => None,
        (false, None) => Some(match extra.reason.as_deref().and_then(clean_reason) {
            Some(r) => Decision::DenyWith(r),
            None => Decision::Deny,
        }),
    }
}

fn wrap(decision: Value) -> String {
    json!({"hookSpecificOutput": {"hookEventName": "PermissionRequest", "decision": decision}}).to_string()
}

/// Tělo odpovědi na `POST /permission`. `Pass` = prázdné (terminál se zeptá normálně).
pub fn body(d: &Decision) -> String {
    match d {
        Decision::Allow => wrap(json!({"behavior": "allow"})),
        Decision::AllowAlways(g) => wrap(json!({"behavior": "allow", "updatedPermissions": [{
            "type": "addRules", "rules": g.rules, "behavior": "allow", "destination": g.destination,
        }]})),
        Decision::Deny => wrap(json!({"behavior": "deny", "message": "Zamítnuto v Ledge."})),
        Decision::DenyWith(r) => wrap(json!({"behavior": "deny", "message": format!("Zamítnuto v Ledge: {r}")})),
        Decision::AllowMode(m) => wrap(super::plan::allow_mode(*m)),
        Decision::ReturnPlan(r) => wrap(json!({"behavior": "deny", "message": super::plan::return_message(r)})),
        Decision::Pass => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// (popis, nástroj, vstup, návrhy, očekávaný label, od Clauda)
    type PickCase<'a> = (&'a str, &'a str, Value, Option<Value>, Option<&'a str>, bool);
    /// (popis, allow, doplněk, návrh, očekávané rozhodnutí)
    type ResolveCase<'a> = (&'a str, bool, Extra, Option<&'a Always>, Option<Decision>);

    fn rule(t: &str, c: Option<&str>) -> Rule {
        Rule { tool_name: t.into(), rule_content: c.map(str::to_string) }
    }
    fn dec(b: &str) -> Value {
        let v: Value = serde_json::from_str(b).unwrap();
        assert_eq!(v["hookSpecificOutput"]["hookEventName"], "PermissionRequest");
        v["hookSpecificOutput"]["decision"].clone()
    }

    #[test]
    fn suggestion_shape_from_docs() {
        // přesně příklad z dokumentace (PermissionRequest input)
        let s = json!([{"type": "addRules", "rules": [{"toolName": "Bash", "ruleContent": "rm -rf node_modules"}], "behavior": "allow", "destination": "localSettings"}]);
        let a = propose("Bash", &json!({"command": "rm -rf node_modules"}), Some(&s)).unwrap();
        assert_eq!(a, Always { rules: vec![rule("Bash", Some("rm -rf node_modules"))], label: "Bash(rm -rf node_modules)".into(), from_claude: true });
    }

    #[test]
    fn pick_rule_table() {
        let lint = json!([{"type": "addRules", "rules": [{"toolName": "Bash", "ruleContent": "npm run lint:*"}], "behavior": "allow", "destination": "localSettings"}]);
        let cases: Vec<PickCase> = vec![
            ("návrh Clauda vyhraje nad přesným příkazem", "Bash", json!({"command": "npm run lint -- --fix"}), Some(lint.clone()), Some("Bash(npm run lint:*)"), true),
            ("bez návrhu: přesný příkaz", "Bash", json!({"command": "npm run lint"}), None, Some("Bash(npm run lint)"), false),
            ("prázdné návrhy → přesný příkaz", "Bash", json!({"command": "cargo test"}), Some(json!([])), Some("Bash(cargo test)"), false),
            ("PowerShell přesně", "PowerShell", json!({"command": "Get-ChildItem"}), None, Some("PowerShell(Get-ChildItem)"), false),
            ("hvězdička by byla wildcard", "Bash", json!({"command": "ls *.txt"}), None, None, false),
            ("složený příkaz &&", "Bash", json!({"command": "cd x && npm test"}), None, None, false),
            ("roura", "Bash", json!({"command": "git log | head"}), None, None, false),
            ("středník", "Bash", json!({"command": "a; b"}), None, None, false),
            ("přesměrování", "Bash", json!({"command": "echo x > f"}), None, None, false),
            ("substituce", "Bash", json!({"command": "echo $(whoami)"}), None, None, false),
            ("víceřádkový", "Bash", json!({"command": "echo a\necho b"}), None, None, false),
            ("mezery na krajích (přesnost)", "Bash", json!({"command": " ls "}), None, None, false),
            ("prázdný příkaz", "Bash", json!({"command": ""}), None, None, false),
            ("bez příkazu", "Bash", json!({}), None, None, false),
            ("příliš dlouhý", "Bash", json!({"command": "x".repeat(301)}), None, None, false),
            ("Edit bez návrhu = nic", "Edit", json!({"file_path": "C:/p/a.rs"}), None, None, false),
            ("Write bez návrhu = nic", "Write", json!({"file_path": "C:/p/a.rs"}), None, None, false),
            ("WebFetch bez návrhu = nic", "WebFetch", json!({"url": "https://x.cz"}), None, None, false),
            ("MCP bez návrhu = nic", "mcp__s__t", json!({}), None, None, false),
            ("návrh pro Edit od Clauda platí", "Edit", json!({"file_path": "a"}), Some(json!([{"type": "addRules", "rules": [{"toolName": "Edit", "ruleContent": "src/**"}], "behavior": "allow", "destination": "session"}])), Some("Edit(src/**)"), true),
            ("MCP celý nástroj z návrhu", "mcp__s__t", json!({}), Some(json!([{"type": "addRules", "rules": [{"toolName": "mcp__s__t"}], "behavior": "allow", "destination": "localSettings"}])), Some("mcp__s__t"), true),
            ("setMode se ignoruje → Edit nic", "Edit", json!({"file_path": "a"}), Some(json!([{"type": "setMode", "mode": "acceptEdits", "destination": "session"}])), None, false),
            ("deny/ask návrh se ignoruje → fallback", "Bash", json!({"command": "ls"}), Some(json!([{"type": "addRules", "rules": [{"toolName": "Bash", "ruleContent": "ls"}], "behavior": "deny", "destination": "localSettings"}])), Some("Bash(ls)"), false),
            ("cizí nástroj v návrhu se ignoruje", "Bash", json!({"command": "ls"}), Some(json!([{"type": "addRules", "rules": [{"toolName": "Read"}], "behavior": "allow", "destination": "localSettings"}])), Some("Bash(ls)"), false),
            ("shell bez ruleContent = vše → nikdy", "Bash", json!({"command": "a | b"}), Some(json!([{"type": "addRules", "rules": [{"toolName": "Bash"}], "behavior": "allow", "destination": "localSettings"}])), None, false),
            ("víceřádkový obsah z návrhu se zahodí", "Bash", json!({"command": "a | b"}), Some(json!([{"type": "addRules", "rules": [{"toolName": "Bash", "ruleContent": "a\nb"}], "behavior": "allow", "destination": "localSettings"}])), None, false),
            ("nečitelné návrhy → fallback", "Bash", json!({"command": "ls"}), Some(json!("nesmysl")), Some("Bash(ls)"), false),
            (
                "víc pravidel, duplikát jednou",
                "Bash",
                json!({"command": "npm test && git status"}),
                Some(json!([
                    {"type": "addRules", "rules": [{"toolName": "Bash", "ruleContent": "npm test:*"}, {"toolName": "Bash", "ruleContent": "git status:*"}], "behavior": "allow", "destination": "localSettings"},
                    {"type": "addRules", "rules": [{"toolName": "Bash", "ruleContent": "npm test:*"}], "behavior": "allow", "destination": "localSettings"}
                ])),
                Some("Bash(npm test:*), Bash(git status:*)"),
                true,
            ),
        ];
        for (what, tool, input, sugg, label, claude) in cases {
            let got = propose(tool, &input, sugg.as_ref());
            assert_eq!(got.as_ref().map(|a| a.label.as_str()), label, "{what}");
            if let Some(a) = got {
                assert_eq!(a.from_claude, claude, "{what}");
            }
        }
    }

    #[test]
    fn too_many_rules_hidden() {
        let rules: Vec<Value> = (0..9).map(|i| json!({"toolName": "Edit", "ruleContent": format!("d{i}/**")})).collect();
        let s = json!([{"type": "addRules", "rules": rules, "behavior": "allow", "destination": "localSettings"}]);
        assert_eq!(propose("Edit", &json!({}), Some(&s)), None);
    }

    #[test]
    fn body_table() {
        let g = |d| Decision::AllowAlways(Grant { rules: vec![rule("Bash", Some("npm run lint:*"))], destination: d });
        let cases: Vec<(&str, Decision, Value)> = vec![
            ("allow", Decision::Allow, json!({"behavior": "allow"})),
            (
                "allow + pravidlo do projektu",
                g(Destination::LocalSettings),
                json!({"behavior": "allow", "updatedPermissions": [{"type": "addRules", "rules": [{"toolName": "Bash", "ruleContent": "npm run lint:*"}], "behavior": "allow", "destination": "localSettings"}]}),
            ),
            (
                "allow + pravidlo uživatelsky",
                g(Destination::UserSettings),
                json!({"behavior": "allow", "updatedPermissions": [{"type": "addRules", "rules": [{"toolName": "Bash", "ruleContent": "npm run lint:*"}], "behavior": "allow", "destination": "userSettings"}]}),
            ),
            (
                "allow + jen session",
                g(Destination::Session),
                json!({"behavior": "allow", "updatedPermissions": [{"type": "addRules", "rules": [{"toolName": "Bash", "ruleContent": "npm run lint:*"}], "behavior": "allow", "destination": "session"}]}),
            ),
            (
                "allow + celý nástroj (bez ruleContent)",
                Decision::AllowAlways(Grant { rules: vec![rule("mcp__s__t", None)], destination: Destination::LocalSettings }),
                json!({"behavior": "allow", "updatedPermissions": [{"type": "addRules", "rules": [{"toolName": "mcp__s__t"}], "behavior": "allow", "destination": "localSettings"}]}),
            ),
            ("deny", Decision::Deny, json!({"behavior": "deny", "message": "Zamítnuto v Ledge."})),
            ("deny + důvod", Decision::DenyWith("použij pnpm".into()), json!({"behavior": "deny", "message": "Zamítnuto v Ledge: použij pnpm"})),
            ("deny + uvozovky a nový řádek (escapuje serde)", Decision::DenyWith("ne \"tohle\"\n}".into()), json!({"behavior": "deny", "message": "Zamítnuto v Ledge: ne \"tohle\"\n}"})),
        ];
        for (what, d, want) in cases {
            assert_eq!(dec(&body(&d)), want, "{what}");
        }
        assert_eq!(body(&Decision::Pass), "");
    }

    #[test]
    fn reason_cleanup() {
        let long = "ž".repeat(600);
        let cases: Vec<(&str, Option<String>)> = vec![
            ("", None),
            ("   \n\t ", None),
            ("  použij pnpm  ", Some("použij pnpm".into())),
            ("a\u{7}b\r\nc", Some("ab\nc".into())),
            (&long, Some("ž".repeat(REASON_MAX))),
        ];
        for (inp, want) in cases {
            assert_eq!(clean_reason(inp), want, "{inp:?}");
        }
    }

    #[test]
    fn resolve_table() {
        let a = Always { rules: vec![rule("Bash", Some("ls"))], label: "Bash(ls)".into(), from_claude: false };
        let ex = |always: Option<Destination>, reason: Option<&str>| Extra { always, reason: reason.map(str::to_string), mode: None, in_claude: false };
        let grant = Decision::AllowAlways(Grant { rules: a.rules.clone(), destination: Destination::Session });
        let cases: Vec<ResolveCase> = vec![
            ("povolit jako dřív", true, Extra::default(), Some(&a), Some(Decision::Allow)),
            ("zamítnout jako dřív", false, Extra::default(), Some(&a), Some(Decision::Deny)),
            ("vždy povolit", true, ex(Some(Destination::Session), None), Some(&a), Some(grant)),
            ("vždy bez návrhu = nic", true, ex(Some(Destination::LocalSettings), None), None, None),
            ("vždy + zamítnout = nic", false, ex(Some(Destination::LocalSettings), None), Some(&a), None),
            ("zamítnout s důvodem", false, ex(None, Some(" jinak ")), None, Some(Decision::DenyWith("jinak".into()))),
            ("prázdný důvod = obyčejné zamítnutí", false, ex(None, Some("  ")), None, Some(Decision::Deny)),
            ("důvod u povolení se ignoruje", true, ex(None, Some("x")), None, Some(Decision::Allow)),
        ];
        for (what, allow, extra, always, want) in cases {
            assert_eq!(resolve(allow, &extra, always), want, "{what}");
        }
    }

    #[test]
    fn extra_from_ipc() {
        let e: Extra = serde_json::from_value(json!({"always": "userSettings"})).unwrap();
        assert_eq!(e.always, Some(Destination::UserSettings));
        let e: Extra = serde_json::from_value(json!({"reason": "x"})).unwrap();
        assert_eq!((e.always, e.reason.as_deref()), (None, Some("x")));
        assert!(serde_json::from_value::<Extra>(json!({"always": "projectSettings"})).is_err(), "jen tři cíle z karty");
    }
}
