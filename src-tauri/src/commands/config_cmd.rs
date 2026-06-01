use crate::config::AppConfig;

#[tauri::command]
#[allow(dead_code)]
pub fn config_get() -> Result<AppConfig, String> {
    AppConfig::load()
}

#[tauri::command]
#[allow(dead_code)]
pub fn config_set(config: AppConfig) -> Result<(), String> {
    config.save()
}
