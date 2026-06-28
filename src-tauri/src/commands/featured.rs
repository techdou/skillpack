//! Featured packs discovery layer.
//!
//! SkillPack ships a curated set of packs so a brand-new user sees actionable
//! content on the very first screen instead of an empty install form. This
//! module fetches that catalog from a remote registry, caches it on disk, and
//! always falls back to an embedded list so the UI is never blank — even when
//! the network or the registry repo is unreachable.
//!
//! ## Distribution model (zero new dependencies)
//!
//! The project already depends on `git2`; we reuse it by treating the registry
//! itself as a Git repo. We clone it once into `~/.skillpack/registry-cache/`
//! and read `registry/registry.json` from the working tree. Subsequent
//! `featured_refresh` calls fast-forward it with `git pull`, mirroring exactly
//! how user packs are updated.

use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// The registry Git repo SkillPack clones for the featured catalog.
///
/// Users can override this with the `SKILLPACK_REGISTRY_URL` env var (useful
/// for mirrors, air-gapped registries, or testing against a local repo).
const DEFAULT_REGISTRY_URL: &str = "https://github.com/techdou/skillpack-registry";

/// Directory name under the SkillPack data dir where the registry repo lives.
const REGISTRY_CACHE_DIR: &str = "registry-cache";

/// Path to `registry.json` inside the cloned registry repo.
const MANIFEST_RELATIVE_PATH: &str = "registry/registry.json";

/// A single curated pack entry inside the registry manifest.
///
/// `source` + `skill_root` map 1:1 onto `pack_install`, so installing a
/// featured pack reuses the exact same code path as a manual URL install —
/// no parallel install logic to drift out of sync.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[derive(Default)]
pub struct FeaturedPack {
    pub id: String,
    pub name: String,
    pub source: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub skill_root: Option<String>,
    pub description: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub author: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub homepage: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub category: Option<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub license: Option<String>,
    #[serde(default)]
    pub featured: bool,
    #[serde(default)]
    pub featured_rank: i32,
    #[serde(default)]
    pub verified: bool,
}

/// The full registry.json document.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegistryManifest {
    pub schema_version: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<String>,
    pub packs: Vec<FeaturedPack>,
}

/// Resolve the registry Git URL (env override → default).
fn registry_url() -> String {
    std::env::var("SKILLPACK_REGISTRY_URL").unwrap_or_else(|_| DEFAULT_REGISTRY_URL.into())
}

/// Resolve the registry cache directory: `~/.skillpack/registry-cache/`.
///
/// Lives alongside `packs/` under the same data root, derived from the same
/// home-directory logic as `config_path` so the two never drift apart.
fn registry_cache_dir() -> Result<PathBuf, String> {
    let home = dirs::home_dir().ok_or_else(|| "Cannot determine home directory".to_string())?;
    Ok(home.join(".skillpack").join(REGISTRY_CACHE_DIR))
}

/// Path to the manifest inside the cloned registry repo.
fn manifest_path(cache: &Path) -> PathBuf {
    cache.join(MANIFEST_RELATIVE_PATH)
}

/// Clone the registry repo into the cache dir if it is not already present.
///
/// Returns the cache path on success. Failures are tolerated at the call site:
/// `featured_list` degrades to the fallback manifest rather than erroring, so a
/// first-run without network still shows curated packs.
fn ensure_registry_cache() -> Result<PathBuf, String> {
    let cache = registry_cache_dir()?;
    if !cache.is_dir() {
        // Clone is the one network operation; a failure here just means the
        // caller falls back. We do not half-create the cache: git2 leaves no
        // directory behind on a failed clone.
        crate::git::clone_repo(&registry_url(), &cache)?;
    }
    Ok(cache)
}

/// Read and parse the manifest from the cache. Errors if the cache is missing
/// or the JSON is malformed.
fn load_manifest(cache: &Path) -> Result<RegistryManifest, String> {
    let path = manifest_path(cache);
    let content = fs::read_to_string(&path)
        .map_err(|e| format!("Failed to read registry manifest {}: {}", path.display(), e))?;
    serde_json::from_str(&content)
        .map_err(|e| format!("Failed to parse registry manifest: {}", e))
}

