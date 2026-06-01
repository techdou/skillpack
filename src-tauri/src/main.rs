#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;
mod config;
mod git;
mod scanner;
mod symlink;
mod toml_handler;

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .invoke_handler(tauri::generate_handler![
            commands::pack::pack_install,
            commands::pack::pack_list,
            commands::pack::pack_remove,
            commands::pack::pack_update,
            commands::link::skill_link,
            commands::link::skill_unlink,
            commands::link::skill_status,
            commands::project::project_add,
            commands::project::project_remove,
            commands::project::project_list,
            commands::plugin::plugin_list,
            commands::plugin::plugin_toggle,
            commands::config_cmd::config_get,
            commands::config_cmd::config_set,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
