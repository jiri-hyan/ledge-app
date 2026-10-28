//! Kdo smí volat vlastní příkazy Ledge (audit N4). Bez `AppManifest` pouští Tauri 2 každý příkaz
//! z `generate_handler!` z KAŽDÉHO okna — lístek (`note-*`) nebo widget na ploše (`desk-*`) by tak směl volat
//! `save_config`, souhlasy akcí a widgetů, export… Proto `build.rs` předá seznam příkazů tauri-build
//! (`AppManifest::commands`, seznam se čte z `generate_handler![…]` v `lib.rs` — jediný zdroj) a každé okno
//! pak smí jen to, co mu povolí jeho capability (`capabilities/*.json`, oprávnění `allow-<příkaz>`):
//!
//! - `default.json` — šuplík (`drawer`): všechny příkazy (hlavní okno beze změny).
//! - `studio.json` — okno Ledge Studio (`studio`, Nastavení v okně): jen to, co Nastavení volá (`STUDIO_WINDOW`).
//! - `pins.json` — lístky `note-*`: jen to, co lístek volá (`pin_*` svého okna + práce s poznámkou).
//! - `desk.json` — widgety `desk-*`: jen `desk_*` svého okna, kliky a výstup widgetu, karty Běží / Agenda.
//! - okna s webem (`web-*`), PDF (`pdf-export-*`) a cokoli jiného: nic.
//!
//! **Nový příkaz** = řádek v `generate_handler!` + `allow-<příkaz>` v `default.json` (a v `pins.json` /
//! `desk.json`, když ho volá i to okno). Zapomenutý = příkaz z okna neprojde; test níž to chytí dřív.
//!
//! Soubor čte i `build.rs` (`#[path]`) — proto bez závislostí na zbytku crate mimo testy.

/// Jména příkazů z `tauri::generate_handler![…]` v textu `lib.rs` (poslední segment cesty, v pořadí).
pub fn handler_commands(lib_rs: &str) -> Vec<String> {
    let Some(start) = lib_rs.find("generate_handler![") else { return Vec::new() };
    let body = &lib_rs[start + "generate_handler![".len()..];
    let Some(end) = body.find(']') else { return Vec::new() };
    body[..end]
        .lines()
        .map(|l| l.split("//").next().unwrap_or(""))
        .flat_map(|l| l.split(','))
        .filter_map(|item| item.trim().rsplit("::").next().map(str::trim).filter(|s| !s.is_empty()).map(String::from))
        .collect()
}

/// Oprávnění, které povolí příkaz `cmd` (tauri-build: `allow-` + snake_case → kebab-case).
pub fn permission_of(cmd: &str) -> String {
    format!("allow-{}", cmd.replace('_', "-"))
}

/// Label okna proti vzoru z capability (`drawer`, `note-*`): `*` jen na konci, jinak přesná shoda.
pub fn label_matches(pattern: &str, label: &str) -> bool {
    match pattern.strip_suffix('*') {
        Some(prefix) => !prefix.contains('*') && label.starts_with(prefix),
        None => pattern == label,
    }
}

/// Jedna capability: vzory oken a oprávnění.
pub struct Cap {
    pub windows: Vec<String>,
    pub permissions: Vec<String>,
}

