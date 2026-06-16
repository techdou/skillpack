use crate::config::{with_config, with_config_mut, AppConfig, PackInfo};
use crate::git;
use crate::scanner;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

/// Result of an update sweep. `failed` carries `(pack_name, error_message)`
/// so one unreachable remote no longer aborts every other pack.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateReport {
    pub updated: Vec<String>,
    pub failed: Vec<UpdateFailure>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateFailure {
    pub pack: String,
    pub error: String,
}

#[tauri::command]
pub fn pack_install(
    source: String,
    name: String,
    skill_root: Option<String>,
) -> Result<PackInfo, String> {
    let source_clone = source.clone();
    let name_clone = name.clone();
    let skill_root_clone = skill_root.clone();

    // Claim the name atomically, then clone outside the config lock (a clone
    // can take seconds; we don't want to hold the lock across network IO).
    let target_dir = with_config_mut(|config| -> Result<PathBuf, String> {
        if config.packs.contains_key(&name_clone) {
            return Err(format!("Pack '{}' already installed", name_clone));
        }
        Ok(config.packs_dir_path().join(&name_clone))
    })?;

    git::clone_repo(&source_clone, &target_dir).inspect_err(|_e| {
        // Best-effort cleanup of a half-cloned directory.
        let _ = fs::remove_dir_all(&target_dir);
    })?;

    // Detect skill_root and scan skills on disk.
    let resolved_root = match &skill_root_clone {
        Some(root) => Some(root.clone()),
        None => scanner::detect_skill_root(&target_dir),
    };
    let scan_dir = match &resolved_root {
        Some(root) => target_dir.join(root),
        None => target_dir.clone(),
    };
    let skill_names: Vec<String> = scanner::scan_skills(&scan_dir)
        .into_iter()
        .map(|s| s.dir_name)
        .collect();

    let pack = PackInfo {
        source: source_clone,
        pack_type: "git".into(),
        installed_at: Utc::now().to_rfc3339(),
        skill_root: resolved_root,
        skills: skill_names,
    };

    // Persist. Re-check the name wasn't taken in the meantime.
    let name_for_insert = name.clone();
    let final_pack = pack.clone();
    with_config_mut(|config| -> Result<PackInfo, String> {
        if config.packs.contains_key(&name_for_insert) {
            // Lost the race; clean up the clone we just made.
            let _ = fs::remove_dir_all(&target_dir);
            return Err(format!("Pack '{}' already installed", name_for_insert));
        }
        config
            .packs
            .insert(name_for_insert.clone(), final_pack.clone());
        Ok(final_pack)
    })
}

#[tauri::command]
pub fn pack_install_local(
    source_dir: String,
    name: String,
    skill_root: Option<String>,
) -> Result<PackInfo, String> {
    let source_path = PathBuf::from(&source_dir);
    if !source_path.is_dir() {
        return Err(format!(
            "Local pack directory does not exist: {}",
            source_path.to_string_lossy()
        ));
    }

    let resolved_root = match &skill_root {
        Some(root) if !root.trim().is_empty() => {
            Some(normalize_skill_root(&source_path, root)?)
        }
        _ => scanner::detect_skill_root(&source_path),
    };

    let name_clone = name.clone();
    let target_dir = with_config_mut(|config| -> Result<PathBuf, String> {
        if config.packs.contains_key(&name_clone) {
            return Err(format!("Pack '{}' already installed", name_clone));
        }
        let target_dir = config.packs_dir_path().join(&name_clone);
        if target_dir.exists() {
            return Err(format!(
                "Pack directory already exists: {}",
                target_dir.to_string_lossy()
            ));
        }
        Ok(target_dir)
    })?;

    // Copy outside the lock.
    crate::commands::skills::copy_dir_recursive(&source_path, &target_dir).inspect_err(|_e| {
        let _ = fs::remove_dir_all(&target_dir);
    })?;

    let scan_dir = match &resolved_root {
        Some(root) => target_dir.join(root),
        None => target_dir.clone(),
    };
    let skill_names: Vec<String> = scanner::scan_skills(&scan_dir)
        .into_iter()
        .map(|s| s.dir_name)
        .collect();

    let pack = PackInfo {
        source: source_path.to_string_lossy().to_string(),
        pack_type: "local".into(),
        installed_at: Utc::now().to_rfc3339(),
        skill_root: resolved_root,
        skills: skill_names,
    };

    let name_for_insert = name.clone();
    let final_pack = pack.clone();
    with_config_mut(|config| -> Result<PackInfo, String> {
        if config.packs.contains_key(&name_for_insert) {
            let _ = fs::remove_dir_all(&target_dir);
            return Err(format!("Pack '{}' already installed", name_for_insert));
        }
        config
            .packs
            .insert(name_for_insert.clone(), final_pack.clone());
        Ok(final_pack)
    })
}

