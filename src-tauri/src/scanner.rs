use std::fs;
use std::path::Path;
use std::path::PathBuf;
use walkdir::WalkDir;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SkillMeta {
    pub name: String,
    pub description: String,
    pub dir_name: String,
    pub path: String,
}

/// Directory basenames we never descend into while scanning. These are either
/// version-control internals (`.git`), build artifacts (`target`), or heavy
/// dependency trees that should never host a `SKILL.md` we care about.
fn is_pruned_dir(name: &str) -> bool {
    matches!(name, ".git" | "node_modules" | "target" | ".svn" | ".hg")
}

/// True for directory entries that should be pruned during traversal. The
/// root itself (`depth == 0`) is never pruned, and a leading `.` is treated
/// as "hidden" (VCS metadata, editor config) and skipped at depth > 0.
fn should_prune_entry(entry: &walkdir::DirEntry) -> bool {
    if entry.depth() == 0 {
        return false;
    }
    let is_dir = entry.file_type().is_dir();
    if !is_dir {
        return false;
    }
    let name = match entry.file_name().to_str() {
        Some(n) => n,
        None => return false,
    };
    is_pruned_dir(name) || (name.starts_with('.') && name != ".")
}

/// Scan a directory for all SKILL.md files and extract name + description.
///
/// Prunes `.git`, `node_modules`, `target` and other hidden/build directories
/// so large repositories scan quickly and stray `SKILL.md` files under
/// `.git` aren't picked up.
pub fn scan_skills(root: &Path) -> Vec<SkillMeta> {
    let mut skills = Vec::new();

    for entry in WalkDir::new(root)
        .into_iter()
        .filter_entry(|e| !should_prune_entry(e))
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
            path: entry
                .path()
                .parent()
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_default(),
        });
    }

    skills
}

/// Find the most likely skill_root: the deepest common parent of all SKILL.md
/// files, returned relative to `repo_root`.
pub fn detect_skill_root(repo_root: &Path) -> Option<String> {
    // Only collect paths that can be expressed relative to the repo root.
    // Previously a failed `strip_prefix` fell back to the absolute path, which
    // then leaked into the stored `skill_root` and broke every later join.
    let paths: Vec<PathBuf> = WalkDir::new(repo_root)
        .into_iter()
        .filter_entry(|e| !should_prune_entry(e))
        .filter_map(|e| e.ok())
        .filter(|e| e.file_name() == "SKILL.md")
        .filter_map(|e| {
            e.path().parent().and_then(|p| {
                p.strip_prefix(repo_root)
                    .ok()
                    .map(|rel| rel.to_path_buf())
            })
        })
        .collect();

    if paths.is_empty() {
        return None;
    }

    // Find the longest common prefix (directory).
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

    // `common` is the parent dir that directly contains the skill folders; go
    // one level up to return the skill_root itself.
    let parent = common.parent()?;
    let rel = parent.to_string_lossy().to_string();
    if rel.is_empty() {
        None
    } else {
        Some(rel)
    }
}

