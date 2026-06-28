//! Parse `marketplace.json` catalogs.
//!
//! Marketplaces live in several places depending on scope:
//! - personal: `~/.agents/plugins/marketplace.json`
//! - repo: `$REPO_ROOT/.agents/plugins/marketplace.json`
//! - legacy-compatible: `$REPO_ROOT/.claude-plugin/marketplace.json`
//! - curated snapshot: `~/.codex/.tmp/plugins/.agents/plugins/marketplace.json`
//!
//! Each entry points its `source.path` at a plugin directory relative to the
//! marketplace root. We parse but do not mutate these files — install/remove
//! always go through the `codex` CLI.
//!
//! The parsing API here is currently used by tests and reserved for a future
//! marketplace browser; it is intentionally public so the UI can adopt it
//! without a refactor.

#![allow(dead_code)]

use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// A plugin source inside a marketplace entry. `source` may be `"local"`,
/// `"url"`, or `"git-subdir"`; for the common `local` case the path is a
/// `./`-prefixed relative path.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PluginSource {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub r#ref: Option<String>,
}

/// A single plugin listing inside a marketplace.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketplacePlugin {
    pub name: String,
    #[serde(default)]
    pub source: PluginSource,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub category: Option<String>,
}

/// A `marketplace.json` catalog.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketplaceCatalog {
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub interface: Option<MarketplaceInterface>,
    #[serde(default)]
    pub plugins: Vec<MarketplacePlugin>,
}

/// `interface` block inside a marketplace catalog.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MarketplaceInterface {
    #[serde(default, rename = "displayName", skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
}

impl MarketplaceCatalog {
    /// Display name, falling back to the marketplace `name`.
    pub fn display_name(&self) -> &str {
        self.interface
            .as_ref()
            .and_then(|i| i.display_name.as_deref())
            .unwrap_or(&self.name)
    }
}

/// Known marketplace.json locations relative to the home directory.
///
/// Returns paths that may or may not exist; callers filter on existence.
pub fn candidate_paths(home: &Path) -> Vec<PathBuf> {
    vec![
        home.join(".agents").join("plugins").join("marketplace.json"),
        // Curated snapshot mirror checked into the Codex managed dir.
        home.join(".codex")
            .join(".tmp")
            .join("plugins")
            .join(".agents")
            .join("plugins")
            .join("marketplace.json"),
    ]
}

/// Read the first existing personal/snapshot marketplace.json, if any.
pub fn read_personal_catalog() -> Option<MarketplaceCatalog> {
    let home = dirs::home_dir()?;
    for p in candidate_paths(&home) {
        if let Ok(content) = fs::read_to_string(&p) {
            if let Ok(cat) = serde_json::from_str::<MarketplaceCatalog>(&content) {
                return Some(cat);
            }
        }
    }
    None
}

/// Parse a marketplace.json from an explicit path (used for arbitrary
/// marketplace roots, e.g. a repo checkout).
pub fn read_catalog(path: &Path) -> Result<MarketplaceCatalog, String> {
    let content = fs::read_to_string(path)
        .map_err(|e| format!("Failed to read {}: {}", path.display(), e))?;
    serde_json::from_str(&content).map_err(|e| format!("Failed to parse marketplace.json: {}", e))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_local_marketplace() {
        let raw = r#"{
            "name": "local-example",
            "interface": {"displayName": "Local Example"},
            "plugins": [
                {
                    "name": "my-plugin",
                    "source": {"source": "local", "path": "./plugins/my-plugin"},
                    "category": "Productivity"
                }
            ]
        }"#;
        let cat: MarketplaceCatalog = serde_json::from_str(raw).unwrap();
        assert_eq!(cat.name, "local-example");
        assert_eq!(cat.display_name(), "Local Example");
        assert_eq!(cat.plugins.len(), 1);
        assert_eq!(cat.plugins[0].name, "my-plugin");
        assert_eq!(cat.plugins[0].source.path.as_deref(), Some("./plugins/my-plugin"));
    }

    #[test]
    fn parses_git_subdir_source() {
        let raw = r#"{
            "name": "remote",
            "plugins": [{
                "name": "r",
                "source": {
                    "source": "git-subdir",
                    "url": "https://github.com/example/plugins.git",
                    "path": "./plugins/r",
                    "ref": "main"
                }
            }]
        }"#;
        let cat: MarketplaceCatalog = serde_json::from_str(raw).unwrap();
        assert_eq!(cat.plugins[0].source.source.as_deref(), Some("git-subdir"));
        assert_eq!(cat.plugins[0].source.r#ref.as_deref(), Some("main"));
    }
}
