//! Katalog nástrojů MCP (názvy = kontrakt `ledge-k9/kontrakt.md`, docs je cituje) a jejich volání.
//! Každý zápis: načti config → změna (`edit` / `look`) → `config::validate_field` (stejná validace
//! jako Nastavení) → snímek dotčených částí do Vrátit → `Host::save` (stejná cesta jako UI) → věta.
//! Chyba nástroje = `isError: true` s lidskou větou; config zůstane beze změny.

use super::rpc::{self, RpcErr};
use super::undo::{self, Entry};
use super::{Host, Server, edit, look, schema};
use crate::actions::Action;
use crate::config::{Config, Project};
use crate::i18n::tr;
use crate::layout::LayoutConfig;
use crate::stripconf::StripConfig;
use crate::appearance::AppearanceConfig;
use serde_json::{Value, json};

/// Nástroj: jméno, titulek, popis pro model, schéma vstupu, zapisuje?
pub struct Tool {
    pub name: &'static str,
    pub title: &'static str,
    pub desc: &'static str,
    pub input: fn() -> Value,
    pub write: bool,
}

fn no_args() -> Value {
    json!({ "type": "object", "properties": {}, "additionalProperties": false })
}

/// Schéma typu configu vložené jako vstup nástroje (`$defs` nahoru, ať sedí `$ref`).
fn embed(root: Value, wrap: Option<&str>, required: &[&str], drop: &[&str]) -> Value {
    let mut s = root;
    let defs = s.as_object_mut().and_then(|o| o.remove("$defs"));
    if let Some(o) = s.as_object_mut() {
        o.remove("$schema");
        o.remove("title");
        if let Some(p) = o.get_mut("properties").and_then(Value::as_object_mut) {
            for d in drop {
                p.remove(*d);
            }
        }
        o.remove("required");
    }
    let mut out = match wrap {
        Some(k) => json!({ "type": "object", "properties": { k: s }, "required": [k] }),
        None => {
            if !required.is_empty() {
                s["required"] = json!(required);
            }
            s
        }
    };
    if let Some(d) = defs {
        out["$defs"] = d;
    }
    out
}

fn with_id(mut s: Value, what: &str) -> Value {
    s["properties"]["id"] = json!({ "type": "string", "description": format!("Id of the {what} to change (from the list tool).") });
    s["required"] = json!(["id"]);
    s
}

fn id_only() -> Value {
    json!({ "type": "object", "properties": { "id": { "type": "string" } }, "required": ["id"], "additionalProperties": false })
}

