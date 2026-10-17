//! Jeden cyklus synchronizace nad existujícím repem: `add -A` → commit jen se změnami → `fetch` →
//! `rebase --autostash` (jen když remote utekl) → `push` (jen když jsme napřed). Konflikt =
//! `rebase --abort` a `Outcome::Konflikt` (lokální commit zůstane). Závisí na `git.rs`.

use crate::i18n::tr;
use super::git::{Fail, commit_all, git, local_fail, net_fail, nul_list};
use super::{Ctx, IGNORE, Outcome, commit_message, ensure_lines};

/// Jeden cyklus nad existujícím repem: commit → fetch → rebase → push.
pub(super) fn cycle(ctx: &Ctx) -> Outcome {
    let (root, remote) = (ctx.root, ctx.remote);
    let gd = root.join(".git");
    if gd.join("rebase-merge").exists() || gd.join("rebase-apply").exists() || gd.join("MERGE_HEAD").exists() {
        let files = git(root, remote, &["diff", "--name-only", "--diff-filter=U", "-z"], false).map(|s| nul_list(&s)).unwrap_or_default();
        return Outcome::Konflikt {
            files,
            message: tr(
                "V kořeni poznámek visí rozpracovaný rebase/merge — dokonči ho, nebo `git rebase --abort`.",
                "An unfinished rebase/merge is pending in the notes folder — finish it, or `git rebase --abort`.",
            )
            .into(),
        };
    }
    let branch = match git(root, remote, &["symbolic-ref", "--short", "HEAD"], false) {
        Ok(b) => b.trim().to_string(),
        Err(_) => return Outcome::Chyba(tr("Repo poznámek není na žádné větvi (detached HEAD).", "The notes repo isn't on any branch (detached HEAD).").into()),
    };
    // .gitignore mohl někdo smazat — bez něj by se commitnul koncept
    let gi = std::fs::read_to_string(root.join(".gitignore")).unwrap_or_default();
    if let Some(text) = ensure_lines(&gi, &IGNORE)
        && let Err(e) = std::fs::write(root.join(".gitignore"), text)
    {
        return Outcome::Chyba(trf!("Zápis .gitignore: {e}", "Writing .gitignore: {e}"));
    }
    if let Err(o) = commit_all(root, remote, &commit_message(ctx.pc, &ctx.now)) {
        return o;
    }
    let upstream = format!("origin/{branch}");
    let (mut pulled, mut pushed) = (false, false);
    for attempt in 0..2 {
        if let Err(f) = git(root, remote, &["fetch", "-q", "origin"], true) {
            return net_fail(f);
        }
        let has_remote = git(root, remote, &["rev-parse", "--verify", "-q", &format!("refs/remotes/{upstream}")], false).is_ok();
        let (ahead, behind) = if has_remote {
            match git(root, remote, &["rev-list", "--left-right", "--count", &format!("HEAD...{upstream}")], false) {
                Ok(s) => {
                    let mut it = s.split_whitespace().map(|n| n.parse::<u32>().unwrap_or(0));
                    (it.next().unwrap_or(0), it.next().unwrap_or(0))
                }
                Err(f) => return local_fail("git rev-list", f),
            }
        } else {
            (1, 0)
        };
        if behind > 0 {
            if let Err(f) = git(root, remote, &["rebase", "-q", "--autostash", &upstream], false) {
                let files = git(root, remote, &["diff", "--name-only", "--diff-filter=U", "-z"], false).map(|s| nul_list(&s)).unwrap_or_default();
                let _ = git(root, remote, &["rebase", "--abort"], false);
                if files.is_empty() {
                    return local_fail("git rebase", f);
                }
                return Outcome::Konflikt {
                    files,
                    message: tr(
                        "Stejná poznámka se změnila na dvou PC. Lokální verze zůstala v lokálním commitu; vyřeš ručně (`git pull --rebase` v kořeni poznámek) nebo jednu verzi zahoď.",
                        "The same note changed on two PCs. The local version stayed in a local commit; resolve it by hand (`git pull --rebase` in the notes folder) or drop one version.",
                    )
                    .into(),
                };
            }
            pulled = true;
        }
        if ahead == 0 {
            break;
        }
        match git(root, remote, &["push", "-q", "-u", "origin", &branch], true) {
            Ok(_) => {
                pushed = true;
                break;
            }
            Err(Fail::Git(e)) if attempt == 0 && ["rejected", "non-fast-forward", "fetch first"].iter().any(|p| e.contains(p)) => continue,
            Err(f) => return net_fail(f),
        }
    }
    Outcome::Ok { pulled, pushed, swapped: None }
}