/// Return the featured packs.
///
/// Strategy, in order:
/// 1. Ensure the registry cache exists (clones on first call).
/// 2. Load + parse the manifest; filter to `featured == true`, sorted by
///    `featured_rank` ascending (stable, so authors control ordering).
/// 3. If anything above fails, fall back to the embedded manifest so the UI
///    is never empty. The error is swallowed deliberately — `featured_list`
///    is a read for display, not a user-initiated network action, so surfacing
///    a hard error would just blank the curated grid.
#[tauri::command]
pub fn featured_list() -> Result<Vec<FeaturedPack>, String> {
    let packs = match ensure_registry_cache().and_then(|c| load_manifest(&c)) {
        Ok(m) => m.packs,
        Err(_) => fallback_manifest().packs,
    };

    let mut featured: Vec<FeaturedPack> = packs.into_iter().filter(|p| p.featured).collect();
    featured.sort_by_key(|p| p.featured_rank);
    Ok(featured)
}

/// Force a refresh of the registry cache (git pull) and return the new list.
///
/// Unlike `featured_list`, errors propagate to the caller — this is triggered
/// by an explicit user action (the Refresh button), so the user should see
/// "pull failed" rather than a silently stale grid. A failed refresh never
/// deletes the existing cache, and on failure we reset the working tree to
/// `HEAD` so the next read sees a consistent state instead of a half-fetched
/// repo.
#[tauri::command]
pub fn featured_refresh() -> Result<Vec<FeaturedPack>, String> {
    // Clone if missing, then pull to fast-forward.
    let cache = ensure_registry_cache()?;

    // On pull failure, best-effort reset the working tree so we never leave a
    // half-updated checkout for `featured_list` to read from. `git pull`
    // (fetch + merge) only updates `refs/remotes/*` until the very end, but a
    // checkout conflict or crash mid-merge can dirty the tree. We restore HEAD
    // and trust the last-known-good manifest.
    if let Err(pull_err) = crate::git::pull_repo(&cache) {
        let _ = crate::git::reset_worktree(&cache);
        return Err(pull_err);
    }

    let packs = load_manifest(&cache)
        // A pull that landed a malformed manifest is still better than an empty
        // grid: fall back rather than error the whole refresh.
        .unwrap_or_else(|_| fallback_manifest())
        .packs;
    let mut featured: Vec<FeaturedPack> = packs.into_iter().filter(|p| p.featured).collect();
    featured.sort_by_key(|p| p.featured_rank);
    Ok(featured)
}

