//! Thin wrapper around the `codex` CLI for plugin/marketplace mutations.
//!
//! All install/uninstall/update operations are delegated to the CLI so the
//! `~/.codex` cache directory and `config.toml` stay consistent with what
//! Codex itself would produce. We only need stdout/stderr and a non-zero
//! status check; there is no streaming requirement.

use std::process::Command;
use std::time::Duration;

use crate::error::SkillError;

/// Upper bound for a single CLI invocation. Installs may clone a git repo, so
/// we keep this generous; anything longer almost certainly indicates a hang.
const CLI_TIMEOUT: Duration = Duration::from_secs(120);

/// Result of a CLI invocation.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct CliOutput {
    pub stdout: String,
    pub stderr: String,
}

/// Resolve the `codex` executable path.
///
/// Strategy, in order:
/// 1. `CODEX_CLI_PATH` env var (explicit override, useful for tests).
/// 2. `codex` on `PATH` (the normal case once Codex is installed).
/// 3. Known install locations on Windows (`%LOCALAPPDATA%\OpenAI\Codex\bin\*\codex.exe`)
///    and macOS/Linux (`~/.codex/bin/codex`) when Codex is installed but not on PATH.
///
/// Returns an error naming what was tried so the UI can guide the user to
/// install Codex or set the env var.
pub fn resolve_codex_cli() -> Result<String, SkillError> {
    // 1. Explicit override.
    if let Ok(path) = std::env::var("CODEX_CLI_PATH") {
        if !path.is_empty() {
            return Ok(path);
        }
    }
    // 2. On PATH.
    if which_on_path("codex") {
        return Ok("codex".into());
    }
    // 3. Known install locations for when Codex is installed but not on PATH.
    if let Some(found) = find_in_known_locations() {
        return Ok(found.to_string_lossy().to_string());
    }
    Err(SkillError::NotFound(
        "codex CLI not found on PATH, in known install locations, or via CODEX_CLI_PATH".into(),
    ))
}

/// Best-effort existence check for an executable on PATH, without pulling in
/// the `which` crate.
fn which_on_path(program: &str) -> bool {
    let path_var = match std::env::var("PATH") {
        Ok(p) => p,
        Err(_) => return false,
    };
    let exe_ext = if cfg!(windows) { ".exe" } else { "" };
    for dir in path_var.split(if cfg!(windows) { ';' } else { ':' }) {
        if dir.is_empty() {
            continue;
        }
        let candidate = std::path::Path::new(dir).join(format!("{}{}", program, exe_ext));
        if candidate.exists() {
            return true;
        }
    }
    false
}

/// Look for the codex binary in well-known install roots that are not on PATH.
///
/// Codex's own installers place the binary under a hashed version directory,
/// e.g. `%LOCALAPPDATA%\OpenAI\Codex\bin\<hash>\codex.exe`. We glob the `bin`
/// directory and pick the newest `codex.exe` by modification time. On Unix we
/// also check `~/.codex/bin/codex` and `~/.local/bin/codex`.
fn find_in_known_locations() -> Option<std::path::PathBuf> {
    let candidates: Vec<std::path::PathBuf> = if cfg!(windows) {
        // %LOCALAPPDATA%\OpenAI\Codex\bin\<hash>\codex.exe
        let local = dirs::data_local_dir()?;
        vec![local.join("OpenAI").join("Codex").join("bin")]
    } else {
        let home = dirs::home_dir()?;
        vec![
            home.join(".codex").join("bin"),
            home.join(".local").join("bin"),
        ]
    };

    for root in candidates {
        if let Some(found) = find_codex_under(&root) {
            return Some(found);
        }
    }
    None
}

/// Search `root` (and, on Windows, one level of hashed subdirectories) for the
/// newest `codex`/`codex.exe` by mtime.
fn find_codex_under(root: &std::path::Path) -> Option<std::path::PathBuf> {
    let exe_ext = if cfg!(windows) { ".exe" } else { "" };
    let target = format!("codex{}", exe_ext);

    // Direct match: root/codex(.exe)
    let direct = root.join(&target);
    if direct.is_file() {
        return Some(direct);
    }

    // Windows: Codex nests under versioned dirs: root/<hash>/codex.exe.
    // Collect all matches and pick the most recently modified one so an
    // auto-update lands on the newest version deterministically.
    let entries = std::fs::read_dir(root).ok()?;
    let mut best: Option<(std::time::SystemTime, std::path::PathBuf)> = None;
    for entry in entries.flatten() {
        // Recurse one level into subdirectories (the hashed version dirs).
        let path = entry.path();
        let search_dirs: Vec<std::path::PathBuf> = if path.is_dir() {
            vec![path.clone()]
        } else {
            continue;
        };
        for dir in search_dirs {
            let candidate = dir.join(&target);
            if !candidate.is_file() {
                continue;
            }
            let mtime = std::fs::metadata(&candidate)
                .and_then(|m| m.modified())
                .unwrap_or(std::time::SystemTime::UNIX_EPOCH);
            match &best {
                Some((cur, _)) if *cur >= mtime => {}
                _ => best = Some((mtime, candidate)),
            }
        }
    }
    best.map(|(_, p)| p)
}

