use crate::config::{with_config, with_config_mut, AppConfig};
use serde::{Deserialize, Serialize};

/// Read-only snapshot of the full config for the Settings page.
#[tauri::command]
pub fn config_get() -> Result<AppConfig, String> {
    with_config(|config| config.clone())
}

/// Field-scoped settings payload. Only these three fields are user-editable
/// from the UI; packs/projects are never overwritten, eliminating the
/// "stale frontend overwrites the whole config" footgun.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SettingsUpdate {
    pub packs_dir: String,
    pub codex_config_path: Option<String>,
    pub default_targets: Vec<String>,
}

/// Update only the user-facing settings fields, preserving packs/projects.
#[tauri::command]
pub fn config_update_settings(settings: SettingsUpdate) -> Result<(), String> {
    with_config_mut(|config| {
        config.packs_dir = settings.packs_dir;
        config.codex_config_path = settings.codex_config_path;
        config.default_targets = settings.default_targets;
        Ok(())
    })
}

/// Single source of truth for the app version (Cargo.toml). The frontend and
/// CLI read this instead of hardcoding a version string.
#[tauri::command]
pub fn app_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}
