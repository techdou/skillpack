use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackInfo {
    pub source: String,
    #[serde(rename = "type")]
    pub pack_type: String,
    pub installed_at: String,
    pub skill_root: Option<String>,
    pub skills: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectLink {
    pub pack: String,
    pub target: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectConfig {
    pub targets: HashMap<String, String>,
    pub links: HashMap<String, ProjectLink>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub version: u32,
    pub packs_dir: String,
    pub default_targets: Vec<String>,
    pub projects: HashMap<String, ProjectConfig>,
    pub packs: HashMap<String, PackInfo>,
    #[serde(default)]
    pub codex_config_path: Option<String>,
}

impl Default for AppConfig {
    fn default() -> Self {
        let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
        let packs_dir = home.join(".skillpack").join("packs");
        Self {
            version: 1,
            packs_dir: packs_dir.to_string_lossy().to_string(),
            default_targets: vec!["codex".into(), "agents".into()],
            projects: HashMap::new(),
            packs: HashMap::new(),
            codex_config_path: Some(
                home.join(".codex")
                    .join("config.toml")
                    .to_string_lossy()
                    .to_string(),
            ),
        }
    }
}

impl AppConfig {
    pub fn config_path() -> PathBuf {
        if let Ok(path) = std::env::var("SKILLPACK_CONFIG_PATH") {
            return PathBuf::from(path);
        }

        let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
        home.join(".skillpack").join("config.json")
    }

    pub fn load() -> Result<Self, String> {
        let path = Self::config_path();
        if !path.exists() {
            let config = Self::default();
            config.save()?;
            return Ok(config);
        }
        let content = fs::read_to_string(&path).map_err(|e| e.to_string())?;
        serde_json::from_str(&content).map_err(|e| e.to_string())
    }

    pub fn save(&self) -> Result<(), String> {
        let path = Self::config_path();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        let content = serde_json::to_string_pretty(self).map_err(|e| e.to_string())?;
        fs::write(&path, content).map_err(|e| e.to_string())
    }

    pub fn packs_dir_path(&self) -> PathBuf {
        PathBuf::from(&self.packs_dir)
    }

    pub fn canonical_project_key(path: &str) -> Result<String, String> {
        fs::canonicalize(path)
            .map_err(|e| format!("Failed to resolve project path '{}': {}", path, e))
            .map(|p| p.to_string_lossy().to_string())
    }

    pub fn codex_config_path(&self) -> Option<PathBuf> {
        self.codex_config_path
            .as_deref()
            .map(Self::resolve_path)
            .filter(|path| path.exists())
    }

    /// Resolve ~ in paths
    pub fn resolve_path(p: &str) -> PathBuf {
        if p.starts_with('~') {
            let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
            let rest = p.strip_prefix('~').unwrap_or(p);
            let rest = rest.strip_prefix(['/', '\\']).unwrap_or(rest);
            home.join(rest)
        } else {
            PathBuf::from(p)
        }
    }

    /// Map target name to relative skills directory
    pub fn target_to_dir(target: &str) -> &'static str {
        match target {
            "codex" => ".codex/skills",
            "agents" => ".agents/skills",
            "claude" => ".claude/skills",
            "cursor" => ".cursor/skills",
            _ => ".agents/skills",
        }
    }
}
