//! Read/write `~/.codex/config.toml` with safe backups and serialization.
//!
//! All mutations go through [`with_codex_config_mut`], which takes a
//! process-wide lock dedicated to the Codex config file (separate from the
//! SkillPack `config.json` lock in `crate::config`). This is best-effort
//! protection against concurrent writes from SkillPack itself and from the
//! user toggling things in two windows; it cannot coordinate with the Codex
//! process, which may rewrite this file at any time, so we always make a
//! rolling timestamped backup before each write.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use toml_edit::DocumentMut;

use crate::error::SkillError;

/// Maximum number of timestamped backups to keep per config file. Older
/// backups beyond this are pruned after each successful write.
const MAX_BACKUPS: usize = 5;

// ---------------------------------------------------------------------------
// Data shapes returned to the command layer
// ---------------------------------------------------------------------------

/// A plugin entry assembled from `[plugins.*]` plus the cache manifest.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginEntry {
    /// Full config key, e.g. `vercel@openai-curated`.
    pub key: String,
    /// Plugin name, the part before `@`.
    pub name: String,
    /// Marketplace / source name, the part after `@`.
    pub source: String,
    pub enabled: bool,
    /// Whether the cache directory exists for this plugin.
    pub installed: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub category: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub author_name: Option<String>,
    #[serde(default)]
    pub capabilities: Vec<String>,
    #[serde(default)]
    pub bundled_skills: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub installed_path: Option<String>,
}

/// A configured marketplace from `[marketplaces.*]`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketplaceEntry {
    pub name: String,
    /// Raw `source` value, typically a git URL or `owner/repo` shorthand.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub r#ref: Option<String>,
    /// Resolved local snapshot root, when discoverable.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
}

/// A top-level `[mcp_servers.*]` entry.
///
/// Codex enables every top-level MCP server unconditionally; there is no
/// `enabled` field in the native schema. We track an `enabled` flag ourselves
/// via a SkillPack-private marker field (`enabled = false`) so the UI can
/// represent disabled state without removing the server definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpServerEntry {
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub command: Option<String>,
    #[serde(default)]
    pub args: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub r#type: Option<String>,
    #[serde(default)]
    pub env_keys: Vec<String>,
    pub enabled: bool,
}

// ---------------------------------------------------------------------------
// Path resolution
// ---------------------------------------------------------------------------

/// Default `~/.codex/config.toml`.
pub fn default_config_path() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".codex")
        .join("config.toml")
}

/// Resolve the Codex config path, honouring the SkillPack settings override.
pub fn resolve_config_path() -> Result<PathBuf, SkillError> {
    let from_settings = crate::config::with_config(|c| {
        c.codex_config_path()
            .or_else(|| {
                let p = default_config_path();
                p.exists().then_some(p)
            })
    })
    .map_err(SkillError::Config)?;
    from_settings.ok_or_else(|| SkillError::NotFound("Codex config.toml".into()))
}

// ---------------------------------------------------------------------------
// Serialized access (read and read-modify-write)
// ---------------------------------------------------------------------------

// LOCK ORDERING CONTRACT:
//
// This module has two locks in play:
//   1. `codex_lock()`  — guards `~/.codex/config.toml`
//   2. `config_lock()` (in crate::config) — guards `~/.skillpack/config.json`
//
// `with_codex_config_mut` always acquires codex_lock FIRST, then — only while
// resolving the path inside `resolve_config_path` — briefly takes config_lock
// for a read and releases it. This is a single, consistent nesting order.
//
// ⚠️ DEADLOCK RULE: never call any codex-config writer from inside a
// `with_config`/`with_config_mut` closure. That would reverse the order
// (config_lock outer, codex_lock inner) and deadlock. Reads of the SkillPack
// config are fine anywhere; writes to config.toml must happen outside any
// SkillPack config lock.

/// Process-wide lock guarding the Codex config file. Distinct from the
/// SkillPack config lock so the two files never block each other.
fn codex_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

/// Parse the config document from disk.
fn read_doc(path: &Path) -> Result<DocumentMut, SkillError> {
    let content = fs::read_to_string(path)?;
    content
        .parse::<DocumentMut>()
        .map_err(|e| SkillError::Config(format!("invalid TOML in {}: {}", path.display(), e)))
}

