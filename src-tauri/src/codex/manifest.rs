//! Parse `.codex-plugin/plugin.json` manifests.
//!
//! The manifest schema is taken from real-world samples (Curated, Bundled and
//! Personal plugins) plus the official build docs. Every field except `name`
//! is optional, so partial manifests and forward-compatible additions
//! deserialize without error.

use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};

/// `author` block in a plugin manifest.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PluginAuthor {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
}

/// `interface` block — controls how the plugin is presented on install
/// surfaces and in the directory.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PluginInterface {
    #[serde(default, rename = "displayName", skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
    #[serde(default, rename = "shortDescription", skip_serializing_if = "Option::is_none")]
    pub short_description: Option<String>,
    #[serde(default, rename = "longDescription", skip_serializing_if = "Option::is_none")]
    pub long_description: Option<String>,
    #[serde(default, rename = "developerName", skip_serializing_if = "Option::is_none")]
    pub developer_name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub category: Option<String>,
    #[serde(default)]
    pub capabilities: Vec<String>,
    #[serde(default, rename = "websiteURL", skip_serializing_if = "Option::is_none")]
    pub website_url: Option<String>,
    #[serde(default, rename = "privacyPolicyURL", skip_serializing_if = "Option::is_none")]
    pub privacy_policy_url: Option<String>,
    #[serde(default, rename = "termsOfServiceURL", skip_serializing_if = "Option::is_none")]
    pub terms_of_service_url: Option<String>,
}

/// The full `.codex-plugin/plugin.json` document.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PluginManifest {
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default)]
    pub author: Option<PluginAuthor>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub homepage: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub repository: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub license: Option<String>,
    #[serde(default)]
    pub keywords: Vec<String>,
    /// Relative path to bundled skills, e.g. `"./skills/"`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub skills: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mcp_servers: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub apps: Option<String>,
    #[serde(default)]
    pub interface: Option<PluginInterface>,
}

impl PluginManifest {
    /// Description, preferring the longer `interface.long_description`, then
    /// `interface.short_description`, then the top-level `description`.
    pub fn display_description(&self) -> Option<String> {
        self.interface
            .as_ref()
            .and_then(|i| i.long_description.clone())
            .or_else(|| self.interface.as_ref().and_then(|i| i.short_description.clone()))
            .or_else(|| self.description.clone())
    }

    pub fn display_category(&self) -> Option<String> {
        self.interface.as_ref().and_then(|i| i.category.clone())
    }

    pub fn display_author_name(&self) -> Option<String> {
        self.author
            .as_ref()
            .and_then(|a| a.name.clone())
            .or_else(|| self.interface.as_ref().and_then(|i| i.developer_name.clone()))
    }

    pub fn display_capabilities(&self) -> Vec<String> {
        self.interface
            .as_ref()
            .map(|i| i.capabilities.clone())
            .unwrap_or_default()
    }

    /// Scan the bundled skills directory for subdirectories named like skills.
    /// `installed_path` is the plugin root directory (the version dir).
    pub fn bundled_skill_names(&self, installed_path: &str) -> Vec<String> {
        let Some(skills_rel) = &self.skills else {
            return Vec::new();
        };
        let rel = skills_rel.trim_start_matches("./").trim_end_matches('/');
        let dir = Path::new(installed_path).join(rel);
        let Ok(entries) = fs::read_dir(&dir) else {
            return Vec::new();
        };
        let mut names = Vec::new();
        for e in entries.flatten() {
            let path = e.path();
            if path.is_dir() && path.join("SKILL.md").exists() {
                if let Some(name) = path.file_name().and_then(|s| s.to_str()) {
                    names.push(name.to_string());
                }
            }
        }
        names.sort();
        names
    }
}

/// Read and parse a `plugin.json` from a path.
#[allow(dead_code)]
pub fn read_manifest(path: &Path) -> Result<PluginManifest, String> {
    let content = fs::read_to_string(path)
        .map_err(|e| format!("Failed to read {}: {}", path.display(), e))?;
    serde_json::from_str(&content).map_err(|e| format!("Failed to parse {}: {}", path.display(), e))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_minimal_manifest() {
        let m: PluginManifest = serde_json::from_str(
            r#"{"name":"my-plugin","version":"1.0.0","description":"hi"}"#,
        )
        .unwrap();
        assert_eq!(m.name, "my-plugin");
        assert_eq!(m.version.as_deref(), Some("1.0.0"));
    }

    #[test]
    fn parses_rich_manifest_with_interface() {
        let raw = r#"{
            "name": "github",
            "version": "0.1.3",
            "author": {"name": "OpenAI"},
            "skills": "./skills/",
            "interface": {
                "displayName": "GitHub",
                "shortDescription": "GitHub integration",
                "longDescription": "Full GitHub automation",
                "category": "Developer",
                "capabilities": ["Read", "Write"]
            }
        }"#;
        let m: PluginManifest = serde_json::from_str(raw).unwrap();
        assert_eq!(m.display_description().as_deref(), Some("Full GitHub automation"));
        assert_eq!(m.display_category().as_deref(), Some("Developer"));
        assert_eq!(m.display_author_name().as_deref(), Some("OpenAI"));
        assert_eq!(m.display_capabilities(), vec!["Read", "Write"]);
    }

    #[test]
    fn display_description_falls_back_to_short_then_top_level() {
        let mut m = PluginManifest::default();
        m.name = "x".into();
        m.interface = Some(PluginInterface {
            short_description: Some("short".into()),
            ..Default::default()
        });
        assert_eq!(m.display_description().as_deref(), Some("short"));

        let mut m2 = PluginManifest::default();
        m2.name = "x".into();
        m2.description = Some("top".into());
        assert_eq!(m2.display_description().as_deref(), Some("top"));
    }
}
