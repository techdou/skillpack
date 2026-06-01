use crate::{config::AppConfig, toml_handler};

#[derive(serde::Serialize, serde::Deserialize)]
pub struct PluginEntry {
    pub key: String,
    pub name: String,
    pub source: String,
    pub enabled: bool,
}

#[tauri::command]
pub fn plugin_list() -> Result<Vec<PluginEntry>, String> {
    let config = AppConfig::load()?;
    let config_path = config
        .codex_config_path()
        .or_else(toml_handler::get_codex_config_path)
        .ok_or("Codex config.toml not found")?;

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
    let config = AppConfig::load()?;
    let config_path = config
        .codex_config_path()
        .or_else(toml_handler::get_codex_config_path)
        .ok_or("Codex config.toml not found")?;

    toml_handler::toggle_plugin(&config_path, &key, enabled)?;

    Ok(())
}
