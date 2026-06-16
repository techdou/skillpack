use crate::config::AppConfig;
use crate::scanner;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillRootInfo {
    pub key: String,
    pub label: String,
    pub path: String,
    pub exists: bool,
    pub skill_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillEntry {
    pub name: String,
    pub description: String,
    pub dir_name: String,
    pub path: String,
}

#[derive(Debug, Clone)]
struct ToolchainSpec {
    key: &'static str,
    label: &'static str,
    rel_dir: &'static str,
}

fn toolchain_specs() -> Vec<ToolchainSpec> {
    vec![
        ToolchainSpec {
            key: "codex",
            label: "Codex",
            rel_dir: ".agents/skills",
        },
        ToolchainSpec {
            key: "claude",
            label: "Claude",
            rel_dir: ".claude/skills",
        },
        ToolchainSpec {
            key: "gemini",
            label: "Gemini",
            // Matches AppConfig::target_to_dir("gemini"); kept distinct from
            // Codex so linking the same skill to both targets doesn't collide.
            rel_dir: ".gemini/skills",
        },
    ]
}

fn spec_for(key: &str) -> Result<ToolchainSpec, String> {
    toolchain_specs()
        .into_iter()
        .find(|spec| spec.key == key)
        .ok_or_else(|| format!("Unknown toolchain: {}", key))
}

fn skill_root_info(key: String, label: String, path: PathBuf) -> SkillRootInfo {
    let exists = path.is_dir();
    let skill_count = if exists {
        scanner::scan_skills(&path).len()
    } else {
        0
    };

    SkillRootInfo {
        key,
        label,
        path: path.to_string_lossy().to_string(),
        exists,
        skill_count,
    }
}

fn scan_entries(root: &Path) -> Vec<SkillEntry> {
    scanner::scan_skills(root)
        .into_iter()
        .map(|skill| SkillEntry {
            name: skill.name,
            description: skill.description,
            dir_name: skill.dir_name,
            path: skill.path,
        })
        .collect()
}

pub fn toolchain_skill_roots_for_home(home: &Path) -> Vec<SkillRootInfo> {
    toolchain_specs()
        .into_iter()
        .map(|spec| {
            skill_root_info(
                spec.key.to_string(),
                spec.label.to_string(),
                home.join(spec.rel_dir),
            )
        })
        .collect()
}

#[tauri::command]
pub fn toolchain_skill_roots() -> Result<Vec<SkillRootInfo>, String> {
    let home = dirs::home_dir().ok_or("Home directory not found")?;
    Ok(toolchain_skill_roots_for_home(&home))
}

#[tauri::command]
pub fn toolchain_skills(toolchain: String) -> Result<Vec<SkillEntry>, String> {
    let home = dirs::home_dir().ok_or("Home directory not found")?;
    let spec = spec_for(&toolchain)?;
    let root = home.join(spec.rel_dir);
    if !root.is_dir() {
        return Ok(Vec::new());
    }
    Ok(scan_entries(&root))
}

pub fn project_skill_roots_for_project(project: &Path) -> Vec<SkillRootInfo> {
    toolchain_specs()
        .into_iter()
        .map(|spec| {
            skill_root_info(
                spec.key.to_string(),
                spec.label.to_string(),
                project.join(spec.rel_dir),
            )
        })
        .collect()
}

#[tauri::command]
pub fn project_skill_roots(project: String) -> Result<Vec<SkillRootInfo>, String> {
    let project_key = AppConfig::canonical_project_key(&project).unwrap_or(project);
    Ok(project_skill_roots_for_project(Path::new(&project_key)))
}

#[tauri::command]
pub fn project_skills(project: String, toolchain: String) -> Result<Vec<SkillEntry>, String> {
    let project_key = AppConfig::canonical_project_key(&project).unwrap_or(project);
    let spec = spec_for(&toolchain)?;
    let root = PathBuf::from(project_key).join(spec.rel_dir);
    if !root.is_dir() {
        return Ok(Vec::new());
    }
    Ok(scan_entries(&root))
}

#[tauri::command]
pub fn open_path(path: String) -> Result<(), String> {
    let path = PathBuf::from(path);
    if !path.exists() {
        return Err(format!("Path does not exist: {}", path.to_string_lossy()));
    }

    #[cfg(target_os = "windows")]
    let status = Command::new("explorer").arg(&path).status();

    #[cfg(target_os = "macos")]
    let status = Command::new("open").arg(&path).status();

    #[cfg(all(unix, not(target_os = "macos")))]
    let status = Command::new("xdg-open").arg(&path).status();

    status
        .map_err(|e| format!("Failed to open path: {}", e))
        .and_then(|status| {
            if status.success() {
                Ok(())
            } else {
                Err(format!("File manager exited with status: {}", status))
            }
        })
}

pub fn copy_dir_recursive(src: &Path, dst: &Path) -> Result<(), String> {
    fs::create_dir_all(dst).map_err(|e| e.to_string())?;
    for entry in fs::read_dir(src).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());
        if src_path.is_dir() {
            copy_dir_recursive(&src_path, &dst_path)?;
        } else {
            fs::copy(&src_path, &dst_path).map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_root(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "{}-{}",
            name,
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ))
    }

    #[test]
    fn toolchain_roots_use_portable_home_relative_paths() {
        let home = temp_root("skillpack-toolchain-roots");
        let roots = toolchain_skill_roots_for_home(&home);

        let codex = roots.iter().find(|root| root.key == "codex").unwrap();
        let claude = roots.iter().find(|root| root.key == "claude").unwrap();
        let gemini = roots.iter().find(|root| root.key == "gemini").unwrap();

        assert_eq!(PathBuf::from(&codex.path), home.join(".agents/skills"));
        assert_eq!(PathBuf::from(&claude.path), home.join(".claude/skills"));
        assert_eq!(PathBuf::from(&gemini.path), home.join(".gemini/skills"));
        assert!(!codex.exists);
        assert_eq!(codex.skill_count, 0);
    }

    #[test]
    fn project_roots_and_scan_entries_report_skill_paths() {
        let project = temp_root("skillpack-project-skill-roots");
        let skill_dir = project.join(".agents/skills/build-web-apps");
        fs::create_dir_all(&skill_dir).unwrap();
        fs::write(
            skill_dir.join("SKILL.md"),
            "---\nname: build-web-apps\ndescription: Build web apps\n---\n",
        )
        .unwrap();

        let roots = project_skill_roots_for_project(&project);
        let codex = roots.iter().find(|root| root.key == "codex").unwrap();
        let claude = roots.iter().find(|root| root.key == "claude").unwrap();
        assert!(codex.exists);
        assert_eq!(codex.skill_count, 1);
        assert!(!claude.exists);

        let entries = scan_entries(&project.join(".agents/skills"));
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].name, "build-web-apps");
        assert_eq!(entries[0].description, "Build web apps");
        assert_eq!(PathBuf::from(&entries[0].path), skill_dir);

        let _ = fs::remove_dir_all(project);
    }
}
