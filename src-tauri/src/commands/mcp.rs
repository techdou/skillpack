//! Tauri commands for top-level `[mcp_servers.*]` management.
//!
//! These manage the standalone MCP servers in `~/.codex/config.toml` (e.g.
//! `context7`, `blender`, `mineru`), distinct from plugin-scoped MCP servers
//! which live under `[plugins."n@m".mcp_servers.*]`.

use crate::codex::config as codex_config;
use crate::error::SkillError;

pub use codex_config::{McpAdd, McpServerEntry};

#[tauri::command]
pub fn mcp_list() -> Result<Vec<McpServerEntry>, String> {
    codex_config::with_codex_config(codex_config::list_mcp_servers).map_err(|e| e.to_string())
}

/// Toggle the SkillPack-private `enabled` marker on an MCP server. Codex
/// ignores the unknown field, but our UI honours it.
#[tauri::command]
pub fn mcp_toggle(name: String, enabled: bool) -> Result<(), String> {
    codex_config::with_codex_config_mut(|doc| codex_config::set_mcp_enabled(doc, &name, enabled))
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn mcp_remove(name: String) -> Result<(), String> {
    codex_config::with_codex_config_mut(|doc| codex_config::remove_mcp(doc, &name))
        .map_err(|e: SkillError| e.to_string())
}

#[tauri::command]
pub fn mcp_add(entry: McpAdd) -> Result<(), String> {
    codex_config::with_codex_config_mut(|doc| codex_config::add_mcp(doc, &entry))
        .map_err(|e: SkillError| e.to_string())
}
