//! Deprecated Codex config handler.
//!
//! All logic has migrated to [`crate::codex::config`], which adds:
//! - a dedicated lock separate from the SkillPack config lock (B1),
//! - rolling timestamped backups instead of a single overwritten `.bak` (B2),
//! - the `[features]plugins` master switch (B4),
//! - marketplace + top-level MCP server management.
//!
//! This module remains only as a back-compat shim so the `lib.rs` re-exports
//! and any external callers keep compiling. New code should use `codex::config`
//! directly.

#![allow(deprecated, dead_code)]

use serde::{Deserialize, Serialize};

/// Legacy plugin descriptor. Prefer [`crate::codex::config::PluginEntry`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginInfo {
    pub key: String,
    pub name: String,
    pub source: String,
    pub enabled: bool,
}

/// Get the default Codex `config.toml` path.
///
/// Deprecated: use [`crate::codex::config::default_config_path`].
#[deprecated(note = "use codex::config::default_config_path")]
pub fn get_codex_config_path() -> Option<std::path::PathBuf> {
    let path = crate::codex::config::default_config_path();
    path.exists().then_some(path)
}