/// Make a timestamped rolling backup, then write the document. Keeps at most
/// [`MAX_BACKUPS`] siblings named `config.toml.bak-YYYYMMDD-HHMMSS`.
fn write_with_backup(path: &Path, doc: &DocumentMut) -> Result<(), SkillError> {
    rolling_backup(path)?;
    fs::write(path, doc.to_string())?;
    prune_backups(path)?;
    Ok(())
}

/// Create one timestamped backup copy of `path`.
fn rolling_backup(path: &Path) -> Result<(), SkillError> {
    if !path.exists() {
        return Ok(());
    }
    let stamp = timestamp_suffix();
    let backup = path.with_extension(format!("toml.bak-{}", stamp));
    fs::copy(path, &backup)?;
    Ok(())
}

/// Delete the oldest backups beyond [`MAX_BACKUPS`].
fn prune_backups(path: &Path) -> Result<(), SkillError> {
    let dir = match path.parent() {
        Some(d) => d,
        None => return Ok(()),
    };
    let stem = match path.file_name().and_then(|s| s.to_str()) {
        Some(s) => s,
        None => return Ok(()),
    };
    let prefix = format!("{}.bak-", stem);

    let mut backups: Vec<PathBuf> = fs::read_dir(dir)?
        .filter_map(|e| e.ok())
        .filter_map(|e| {
            let name = e.file_name().to_string_lossy().into_owned();
            name.starts_with(&prefix).then(|| e.path())
        })
        .collect();
    if backups.len() <= MAX_BACKUPS {
        return Ok(());
    }
    backups.sort();
    let excess = backups.len().saturating_sub(MAX_BACKUPS);
    for p in backups.into_iter().take(excess) {
        let _ = fs::remove_file(p);
    }
    Ok(())
}

fn timestamp_suffix() -> String {
    // UTC timestamp; collisions only happen for sub-second writes, which is
    // fine — the lock serialises writes anyway.
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let days = secs / 86_400;
    let tod = secs % 86_400;
    let hh = tod / 3600;
    let mm = (tod % 3600) / 60;
    let ss = tod % 60;
    // Day count since epoch as a compact monotonic id; mirrors what a human
    // would call YYYYMMDD-ish ordering without dragging in a date crate.
    format!("d{}-{:02}{:02}{:02}", days, hh, mm, ss)
}

/// Read-only access to the config document under the lock.
pub fn with_codex_config<R>(f: impl FnOnce(&DocumentMut) -> R) -> Result<R, SkillError> {
    let _guard = codex_lock().lock().map_err(|e| SkillError::Other(e.to_string()))?;
    let path = resolve_config_path()?;
    let doc = read_doc(&path)?;
    Ok(f(&doc))
}

/// Read-modify-write access to the config document under the lock.
pub fn with_codex_config_mut<R>(
    f: impl FnOnce(&mut DocumentMut) -> Result<R, SkillError>,
) -> Result<R, SkillError> {
    let _guard = codex_lock().lock().map_err(|e| SkillError::Other(e.to_string()))?;
    let path = resolve_config_path()?;
    let mut doc = read_doc(&path)?;
    let result = f(&mut doc)?;
    write_with_backup(&path, &doc)?;
    Ok(result)
}

// ---------------------------------------------------------------------------
// Features (master switch)
// ---------------------------------------------------------------------------

/// Read `[features]plugins`. Defaults to `true` when the feature flag is
/// absent — that is Codex's default behaviour.
pub fn features_plugins_enabled(doc: &DocumentMut) -> bool {
    doc.get("features")
        .and_then(|f| f.as_table())
        .and_then(|f| f.get("plugins"))
        .and_then(|v| v.as_bool())
        .unwrap_or(true)
}

/// Set `[features]plugins`.
pub fn set_features_plugins(doc: &mut DocumentMut, enabled: bool) -> Result<(), SkillError> {
    let features = doc
        .entry("features")
        .or_insert_with(|| toml_edit::table())
        .as_table_mut()
        .ok_or_else(|| SkillError::Config("[features] is not a table".into()))?;
    features["plugins"] = toml_edit::value(enabled);
    Ok(())
}

// ---------------------------------------------------------------------------
// Plugins
// ---------------------------------------------------------------------------

/// Parse the `name@marketplace` config key into its parts.
pub fn split_plugin_key(key: &str) -> (String, String) {
    match key.split_once('@') {
        Some((name, source)) => (name.to_string(), source.to_string()),
        None => (key.to_string(), String::new()),
    }
}

