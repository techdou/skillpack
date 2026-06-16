use std::fs;
use std::path::Path;

#[derive(Debug, Clone, serde::Serialize)]
pub enum LinkType {
    Symlink,
    Copy,
}

/// Create a skill link (symlink or directory copy as fallback)
pub fn create_skill_link(
    skill_path: &Path,
    project_skills_dir: &Path,
    skill_name: &str,
) -> Result<LinkType, String> {
    let link_path = project_skills_dir.join(skill_name);

    // Ensure parent directory exists
    if let Some(parent) = link_path.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("Failed to create directory: {}", e))?;
    }

    // Remove existing link/dir if present
    if link_path.exists() || link_path.symlink_metadata().is_ok() {
        if link_path.is_symlink() {
            fs::remove_file(&link_path)
                .or_else(|_| fs::remove_dir(&link_path))
                .map_err(|e| format!("Failed to remove existing link: {}", e))?;
        } else {
            fs::remove_dir_all(&link_path)
                .map_err(|e| format!("Failed to remove existing directory: {}", e))?;
        }
    }

    // Try symlink first
    #[cfg(unix)]
    let result = std::os::unix::fs::symlink(skill_path, &link_path);

    #[cfg(windows)]
    let result = std::os::windows::fs::symlink_dir(skill_path, &link_path);

    match result {
        Ok(_) => Ok(LinkType::Symlink),
        Err(_) => {
            // Fallback: copy directory recursively
            copy_dir_recursive(skill_path, &link_path)?;
            Ok(LinkType::Copy)
        }
    }
}

/// Remove a skill link (handles both symlinks and copied dirs)
pub fn remove_skill_link(project_skills_dir: &Path, skill_name: &str) -> Result<(), String> {
    let link_path = project_skills_dir.join(skill_name);

    if !link_path.exists() && link_path.symlink_metadata().is_err() {
        return Err(format!("Skill link not found: {}", skill_name));
    }

    if link_path.is_symlink() {
        fs::remove_file(&link_path)
            .or_else(|_| fs::remove_dir(&link_path))
            .map_err(|e| format!("Failed to remove symlink: {}", e))
    } else {
        fs::remove_dir_all(&link_path).map_err(|e| format!("Failed to remove directory: {}", e))
    }
}

/// Recursively copy a directory
fn copy_dir_recursive(src: &Path, dst: &Path) -> Result<(), String> {
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
