use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackInfo {
    pub source: String,
    #[serde(rename = "type")]
    pub pack_type: String,
    pub installed_at: String,
    pub skill_root: Option<String>,
    pub skills: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectLink {
    pub pack: String,
    pub target: String,
    /// How the link was materialized on disk: "symlink" or "copy".
    /// `None` for legacy entries created before this field existed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub link_type: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectConfig {
    pub targets: HashMap<String, String>,
    pub links: HashMap<String, ProjectLink>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub version: u32,
    pub packs_dir: String,
    pub default_targets: Vec<String>,
    pub projects: HashMap<String, ProjectConfig>,
    pub packs: HashMap<String, PackInfo>,
    #[serde(default)]
    pub codex_config_path: Option<String>,
}

/// Resolve the user's home directory, failing loudly instead of silently
/// falling back to the current working directory (which could write
/// `config.json` into an arbitrary directory).
fn home_dir() -> Result<PathBuf, String> {
    dirs::home_dir().ok_or_else(|| "Cannot determine home directory".to_string())
}

impl Default for AppConfig {
    fn default() -> Self {
        // `default()` must remain infallible for derive consumers; we resolve
        // home best-effort here. The authoritative path resolution happens
        // through `load()`/`save()` which surface a real error.
        let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
        let packs_dir = home.join(".skillpack").join("packs");
        Self {
            version: 1,
            packs_dir: packs_dir.to_string_lossy().to_string(),
            default_targets: vec!["codex".into(), "claude".into(), "gemini".into()],
            projects: HashMap::new(),
            packs: HashMap::new(),
            codex_config_path: Some(
                home.join(".codex")
                    .join("config.toml")
                    .to_string_lossy()
                    .to_string(),
            ),
        }
    }
}

impl AppConfig {
    pub fn config_path() -> Result<PathBuf, String> {
        if let Ok(path) = std::env::var("SKILLPACK_CONFIG_PATH") {
            return Ok(PathBuf::from(path));
        }

        let home = home_dir()?;
        Ok(home.join(".skillpack").join("config.json"))
    }

    pub fn load() -> Result<Self, String> {
        let path = Self::config_path()?;
        if !path.exists() {
            let config = Self::default();
            config.save()?;
            return Ok(config);
        }
        let content = fs::read_to_string(&path).map_err(|e| e.to_string())?;
        serde_json::from_str(&content).map_err(|e| e.to_string())
    }

    pub fn save(&self) -> Result<(), String> {
        let path = Self::config_path()?;
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        let content = serde_json::to_string_pretty(self).map_err(|e| e.to_string())?;
        fs::write(&path, content).map_err(|e| e.to_string())
    }

    pub fn packs_dir_path(&self) -> PathBuf {
        PathBuf::from(&self.packs_dir)
    }

    pub fn canonical_project_key(path: &str) -> Result<String, String> {
        fs::canonicalize(path)
            .map_err(|e| format!("Failed to resolve project path '{}': {}", path, e))
            .map(|p| p.to_string_lossy().to_string())
    }

    pub fn codex_config_path(&self) -> Option<PathBuf> {
        self.codex_config_path
            .as_deref()
            .map(Self::resolve_path)
            .filter(|path| path.exists())
    }

    /// Resolve ~ in paths
    pub fn resolve_path(p: &str) -> PathBuf {
        if p.starts_with('~') {
            let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
            let rest = p.strip_prefix('~').unwrap_or(p);
            let rest = rest.strip_prefix(['/', '\\']).unwrap_or(rest);
            home.join(rest)
        } else {
            PathBuf::from(p)
        }
    }

    /// Map target name to relative skills directory.
    ///
    /// Each toolchain now maps to its own directory. Gemini uses
    /// `.gemini/skills` (the Gemini CLI convention) so that linking the same
    /// skill to both Codex and Gemini no longer overwrites itself.
    pub fn target_to_dir(target: &str) -> &'static str {
        match target {
            "codex" | "agents" => ".agents/skills",
            "gemini" => ".gemini/skills",
            "claude" => ".claude/skills",
            "cursor" => ".cursor/skills",
            _ => ".agents/skills",
        }
    }
}

