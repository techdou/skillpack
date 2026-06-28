use git2::{FetchOptions, RemoteCallbacks, Repository};
use std::path::Path;

/// Clone a git repository to target directory
pub fn clone_repo(url: &str, target: &Path) -> Result<(), String> {
    // Progress callback
    let mut cb = RemoteCallbacks::new();
    cb.transfer_progress(|_prog| true);

    let mut fo = FetchOptions::new();
    fo.remote_callbacks(cb);

    Repository::clone(url, target).map_err(|e| format!("Git clone failed: {}", e.message()))?;
    Ok(())
}

/// Pull latest changes for a repository
pub fn pull_repo(repo_path: &Path) -> Result<String, String> {
    let repo =
        Repository::open(repo_path).map_err(|e| format!("Failed to open repo: {}", e.message()))?;

    let mut remote = repo
        .find_remote("origin")
        .or_else(|_| repo.remote_anonymous("origin"))
        .map_err(|e| format!("Failed to find remote: {}", e.message()))?;

    let mut cb = RemoteCallbacks::new();
    cb.transfer_progress(|_prog| true);

    let mut fo = FetchOptions::new();
    fo.remote_callbacks(cb);

    let head = repo
        .head()
        .map_err(|e| format!("Failed to get HEAD: {}", e.message()))?;
    let branch_name = head
        .shorthand()
        .ok_or("Cannot update a repository with detached HEAD")?
        .to_string();
    drop(head);

    remote
        .fetch(&[&branch_name], Some(&mut fo), None)
        .map_err(|e| format!("Git fetch failed: {}", e.message()))?;

    let remote_ref_name = format!("refs/remotes/origin/{}", branch_name);
    let remote_ref = repo
        .find_reference(&remote_ref_name)
        .map_err(|e| format!("Failed to find fetched branch: {}", e.message()))?;
    let remote_oid = remote_ref
        .target()
        .ok_or("Fetched branch does not point to a commit")?;
    let annotated = repo
        .find_annotated_commit(remote_oid)
        .map_err(|e| format!("Failed to inspect fetched commit: {}", e.message()))?;
    let (analysis, _) = repo
        .merge_analysis(&[&annotated])
        .map_err(|e| format!("Failed to analyze update: {}", e.message()))?;

    if analysis.is_fast_forward() {
        let statuses = repo
            .statuses(None)
            .map_err(|e| format!("Failed to inspect working tree: {}", e.message()))?;
        if !statuses.is_empty() {
            return Err("Working tree has local changes; refusing to overwrite pack files".into());
        }

        let local_ref_name = format!("refs/heads/{}", branch_name);
        let mut local_ref = repo
            .find_reference(&local_ref_name)
            .map_err(|e| format!("Failed to find local branch: {}", e.message()))?;
        local_ref
            .set_target(remote_oid, "Fast-forward")
            .map_err(|e| format!("Failed to update branch: {}", e.message()))?;
        repo.set_head(&local_ref_name)
            .map_err(|e| format!("Failed to set HEAD: {}", e.message()))?;
        repo.checkout_head(Some(git2::build::CheckoutBuilder::default().force()))
            .map_err(|e| format!("Failed to update working tree: {}", e.message()))?;
    } else if !analysis.is_up_to_date() {
        return Err("Non-fast-forward updates are not supported".into());
    }

    let head = repo
        .head()
        .map_err(|e| format!("Failed to get HEAD: {}", e.message()))?;
    let commit = head
        .peel_to_commit()
        .map_err(|e| format!("Failed to peel commit: {}", e.message()))?;
    let msg = commit.message().unwrap_or("(no message)").to_string();

    Ok(msg)
}

/// Best-effort restore of `repo_path` to a clean `HEAD` state.
///
/// Used after a failed `pull_repo` so a half-fetched / mid-merge checkout does
/// not leave a dirty working tree for subsequent reads. We abort any in-progress
/// merge, then force-checkout HEAD. Errors are swallowed: this is a recovery
/// path, and the caller already has an error to report.
pub fn reset_worktree(repo_path: &Path) -> Result<(), String> {
    let repo =
        Repository::open(repo_path).map_err(|e| format!("Failed to open repo: {}", e.message()))?;

    // Abort an in-progress merge if one exists (state MergeNeeded can leave a
    // MERGE_HEAD). `cleanup_state` is the safe no-op-if-clean helper.
    let _ = repo.cleanup_state();

    repo.checkout_head(Some(git2::build::CheckoutBuilder::default().force()))
        .map_err(|e| format!("Failed to reset working tree: {}", e.message()))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use git2::Signature;

    /// Create a throwaway repo with one committed file, return its path.
    fn fresh_repo() -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "skillpack-git-test-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let repo = Repository::init(&dir).unwrap();
        // Configure a local identity so the commit succeeds without relying on
        // the user's global git config.
        let sig = Signature::now("test", "test@example.com").unwrap();
        // Write and commit a tracked file so HEAD points somewhere meaningful.
        let file = dir.join("tracked.txt");
        std::fs::write(&file, "original\n").unwrap();
        let mut index = repo.index().unwrap();
        index.add_path(std::path::Path::new("tracked.txt")).unwrap();
        index.write().unwrap();
        let tree_oid = index.write_tree().unwrap();
        let tree = repo.find_tree(tree_oid).unwrap();
        repo.commit(Some("HEAD"), &sig, &sig, "init", &tree, &[])
            .unwrap();
        dir
    }

    #[test]
    fn reset_worktree_restores_modified_tracked_file() {
        let dir = fresh_repo();
        let file = dir.join("tracked.txt");

        // Dirty the working tree.
        std::fs::write(&file, "modified\n").unwrap();
        let repo = Repository::open(&dir).unwrap();
        let dirty = repo.statuses(None).unwrap();
        assert!(!dirty.is_empty(), "precondition: tree should be dirty");

        reset_worktree(&dir).unwrap();

        let repo = Repository::open(&dir).unwrap();
        let clean = repo.statuses(None).unwrap();
        assert!(clean.is_empty(), "reset_worktree should clean the tree");
        // git2 may apply autocrlf on Windows, so normalise line endings before
        // comparing content — what matters is that the original content was
        // restored, not its exact byte representation.
        let restored = std::fs::read_to_string(&file).unwrap();
        assert_eq!(restored.trim_end(), "original");

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn reset_worktree_is_noop_on_clean_tree() {
        let dir = fresh_repo();
        // Tree is already clean after commit.
        reset_worktree(&dir).unwrap();
        let repo = Repository::open(&dir).unwrap();
        let clean = repo.statuses(None).unwrap();
        assert!(clean.is_empty(), "clean tree must stay clean");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn reset_worktree_errors_on_non_repo() {
        let nowhere = std::env::temp_dir().join("skillpack-not-a-repo-12345");
        assert!(reset_worktree(&nowhere).is_err());
    }
}
