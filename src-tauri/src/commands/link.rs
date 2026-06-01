use crate::config::{AppConfig, ProjectLink};
use crate::symlink;
use std::path::PathBuf;

#[derive(serde::Serialize, serde::Deserialize)]
#[allow(dead_code)]
pub struct SkillLinkInfo {
    pub skill_name: String,
    pub pack: String,
    pub target: String,
    pub target_dir: String,
}

#[tauri::command]
pub fn skill_link(
    project: String,
    skill_name: String,
    pack: String,
    target: String,
) -> Result<(), String> {
    let mut config = AppConfig::load()?;
    let project_key = AppConfig::canonical_project_key(&project).unwrap_or(project);

    // Validate pack and skill exist
    let pack_info = config
        .packs
        .get(&pack)
        .ok_or_else(|| format!("Pack '{}' not found", pack))?;

    if !pack_info.skills.contains(&skill_name) {
        return Err(format!(
            "Skill '{}' not found in pack '{}'",
            skill_name, pack
        ));
    }

    // Resolve paths
    let packs_dir = config.packs_dir_path();
    let skill_path = match &pack_info.skill_root {
        Some(root) => packs_dir.join(&pack).join(root).join(&skill_name),
        None => packs_dir.join(&pack).join(&skill_name),
    };

    let target_rel = AppConfig::target_to_dir(&target);
    let project_dir = PathBuf::from(&project_key);
    let project_skills_dir = project_dir.join(target_rel);

    // Create the link
    let _link_type = symlink::create_skill_link(&skill_path, &project_skills_dir, &skill_name)?;

    // Register in config
    let proj_config =
        config
            .projects
            .entry(project_key)
            .or_insert_with(|| crate::config::ProjectConfig {
                targets: {
                    let mut m = std::collections::HashMap::new();
                    m.insert(target.clone(), target_rel.to_string());
                    m
                },
                links: std::collections::HashMap::new(),
            });

    proj_config.links.insert(
        skill_name.clone(),
        ProjectLink {
            pack,
            target: target.clone(),
        },
    );

    config.save()?;

    Ok(())
}

#[tauri::command]
pub fn skill_unlink(project: String, skill_name: String) -> Result<(), String> {
    let mut config = AppConfig::load()?;
    let project_key = AppConfig::canonical_project_key(&project).unwrap_or(project);

    let proj_config = config
        .projects
        .get(&project_key)
        .ok_or("Project not found")?;

    let link_info = proj_config
        .links
        .get(&skill_name)
        .ok_or_else(|| format!("Skill '{}' not linked to this project", skill_name))?;

    let target_rel = AppConfig::target_to_dir(&link_info.target);
    let project_dir = PathBuf::from(&project_key);
    let project_skills_dir = project_dir.join(target_rel);

    symlink::remove_skill_link(&project_skills_dir, &skill_name)?;

    // Remove from config
    if let Some(proj_config) = config.projects.get_mut(&project_key) {
        proj_config.links.remove(&skill_name);
    }

    config.save()?;

    Ok(())
}

#[tauri::command]
#[allow(dead_code)]
pub fn skill_status(project: String) -> Result<Vec<SkillLinkInfo>, String> {
    let config = AppConfig::load()?;
    let project_key = AppConfig::canonical_project_key(&project).unwrap_or(project);

    let proj_config = config
        .projects
        .get(&project_key)
        .ok_or("Project not found")?;

    let links: Vec<SkillLinkInfo> = proj_config
        .links
        .iter()
        .map(|(name, link)| SkillLinkInfo {
            skill_name: name.clone(),
            pack: link.pack.clone(),
            target: link.target.clone(),
            target_dir: AppConfig::target_to_dir(&link.target).to_string(),
        })
        .collect();

    Ok(links)
}