pub const TOOLS: &[Tool] = &[
    Tool { name: "ledge_status", title: "Ledge status", write: false, input: no_args,
        desc: "Read Ledge's current state: version, whether MCP is on, how many shortcuts (tiles), quick actions and tabs there are, the active design style and theme, the strip style, the UI language, whether the drawer is open and how many changes undo_last can revert. Call this first." },
    Tool { name: "apps_discover", title: "Discover apps and projects", write: false,
        input: || json!({ "type": "object", "properties": { "repos": { "type": "boolean", "description": "Also scan the user's folders for git repositories and suggest project shortcuts (default true)." } }, "additionalProperties": false }),
        desc: "List what the user actually has on this PC, read-only: apps from the Start menu (shortcut name and target .exe, user and all-users Start menu) and git repositories in the usual folders with a ready project suggestion (kind dev/app, run command, port). Nothing comes from the browser. alreadyInLedge marks things Ledge already shows. Use the results for tiles_add (projects, apps) and actions_add (apps with arguments, websites). Can take a few seconds." },
    Tool { name: "tiles_list", title: "List shortcuts", write: false, input: no_args,
        desc: "List the shortcuts (tiles) on the Projects card: each with id, name, kind (dev = local dev server with run command and port, app = desktop program with exe or appName), folder, look. Also says on which tabs the Projects card is." },
    Tool { name: "tiles_add", title: "Add a shortcut", write: true,
        input: || embed(schema::of::<Project>(), None, &["name"], &["links", "args"]),
        desc: "Add a shortcut (tile) to the Projects card. Give name plus either path (an absolute project folder — Ledge then detects kind, run command, port and remote itself, like Settings → Projects → Add) or kind app with exe (absolute path to an .exe, e.g. from apps_discover) or appName. For safety a shortcut's run command can only be a package-manager script (\"npm run dev\", \"pnpm dev\") or a command Ledge itself finds in the folder; anything else belongs to actions_add (kind command), which asks the user before its first run. Example: {\"name\":\"Web shop\",\"path\":\"C:\\\\Users\\\\me\\\\code\\\\shop\"} or {\"name\":\"Notepad\",\"kind\":\"app\",\"exe\":\"C:\\\\Windows\\\\notepad.exe\"}. id is generated from the name when missing." },
    Tool { name: "tiles_update", title: "Change a shortcut", write: true,
        input: || with_id(embed(schema::of::<Project>(), None, &[], &["links"]), "shortcut"),
        desc: "Change fields of one shortcut by id (only the fields you send; null clears an optional field). Example: {\"id\":\"web-shop\",\"port\":5180,\"look\":{\"icon\":{\"kind\":\"emoji\",\"value\":\"🛒\"},\"color\":\"#2563eb\"}}. Same command rules as tiles_add." },
    Tool { name: "tiles_remove", title: "Remove a shortcut", write: true, input: id_only,
        desc: "Remove one shortcut (tile) from the Projects card by id (see tiles_list). The project folder on disk is not touched. Reversible with undo_last." },
    Tool { name: "actions_list", title: "List quick actions", write: false, input: no_args,
        desc: "List quick actions and links (one list; a link is an action that opens something): id, label, kind (url, web, file, folder, app, command), target, group, showOn (work / personal / both)." },
    Tool { name: "actions_add", title: "Add a quick action or link", write: true,
        input: || embed(schema::of::<Action>(), None, &["label", "kind", "target"], &[]),
        desc: "Add a quick action or link. kind: url (website in the browser, https://…), web (website in a small Ledge window), file / folder (absolute path), app (.exe, optional args), command (cmd or PowerShell line). Websites the user uses belong here as kind url. A command or an app with args is never pre-approved: Ledge asks the user before its first run. Examples: {\"label\":\"Mail\",\"kind\":\"url\",\"target\":\"https://mail.example.com\"}, {\"label\":\"Build\",\"kind\":\"command\",\"target\":\"npm run build\",\"cwd\":\"C:\\\\code\\\\app\"}. id is generated from the label when missing." },
    Tool { name: "actions_update", title: "Change a quick action", write: true,
        input: || with_id(embed(schema::of::<Action>(), None, &[], &[]), "action"),
        desc: "Change fields of one quick action by id (only the fields you send; null clears an optional field). Changing what a command runs drops any earlier consent, so Ledge asks again." },
    Tool { name: "actions_remove", title: "Remove a quick action", write: true, input: id_only,
        desc: "Remove one quick action or link by id (see actions_list). Any consent the user gave for it stays only for that exact command. Reversible with undo_last." },
    Tool { name: "layout_get", title: "Get the drawer layout", write: false, input: no_args,
        desc: "Read the drawer layout: tabs (pages, max 9; built-in prace = Work and osobni = Personal) with their cards in order, each card's width (s = 1 column, m = 2, l = 3, xl = whole row), optional height h (1–4 rows), Projects card tile options, card look; hidden card parts. The catalog lists every card id and allowed value. Edit this JSON and pass it whole to layout_preview, then layout_apply." },
    Tool { name: "layout_preview", title: "Check a layout", write: false,
        input: || embed(schema::of::<LayoutConfig>(), Some("layout"), &[], &[]),
        desc: "Validate a whole layout (same rules as Ledge's layout editor) without saving. Returns what each tab would show and which cards become visible or hidden, or a sentence saying what is wrong. Always preview before layout_apply." },
    Tool { name: "layout_apply", title: "Apply a layout", write: true,
        input: || embed(schema::of::<LayoutConfig>(), Some("layout"), &[], &[]),
        desc: "Save a whole layout (tabs, cards, sizes) — same validation as layout_preview. A card on no tab is hidden and fetches nothing. Example: {\"layout\":{\"pages\":[{\"id\":\"prace\",\"name\":\"\",\"cards\":[{\"id\":\"projects\",\"size\":\"xl\"},{\"id\":\"claude\",\"size\":\"m\"},{\"id\":\"repos\",\"size\":\"m\"}]}],\"hiddenParts\":[]}}." },
    Tool { name: "strip_get", title: "Get the closed strip", write: false, input: no_args,
        desc: "Read what the closed Ledge shows at the top edge: strip on/off, items in order (clock, mode, running, claude, waiting, repos, limits, custom, timer, media, github, system, mail, widget:<file>), custom text, per-item colors, items hidden when idle, and the closed look (strip.look). The catalog lists allowed values." },
    Tool { name: "strip_set", title: "Change the closed strip", write: true,
        input: || embed(schema::of::<StripConfig>(), None, &[], &["itemsRev"]),
        desc: "Change the closed strip (only the fields you send). Example: {\"items\":[\"mode\",\"clock\",\"waiting\",\"running\"],\"custom\":\"\",\"look\":\"classic\"}." },
    Tool { name: "appearance_get", title: "Get the appearance", write: false, input: no_args,
        desc: "Read the look: design style, theme, accent, density, font, material, layout of the main page, drawer width preset, corner radius, opacity, animation style and speed, strip shape and position, blur. The catalog lists the design styles and every allowed value." },
    Tool { name: "appearance_set", title: "Change the appearance", write: true,
        input: || {
            let mut s = embed(schema::of::<AppearanceConfig>(), None, &[], &[]);
            s["properties"]["style"] = json!({ "type": "string", "enum": look::STYLES.iter().map(|s| s.id).collect::<Vec<_>>(), "description": "Design style package; applied first, other fields override it." });
            s["properties"]["blur"] = json!({ "type": "boolean", "description": "Blur the desktop behind the drawer (acrylic)." });
            s
        },
        desc: "Change the look using only catalog values. style picks a whole design style (classic, glass, terminal, paper, neon, brutal, pastel) exactly like the gallery in Settings; other fields (theme, accent #rrggbb, density, font, material, layout, width, radius, motion, strip) override parts of it. Example: {\"style\":\"paper\"} or {\"theme\":\"nord\",\"accent\":\"#2563eb\",\"motion\":{\"style\":\"none\"}}." },
    Tool { name: "undo_last", title: "Undo the last change", write: true, input: no_args,
        desc: "Revert the most recent change made over MCP (up to 20 kept in memory until Ledge restarts). It only restores the parts that change touched and refuses if the user changed them since in Settings." },
    Tool { name: "open_drawer", title: "Show the drawer", write: false,
        input: || json!({ "type": "object", "properties": { "tab": { "type": "string", "description": "Tab id to show (from layout_get), e.g. prace or osobni." } }, "additionalProperties": false }),
        desc: "Open the Ledge drawer so the user sees the result (it does not take keyboard focus and does nothing while a fullscreen game runs). Optionally on a given tab." },
];