// ---------------------------------------------------------------------------
// Serialized access to the config file
// ---------------------------------------------------------------------------
//
// Every mutating command historically did `load() -> mutate -> save()` on the
// JSON config. Under Tauri these commands run on a thread pool, so two
// concurrent invocations could race and silently drop one another's writes.
//
// We now funnel every read-modify-write through a process-wide mutex. The
// closure receives a freshly-loaded `AppConfig`; mutating and returning
// `Ok` triggers a single `save()`. Returning `Err` leaves the file untouched.
// The CLI binary shares the same lock for free.

fn global_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

/// Read-only access to the config under the global lock.
pub fn with_config<R>(f: impl FnOnce(&AppConfig) -> R) -> Result<R, String> {
    let _guard = global_lock().lock().map_err(|e| e.to_string())?;
    let config = AppConfig::load()?;
    Ok(f(&config))
}

/// Read-modify-write access to the config under the global lock. The closure
/// mutates the loaded config in place; if it returns `Ok`, the result is
/// persisted atomically (relative to other SkillPack callers). If it returns
/// `Err`, the on-disk file is left untouched.
pub fn with_config_mut<R>(
    f: impl FnOnce(&mut AppConfig) -> Result<R, String>,
) -> Result<R, String> {
    let _guard = global_lock().lock().map_err(|e| e.to_string())?;
    let mut config = AppConfig::load()?;
    let result = f(&mut config)?;
    config.save()?;
    Ok(result)
}

#[cfg(test)]
pub(crate) mod test_support {
    use std::sync::{Mutex, MutexGuard, OnceLock};

    /// Tests that set the `SKILLPACK_CONFIG_PATH` env var must hold this guard
    /// for their entire body, because the env var is process-global and
    /// `cargo test` runs tests in parallel. All test modules share the same
    /// underlying mutex (this is the single source of the lock).
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();

    pub fn env_lock() -> MutexGuard<'static, ()> {
        LOCK.get_or_init(|| Mutex::new(()))
            .lock()
            .unwrap_or_else(|e| e.into_inner())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::test_support::env_lock;

    #[test]
    fn gemini_target_uses_distinct_directory() {
        assert_eq!(AppConfig::target_to_dir("codex"), ".agents/skills");
        assert_eq!(AppConfig::target_to_dir("agents"), ".agents/skills");
        assert_eq!(AppConfig::target_to_dir("gemini"), ".gemini/skills");
        assert_eq!(AppConfig::target_to_dir("claude"), ".claude/skills");
        assert_ne!(
            AppConfig::target_to_dir("codex"),
            AppConfig::target_to_dir("gemini")
        );
    }

    #[test]
    fn project_link_link_type_is_optional_and_backwards_compatible() {
        // A legacy config without link_type should still deserialize.
        let json = r#"{"pack":"core","target":"codex"}"#;
        let link: ProjectLink = serde_json::from_str(json).unwrap();
        assert_eq!(link.pack, "core");
        assert_eq!(link.link_type, None);
    }

    #[test]
    fn with_config_mut_persists_and_is_atomic_on_error() {
        let _guard = env_lock();
        let tmp = std::env::temp_dir().join(format!(
            "skillpack-lock-test-{}.json",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::env::set_var("SKILLPACK_CONFIG_PATH", &tmp);
        // seed a default config
        AppConfig::default().save().unwrap();

        // A returning-Err closure must NOT persist its in-memory mutation.
        let err = with_config_mut(|c| -> Result<(), String> {
            c.version = 999;
            Err("abort".into())
        });
        assert!(err.is_err());
        assert_eq!(AppConfig::load().unwrap().version, 1, "error aborted save");

        // A returning-Ok closure persists.
        with_config_mut(|c| {
            c.version = 2;
            Ok(())
        })
        .unwrap();
        assert_eq!(AppConfig::load().unwrap().version, 2);

        std::env::remove_var("SKILLPACK_CONFIG_PATH");
        let _ = std::fs::remove_file(tmp);
    }
}
