//! Tauri commands for Codex plugin + marketplace management.
//!
//! Reads (list/status) parse `config.toml` and the cache directly for speed
//! and rich metadata. Writes that mutate the cache (add/remove) or the
//! marketplaces go through the `codex` CLI so on-disk state stays consistent
//! with Codex itself. Toggles edit `config.toml` under a dedicated lock with
//! rolling backups.

use crate::codex::{self, config as codex_config};
use crate::error::SkillError;

pub use codex_config::{MarketplaceEntry, PluginEntry};

/// Snapshot of the master switch + installed plugins, returned together so the
/// UI can render the features banner and the plugin list in one call.
#[derive(serde::Serialize, serde::Deserialize)]
pub struct PluginOverview {
    pub features_plugins_enabled: bool,
    pub plugins: Vec<PluginEntry>,
}

/// Resolve the Codex config path (settings override → default discovery),
/// surfacing a friendly error string when Codex is not installed.
fn require_config_path() -> Result<std::path::PathBuf, String> {
    codex_config::resolve_config_path().map_err(|e: SkillError| e.to_string())
}

#[tauri::command]
pub fn plugin_list() -> Result<Vec<PluginEntry>, String> {
    codex_config::with_codex_config(|doc| codex_config::list_plugins(doc)).map_err(|e| e.to_string())
}

/// Combined read: master switch + plugin list. One round-trip for the UI.
#[tauri::command]
pub fn plugin_overview() -> Result<PluginOverview, String> {
    codex_config::with_codex_config(|doc| PluginOverview {
        features_plugins_enabled: codex_config::features_plugins_enabled(doc),
        plugins: codex_config::list_plugins(doc),
    })
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn plugin_toggle(key: String, enabled: bool) -> Result<(), String> {
    codex_config::with_codex_config_mut(|doc| {
        codex_config::set_plugin_enabled(doc, &key, enabled)
    })
    .map_err(|e: SkillError| e.to_string())
}

/// Master switch status for `[features]plugins`.
#[tauri::command]
pub fn features_plugins_status() -> Result<bool, String> {
    codex_config::with_codex_config(codex_config::features_plugins_enabled).map_err(|e| e.to_string())
}

/// Toggle `[features]plugins`.
#[tauri::command]
pub fn features_plugins_toggle(enabled: bool) -> Result<(), String> {
    codex_config::with_codex_config_mut(|doc| {
        codex_config::set_features_plugins(doc, enabled)
    })
    .map_err(|e: SkillError| e.to_string())
}

/// Install a plugin via the `codex` CLI.
#[tauri::command]
pub fn plugin_add(name: String, marketplace: Option<String>) -> Result<String, String> {
    let _path = require_config_path()?;
    codex::cli::plugin_add(&name, marketplace.as_deref())
        .map(|o| o.stdout)
        .map_err(|e: SkillError| e.to_string())
}

/// Remove a plugin via the `codex` CLI.
#[tauri::command]
pub fn plugin_remove(key: String) -> Result<String, String> {
    let _path = require_config_path()?;
    codex::cli::plugin_remove(&key)
        .map(|o| o.stdout)
        .map_err(|e: SkillError| e.to_string())
}

/// Read configured marketplaces from `[marketplaces.*]`.
#[tauri::command]
pub fn marketplace_list() -> Result<Vec<MarketplaceEntry>, String> {
    codex_config::with_codex_config(codex_config::list_marketplaces).map_err(|e| e.to_string())
}

/// Add a marketplace via the `codex` CLI.
#[tauri::command]
pub fn marketplace_add(source: String) -> Result<String, String> {
    let _path = require_config_path()?;
    codex::cli::marketplace_add(&source)
        .map(|o| o.stdout)
        .map_err(|e: SkillError| e.to_string())
}

/// Upgrade one or all marketplaces via the `codex` CLI.
#[tauri::command]
pub fn marketplace_upgrade(name: Option<String>) -> Result<String, String> {
    let _path = require_config_path()?;
    codex::cli::marketplace_upgrade(name.as_deref())
        .map(|o| o.stdout)
        .map_err(|e: SkillError| e.to_string())
}

/// Remove a marketplace via the `codex` CLI.
#[tauri::command]
pub fn marketplace_remove(name: String) -> Result<String, String> {
    let _path = require_config_path()?;
    codex::cli::marketplace_remove(&name)
        .map(|o| o.stdout)
        .map_err(|e: SkillError| e.to_string())
}