/// Nástroje pro `rpc` nad stavem serveru a Ledge.
pub struct Handler<'a> {
    pub srv: &'a Server,
    pub host: &'a dyn Host,
}

fn result(text: String, is_error: bool) -> Value {
    json!({ "content": [{ "type": "text", "text": text }], "isError": is_error })
}

fn pretty(v: &Value) -> String {
    serde_json::to_string_pretty(v).unwrap_or_default()
}

impl rpc::Tools for Handler<'_> {
    fn list(&self) -> Value {
        Value::Array(
            TOOLS
                .iter()
                .map(|t| {
                    json!({
                        "name": t.name, "title": t.title, "description": t.desc, "inputSchema": (t.input)(),
                        "annotations": { "title": t.title, "readOnlyHint": !t.write, "destructiveHint": t.name.ends_with("_remove"), "idempotentHint": !t.write, "openWorldHint": false },
                    })
                })
                .collect(),
        )
    }

    fn call(&self, name: &str, args: &Value) -> Result<Value, RpcErr> {
        if !TOOLS.iter().any(|t| t.name == name) {
            return Err(RpcErr(rpc::INVALID_PARAMS, format!("Unknown tool: {name}")));
        }
        Ok(match self.run(name, args) {
            Ok(text) => result(text, false),
            Err(e) => result(e, true),
        })
    }

    fn version(&self) -> String {
        self.host.version()
    }
}