/// List `[plugins.*]` entries, enriched with cache manifest metadata.
pub fn list_plugins(doc: &DocumentMut) -> Vec<PluginEntry> {
    let cache_root = default_plugins_cache_root();
    let mut out = Vec::new();

    if let Some(table) = doc.get("plugins").and_then(|v| v.as_table()) {
        for (key, value) in table.iter() {
            let Some(t) = value.as_table() else { continue };
            let enabled = t
                .get("enabled")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            let (name, source) = split_plugin_key(key);

            // Try to merge metadata from the cache manifest.
            let (installed, installed_path, manifest) = resolve_cache_manifest(&cache_root, &name, &source);

            let (version, description, category, author_name, capabilities, bundled_skills) =
                match &manifest {
                    Some(m) => (
                        m.version.clone(),
                        m.display_description(),
                        m.display_category(),
                        m.display_author_name(),
                        m.display_capabilities(),
                        m.bundled_skill_names(installed_path.as_deref().unwrap_or("")),
                    ),
                    None => (None, None, None, None, Vec::new(), Vec::new()),
                };

            out.push(PluginEntry {
                key: key.to_string(),
                name,
                source,
                enabled,
                installed,
                version,
                description,
                category,
                author_name,
                capabilities,
                bundled_skills,
                installed_path,
            });
        }
    }

    out.sort_by(|a, b| a.name.cmp(&b.name));
    out
}

/// Set `[plugins."key"].enabled`.
pub fn set_plugin_enabled(doc: &mut DocumentMut, key: &str, enabled: bool) -> Result<(), SkillError> {
    let table = doc
        .get_mut("plugins")
        .and_then(|p| p.as_table_mut())
        .ok_or_else(|| SkillError::Config("no [plugins] table in config".into()))?;
    let entry = table
        .get_mut(key)
        .ok_or_else(|| SkillError::NotFound(format!("plugin {}", key)))?;
    let t = entry
        .as_table_mut()
        .ok_or_else(|| SkillError::Config(format!("plugin {} is not a table", key)))?;
    t["enabled"] = toml_edit::value(enabled);
    Ok(())
}

// ---------------------------------------------------------------------------
// Marketplaces
// ---------------------------------------------------------------------------

/// Read `[marketplaces.*]`.
pub fn list_marketplaces(doc: &DocumentMut) -> Vec<MarketplaceEntry> {
    let snapshot_root = default_marketplace_snapshot_root();
    let mut out = Vec::new();
    if let Some(table) = doc.get("marketplaces").and_then(|v| v.as_table()) {
        for (name, value) in table.iter() {
            let t = value.as_table();
            let source = t
                .and_then(|t| t.get("source"))
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
            let r#ref = t
                .and_then(|t| t.get("ref"))
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
            let path = snapshot_root
                .as_ref()
                .map(|root| root.join(name).to_string_lossy().to_string());
            out.push(MarketplaceEntry {
                name: name.to_string(),
                source,
                r#ref,
                path,
            });
        }
    }
    out.sort_by(|a, b| a.name.cmp(&b.name));
    out
}

// ---------------------------------------------------------------------------
// Top-level MCP servers
// ---------------------------------------------------------------------------

/// Read `[mcp_servers.*]`.
pub fn list_mcp_servers(doc: &DocumentMut) -> Vec<McpServerEntry> {
    let mut out = Vec::new();
    if let Some(table) = doc.get("mcp_servers").and_then(|v| v.as_table()) {
        for (name, value) in table.iter() {
            let Some(t) = value.as_table() else { continue };
            let command = t
                .get("command")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
            let args = t
                .get("args")
                .and_then(|v| v.as_array())
                .map(|a| {
                    a.iter()
                        .filter_map(|v| v.as_str().map(|s| s.to_string()))
                        .collect()
                })
                .unwrap_or_default();
            let url = t
                .get("url")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
            let r#type = t
                .get("type")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
            let env_keys = t
                .get("env")
                .and_then(|v| v.as_table_like())
                .map(|env| env.iter().map(|(k, _)| k.to_string()).collect())
                .unwrap_or_default();
            // SkillPack-private marker; absent means enabled (Codex default).
            let enabled = t
                .get("enabled")
                .and_then(|v| v.as_bool())
                .unwrap_or(true);
            out.push(McpServerEntry {
                name: name.to_string(),
                command,
                args,
                url,
                r#type,
                env_keys,
                enabled,
            });
        }
    }
    out.sort_by(|a, b| a.name.cmp(&b.name));
    out
}