/// Extract a field from YAML front matter (`--- ... ---`).
///
/// Performs exact key matching so a `name:` field is not accidentally matched
/// by a later `name_full:` line (the old `starts_with(field)` bug). Multiline
/// `|` / `>` blocks are collected by tracking the current line index instead
/// of re-finding the key text inside the buffer (the old `yaml.find(trimmed)`
/// bug that could match the wrong occurrence).
fn extract_yaml_field(content: &str, field: &str) -> Option<String> {
    if !content.starts_with("---") {
        return None;
    }

    let end = content[3..].find("---")?;
    let yaml = &content[3..3 + end];

    let lines: Vec<&str> = yaml.lines().collect();
    let prefix_exact = format!("{}:", field);

    for (i, line) in lines.iter().enumerate() {
        let trimmed = line.trim_start();
        // Exact key match: the trimmed line must be `<field>: ...` (or just `<field>:`).
        // This avoids `name` matching `name_full`.
        if !trimmed.starts_with(&prefix_exact) {
            continue;
        }
        // The character immediately after `field:` must be whitespace or end of
        // value, never an identifier char — guards against `name:` vs `names:`.
        let after = &trimmed[prefix_exact.len()..];
        if let Some(first) = after.chars().next() {
            if !(first.is_whitespace()) {
                continue;
            }
        }

        let value = after.trim();

        // Handle quoted values.
        if (value.starts_with('"') && value.ends_with('"') && value.len() >= 2)
            || (value.starts_with('\'') && value.ends_with('\'') && value.len() >= 2)
        {
            return Some(value[1..value.len() - 1].to_string());
        }

        // Handle multiline block scalars (`|` literal, `>` folded).
        if value == "|" || value == ">" {
            let mut multi = String::new();
            for sub_line in &lines[i + 1..] {
                if sub_line.is_empty() || sub_line.starts_with(' ') || sub_line.starts_with('\t')
                {
                    if !multi.is_empty() {
                        multi.push(' ');
                    }
                    multi.push_str(sub_line.trim());
                } else {
                    break;
                }
            }
            let trimmed_multi = multi.trim();
            if trimmed_multi.is_empty() {
                return None;
            }
            return Some(trimmed_multi.to_string());
        }

        return Some(value.to_string());
    }

    None
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
    fn exact_key_match_does_not_collide_with_prefix_keys() {
        // `name` must not match `name_full`.
        let content = "---\nname_full: full value\nname: real name\n---\n";
        assert_eq!(extract_yaml_field(content, "name").as_deref(), Some("real name"));
        assert_eq!(
            extract_yaml_field(content, "name_full").as_deref(),
            Some("full value")
        );

        // `description` must not match `description_long`.
        let content2 =
            "---\ndescription_long: long\ndescription: short\n---\n";
        assert_eq!(
            extract_yaml_field(content2, "description").as_deref(),
            Some("short")
        );
    }

    #[test]
    fn quoted_values_are_unwrapped() {
        let content = "---\nname: \"quoted name\"\ndescription: 'single'\n---\n";
        assert_eq!(extract_yaml_field(content, "name").as_deref(), Some("quoted name"));
        assert_eq!(extract_yaml_field(content, "description").as_deref(), Some("single"));
    }

    #[test]
    fn multiline_literal_block_is_collected() {
        let content = "---\nname: writer\ndescription: |\n  First line\n  Second line\nlicense: mit\n---\n";
        let desc = extract_yaml_field(content, "description").unwrap();
        assert!(desc.contains("First line"));
        assert!(desc.contains("Second line"));
        // Should stop at the non-indented `license:` line.
        assert!(!desc.contains("license"));
        assert_eq!(extract_yaml_field(content, "name").as_deref(), Some("writer"));
    }

    #[test]
    fn multiline_folded_block_joins_with_spaces() {
        let content = "---\ndescription: >\n  One\n  Two\nnext: x\n---\n";
        let desc = extract_yaml_field(content, "description").unwrap();
        assert_eq!(desc, "One Two");
    }

    #[test]
    fn missing_frontmatter_returns_none() {
        assert_eq!(extract_yaml_field("no front matter here", "name"), None);
    }

    #[test]
    fn missing_field_returns_none() {
        let content = "---\nname: only name\n---\n";
        assert_eq!(extract_yaml_field(content, "description"), None);
    }

    #[test]
    fn scan_skills_prunes_git_and_node_modules() {
        let root = temp_root("skillpack-scan-prune");
        let real = root.join("skills").join("real");
        let git_skill = root.join(".git").join("hooks").join("fake");
        let nm_skill = root.join("node_modules").join("pkg").join("dep-skill");
        for d in [&real, &git_skill, &nm_skill] {
            fs::create_dir_all(d).unwrap();
            fs::write(
                d.join("SKILL.md"),
                "---\nname: x\ndescription: y\n---\n",
            )
            .unwrap();
        }

        let skills = scan_skills(&root);
        assert_eq!(skills.len(), 1, "should only pick up the real skill");
        assert_eq!(skills[0].dir_name, "real");

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn detect_skill_root_returns_relative_path() {
        let root = temp_root("skillpack-detect-root");
        for skill in ["alpha", "beta"] {
            let dir = root.join("nested").join("skills").join(skill);
            fs::create_dir_all(&dir).unwrap();
            fs::write(dir.join("SKILL.md"), "---\nname: x\n---\n").unwrap();
        }

        let detected = detect_skill_root(&root).unwrap();
        // The common parent is nested/skills; one level up => nested.
        assert_eq!(detected.replace('\\', "/"), "nested");

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn detect_skill_root_none_when_skills_at_root() {
        let root = temp_root("skillpack-detect-root-flat");
        let dir = root.join("alpha");
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("SKILL.md"), "---\nname: x\n---\n").unwrap();

        // skill folders directly under repo root => parent is "" => None.
        assert_eq!(detect_skill_root(&root), None);

        let _ = fs::remove_dir_all(root);
    }
}