#[tauri::command]
pub fn pack_list() -> Result<Vec<(String, PackInfo)>, String> {
    with_config(|config| {
        config
            .packs
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect::<Vec<_>>()
    })
}

#[tauri::command]
pub fn pack_open(name: String) -> Result<(), String> {
    let path = with_config(|config| -> Result<PathBuf, String> {
        if !config.packs.contains_key(&name) {
            return Err(format!("Pack '{}' not found", name));
        }
        Ok(config.packs_dir_path().join(&name))
    })??;
    crate::commands::skills::open_path(path.to_string_lossy().to_string())
}

#[tauri::command]
pub fn pack_remove(name: String) -> Result<(), String> {
    with_config_mut(|config| -> Result<(), String> {
        // Remove pack directory.
        let pack_dir = config.packs_dir_path().join(&name);
        if pack_dir.exists() {
            fs::remove_dir_all(&pack_dir).map_err(|e| e.to_string())?;
        }

        // Remove concrete links/copies before dropping config entries. This
        // matters on Windows where symlink creation can fall back to a real
        // directory copy.
        let links_to_remove: Vec<(String, String, String)> = config
            .projects
            .iter()
            .flat_map(|(proj_path, proj_config)| {
                proj_config
                    .links
                    .iter()
                    .filter_map(|(skill_name, link)| {
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
                PathBuf::from(proj_path).join(AppConfig::target_to_dir(&target));
            let _ = crate::symlink::remove_skill_link(&project_skills_dir, &skill_name);
        }

        // Remove from all projects' links.
        for proj_config in config.projects.values_mut() {
            proj_config.links.retain(|_, link| link.pack != name);
        }

        config.packs.remove(&name);
        Ok(())
    })
}

fn normalize_skill_root(pack_root: &Path, skill_root: &str) -> Result<String, String> {
    let root_path = AppConfig::resolve_path(skill_root);
    let relative = if root_path.is_absolute() {
        root_path
            .strip_prefix(pack_root)
            .map_err(|_| {
                format!(
                    "Skill root '{}' must be inside pack directory '{}'",
                    root_path.to_string_lossy(),
                    pack_root.to_string_lossy()
                )
            })?
            .to_path_buf()
    } else {
        root_path
    };

    let normalized = relative.to_string_lossy().replace('\\', "/");
    if normalized.is_empty() || normalized == "." {
        return Ok(String::new());
    }
    Ok(normalized)
}

#[tauri::command]
pub fn pack_update(name: Option<String>) -> Result<UpdateReport, String> {
    // Snapshot which packs to update; git operations happen outside the lock.
    let (packs_dir, packs_to_update): (PathBuf, Vec<String>) = with_config(|config| {
        let dir = config.packs_dir_path();
        let names: Vec<String> = match &name {
            Some(n) => vec![n.clone()],
            None => config.packs.keys().cloned().collect(),
        };
        (dir, names)
    })?;

    let mut updated: Vec<String> = Vec::new();
    let mut failed: Vec<UpdateFailure> = Vec::new();

    for pack_name in &packs_to_update {
        let pack_dir = packs_dir.join(pack_name);
        if !pack_dir.exists() {
            failed.push(UpdateFailure {
                pack: pack_name.clone(),
                error: "pack directory missing".into(),
            });
            continue;
        }
        match git::pull_repo(&pack_dir) {
            Ok(_) => updated.push(pack_name.clone()),
            Err(e) => failed.push(UpdateFailure {
                pack: pack_name.clone(),
                error: e,
            }),
        }
    }

    // Re-scan skills for updated packs and refresh copy-type links under a
    // single lock so the on-disk config reflects the new state coherently.
    with_config_mut(|config| -> Result<(), String> {
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

        // Refresh copy-type links so non-symlinked projects pick up the new
        // pack contents. Symlink links follow the pack automatically.
        refresh_copy_links(config, &updated);

        Ok(())
    })?;

    Ok(UpdateReport { updated, failed })
}

/// Re-copy the on-disk skill folders for every project link whose pack was just
/// updated and whose link was materialized as a copy (typical on Windows
/// without developer mode).
fn refresh_copy_links(config: &mut AppConfig, updated_packs: &[String]) {
    let packs_dir = config.packs_dir_path();

    // First snapshot the link records (owned), so we don't hold a borrow into
    // `config.projects`/`config.packs` while iterating. We resolve each link's
    // source path from the packs map at snapshot time.
    struct Pending {
        project_dir: PathBuf,
        skill_name: String,
        src: PathBuf,
        target_dir: String,
    }

    let mut pending: Vec<Pending> = Vec::new();
    for (proj_path, proj_config) in config.projects.iter() {
        for (skill_name, link) in proj_config.links.iter() {
            let is_copy = link
                .link_type
                .as_deref()
                .map(|t| t == "copy")
                .unwrap_or(false);
            if !is_copy || !updated_packs.contains(&link.pack) {
                continue;
            }
            let Some(pack_info) = config.packs.get(&link.pack) else {
                continue;
            };
            let src = match &pack_info.skill_root {
                Some(root) => packs_dir.join(&link.pack).join(root).join(skill_name),
                None => packs_dir.join(&link.pack).join(skill_name),
            };
            let target_dir = AppConfig::target_to_dir(&link.target).to_string();
            pending.push(Pending {
                project_dir: PathBuf::from(proj_path),
                skill_name: skill_name.clone(),
                src,
                target_dir,
            });
        }
    }

    for p in pending {
        let project_skills_dir = p.project_dir.join(&p.target_dir);
        if p.src.is_dir() {
            // Re-create the link; create_skill_link removes the existing entry
            // first, so this overwrites the stale copy in place.
            let _ = crate::symlink::create_skill_link(&p.src, &project_skills_dir, &p.skill_name);
        }
    }
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

    /// Serialise config-file-backed tests via the shared config-test lock.
    fn env_lock() -> std::sync::MutexGuard<'static, ()> {
        crate::config::test_support::env_lock()
    }

    #[test]
    fn pack_remove_deletes_project_skill_directory_and_config_links() {
        let _guard = env_lock();
        let root = temp_root("skillpack-pack-remove");
        let config_path = root.join("config.json");
        std::env::set_var("SKILLPACK_CONFIG_PATH", &config_path);

        let packs_dir = root.join("packs");
        let project_dir = root.join("project");
        let pack_dir = packs_dir.join("core");
        let linked_skill_dir = project_dir
            .join(".agents")
            .join("skills")
            .join("old-skill");

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
                link_type: None,
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

    #[test]
    fn pack_install_local_copies_pack_and_saves_relative_skill_root() {
        let _guard = env_lock();
        let root = temp_root("skillpack-local-install");
        let config_path = root.join("config.json");
        std::env::set_var("SKILLPACK_CONFIG_PATH", &config_path);

        let source_dir = root.join("source-pack");
        let skill_root = source_dir.join("skills");
        let skill_dir = skill_root.join("writer");
        fs::create_dir_all(&skill_dir).unwrap();
        fs::write(
            skill_dir.join("SKILL.md"),
            "---\nname: writer\ndescription: Writes docs\n---\n",
        )
        .unwrap();

        AppConfig {
            version: 1,
            packs_dir: root.join("packs").to_string_lossy().to_string(),
            default_targets: vec!["codex".into()],
            projects: HashMap::new(),
            packs: HashMap::new(),
            codex_config_path: None,
        }
        .save()
        .unwrap();

        let pack = pack_install_local(
            source_dir.to_string_lossy().to_string(),
            "docs".into(),
            Some(skill_root.to_string_lossy().to_string()),
        )
        .unwrap();

        assert_eq!(pack.pack_type, "local");
        assert_eq!(pack.skill_root.as_deref(), Some("skills"));
        assert_eq!(pack.skills, vec!["writer".to_string()]);
        assert!(root
            .join("packs")
            .join("docs")
            .join("skills")
            .join("writer")
            .join("SKILL.md")
            .exists());

        std::env::remove_var("SKILLPACK_CONFIG_PATH");
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn absolute_skill_root_must_be_inside_local_pack() {
        let pack_root = temp_root("skillpack-root");
        let outside = temp_root("skillpack-outside");
        let err = normalize_skill_root(&pack_root, &outside.to_string_lossy()).unwrap_err();
        assert!(err.contains("must be inside pack directory"));
    }

    #[test]
    fn refresh_copy_links_overwrites_stale_copy_with_new_pack_contents() {
        // refresh_copy_links is the Windows-critical path: after a `git pull`
        // rewrites the pack, copy-type links in projects must be re-copied so
        // they pick up the new contents. Symlink links are skipped (they
        // already follow the pack). This test exercises both branches on the
        // real filesystem.
        let _guard = env_lock();
        let root = temp_root("skillpack-refresh-copy");
        let config_path = root.join("config.json");
        std::env::set_var("SKILLPACK_CONFIG_PATH", &config_path);

        let packs_dir = root.join("packs");
        let project_dir = root.join("project");
        let target_rel = AppConfig::target_to_dir("codex"); // ".agents/skills"

        // A pack "core" with one skill "alpha".
        let pack_skill_dir = packs_dir.join("core").join("alpha");
        fs::create_dir_all(&pack_skill_dir).unwrap();
        fs::write(pack_skill_dir.join("SKILL.md"), "# alpha v1\n").unwrap();

        // A copy-type link already materialised in the project (stale content).
        let copied_skill_dir = project_dir.join(target_rel).join("alpha");
        fs::create_dir_all(&copied_skill_dir).unwrap();
        fs::write(copied_skill_dir.join("SKILL.md"), "# alpha STALE\n").unwrap();
        assert_eq!(
            fs::read_to_string(copied_skill_dir.join("SKILL.md")).unwrap(),
            "# alpha STALE\n",
            "sanity: pre-refresh project copy is stale"
        );

        // A symlink-type link for "beta" (also stale on disk) must be LEFT
        // ALONE by refresh_copy_links — it follows the pack automatically.
        let beta_pack_dir = packs_dir.join("core").join("beta");
        fs::create_dir_all(&beta_pack_dir).unwrap();
        fs::write(beta_pack_dir.join("SKILL.md"), "# beta v1\n").unwrap();
        let beta_proj_dir = project_dir.join(target_rel).join("beta");
        fs::create_dir_all(&beta_proj_dir).unwrap();
        fs::write(beta_proj_dir.join("SKILL.md"), "# beta SHOULD NOT BE TOUCHED\n").unwrap();

        let mut links = HashMap::new();
        links.insert(
            "alpha".into(),
            ProjectLink {
                pack: "core".into(),
                target: "codex".into(),
                link_type: Some("copy".into()),
            },
        );
        links.insert(
            "beta".into(),
            ProjectLink {
                pack: "core".into(),
                target: "codex".into(),
                link_type: Some("symlink".into()),
            },
        );
        let mut projects = HashMap::new();
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
                skills: vec!["alpha".into(), "beta".into()],
            },
        );

        let mut config = AppConfig {
            version: 1,
            packs_dir: packs_dir.to_string_lossy().to_string(),
            default_targets: vec!["codex".into()],
            projects,
            packs,
            codex_config_path: None,
        };

        // Simulate a pack update that rewrote alpha's content to v2.
        fs::write(pack_skill_dir.join("SKILL.md"), "# alpha v2\n").unwrap();

        refresh_copy_links(&mut config, &["core".to_string()]);

        // Copy link was refreshed: project now reflects the new pack content.
        assert_eq!(
            fs::read_to_string(copied_skill_dir.join("SKILL.md")).unwrap(),
            "# alpha v2\n",
            "copy-type link must be refreshed to new pack content"
        );
        // Symlink link was NOT touched by refresh (it is skipped by design).
        assert_eq!(
            fs::read_to_string(beta_proj_dir.join("SKILL.md")).unwrap(),
            "# beta SHOULD NOT BE TOUCHED\n",
            "symlink-type link must not be re-copied"
        );

        std::env::remove_var("SKILLPACK_CONFIG_PATH");
        let _ = fs::remove_dir_all(root);
    }
}