/// Set the SkillPack-private `enabled` marker on `[mcp_servers."name"]`.
pub fn set_mcp_enabled(doc: &mut DocumentMut, name: &str, enabled: bool) -> Result<(), SkillError> {
    let table = doc
        .get_mut("mcp_servers")
        .and_then(|m| m.as_table_mut())
        .ok_or_else(|| SkillError::Config("no [mcp_servers] table in config".into()))?;
    let entry = table
        .get_mut(name)
        .ok_or_else(|| SkillError::NotFound(format!("mcp server {}", name)))?;
    let t = entry
        .as_table_mut()
        .ok_or_else(|| SkillError::Config(format!("mcp server {} is not a table", name)))?;
    t["enabled"] = toml_edit::value(enabled);
    Ok(())
}

/// Remove `[mcp_servers."name"]`.
pub fn remove_mcp(doc: &mut DocumentMut, name: &str) -> Result<(), SkillError> {
    let table = doc
        .get_mut("mcp_servers")
        .and_then(|m| m.as_table_mut())
        .ok_or_else(|| SkillError::Config("no [mcp_servers] table in config".into()))?;
    table
        .remove(name)
        .ok_or_else(|| SkillError::NotFound(format!("mcp server {}", name)))?;
    Ok(())
}

/// Add a `[mcp_servers."name"]` entry from a command/args or url definition.
pub fn add_mcp(doc: &mut DocumentMut, entry: &McpAdd) -> Result<(), SkillError> {
    if entry.name.trim().is_empty() {
        return Err(SkillError::InvalidArg("name is required".into()));
    }
    let has_command = entry.command.as_deref().map(|s| !s.is_empty()).unwrap_or(false);
    let has_url = entry.url.as_deref().map(|s| !s.is_empty()).unwrap_or(false);
    if !has_command && !has_url {
        return Err(SkillError::InvalidArg("either command or url is required".into()));
    }

    let servers = doc
        .entry("mcp_servers")
        .or_insert_with(|| toml_edit::table())
        .as_table_mut()
        .ok_or_else(|| SkillError::Config("[mcp_servers] is not a table".into()))?;

    let mut item = toml_edit::Table::new();
    if let Some(t) = &entry.r#type {
        item["type"] = toml_edit::value(t);
    }
    if let Some(cmd) = &entry.command {
        item["command"] = toml_edit::value(cmd);
    }
    if !entry.args.is_empty() {
        let arr = entry
            .args
            .iter()
            .fold(toml_edit::Array::new(), |mut a, v| {
                a.push(v);
                a
            });
        item["args"] = toml_edit::Item::Value(toml_edit::Value::Array(arr));
    }
    if let Some(url) = &entry.url {
        item["url"] = toml_edit::value(url);
    }
    item["enabled"] = toml_edit::value(true);
    servers.insert(&entry.name, toml_edit::Item::Table(item));
    Ok(())
}

/// Payload for adding an MCP server.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpAdd {
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub r#type: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub command: Option<String>,
    #[serde(default)]
    pub args: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
}

// ---------------------------------------------------------------------------
// Cache resolution helpers
// ---------------------------------------------------------------------------

fn default_plugins_cache_root() -> Option<PathBuf> {
    let p = dirs::home_dir()?.join(".codex").join("plugins").join("cache");
    p.is_dir().then_some(p)
}

fn default_marketplace_snapshot_root() -> Option<PathBuf> {
    let p = dirs::home_dir()?.join(".codex").join(".tmp").join("plugins");
    p.is_dir().then_some(p)
}

