use std::fs;
use std::path::Path;
use std::path::PathBuf;
use walkdir::WalkDir;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SkillMeta {
    pub name: String,
    pub description: String,
    pub dir_name: String,
}

/// Scan a directory for all SKILL.md files and extract name + description
pub fn scan_skills(root: &Path) -> Vec<SkillMeta> {
    let mut skills = Vec::new();

    for entry in WalkDir::new(root)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_name() == "SKILL.md")
    {
        let content = match fs::read_to_string(entry.path()) {
            Ok(c) => c,
            Err(_) => continue,
        };

        let name = extract_yaml_field(&content, "name").unwrap_or_else(|| {
            entry
                .path()
                .parent()
                .and_then(|p| p.file_name())
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default()
        });

        let description = extract_yaml_field(&content, "description")
            .unwrap_or_else(|| "(no description)".into());

        let dir_name = entry
            .path()
            .parent()
            .and_then(|p| p.file_name())
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();

        skills.push(SkillMeta {
            name: name.trim().to_string(),
            description: description.trim().to_string(),
            dir_name,
        });
    }

    skills
}

/// Find the most likely skill_root: the deepest common parent of all SKILL.md files
pub fn detect_skill_root(repo_root: &Path) -> Option<String> {
    let paths: Vec<_> = WalkDir::new(repo_root)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_name() == "SKILL.md")
        .filter_map(|e| {
            e.path()
                .parent()
                .map(|p| p.strip_prefix(repo_root).unwrap_or(p).to_path_buf())
        })
        .collect();

    if paths.is_empty() {
        return None;
    }

    // Find the longest common prefix (directory)
    let mut common = paths[0].clone();
    for p in &paths[1..] {
        let mut new_common = PathBuf::new();
        for (a, b) in common.iter().zip(p.iter()) {
            if a == b {
                new_common.push(a);
            } else {
                break;
            }
        }
        common = new_common;
    }

    // common is the parent dir of skills; go one level up for the skill_root
    if let Some(parent) = common.parent() {
        let rel = parent.to_string_lossy().to_string();
        if rel.is_empty() {
            None
        } else {
            Some(rel)
        }
    } else {
        None
    }
}

/// Extract a field from YAML front matter (--- ... ---)
fn extract_yaml_field(content: &str, field: &str) -> Option<String> {
    let in_frontmatter = content.starts_with("---");
    if !in_frontmatter {
        return None;
    }

    let end = content[3..].find("---")?;
    let yaml = &content[3..3 + end];

    // Simple field extraction - handles `name: value` and `name: "value"` and `name: |` (multiline)
    for line in yaml.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with(field) && trimmed.contains(':') {
            let value = trimmed.splitn(2, ':').nth(1)?;
            let value = value.trim();

            // Handle quoted values
            if (value.starts_with('"') && value.ends_with('"'))
                || (value.starts_with('\'') && value.ends_with('\''))
            {
                return Some(value[1..value.len() - 1].to_string());
            }

            // Handle multiline indicator |
            if value == "|" || value == ">" {
                // Collect subsequent indented lines
                let field_start = yaml.find(trimmed).unwrap_or(0) + trimmed.len();
                let rest = &yaml[field_start..];
                let mut multi = String::new();
                for sub_line in rest.lines() {
                    if sub_line.is_empty()
                        || sub_line.starts_with(' ')
                        || sub_line.starts_with('\t')
                    {
                        multi.push_str(sub_line.trim());
                        multi.push(' ');
                    } else {
                        break;
                    }
                }
                return Some(multi.trim().to_string());
            }

            return Some(value.to_string());
        }
    }

    None
}
