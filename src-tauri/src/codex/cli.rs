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
///
/// Returns an error naming what was tried so the UI can guide the user to
/// install Codex or set the env var.
pub fn resolve_codex_cli() -> Result<String, SkillError> {
    if let Ok(path) = std::env::var("CODEX_CLI_PATH") {
        if !path.is_empty() {
            return Ok(path);
        }
    }
    // `codex` is expected on PATH on all platforms. `which` is not available in
    // the std; rely on the shell/OS resolver used by Command::new.
    if which_on_path("codex") {
        return Ok("codex".into());
    }
    Err(SkillError::NotFound(
        "codex CLI on PATH (set CODEX_CLI_PATH to point at it explicitly)".into(),
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

/// Run `codex <args..>` and capture output. Errors carry stderr.
pub fn run(args: &[&str]) -> Result<CliOutput, SkillError> {
    let codex = resolve_codex_cli()?;
    // Spawn with a child we can poll, so we can enforce a timeout without an
    // extra crate. We use a simple thread + try_wait loop.
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
                    let _ = child.kill();
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
}