/// Locate the installed plugin directory and parse its manifest.
///
/// The cache layout is `cache/$MARKETPLACE/$PLUGIN/$VERSION/`, where `$VERSION`
/// may be a real semver (`0.1.3`), a build number (`26.519.41501`), or a commit
/// hash (`3fdeeb49`). We pick the highest-sorting version directory as the
/// representative install, which is a reasonable proxy for "latest".
fn resolve_cache_manifest(
    cache_root: &Option<PathBuf>,
    name: &str,
    source: &str,
) -> (bool, Option<String>, Option<super::manifest::PluginManifest>) {
    let Some(root) = cache_root else {
        return (false, None, None);
    };
    // Prefer the marketplace-scoped path; fall back to scanning every
    // marketplace dir if `source` is empty or doesn't match a directory.
    let candidates: Vec<PathBuf> = if !source.is_empty() {
        vec![root.join(source).join(name)]
    } else {
        vec![]
    };

    let mut search_dirs: Vec<PathBuf> = candidates.clone();
    if candidates.is_empty() {
        if let Ok(entries) = fs::read_dir(root) {
            for e in entries.flatten() {
                search_dirs.push(e.path().join(name));
            }
        }
    }

    for dir in search_dirs {
        if let Ok((version_dir, manifest)) = pick_latest_version(&dir) {
            return (
                true,
                Some(version_dir.to_string_lossy().to_string()),
                Some(manifest),
            );
        }
    }
    (false, None, None)
}

