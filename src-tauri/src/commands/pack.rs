use crate::config::{AppConfig, PackInfo};
use crate::git;
use crate::scanner;
use chrono::Utc;
use std::fs;

#[tauri::command]
pub fn pack_install(
    source: String,
    name: String,
    skill_root: Option<String>,
) -> Result<PackInfo, String> {
    let mut config = AppConfig::load()?;

    if config.packs.contains_key(&name) {
        return Err(format!("Pack '{}' already installed", name));
    }

    let packs_dir = config.packs_dir_path();
    let target_dir = packs_dir.join(&name);

    // Clone repository
    git::clone_repo(&source, &target_dir)?;

    // Detect or use provided skill_root
    let resolved_root = match skill_root {
        Some(root) => Some(root),
        None => scanner::detect_skill_root(&target_dir),
    };

    // Scan skills
    let scan_dir = match &resolved_root {
        Some(root) => target_dir.join(root),
        None => target_dir.clone(),
    };

    let skills_meta = scanner::scan_skills(&scan_dir);
    let skill_names: Vec<String> = skills_meta.iter().map(|s| s.dir_name.clone()).collect();

    let pack = PackInfo {
        source,
        pack_type: "git".into(),
        installed_at: Utc::now().to_rfc3339(),
        skill_root: resolved_root,
        skills: skill_names,
    };

    config.packs.insert(name, pack.clone());
    config.save()?;

    Ok(pack)
}

#[tauri::command]
pub fn pack_list() -> Result<Vec<(String, PackInfo)>, String> {
    let config = AppConfig::load()?;
    Ok(config
        .packs
        .iter()
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect())
}

#[tauri::command]
pub fn pack_remove(name: String) -> Result<(), String> {
    let mut config = AppConfig::load()?;

    // Remove pack directory
    let pack_dir = config.packs_dir_path().join(&name);
    if pack_dir.exists() {
        fs::remove_dir_all(&pack_dir).map_err(|e| e.to_string())?;
    }

    // Remove concrete links/copies before dropping config entries. This matters on
    // Windows where symlink creation can fall back to a real directory copy.
    let links_to_remove: Vec<(String, String, String)> = config
        .projects
        .iter()
        .flat_map(|(proj_path, proj_config)| {
            proj_config.links.iter().filter_map(|(skill_name, link)| {
                if link.pack == name {
                    Some((proj_path.clone(), skill_name.clone(), link.target.clone()))
                } else {
                    None
                }
            })
        })
        .collect();

    for (proj_path, skill_name, target) in links_to_remove {
        let project_skills_dir =
            std::path::PathBuf::from(proj_path).join(AppConfig::target_to_dir(&target));
        let _ = crate::symlink::remove_skill_link(&project_skills_dir, &skill_name);
    }

    // Remove from all projects' links.
    for (_proj_path, proj_config) in config.projects.iter_mut() {
        proj_config.links.retain(|_, link| link.pack != name);
    }

    config.packs.remove(&name);
    config.save()?;

    Ok(())
}

#[tauri::command]
pub fn pack_update(name: Option<String>) -> Result<Vec<String>, String> {
    let config = AppConfig::load()?;
    let mut updated = Vec::new();

    let packs_to_update: Vec<String> = match name {
        Some(n) => vec![n],
        None => config.packs.keys().cloned().collect(),
    };

    let packs_dir = config.packs_dir_path();
    for pack_name in packs_to_update {
        if let Some(pack_dir) = config
            .packs
            .get(&pack_name)
            .map(|_| packs_dir.join(&pack_name))
        {
            if pack_dir.exists() {
                match git::pull_repo(&pack_dir) {
                    Ok(_) => updated.push(pack_name),
                    Err(e) => return Err(format!("Failed to update {}: {}", pack_name, e)),
                }
            }
        }
    }

    // Re-scan skills after update
    let mut config = AppConfig::load()?;
    let packs_dir = config.packs_dir_path();
    for pack_name in &updated {
        if let Some(pack) = config.packs.get_mut(pack_name) {
            let scan_dir = match &pack.skill_root {
                Some(root) => packs_dir.join(pack_name).join(root),
                None => packs_dir.join(pack_name),
            };
            let skills_meta = scanner::scan_skills(&scan_dir);
            pack.skills = skills_meta.iter().map(|s| s.dir_name.clone()).collect();
        }
    }
    config.save()?;

    Ok(updated)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{ProjectConfig, ProjectLink};
    use std::collections::HashMap;
    use std::path::PathBuf;

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
    fn pack_remove_deletes_project_skill_directory_and_config_links() {
        let root = temp_root("skillpack-pack-remove");
        let config_path = root.join("config.json");
        std::env::set_var("SKILLPACK_CONFIG_PATH", &config_path);

        let packs_dir = root.join("packs");
        let project_dir = root.join("project");
        let pack_dir = packs_dir.join("core");
        let linked_skill_dir = project_dir.join(".codex").join("skills").join("old-skill");

        fs::create_dir_all(&pack_dir).unwrap();
        fs::create_dir_all(&linked_skill_dir).unwrap();
        fs::write(linked_skill_dir.join("SKILL.md"), "old").unwrap();

        let mut projects = HashMap::new();
        let mut links = HashMap::new();
        links.insert(
            "old-skill".into(),
            ProjectLink {
                pack: "core".into(),
                target: "codex".into(),
            },
        );
        projects.insert(
            project_dir.to_string_lossy().to_string(),
            ProjectConfig {
                targets: HashMap::new(),
                links,
            },
        );

        let mut packs = HashMap::new();
        packs.insert(
            "core".into(),
            PackInfo {
                source: "local".into(),
                pack_type: "git".into(),
                installed_at: "test".into(),
                skill_root: None,
                skills: vec!["old-skill".into()],
            },
        );

        AppConfig {
            version: 1,
            packs_dir: packs_dir.to_string_lossy().to_string(),
            default_targets: vec!["codex".into()],
            projects,
            packs,
            codex_config_path: None,
        }
        .save()
        .unwrap();

        pack_remove("core".into()).unwrap();

        let config = AppConfig::load().unwrap();
        let project = config
            .projects
            .get(&project_dir.to_string_lossy().to_string())
            .unwrap();
        assert!(!pack_dir.exists());
        assert!(!linked_skill_dir.exists());
        assert!(project.links.is_empty());

        std::env::remove_var("SKILLPACK_CONFIG_PATH");
        let _ = fs::remove_dir_all(root);
    }
}
