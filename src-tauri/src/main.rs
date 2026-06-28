#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;
mod codex;
mod config;
mod error;
mod git;
mod scanner;
mod symlink;
mod toml_handler;

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            commands::pack::pack_install,
            commands::pack::pack_install_local,
            commands::pack::pack_list,
            commands::pack::pack_open,
            commands::pack::pack_remove,
            commands::pack::pack_update,
            commands::link::skill_link,
            commands::link::skill_unlink,
            commands::link::skill_status,
            commands::project::project_add,
            commands::project::project_remove,
            commands::project::project_list,
            commands::plugin::plugin_list,
            commands::plugin::plugin_overview,
            commands::plugin::plugin_toggle,
            commands::plugin::plugin_add,
            commands::plugin::plugin_remove,
            commands::plugin::features_plugins_status,
            commands::plugin::features_plugins_toggle,
            commands::plugin::marketplace_list,
            commands::plugin::marketplace_add,
            commands::plugin::marketplace_upgrade,
            commands::plugin::marketplace_remove,
            commands::mcp::mcp_list,
            commands::mcp::mcp_toggle,
            commands::mcp::mcp_remove,
            commands::mcp::mcp_add,
            commands::skills::toolchain_skill_roots,
            commands::skills::toolchain_skills,
            commands::skills::project_skill_roots,
            commands::skills::project_skills,
            commands::skills::open_path,
            commands::config_cmd::config_get,
            commands::config_cmd::config_update_settings,
            commands::config_cmd::app_version,
            commands::featured::featured_list,
            commands::featured::featured_refresh,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