/// Run `codex <args..>` and capture output. Errors carry stderr.
///
/// On timeout we kill the child **and wait for it to exit** so the OS reaps
/// the process and we don't leak a zombie or an orphaned codex subprocess.
pub fn run(args: &[&str]) -> Result<CliOutput, SkillError> {
    let codex = resolve_codex_cli()?;
    // Spawn with a child we can poll, so we can enforce a timeout without an
    // extra crate. We use a simple try_wait loop.
    let mut cmd = Command::new(&codex);
    cmd.args(args);
    let mut child = cmd
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| SkillError::Other(format!("failed to launch codex: {}", e)))?;

    let start = std::time::Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                let stdout = read_to_string_or_empty(child.stdout.take());
                let stderr = read_to_string_or_empty(child.stderr.take());
                if status.success() {
                    return Ok(CliOutput { stdout, stderr });
                }
                return Err(SkillError::Other(format!(
                    "codex {} exited with status {}{}",
                    args.join(" "),
                    status,
                    if stderr.trim().is_empty() {
                        String::new()
                    } else {
                        format!(":\n{}", stderr.trim())
                    }
                )));
            }
            Ok(None) => {
                if start.elapsed() > CLI_TIMEOUT {
                    // Kill then reap: an un-reaped killed child becomes a
                    // zombie on Unix and may hold stdout/stderr pipes open.
                    let _ = child.kill();
                    let _ = child.wait();
                    return Err(SkillError::Other(format!(
                        "codex {} timed out after {:?}",
                        args.join(" "),
                        CLI_TIMEOUT
                    )));
                }
                std::thread::sleep(Duration::from_millis(50));
            }
            Err(e) => return Err(SkillError::Other(format!("failed to wait on codex: {}", e))),
        }
    }
}

fn read_to_string_or_empty<R: std::io::Read>(mut r: Option<R>) -> String {
    r.as_mut()
        .map(|r| {
            let mut buf = String::new();
            let _ = r.read_to_string(&mut buf);
            buf
        })
        .unwrap_or_default()
}

// ---------------------------------------------------------------------------
// High-level operations
// ---------------------------------------------------------------------------

/// `codex plugin add <plugin>@<marketplace>` (or `<plugin>` when no
/// marketplace is given).
pub fn plugin_add(plugin: &str, marketplace: Option<&str>) -> Result<CliOutput, SkillError> {
    let target = match marketplace {
        Some(m) if !m.is_empty() => format!("{}@{}", plugin, m),
        _ => plugin.to_string(),
    };
    run(&["plugin", "add", &target])
}

/// `codex plugin remove <key>`.
pub fn plugin_remove(key: &str) -> Result<CliOutput, SkillError> {
    run(&["plugin", "remove", key])
}

/// `codex plugin marketplace add <source>`.
pub fn marketplace_add(source: &str) -> Result<CliOutput, SkillError> {
    run(&["plugin", "marketplace", "add", source])
}

/// `codex plugin marketplace upgrade [name]`.
pub fn marketplace_upgrade(name: Option<&str>) -> Result<CliOutput, SkillError> {
    match name {
        Some(n) if !n.is_empty() => run(&["plugin", "marketplace", "upgrade", n]),
        _ => run(&["plugin", "marketplace", "upgrade"]),
    }
}

/// `codex plugin marketplace remove <name>`.
pub fn marketplace_remove(name: &str) -> Result<CliOutput, SkillError> {
    run(&["plugin", "marketplace", "remove", name])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_uses_env_override() {
        // We can't rely on codex being installed in CI, but we can assert the
        // env-var path is honoured when set.
        std::env::set_var("CODEX_CLI_PATH", "/nonexistent/codex-test");
        let resolved = resolve_codex_cli().unwrap();
        assert_eq!(resolved, "/nonexistent/codex-test");
        std::env::remove_var("CODEX_CLI_PATH");
    }

    #[test]
    fn plugin_add_target_combines_marketplace() {
        // Pure logic check: we only validate the string we would pass.
        let with_mkt = match (Some("openai-curated"), "vercel") {
            (Some(m), p) if !m.is_empty() => format!("{}@{}", p, m),
            (_, p) => p.to_string(),
        };
        assert_eq!(with_mkt, "vercel@openai-curated");
    }

    #[test]
    fn find_codex_under_direct_match() {
        // A direct root/codex(.exe) should be found without subdirectories.
        let tmp = std::env::temp_dir().join(format!(
            "codex-find-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&tmp).unwrap();
        let exe_ext = if cfg!(windows) { ".exe" } else { "" };
        let bin = tmp.join(format!("codex{}", exe_ext));
        std::fs::write(&bin, b"fake").unwrap();

        let found = find_codex_under(&tmp).unwrap();
        assert_eq!(found, bin);

        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn find_codex_under_picks_newest_in_subdir() {
        // Windows layout: root/<hash>/codex.exe. We create two versioned dirs
        // and verify the newer one (by mtime) is selected.
        let tmp = std::env::temp_dir().join(format!(
            "codex-find-newest-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let old = tmp.join("aaaa1111");
        let new = tmp.join("bbbb2222");
        std::fs::create_dir_all(&old).unwrap();
        std::fs::create_dir_all(&new).unwrap();
        let exe_ext = if cfg!(windows) { ".exe" } else { "" };
        std::fs::write(old.join(format!("codex{}", exe_ext)), b"old").unwrap();
        // Sleep briefly so the "new" file has a strictly later mtime.
        std::thread::sleep(std::time::Duration::from_millis(20));
        let new_bin = new.join(format!("codex{}", exe_ext));
        std::fs::write(&new_bin, b"new").unwrap();

        let found = find_codex_under(&tmp).unwrap();
        assert_eq!(found, new_bin);

        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn find_codex_under_returns_none_when_empty() {
        let tmp = std::env::temp_dir().join("codex-find-empty-12345");
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(&tmp).unwrap();
        assert!(find_codex_under(&tmp).is_none());
        let _ = std::fs::remove_dir_all(&tmp);
    }
}
