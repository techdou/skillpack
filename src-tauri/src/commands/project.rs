use crate::config::{with_config, with_config_mut, AppConfig};
use std::path::PathBuf;

#[derive(serde::Serialize, serde::Deserialize)]
pub struct ProjectInfo {
    pub path: String,
    pub name: String,
    pub linked_skills_count: usize,
    pub targets: Vec<String>,
}

#[tauri::command]
pub fn project_add(path: String) -> Result<ProjectInfo, String> {
    let project_dir = PathBuf::from(&path);
    if !project_dir.exists() {
        return Err(format!("Directory does not exist: {}", path));
    }
    let canonical_path = AppConfig::canonical_project_key(&path)?;
    let project_dir = PathBuf::from(&canonical_path);

    let canonical_for_insert = canonical_path.clone();
    with_config_mut(|config| -> Result<(), String> {
        if config.projects.contains_key(&canonical_for_insert) {
            return Err(format!(
                "Project '{}' already registered",
                canonical_for_insert
            ));
        }

        let targets: std::collections::HashMap<String, String> = config
            .default_targets
            .iter()
            .map(|t| (t.clone(), AppConfig::target_to_dir(t).to_string()))
            .collect();

        config.projects.insert(
            canonical_for_insert.clone(),
            crate::config::ProjectConfig {
                targets,
                links: std::collections::HashMap::new(),
            },
        );
        Ok(())
    })?;

    // Build the response from the freshly-saved config.
    let name = project_dir
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "unnamed".into());
    with_config(|config| {
        Ok(ProjectInfo {
            path: canonical_path,
            name,
            linked_skills_count: 0,
            targets: config.default_targets.clone(),
        })
    })?
}

#[tauri::command]
pub fn project_remove(path: String) -> Result<(), String> {
    let project_key = AppConfig::canonical_project_key(&path).unwrap_or(path);

    with_config_mut(|config| -> Result<(), String> {
        // Remove all skill links for this project.
        if let Some(proj_config) = config.projects.get(&project_key) {
            let project_dir = PathBuf::from(&project_key);
            for (skill_name, link_info) in &proj_config.links {
                let target_rel = AppConfig::target_to_dir(&link_info.target);
                let project_skills_dir = project_dir.join(target_rel);
                let _ = crate::symlink::remove_skill_link(&project_skills_dir, skill_name);
            }
        }

        config.projects.remove(&project_key);
        Ok(())
    })
}

#[tauri::command]
pub fn project_list() -> Result<Vec<ProjectInfo>, String> {
    with_config(|config| {
        config
            .projects
            .iter()
            .map(|(path, proj_config)| {
                let project_dir = PathBuf::from(path);
                let name = project_dir
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_else(|| "unnamed".into());

                ProjectInfo {
                    path: path.clone(),
                    name,
                    linked_skills_count: proj_config.links.len(),
                    targets: proj_config.targets.keys().cloned().collect(),
                }
            })
            .collect::<Vec<_>>()
    })
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
    fn canonical_project_key_normalizes_trailing_separator() {
        let root = temp_root("skillpack-project-add");
        let project_dir = root.join("project");
        std::fs::create_dir_all(&project_dir).unwrap();

        let with_trailing_separator = format!("{}\\", project_dir.to_string_lossy());
        let plain = AppConfig::canonical_project_key(&project_dir.to_string_lossy()).unwrap();
        let trailing = AppConfig::canonical_project_key(&with_trailing_separator).unwrap();
        assert_eq!(plain, trailing);

        let _ = std::fs::remove_dir_all(root);
    }
}