/// Smí okno `label` volat příkaz `cmd`? Stejné pravidlo jako Tauri: aspoň jedna capability, jejíž okno sedí,
/// příkaz povoluje. Čistá funkce.
pub fn allowed(caps: &[Cap], label: &str, cmd: &str) -> bool {
    let perm = permission_of(cmd);
    caps.iter().any(|c| c.windows.iter().any(|w| label_matches(w, label)) && c.permissions.contains(&perm))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;
    use std::path::Path;

    const LIB: &str = include_str!("lib.rs");

    fn caps() -> Vec<(String, Cap)> {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("capabilities");
        let mut out = Vec::new();
        for e in std::fs::read_dir(&dir).unwrap() {
            let p = e.unwrap().path();
            if p.extension().is_none_or(|x| x != "json") {
                continue;
            }
            let v: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(&p).unwrap()).unwrap();
            let strs = |k: &str| v[k].as_array().into_iter().flatten().map(|x| x.as_str().unwrap().to_string()).collect::<Vec<_>>();
            out.push((p.file_name().unwrap().to_string_lossy().into_owned(), Cap { windows: strs("windows"), permissions: strs("permissions") }));
        }
        out.sort_by(|a, b| a.0.cmp(&b.0));
        out
    }

    fn app_perms(c: &Cap) -> BTreeSet<String> {
        c.permissions.iter().filter(|p| p.starts_with("allow-")).cloned().collect()
    }

    /// Co smí lístek: `pin_*` svého okna a práce s jeho poznámkou (číst, uložit, hotovo, archiv, koš, koncept,
    /// odkazy `[[…]]`, export, otevřít v aplikaci). Ne: config, akce, widgety, šuplík, `pin_*` šuplíku.
    const PIN_WINDOW: &[&str] = &[
        "note_archive", "note_delete", "note_delete_undo", "note_done", "note_draft_clear", "note_draft_load", "note_draft_save",
        "note_edit_draft_load", "note_edit_draft_save", "note_export", "note_link_create", "note_link_resolve", "note_link_suggest",
        "note_open", "note_open_with", "note_open_with_apps", "note_read", "note_render", "note_save", "note_set_meta",
        "note_suggest_remind", "note_toggle_task", "note_unarchive", "open_url", "pin_drag", "pin_edit_in_ledge", "pin_hide",
        "pin_hint_seen", "pin_init", "pin_look_all", "pin_open", "pin_set", "pin_unpin",
    ];
    /// Co smí widget na ploše: `desk_*` svého okna, kliky / výstup / obnovení widgetu (bez souhlasu — ten jen
    /// v šuplíku), karta Běží (stop, logy, zavřít hlášku), Agenda (připojit se, Outlook), vzhled (čtení configu).
    const DESK_WINDOW: &[&str] = &[
        "anim_stats", "calendar_join", "calendar_outlook_load", "desk_drag", "desk_init", "desk_menu", "desk_open_drawer", "desk_set",
        "desk_unpin", "dismiss", "get_config", "logs", "open_url", "stop", "widget_click", "widget_edit", "widget_output",
        "widget_refresh", "widgets_list",
    ];

    /// Co smí Ledge Studio: config (číst, ověřit, zapsat, výchozí, export / import), dialogy a příkazy panelů
    /// Nastavení; okenní tlačítka `studio_win`. Ne: fokus a velikost šuplíku, spouštění projektů z karet, `pin_*`, `desk_drag`.
    const STUDIO_WINDOW: &[&str] = &[
        "account_cancel", "account_connect", "account_disconnect", "account_google_calendars", "accounts_status",
        "action_check", "action_consent", "action_icon", "action_icon_refresh", "anim_stats", "app_info",
        "calendar_outlook_load", "calendar_outlook_probe", "calendar_test", "check_config", "clip_apps", "clip_clear",
        "clip_enable", "config_defaults", "config_export", "config_import_preview", "desk_hide_all", "desk_pin",
        "desk_profile_apply", "desk_profile_delete", "desk_profile_save", "desk_set", "desk_unpin", "geo_search",
        "get_config", "get_state", "hooks_install", "hooks_remove", "hooks_status", "launcher_catalog_read",
        "launcher_import_check", "logo_preview", "look_image", "look_image_add", "mcp_reveal_key", "mcp_rotate_key",
        "mcp_set_enabled", "mcp_state", "note_folder_check", "note_folder_clear_conflicts", "note_folder_sync_now",
        "note_open", "note_tags", "notes_sync_now", "open_app_file", "open_config_file", "open_url", "pick_path",
        "pick_save", "pkg_context", "project_candidates", "project_detect", "project_logo", "relaunch_outside",
        "run_action", "run_script", "save_config", "scripts_list", "scripts_open_dir", "set_game_mode",
        "statusline_install", "statusline_remove", "statusline_status", "studio_win", "update_check_now",
        "update_rollback", "widget_consent", "widget_install", "widget_templates", "widgets_list", "widgets_open_dir",
        "win_settings_open",
    ];

    #[test]
    fn handler_list_parses() {
        let cmds = handler_commands(LIB);
        assert!(cmds.len() > 150, "{}", cmds.len());
        for c in ["get_state", "save_config", "pin_set", "desk_set", "update_install", "mcp_reveal_key"] {
            assert!(cmds.iter().any(|x| x == c), "{c}");
        }
        let uniq: BTreeSet<&String> = cmds.iter().collect();
        assert_eq!(uniq.len(), cmds.len(), "příkaz dvakrát v generate_handler!");
        assert!(cmds.iter().all(|c| c.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_')), "{cmds:?}");
        let txt = "x tauri::generate_handler![\n  a::b::one, // komentář\n  two,\n  c::three\n]) y";
        assert_eq!(handler_commands(txt), ["one", "two", "three"]);
        assert!(handler_commands("nic").is_empty());
        assert_eq!(permission_of("note_set_meta"), "allow-note-set-meta");
    }

    /// Šuplík smí všechno (beze změny proti době bez ACL), lístek a widget přesně svou tabulku; žádná
    /// capability nepovoluje neexistující příkaz ani plugin (fs, shell, opener, dialog…) — jen poslech událostí.
    #[test]
    fn capabilities_match_tables() {
        let cmds = handler_commands(LIB);
        let all: BTreeSet<String> = cmds.iter().map(|c| permission_of(c)).collect();
        let caps = caps();
        let by = |f: &str| &caps.iter().find(|(n, _)| n == f).unwrap_or_else(|| panic!("chybí capabilities/{f}")).1;
        let drawer = by("default.json");
        let drawer_perms = app_perms(drawer);
        let missing: Vec<_> = all.difference(&drawer_perms).collect();
        assert!(missing.is_empty(), "šuplík (capabilities/default.json) nemá: {missing:?}");
        let table = |t: &[&str]| t.iter().map(|c| permission_of(c)).collect::<BTreeSet<_>>();
        assert_eq!(app_perms(by("pins.json")), table(PIN_WINDOW), "capabilities/pins.json ≠ PIN_WINDOW");
        assert_eq!(app_perms(by("desk.json")), table(DESK_WINDOW), "capabilities/desk.json ≠ DESK_WINDOW");
        assert_eq!(app_perms(by("studio.json")), table(STUDIO_WINDOW), "capabilities/studio.json ≠ STUDIO_WINDOW");
        for (name, c) in &caps {
            for p in &c.permissions {
                let ok = matches!(p.as_str(), "core:event:allow-listen" | "core:event:allow-unlisten") || all.contains(p);
                assert!(ok, "{name}: nečekané oprávnění {p}");
            }
        }
        assert_eq!(drawer.windows, [crate::drawer::LABEL]);
        assert_eq!(by("pins.json").windows, [format!("{}*", crate::pins::LABEL_PREFIX)]);
        assert_eq!(by("desk.json").windows, [format!("{}*", crate::desk::LABEL_PREFIX)]);
        assert_eq!(by("studio.json").windows, [crate::studio::LABEL]);
        // a build.rs opravdu zapíná AppManifest (bez něj by capability příkazy neomezovala)
        let build = include_str!("../build.rs");
        assert!(build.contains("AppManifest::new().commands(") && build.contains("handler_commands("), "build.rs bez AppManifest");
    }

    /// Povolit akci Clauda smí jen člověk v šuplíku: okno Studia nesmí volat `decide`, `question_*`, `agent_*`.
    #[test]
    fn studio_cannot_approve_claude() {
        for c in STUDIO_WINDOW {
            assert!(*c != "decide" && !c.starts_with("question_") && !c.starts_with("agent_"), "Studio smí {c}");
        }
        let caps = caps();
        let studio = &caps.iter().find(|(n, _)| n == "studio.json").expect("studio.json").1;
        for p in &studio.permissions {
            assert!(!p.contains("allow-decide") && !p.contains("allow-question-") && !p.contains("allow-agent-"), "studio.json: {p}");
        }
    }

    #[test]
    fn allowed_table() {
        let caps: Vec<Cap> = caps().into_iter().map(|(_, c)| c).collect();
        let note = format!("{}abc", crate::pins::LABEL_PREFIX);
        let desk = format!("{}x1", crate::desk::LABEL_PREFIX);
        let web = crate::webpanel::label("x");
        #[rustfmt::skip]
        let cases: Vec<(&str, &str, bool)> = vec![
            ("drawer", "get_state", true),
            ("drawer", "save_config", true),
            ("drawer", "pin_toggle", true),
            ("drawer", "desk_pin", true),
            ("drawer", "update_install", true),
            (&note, "pin_set", true),
            (&note, "note_save", true),
            (&note, "note_read", true),
            (&note, "save_config", false),
            (&note, "get_config", false),
            (&note, "pin_toggle", false),
            (&note, "pin_new", false),
            (&note, "run_action", false),
            (&note, "action_consent", false),
            (&note, "desk_set", false),
            (&note, "update_install", false),
            (&desk, "desk_set", true),
            (&desk, "desk_drag", true),
            (&desk, "widget_click", true),
            (&desk, "stop", true),
            (&desk, "get_config", true),
            (&desk, "save_config", false),
            (&desk, "widget_consent", false),
            (&desk, "widget_install", false),
            (&desk, "desk_pin", false),
            (&desk, "desk_profile_apply", false),
            (&desk, "run_action", false),
            (&desk, "export_usage", false),
            (&desk, "pin_set", false),
            (&desk, "mcp_reveal_key", false),
            ("studio", "save_config", true),
            ("studio", "check_config", true),
            ("studio", "studio_win", true),
            ("studio", "action_consent", true),
            ("studio", "focus_drawer", false),
            ("studio", "release_focus", false),
            ("studio", "drawer_size", false),
            ("studio", "launch", false),
            ("studio", "pin_set", false),
            ("studio", "desk_drag", false),
            ("studio", "studio_open", false),
            ("studio2", "save_config", false),
            (&note, "studio_win", false),
            (&desk, "studio_open", false),
            (&web, "get_state", false),
            (&web, "open_url", false),
            ("pdf-export-1", "get_state", false),
            ("drawer2", "get_state", false),
            ("Drawer", "get_state", false),
            ("note", "pin_set", false),
            ("", "get_state", false),
        ];
        for (label, cmd, want) in cases {
            assert_eq!(allowed(&caps, label, cmd), want, "{label} → {cmd}");
        }
        assert!(label_matches("note-*", "note-") && !label_matches("note-*", "notes") && !label_matches("a*b*", "ab"));
    }
}