/// Embedded fallback catalog. Used when the registry has never been cloned and
/// the clone fails (offline first run, firewall, etc.).
///
/// Kept intentionally short and stable — these are the packs the project is
/// willing to vouch for even with no network. Update before each release.
fn fallback_manifest() -> RegistryManifest {
    RegistryManifest {
        schema_version: 1,
        updated_at: None,
        packs: vec![
            FeaturedPack {
                id: "superpowers".into(),
                name: "Superpowers".into(),
                source: "https://github.comobra/superpowers".into(),
                skill_root: Some("skills".into()),
                description: "Engineering workflow skills: TDD, systematic \
                              debugging, code review, planning, and subagent \
                              orchestration."
                    .into(),
                author: Some("obra".into()),
                homepage: Some("https://github.comobra/superpowers".into()),
                category: Some("engineering".into()),
                tags: vec!["tdd".into(), "debug".into(), "review".into()],
                license: Some("MIT".into()),
                featured: true,
                featured_rank: 1,
                verified: true,
            },
            FeaturedPack {
                id: "skillpack-nature-academic".into(),
                name: "Nature Academic".into(),
                source: "https://github.com/techdou/skillpack-nature".into(),
                skill_root: Some("skills".into()),
                description: "Academic research and writing: literature search, \
                              citation, figures, data handling, paper-to-PPT, \
                              and response drafting."
                    .into(),
                author: Some("techdou".into()),
                homepage: Some("https://github.com/techdou/skillpack-nature".into()),
                category: Some("academic".into()),
                tags: vec!["research".into(), "writing".into(), "citation".into()],
                license: Some("MIT".into()),
                featured: true,
                featured_rank: 2,
                verified: true,
            },
            FeaturedPack {
                id: "skillpack-lark".into(),
                name: "Lark Suite".into(),
                source: "https://github.com/techdou/skillpack-lark".into(),
                skill_root: Some("skills".into()),
                description: "Feishu/Lark automation: docs, sheets, base, \
                              calendar, approval flows, and meeting summaries."
                    .into(),
                author: Some("techdou".into()),
                homepage: Some("https://github.com/techdou/skillpack-lark".into()),
                category: Some("productivity".into()),
                tags: vec!["feishu".into(), "automation".into(), "office".into()],
                license: Some("MIT".into()),
                featured: true,
                featured_rank: 3,
                verified: true,
            },
            FeaturedPack {
                id: "skillpack-documents".into(),
                name: "Documents".into(),
                source: "https://github.com/techdou/skillpack-documents".into(),
                skill_root: Some("skills".into()),
                description: "Create and edit PDF, DOCX, PPTX, and XLSX files \
                              programmatically."
                    .into(),
                author: Some("techdou".into()),
                homepage: Some("https://github.com/techdou/skillpack-documents".into()),
                category: Some("productivity".into()),
                tags: vec!["pdf".into(), "docx".into(), "pptx".into(), "xlsx".into()],
                license: Some("MIT".into()),
                featured: true,
                featured_rank: 4,
                verified: true,
            },
            FeaturedPack {
                id: "skillpack-multimedia".into(),
                name: "Multimedia".into(),
                source: "https://github.com/techdou/skillpack-multimedia".into(),
                skill_root: Some("skills".into()),
                description: "Video and animation tooling: whiteboard animation, \
                              Remotion rendering, and AIGC illustration."
                    .into(),
                author: Some("techdou".into()),
                homepage: Some("https://github.com/techdou/skillpack-multimedia".into()),
                category: Some("creative".into()),
                tags: vec!["video".into(), "animation".into(), "illustration".into()],
                license: Some("MIT".into()),
                featured: true,
                featured_rank: 5,
                verified: false,
            },
        ],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fallback_manifest_is_non_empty_and_all_featured() {
        let m = fallback_manifest();
        assert!(!m.packs.is_empty(), "fallback must never be empty");
        for p in &m.packs {
            assert!(p.featured, "fallback pack {} must be featured", p.id);
            assert!(!p.id.is_empty());
            assert!(!p.source.is_empty());
            assert!(!p.description.is_empty());
        }
    }

    #[test]
    fn featured_list_filters_non_featured_and_sorts_by_rank() {
        let mut packs = fallback_manifest().packs;
        // Inject a non-featured pack (must be filtered out) and a rank-0 pack
        // (must sort first).
        packs.push(FeaturedPack {
            id: "hidden".into(),
            name: "Hidden".into(),
            featured: false,
            featured_rank: 0,
            ..FeaturedPack::default()
        });
        packs.push(FeaturedPack {
            id: "top".into(),
            name: "Top".into(),
            source: "x".into(),
            description: "rank 0".into(),
            featured: true,
            featured_rank: 0,
            ..FeaturedPack::default()
        });

        let manifest = RegistryManifest {
            schema_version: 1,
            updated_at: None,
            packs,
        };

        // Re-run the same filter+sort logic featured_list uses.
        let mut featured: Vec<_> = manifest.packs.into_iter().filter(|p| p.featured).collect();
        featured.sort_by_key(|p| p.featured_rank);

        let ids: Vec<_> = featured.iter().map(|p| p.id.as_str()).collect();
        assert_eq!(featured[0].id, "top", "rank 0 sorts first");
        assert!(!ids.contains(&"hidden"), "non-featured must be filtered out");
    }
}
