pub mod config;
pub mod git;
pub mod scanner;
pub mod symlink;
pub mod toml_handler;

// Re-export command functions for CLI usage

pub fn pack_install(
    source: String,
    name: String,
    skill_root: Option<String>,
) -> Result<config::PackInfo, String> {
    commands::pack::pack_install(source, name, skill_root)
}

pub fn pack_list() -> Result<Vec<(String, config::PackInfo)>, String> {
    commands::pack::pack_list()
}

pub fn pack_remove(name: String) -> Result<(), String> {
    commands::pack::pack_remove(name)
}

pub fn pack_update(name: Option<String>) -> Result<Vec<String>, String> {
    commands::pack::pack_update(name)
}

pub fn skill_link(
    project: String,
    skill_name: String,
    pack: String,
    target: String,
) -> Result<(), String> {
    commands::link::skill_link(project, skill_name, pack, target)
}

pub fn skill_unlink(project: String, skill_name: String) -> Result<(), String> {
    commands::link::skill_unlink(project, skill_name)
}

pub fn project_add(path: String) -> Result<commands::project::ProjectInfo, String> {
    commands::project::project_add(path)
}

pub fn project_remove(path: String) -> Result<(), String> {
    commands::project::project_remove(path)
}

pub fn project_list() -> Result<Vec<commands::project::ProjectInfo>, String> {
    commands::project::project_list()
}

pub fn plugin_list() -> Result<Vec<toml_handler::PluginInfo>, String> {
    commands::plugin::plugin_list().map(|plugins| {
        plugins
            .into_iter()
            .map(|p| toml_handler::PluginInfo {
                key: p.key,
                name: p.name,
                source: p.source,
                enabled: p.enabled,
                skill_count: None,
            })
            .collect()
    })
}

pub fn plugin_toggle(key: String, enabled: bool) -> Result<(), String> {
    commands::plugin::plugin_toggle(key, enabled)
}

mod commands;
