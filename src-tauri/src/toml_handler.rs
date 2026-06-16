use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use toml_edit::DocumentMut;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginInfo {
    pub key: String,    // e.g. "vercel@openai-curated"
    pub name: String,   // e.g. "vercel"
    pub source: String, // e.g. "openai-curated"
    pub enabled: bool,
}

/// Get the default Codex config.toml path
pub fn get_codex_config_path() -> Option<PathBuf> {
    let home = dirs::home_dir()?;
    let path = home.join(".codex").join("config.toml");
    if path.exists() {
        Some(path)
    } else {
        None
    }
}

/// List all plugins from config.toml
pub fn list_plugins(config_path: &PathBuf) -> Result<Vec<PluginInfo>, String> {
    let content = fs::read_to_string(config_path).map_err(|e| e.to_string())?;
    let doc: DocumentMut = content.parse::<DocumentMut>().map_err(|e| e.to_string())?;

    let mut plugins = Vec::new();

    // Find [plugins."xxx@yyy"] sections
    if let Some(plugins_table) = doc.get("plugins") {
        if let Some(plugins_table) = plugins_table.as_table() {
            for (key, value) in plugins_table.iter() {
                if let Some(table) = value.as_table() {
                    let enabled = table
                        .get("enabled")
                        .and_then(|v| v.as_bool())
                        .unwrap_or(false);

                    // Parse "name@source" format
                    let parts: Vec<&str> = key.split('@').collect();
                    let name = parts
                        .first()
                        .map(|s| s.to_string())
                        .unwrap_or_else(|| key.to_string());
                    let source = parts.get(1).map(|s| s.to_string()).unwrap_or_default();

                    plugins.push(PluginInfo {
                        key: key.to_string(),
                        name,
                        source,
                        enabled,
                    });
                }
            }
        }
    }

    Ok(plugins)
}

/// Toggle a plugin's enabled state in config.toml
pub fn toggle_plugin(config_path: &PathBuf, plugin_key: &str, enabled: bool) -> Result<(), String> {
    // Backup first
    let backup_path = config_path.with_extension("toml.bak");
    fs::copy(config_path, &backup_path).map_err(|e| format!("Backup failed: {}", e))?;

    let content = fs::read_to_string(config_path).map_err(|e| e.to_string())?;
    let mut doc: DocumentMut = content.parse::<DocumentMut>().map_err(|e| e.to_string())?;

    let plugins_table = doc
        .get_mut("plugins")
        .and_then(|plugins| plugins.as_table_mut())
        .ok_or("Plugins table not found in config")?;

    let plugin_entry = plugins_table
        .get_mut(plugin_key)
        .ok_or_else(|| format!("Plugin {} not found in config", plugin_key))?;

    let table = plugin_entry
        .as_table_mut()
        .ok_or_else(|| format!("Plugin {} is not a table", plugin_key))?;

    table["enabled"] = toml_edit::value(enabled);

    fs::write(config_path, doc.to_string()).map_err(|e| e.to_string())?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_file(name: &str) -> PathBuf {
        let unique = format!(
            "{}-{}",
            name,
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        );
        std::env::temp_dir().join(unique).with_extension("toml")
    }

    #[test]
    fn toggle_plugin_updates_nested_plugin_table() {
        let config_path = temp_file("skillpack-codex-config");
        fs::write(
            &config_path,
            r#"
[plugins."vercel@openai-curated"]
enabled = false
"#,
        )
        .unwrap();

        toggle_plugin(&config_path, "vercel@openai-curated", true).unwrap();

        let plugins = list_plugins(&config_path).unwrap();
        let plugin = plugins
            .iter()
            .find(|plugin| plugin.key == "vercel@openai-curated")
            .unwrap();
        assert!(plugin.enabled);

        let _ = fs::remove_file(&config_path);
        let _ = fs::remove_file(config_path.with_extension("toml.bak"));
    }
}
