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
