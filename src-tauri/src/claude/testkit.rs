//! Testovací pomůcky pro `claude::*::tests`: dočasná git repa (i s bare remotem), hook eventy a
//! čtení session. Jen `#[cfg(test)]`; nikdy nesahá na skutečné `~/.claude`.

use super::{Claude, Session};
use serde_json::{Value, json};
use std::path::Path;

pub(super) fn git(dir: &Path, args: &[&str]) {
    let st = std::process::Command::new("git")
        .args(["-c", "user.name=t", "-c", "user.email=t@t", "-c", "commit.gpgsign=false"])
        .args(args)
        .current_dir(dir)
        .output()
        .unwrap();
    assert!(st.status.success(), "git {args:?}: {}", String::from_utf8_lossy(&st.stderr));
}

/// dočasné repo s jedním commitem
pub(super) fn temp_repo() -> tempfile::TempDir {
    let t = tempfile::tempdir().unwrap();
    git(t.path(), &["init", "-q"]);
    std::fs::write(t.path().join("base.txt"), "x").unwrap();
    git(t.path(), &["add", "."]);
    git(t.path(), &["commit", "-q", "-m", "base"]);
    t
}

/// repo s bare remotem a upstreamem aktuální větve
pub(super) fn temp_repo_upstream() -> (tempfile::TempDir, tempfile::TempDir) {
    let repo = temp_repo();
    let remote = tempfile::tempdir().unwrap();
    git(remote.path(), &["init", "-q", "--bare"]);
    git(repo.path(), &["remote", "add", "origin", &remote.path().to_string_lossy()]);
    git(repo.path(), &["push", "-q", "-u", "origin", "HEAD:refs/heads/main"]);
    (repo, remote)
}

pub(super) fn ev(name: &str, sid: &str, cwd: &str, extra: Value) -> Value {
    let mut v = json!({"hook_event_name": name, "session_id": sid, "cwd": cwd});
    if let (Some(o), Value::Object(e)) = (v.as_object_mut(), extra) {
        o.extend(e);
    }
    v
}

pub(super) fn sess(c: &Claude, id: &str) -> Session {
    c.sessions().into_iter().find(|s| s.id == id).unwrap()
}

pub(super) fn write_tool(c: &Claude, sid: &str, cwd: &str, path: &Path, content: &str) -> Option<String> {
    std::fs::write(path, content).unwrap();
    c.handle_event(&ev("PostToolUse", sid, cwd, json!({"tool_name":"Write","tool_input":{"file_path": path}})))
}
