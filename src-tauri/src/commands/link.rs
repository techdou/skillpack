use crate::config::{with_config, with_config_mut, AppConfig, ProjectLink};
use crate::symlink::{self, LinkType};
use std::path::PathBuf;

/// Information about a skill linked into a project. `link_type` reports how
/// the link was materialized on disk (`"symlink"` or `"copy"`) so the UI can
/// warn that copy-type links are not live-synced.
#[derive(serde::Serialize, serde::Deserialize)]
pub struct SkillLinkInfo {
    pub skill_name: String,
    pub pack: String,
    pub target: String,
    pub target_dir: String,
    pub link_type: String,
}

fn link_type_to_str(t: &LinkType) -> &'static str {
    match t {
        LinkType::Symlink => "symlink",
        LinkType::Copy => "copy",
    }
}

#[tauri::command]
pub fn skill_link(
    project: String,
    skill_name: String,
    pack: String,
    target: String,
) -> Result<String, String> {
    let project_key = AppConfig::canonical_project_key(&project).unwrap_or(project);

    // We need pack/skill_root info plus the packs_dir to resolve the source
    // path. The actual filesystem link is created outside the lock.
    let (skill_path, target_rel, project_skills_dir) = with_config(|config| -> Result<
        (PathBuf, String, PathBuf),
        String,
    > {
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
        let packs_dir = config.packs_dir_path();
        let skill_path = match &pack_info.skill_root {
            Some(root) => packs_dir.join(&pack).join(root).join(&skill_name),
            None => packs_dir.join(&pack).join(&skill_name),
        };
        let target_rel = AppConfig::target_to_dir(&target).to_string();
        let project_skills_dir = PathBuf::from(&project_key).join(&target_rel);
        Ok((skill_path, target_rel, project_skills_dir))
    })??;

    // Create the on-disk link outside the config lock.
    let link_type = symlink::create_skill_link(&skill_path, &project_skills_dir, &skill_name)?;
    let link_type_str = link_type_to_str(&link_type).to_string();

    // Register in config.
    let target_for_link = target.clone();
    let pack_for_link = pack.clone();
    let skill_name_for_link = skill_name.clone();
    let target_rel_for_link = target_rel.clone();
    with_config_mut(|config| -> Result<(), String> {
        let proj_config = config.projects.entry(project_key).or_insert_with(|| {
            crate::config::ProjectConfig {
                targets: {
                    let mut m = std::collections::HashMap::new();
                    m.insert(target_for_link.clone(), target_rel_for_link.clone());
                    m
                },
                links: std::collections::HashMap::new(),
            }
        });

        proj_config.links.insert(
            skill_name_for_link.clone(),
            ProjectLink {
                pack: pack_for_link,
                target: target_for_link.clone(),
                link_type: Some(link_type_str.clone()),
            },
        );
        Ok(())
    })?;

    Ok(link_type_str)
}

#[tauri::command]
pub fn skill_unlink(project: String, skill_name: String) -> Result<(), String> {
    let project_key = AppConfig::canonical_project_key(&project).unwrap_or(project);

    // Resolve where the link lives on disk, then remove it outside the lock.
    let project_skills_dir = with_config(|config| -> Result<PathBuf, String> {
        let proj_config = config
            .projects
            .get(&project_key)
            .ok_or("Project not found")?;
        let link_info = proj_config.links.get(&skill_name).ok_or_else(|| {
            format!("Skill '{}' not linked to this project", skill_name)
        })?;
        let target_rel = AppConfig::target_to_dir(&link_info.target);
        Ok(PathBuf::from(&project_key).join(target_rel))
    })??;

    symlink::remove_skill_link(&project_skills_dir, &skill_name)?;

    with_config_mut(|config| -> Result<(), String> {
        if let Some(proj_config) = config.projects.get_mut(&project_key) {
            proj_config.links.remove(&skill_name);
        }
        Ok(())
    })
}

#[tauri::command]
pub fn skill_status(project: String) -> Result<Vec<SkillLinkInfo>, String> {
    let result = with_config(|config| -> Result<Vec<SkillLinkInfo>, String> {
        let project_key = AppConfig::canonical_project_key(&project).unwrap_or(project);
        let proj_config = config
            .projects
            .get(&project_key)
            .ok_or("Project not found")?;

        let links: Vec<SkillLinkInfo> = proj_config
            .links
            .iter()
            .map(|(name, link)| {
                let target_dir = AppConfig::target_to_dir(&link.target).to_string();
                // Probe the on-disk entry to report the true materialization,
                // falling back to the persisted record if the file is gone.
                let link_path = PathBuf::from(&project_key)
                    .join(&target_dir)
                    .join(name);
                let probed = if link_path.is_symlink() {
                    Some("symlink".to_string())
                } else if link_path.exists() {
                    Some("copy".to_string())
                } else {
                    None
                };
                SkillLinkInfo {
                    skill_name: name.clone(),
                    pack: link.pack.clone(),
                    target: link.target.clone(),
                    target_dir,
                    link_type: probed
                        .or_else(|| link.link_type.clone())
                        .unwrap_or_else(|| "copy".to_string()),
                }
            })
            .collect();

        Ok(links)
    })??;
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn link_type_serializes_to_lowercase_string() {
        assert_eq!(link_type_to_str(&LinkType::Symlink), "symlink");
        assert_eq!(link_type_to_str(&LinkType::Copy), "copy");
    }
}
