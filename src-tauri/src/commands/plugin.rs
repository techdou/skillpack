use crate::{config::with_config, toml_handler};

#[derive(serde::Serialize, serde::Deserialize)]
pub struct PluginEntry {
    pub key: String,
    pub name: String,
    pub source: String,
    pub enabled: bool,
}

/// Resolve the Codex config.toml path from the saved config (falling back to
/// the default discovery) under the global config lock.
fn resolve_codex_config() -> Result<std::path::PathBuf, String> {
    with_config(|config| {
        config
            .codex_config_path()
            .or_else(toml_handler::get_codex_config_path)
            .ok_or_else(|| "Codex config.toml not found".to_string())
    })?
}

#[tauri::command]
pub fn plugin_list() -> Result<Vec<PluginEntry>, String> {
    let config_path = resolve_codex_config()?;
    let plugins = toml_handler::list_plugins(&config_path)?;

    Ok(plugins
        .into_iter()
        .map(|p| PluginEntry {
            key: p.key,
            name: p.name,
            source: p.source,
            enabled: p.enabled,
        })
        .collect())
}

#[tauri::command]
pub fn plugin_toggle(key: String, enabled: bool) -> Result<(), String> {
    let config_path = resolve_codex_config()?;
    toml_handler::toggle_plugin(&config_path, &key, enabled)?;
    Ok(())
}
