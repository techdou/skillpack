//! Codex extension management.
//!
//! Codex exposes three layers of "extensions", all rooted at `~/.codex/`:
//!
//! - **CLI layer** (authoritative for mutations): `codex plugin add/list/remove`
//!   and `codex plugin marketplace add/list/upgrade/remove`. We shell out to
//!   these for install/uninstall/update so the cache directory and config stay
//!   consistent with what Codex itself would produce.
//! - **config.toml layer**: `[features]plugins` (master switch),
//!   `[marketplaces.*]`, `[plugins."name@marketplace"]`,
//!   `[plugins."n@m".mcp_servers.*]` (plugin-scoped MCP), and the top-level
//!   `[mcp_servers.*]`. We edit this directly for toggles and metadata reads.
//! - **cache layer**: `~/.codex/plugins/cache/$MARKETPLACE/$PLUGIN/$VERSION/`
//!   with a `.codex-plugin/plugin.json` manifest carrying rich metadata we
//!   surface in the UI.
//!
//! This module groups those concerns into focused submodules instead of one
//! overloaded `toml_handler.rs`.

pub mod cli;
pub mod config;
pub mod manifest;
pub mod marketplace;