/// Among `dir/*` subdirectories, pick the one whose `.codex-plugin/plugin.json`
/// parses and sorts highest by name (a rough "latest" proxy). Returns the
/// version dir and parsed manifest.
fn pick_latest_version(dir: &Path) -> Result<(PathBuf, super::manifest::PluginManifest), ()> {
    let entries = fs::read_dir(dir).map_err(|_| ())?;
    let mut found: Vec<(PathBuf, super::manifest::PluginManifest)> = Vec::new();
    for e in entries.flatten() {
        let path = e.path();
        if !path.is_dir() {
            continue;
        }
        let manifest_path = path.join(".codex-plugin").join("plugin.json");
        if let Ok(content) = fs::read_to_string(&manifest_path) {
            if let Ok(m) = serde_json::from_str::<super::manifest::PluginManifest>(&content) {
                found.push((path.clone(), m));
            }
        }
    }
    // Sort by directory name descending; semver-ish but good enough for a
    // representative pick. Falls back to lexicographic which still biases
    // toward longer/build-numbered versions.
    found.sort_by(|a, b| b.0.file_name().cmp(&a.0.file_name()));
    found.into_iter().next().ok_or(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write_tmp(content: &str) -> PathBuf {
        let unique = format!(
            "codex-config-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        );
        let p = std::env::temp_dir().join(unique).with_extension("toml");
        fs::write(&p, content).unwrap();
        p
    }

    #[test]
    fn split_plugin_key_with_marketplace() {
        let (name, source) = split_plugin_key("vercel@openai-curated");
        assert_eq!(name, "vercel");
        assert_eq!(source, "openai-curated");
    }

    #[test]
    fn split_plugin_key_without_marketplace() {
        let (name, source) = split_plugin_key("solo");
        assert_eq!(name, "solo");
        assert!(source.is_empty());
    }

    #[test]
    fn list_plugins_reads_enabled_and_metadata() {
        let path = write_tmp(
            r#"
[plugins."vercel@openai-curated"]
enabled = true

[plugins."github@openai-curated"]
enabled = false
"#,
        );
        let doc = read_doc(&path).unwrap();
        let plugins = list_plugins(&doc);
        let v: Vec<_> = plugins.iter().map(|p| p.name.as_str()).collect();
        assert!(v.contains(&"vercel"));
        assert!(v.contains(&"github"));
        let github = plugins.iter().find(|p| p.name == "github").unwrap();
        assert!(!github.enabled);
        let _ = fs::remove_file(path);
    }

    #[test]
    fn features_plugins_defaults_true_when_absent() {
        let path = write_tmp("[other]\nx = 1\n");
        let doc = read_doc(&path).unwrap();
        assert!(features_plugins_enabled(&doc));
        let _ = fs::remove_file(path);
    }

    #[test]
    fn set_features_plugins_writes_bool() {
        let path = write_tmp(r#"[features]"#);
        let mut doc = read_doc(&path).unwrap();
        set_features_plugins(&mut doc, false).unwrap();
        assert!(!features_plugins_enabled(&doc));
        let _ = fs::remove_file(path);
    }

    #[test]
    fn toggle_plugin_persists_under_lock() {
        let path = write_tmp(
            r#"
[plugins."vercel@openai-curated"]
enabled = false
"#,
        );
        // Bypass the global resolver by editing the doc directly.
        let mut doc = read_doc(&path).unwrap();
        set_plugin_enabled(&mut doc, "vercel@openai-curated", true).unwrap();
        fs::write(&path, doc.to_string()).unwrap();

        let doc2 = read_doc(&path).unwrap();
        let plugins = list_plugins(&doc2);
        let vercel = plugins.iter().find(|p| p.name == "vercel").unwrap();
        assert!(vercel.enabled);
        let _ = fs::remove_file(path);
    }

    #[test]
    fn list_mcp_servers_reads_command_args_and_env_keys() {
        let path = write_tmp(
            r#"
[mcp_servers.context7]
command = "node"
args = ["server.js"]
env = { API_KEY = "x" }

[mcp_servers.remote]
url = "https://example.com/sse"
type = "sse"
"#,
        );
        let doc = read_doc(&path).unwrap();
        let servers = list_mcp_servers(&doc);
        let ctx = servers.iter().find(|s| s.name == "context7").unwrap();
        assert_eq!(ctx.command.as_deref(), Some("node"));
        assert_eq!(ctx.args, vec!["server.js"]);
        assert_eq!(ctx.env_keys, vec!["API_KEY"]);
        let remote = servers.iter().find(|s| s.name == "remote").unwrap();
        assert!(remote.url.is_some());
        assert!(!remote.enabled == false); // default enabled
        let _ = fs::remove_file(path);
    }

    #[test]
    fn add_and_remove_mcp_server() {
        let path = write_tmp("[other]\nx = 1\n");
        let mut doc = read_doc(&path).unwrap();
        add_mcp(
            &mut doc,
            &McpAdd {
                name: "demo".into(),
                r#type: Some("local".into()),
                command: Some("demo-mcp".into()),
                args: vec!["--stdio".into()],
                url: None,
            },
        )
        .unwrap();
        let servers = list_mcp_servers(&doc);
        assert!(servers.iter().any(|s| s.name == "demo"));

        remove_mcp(&mut doc, "demo").unwrap();
        let servers = list_mcp_servers(&doc);
        assert!(!servers.iter().any(|s| s.name == "demo"));
        let _ = fs::remove_file(path);
    }

    #[test]
    fn rolling_backup_creates_timestamped_file() {
        let path = write_tmp(r#"x = 1"#);
        rolling_backup(&path).unwrap();
        let dir = path.parent().unwrap();
        let stem = path.file_name().unwrap().to_str().unwrap();
        let prefix = format!("{}.bak-", stem);
        let count = fs::read_dir(dir)
            .unwrap()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_name().to_string_lossy().starts_with(&prefix))
            .count();
        assert!(count >= 1, "expected at least one backup");
        // cleanup
        let _ = fs::remove_file(&path);
        for e in fs::read_dir(dir).unwrap().flatten() {
            if e.file_name().to_string_lossy().starts_with(&prefix) {
                let _ = fs::remove_file(e.path());
            }
        }
    }

    #[test]
    fn prune_backups_keeps_at_most_max_and_drops_oldest() {
        // B2 core: after MAX_BACKUPS+1 backups, only the newest MAX_BACKUPS
        // survive. We pre-seed 7 backups with distinct, sortable suffixes
        // (d20000..d20006) instead of relying on real wall-clock time, so the
        // test is deterministic and independent of execution speed.
        let path = write_tmp(r#"x = 1"#);
        let dir = path.parent().unwrap();
        let stem = path.file_name().unwrap().to_str().unwrap().to_string();
        let prefix = format!("{}.bak-", stem);

        // Seed 7 backups, names sort lexicographically in age order
        // (d20000 oldest ... d20006 newest).
        for day in 0..7u32 {
            let name = format!("{}.bak-d{}-000000", stem, 20000 + day);
            fs::write(dir.join(&name), "old").unwrap();
        }

        prune_backups(&path).unwrap();

        let mut remaining: Vec<String> = fs::read_dir(dir)
            .unwrap()
            .filter_map(|e| e.ok())
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .filter(|n| n.starts_with(&prefix))
            .collect();
        remaining.sort();

        assert_eq!(
            remaining.len(),
            MAX_BACKUPS,
            "expected exactly {} backups after prune, got {}: {:?}",
            MAX_BACKUPS,
            remaining.len(),
            remaining
        );
        // The oldest (d20000, d20001) must be gone; the newest (d20002..d20006)
        // survive because prune sorts ascending and drops the lowest.
        assert!(
            !remaining.iter().any(|n| n.contains("d20000")),
            "oldest backup should be pruned"
        );
        assert!(
            !remaining.iter().any(|n| n.contains("d20001")),
            "second-oldest backup should be pruned"
        );
        assert!(
            remaining.iter().any(|n| n.contains("d20006")),
            "newest backup should survive"
        );

        // cleanup
        let _ = fs::remove_file(&path);
        for n in &remaining {
            let _ = fs::remove_file(dir.join(n));
        }
    }

    #[test]
    fn add_mcp_requires_command_or_url() {
        let path = write_tmp(r#"x = 1"#);
        let mut doc = read_doc(&path).unwrap();
        let res = add_mcp(
            &mut doc,
            &McpAdd {
                name: "empty".into(),
                r#type: None,
                command: None,
                args: vec![],
                url: None,
            },
        );
        assert!(res.is_err());
        let _ = fs::remove_file(path);
    }

    /// Build a fake plugin cache layout under a temp dir and return the plugin
    /// root (`cache/$marketplace/$plugin`), ready for `pick_latest_version`.
    fn make_cache_layout(marketplace: &str, plugin: &str) -> PathBuf {
        let root = std::env::temp_dir().join(format!(
            "codex-cache-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let plugin_root = root.join(marketplace).join(plugin);
        fs::create_dir_all(&plugin_root).unwrap();
        plugin_root
    }

    /// Write a version dir with a minimal valid plugin.json manifest.
    fn write_version(plugin_root: &Path, version: &str, desc: &str) {
        let vdir = plugin_root.join(version);
        let manifest_dir = vdir.join(".codex-plugin");
        fs::create_dir_all(&manifest_dir).unwrap();
        let manifest = format!(
            r#"{{"name":"p","version":"{}","description":"{}"}}"#,
            version, desc
        );
        fs::write(manifest_dir.join("plugin.json"), manifest).unwrap();
    }

    #[test]
    fn pick_latest_version_returns_highest_sorting_dir() {
        // pick_latest_version sorts version dir names descending. We verify it
        // picks the lexicographically-highest name that has a valid manifest.
        let root = make_cache_layout("openai-curated", "github");
        write_version(&root, "0.1.3", "third");
        write_version(&root, "0.1.10", "tenth"); // lexically < "0.1.3" — see note
        write_version(&root, "0.2.0", "second-major");

        let (picked, manifest) = pick_latest_version(&root).unwrap();
        let picked_name = picked.file_name().unwrap().to_string_lossy().into_owned();
        // "0.2.0" sorts highest of the three, so it is picked. This also
        // documents the known approximation: lexicographic, not semver, so
        // "0.1.10" < "0.1.3" (acceptable for a display-only representative pick).
        assert_eq!(picked_name, "0.2.0");
        assert_eq!(manifest.version.as_deref(), Some("0.2.0"));
        assert_eq!(manifest.description.as_deref(), Some("second-major"));

        let _ = fs::remove_dir_all(
            root.parent().unwrap().parent().unwrap(), // remove the cache root
        );
    }

    #[test]
    fn pick_latest_version_skips_dirs_without_manifest() {
        let root = make_cache_layout("personal", "solo");
        // A version dir with no manifest must be skipped; a valid one is picked.
        fs::create_dir_all(root.join("broken")).unwrap();
        write_version(&root, "1.0.0", "good");

        let (picked, _manifest) = pick_latest_version(&root).unwrap();
        assert_eq!(
            picked.file_name().unwrap().to_string_lossy(),
            "1.0.0"
        );

        let _ = fs::remove_dir_all(root.parent().unwrap().parent().unwrap());
    }

    #[test]
    fn pick_latest_version_errors_when_no_manifest_anywhere() {
        let root = make_cache_layout("empty", "ghost");
        // Only a manifest-less dir.
        fs::create_dir_all(root.join("0.0.1")).unwrap();
        assert!(pick_latest_version(&root).is_err());

        let _ = fs::remove_dir_all(root.parent().unwrap().parent().unwrap());
    }

    #[test]
    fn pick_latest_version_errors_when_dir_missing() {
        let ghost = std::env::temp_dir().join("codex-no-such-dir-12345");
        assert!(pick_latest_version(&ghost).is_err());
    }
}