impl Handler<'_> {
    fn run(&self, name: &str, args: &Value) -> Result<String, String> {
        let host = self.host;
        match name {
            "ledge_status" => Ok(pretty(&self.status())),
            "apps_discover" => Ok(pretty(&host.discover(args.get("repos").and_then(Value::as_bool).unwrap_or(true)))),
            "tiles_list" => Ok(pretty(&edit::tiles_list(&host.config()))),
            "tiles_add" => self.write(&["/projects"], |c| edit::tile_add(c, args, host)),
            "tiles_update" => self.write(&["/projects"], |c| edit::tile_update(c, args, host)),
            "tiles_remove" => self.write(&["/projects"], |c| edit::tile_remove(c, args)),
            "actions_list" => Ok(pretty(&edit::actions_list(&host.config()))),
            "actions_add" => self.write(&["/actions"], |c| edit::action_add(c, args)),
            "actions_update" => self.write(&["/actions"], |c| edit::action_update(c, args)),
            "actions_remove" => self.write(&["/actions"], |c| edit::action_remove(c, args)),
            "layout_get" => Ok(pretty(&look::layout_get(&host.config()))),
            "layout_preview" => {
                let cur = host.config();
                let next = look::layout_parse(&cur, args)?;
                crate::config::validate_field(&next).map_err(|e| e.message)?;
                Ok(trf!("V pořádku, nic se neuložilo. Takhle by to vypadalo:\n{}", "Valid, nothing was saved. This is how it would look:\n{}", look::layout_summary(&cur.layout, &next.layout)))
            }
            "layout_apply" => self.write(&["/layout"], |c| {
                let next = look::layout_parse(c, args)?;
                let s = look::layout_summary(&c.layout, &next.layout);
                Ok((next, trf!("Rozložení uloženo:\n{s}", "Layout saved:\n{s}")))
            }),
            "strip_get" => Ok(pretty(&look::strip_get(&host.config()))),
            "strip_set" => self.write(&["/strip"], |c| Ok((look::strip_set(c, args)?, tr("Proužek změněn.", "Strip changed.").into()))),
            "appearance_get" => Ok(pretty(&look::appearance_get(&host.config()))),
            "appearance_set" => self.write(&["/appearance", "/drawer/glass"], |c| {
                let next = look::appearance_set(c, args)?;
                Ok((next.clone(), trf!("Vzhled změněn (styl {}, motiv {}).", "Appearance changed (style {}, theme {}).", next.appearance.style, serde_json::to_value(next.appearance.theme).unwrap_or_default())))
            }),
            "undo_last" => self.undo(),
            "open_drawer" => host.open_drawer(args.get("tab").and_then(Value::as_str)),
            _ => Err(format!("Unknown tool: {name}")),
        }
    }

    fn status(&self) -> Value {
        let cfg = self.host.config();
        let tabs: Vec<Value> = cfg.layout.pages.iter().map(|p| json!({ "id": p.id, "title": crate::layout::title(p), "cards": p.cards.len() })).collect();
        json!({
            "version": self.host.version(),
            "mcp": { "enabled": cfg.mcp.enabled },
            "drawerOpen": self.host.drawer_open(),
            "language": cfg.ui.lang,
            "counts": { "tiles": cfg.projects.len(), "actions": cfg.actions.len(), "tabs": cfg.layout.pages.len() },
            "tabs": tabs,
            "appearance": { "style": cfg.appearance.style, "theme": cfg.appearance.theme },
            "strip": { "enabled": cfg.strip.enabled, "look": cfg.strip.look, "items": cfg.strip.items },
            "undoAvailable": self.srv.undo.lock().len(),
        })
    }

    /// Společný zápis (viz hlavička modulu). Beze změny = nic se neukládá ani nepřidá do Vrátit.
    fn write(&self, ptrs: &[&str], f: impl FnOnce(&Config) -> Result<(Config, String), String>) -> Result<String, String> {
        let _one = self.srv.write.lock();
        let cur = self.host.config();
        let (next, summary) = f(&cur)?;
        crate::config::validate_field(&next).map_err(|e| e.message)?;
        if next == cur {
            return Ok(tr("Beze změny — už to tak je.", "No change — it's already like that.").into());
        }
        let before = undo::pick(&cur, ptrs);
        self.host.save(next)?;
        let after = undo::pick(&self.host.config(), ptrs);
        let parts = before.into_iter().zip(after).map(|((p, b), (_, a))| (p, b, a)).collect();
        let label = summary.lines().next().unwrap_or_default().to_string();
        self.srv.undo.lock().push(Entry { label, parts });
        Ok(format!("{summary}\n{}", tr("(Vrátit jde přes undo_last.)", "(Revert with undo_last.)")))
    }

    fn undo(&self) -> Result<String, String> {
        let _one = self.srv.write.lock();
        let e = self.srv.undo.lock().pop().ok_or_else(|| tr("Není co vrátit — přes MCP se od spuštění Ledge nic nezměnilo.", "Nothing to undo — nothing was changed over MCP since Ledge started."))?;
        let cur = self.host.config();
        let next = undo::revert(&cur, &e)?;
        crate::config::validate_field(&next).map_err(|e| e.message)?;
        self.host.save(next)?;
        Ok(trf!("Vráceno: {}", "Reverted: {}", e.label))
    }
}


