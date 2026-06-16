pub mod commands;
pub mod config;
pub mod error;
pub mod git;
pub mod scanner;
pub mod symlink;
pub mod toml_handler;

// Re-export command functions for CLI usage. These thin wrappers let the
// `spack` CLI call into the exact same logic the Tauri GUI uses, without
// pulling in the `#[tauri::command]` macro machinery.

pub fn pack_install(
    source: String,
    name: String,
    skill_root: Option<String>,
) -> Result<config::PackInfo, String> {
    commands::pack::pack_install(source, name, skill_root)
}

pub fn pack_install_local(
    source_dir: String,
    name: String,
    skill_root: Option<String>,
) -> Result<config::PackInfo, String> {
    commands::pack::pack_install_local(source_dir, name, skill_root)
}

pub fn pack_list() -> Result<Vec<(String, config::PackInfo)>, String> {
    commands::pack::pack_list()
}

pub fn pack_open(name: String) -> Result<(), String> {
    commands::pack::pack_open(name)
}

pub fn pack_remove(name: String) -> Result<(), String> {
    commands::pack::pack_remove(name)
}

pub fn pack_update(name: Option<String>) -> Result<commands::pack::UpdateReport, String> {
    commands::pack::pack_update(name)
}

pub fn skill_link(
    project: String,
    skill_name: String,
    pack: String,
    target: String,
) -> Result<String, String> {
    commands::link::skill_link(project, skill_name, pack, target)
}

pub fn skill_unlink(project: String, skill_name: String) -> Result<(), String> {
    commands::link::skill_unlink(project, skill_name)
}

pub fn skill_status(project: String) -> Result<Vec<commands::link::SkillLinkInfo>, String> {
    commands::link::skill_status(project)
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

pub fn plugin_list() -> Result<Vec<commands::plugin::PluginEntry>, String> {
    commands::plugin::plugin_list()
}

pub fn plugin_toggle(key: String, enabled: bool) -> Result<(), String> {
    commands::plugin::plugin_toggle(key, enabled)
}

pub fn toolchain_skill_roots() -> Result<Vec<commands::skills::SkillRootInfo>, String> {
    commands::skills::toolchain_skill_roots()
}

pub fn toolchain_skills(toolchain: String) -> Result<Vec<commands::skills::SkillEntry>, String> {
    commands::skills::toolchain_skills(toolchain)
}

pub fn project_skill_roots(
    project: String,
) -> Result<Vec<commands::skills::SkillRootInfo>, String> {
    commands::skills::project_skill_roots(project)
}

pub fn project_skills(
    project: String,
    toolchain: String,
) -> Result<Vec<commands::skills::SkillEntry>, String> {
    commands::skills::project_skills(project, toolchain)
}

pub fn open_path(path: String) -> Result<(), String> {
    commands::skills::open_path(path)
}

pub fn app_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}
