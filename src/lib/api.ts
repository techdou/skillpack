import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";

export interface PackInfo {
  source: string;
  type: string;
  installed_at: string;
  skill_root: string | null;
  skills: string[];
}

export interface ProjectInfo {
  path: string;
  name: string;
  linked_skills_count: number;
  targets: string[];
}

/**
 * How a link was materialized on disk.
 *  - "symlink": live — follows the pack on update.
 *  - "copy":    snapshot — refreshed on pack update, not live.
 */
export type LinkType = "symlink" | "copy";

export interface SkillLinkInfo {
  skill_name: string;
  pack: string;
  target: string;
  target_dir: string;
  link_type: LinkType;
}

export interface SkillRootInfo {
  key: string;
  label: string;
  path: string;
  exists: boolean;
  skill_count: number;
}

export interface SkillEntry {
  name: string;
  description: string;
  dir_name: string;
  path: string;
}

export interface PluginEntry {
  /** Full config key, e.g. `vercel@openai-curated`. */
  key: string;
  /** Plugin name, the part before `@`. */
  name: string;
  /** Marketplace / source name, the part after `@`. */
  source: string;
  enabled: boolean;
  /** Whether the cache directory exists for this plugin. */
  installed: boolean;
  version?: string;
  description?: string;
  category?: string;
  author_name?: string;
  capabilities: string[];
  bundled_skills: string[];
  installed_path?: string;
}

/** Master switch + plugins, fetched in one round-trip. */
export interface PluginOverview {
  features_plugins_enabled: boolean;
  plugins: PluginEntry[];
}

/** A configured marketplace from `[marketplaces.*]`. */
export interface MarketplaceEntry {
  name: string;
  source?: string;
  ref?: string;
  path?: string;
}

/** A top-level `[mcp_servers.*]` entry. */
export interface McpServerEntry {
  name: string;
  command?: string;
  args: string[];
  url?: string;
  type?: string;
  env_keys: string[];
  enabled: boolean;
}

/** Payload for adding an MCP server. */
export interface McpAdd {
  name: string;
  type?: string;
  command?: string;
  args: string[];
  url?: string;
}

export interface ProjectLink {
  pack: string;
  target: string;
  link_type?: LinkType;
}

export interface ProjectConfig {
  targets: Record<string, string>;
  links: Record<string, ProjectLink>;
}

export interface AppConfig {
  version: number;
  packs_dir: string;
  default_targets: string[];
  projects: Record<string, ProjectConfig>;
  packs: Record<string, PackInfo>;
  codex_config_path: string | null;
}

/** Result of a pack update sweep. One failed pack no longer aborts the rest. */
export interface UpdateReport {
  updated: string[];
  failed: { pack: string; error: string }[];
}

/**
 * A curated pack entry from the registry. `source` + `skill_root` map directly
 * onto `packInstall`, so installing a featured pack reuses the same code path
 * as a manual URL install.
 *
 * `installed` is derived on the client: it's true when a pack with the same
 * `name` already appears in `packList()`.
 */
export interface FeaturedPack {
  id: string;
  name: string;
  source: string;
  skill_root?: string;
  description: string;
  author?: string;
  homepage?: string;
  category?: string;
  tags: string[];
  license?: string;
  featured: boolean;
  featured_rank: number;
  verified: boolean;
  /** Derived client-side: whether a pack with this name is already installed. */
  installed?: boolean;
}

/** Field-scoped settings payload (only these are user-editable from the UI). */
export interface SettingsUpdate {
  packs_dir: string;
  codex_config_path: string | null;
  default_targets: string[];
}

const isTauriRuntime = () =>
  typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

const defaultConfig = (): AppConfig => ({
  version: 1,
  packs_dir: "~/.skillpack/packs",
  default_targets: ["codex", "claude", "gemini"],
  projects: {},
  packs: {},
  codex_config_path: "~/.codex/config.toml",
});

/**
 * Sample featured packs for the non-Tauri preview (browser `npm run dev`),
 * so the Featured grid renders out of the box during frontend development.
 * Mirrors the Rust fallback_manifest shape.
 */
const previewFeaturedPacks = (): FeaturedPack[] => [
  {
    id: "superpowers",
    name: "Superpowers",
    source: "https://github.comobra/superpowers",
    skill_root: "skills",
    description: "Engineering workflow skills: TDD, systematic debugging, code review, planning, and subagent orchestration.",
    author: "obra",
    homepage: "https://github.comobra/superpowers",
    category: "engineering",
    tags: ["tdd", "debug", "review"],
    license: "MIT",
    featured: true,
    featured_rank: 1,
    verified: true,
  },
  {
    id: "skillpack-nature-academic",
    name: "Nature Academic",
    source: "https://github.com/techdou/skillpack-nature",
    skill_root: "skills",
    description: "Academic research and writing: literature search, citation, figures, data handling, paper-to-PPT, and response drafting.",
    author: "techdou",
    homepage: "https://github.com/techdou/skillpack-nature",
    category: "academic",
    tags: ["research", "writing", "citation"],
    license: "MIT",
    featured: true,
    featured_rank: 2,
    verified: true,
  },
  {
    id: "skillpack-lark",
    name: "Lark Suite",
    source: "https://github.com/techdou/skillpack-lark",
    skill_root: "skills",
    description: "Feishu/Lark automation: docs, sheets, base, calendar, approval flows, and meeting summaries.",
    author: "techdou",
    homepage: "https://github.com/techdou/skillpack-lark",
    category: "productivity",
    tags: ["feishu", "automation", "office"],
    license: "MIT",
    featured: true,
    featured_rank: 3,
    verified: true,
  },
];

const invokeOrPreview = async <T,>(command: string, args?: Record<string, unknown>): Promise<T> => {
  if (isTauriRuntime()) {
    return invoke<T>(command, args);
  }

  switch (command) {
    case "pack_list":
    case "project_list":
    case "plugin_list":
    case "skill_status":
    case "toolchain_skill_roots":
    case "toolchain_skills":
    case "project_skill_roots":
    case "project_skills":
    case "marketplace_list":
    case "mcp_list":
      return [] as T;
    case "plugin_overview":
      return { features_plugins_enabled: true, plugins: [] } as T;
    case "features_plugins_status":
      return true as T;
    case "featured_list":
      return previewFeaturedPacks() as T;
    case "featured_refresh":
      return previewFeaturedPacks() as T;
    case "config_get":
      return defaultConfig() as T;
    case "app_version":
      return "0.1.0" as T;
    default:
      throw new Error("This action requires the Tauri desktop runtime.");
  }
};

export const packInstall = (source: string, name: string, skillRoot?: string) =>
  invokeOrPreview<PackInfo>("pack_install", { source, name, skillRoot });

export const packInstallLocal = (sourceDir: string, name: string, skillRoot?: string) =>
  invokeOrPreview<PackInfo>("pack_install_local", { sourceDir, name, skillRoot });

export const packList = () =>
  invokeOrPreview<[string, PackInfo][]>("pack_list");

export const packOpen = (name: string) =>
  invokeOrPreview<void>("pack_open", { name });

export const packRemove = (name: string) =>
  invokeOrPreview<void>("pack_remove", { name });

export const packUpdate = (name?: string) =>
  invokeOrPreview<UpdateReport>("pack_update", { name });

/** Curated featured packs from the registry. */
export const featuredList = () =>
  invokeOrPreview<FeaturedPack[]>("featured_list");

/** Force-refresh the registry cache and return the new featured list. */
export const featuredRefresh = () =>
  invokeOrPreview<FeaturedPack[]>("featured_refresh");

/** Returns the link type ("symlink" | "copy") so the UI can warn about copies. */
export const skillLink = (project: string, skillName: string, pack: string, target: string) =>
  invokeOrPreview<LinkType>("skill_link", { project, skillName, pack, target });

export const skillUnlink = (project: string, skillName: string) =>
  invokeOrPreview<void>("skill_unlink", { project, skillName });

export const skillStatus = (project: string) =>
  invokeOrPreview<SkillLinkInfo[]>("skill_status", { project });

export const toolchainSkillRoots = () =>
  invokeOrPreview<SkillRootInfo[]>("toolchain_skill_roots");

export const toolchainSkills = (toolchain: string) =>
  invokeOrPreview<SkillEntry[]>("toolchain_skills", { toolchain });

export const projectSkillRoots = (project: string) =>
  invokeOrPreview<SkillRootInfo[]>("project_skill_roots", { project });

export const projectSkills = (project: string, toolchain: string) =>
  invokeOrPreview<SkillEntry[]>("project_skills", { project, toolchain });

export const openPath = (path: string) =>
  invokeOrPreview<void>("open_path", { path });

export const pickDirectory = async () => {
  if (!isTauriRuntime()) {
    return null;
  }
  const selected = await open({ directory: true, multiple: false });
  return typeof selected === "string" ? selected : null;
};

export const projectAdd = (path: string) =>
  invokeOrPreview<ProjectInfo>("project_add", { path });

export const projectRemove = (path: string) =>
  invokeOrPreview<void>("project_remove", { path });

export const projectList = () =>
  invokeOrPreview<ProjectInfo[]>("project_list");

export const pluginList = () =>
  invokeOrPreview<PluginEntry[]>("plugin_list");

export const pluginOverview = () =>
  invokeOrPreview<PluginOverview>("plugin_overview");

export const pluginToggle = (key: string, enabled: boolean) =>
  invokeOrPreview<void>("plugin_toggle", { key, enabled });

/** Install a plugin via the codex CLI. Returns the CLI stdout. */
export const pluginAdd = (name: string, marketplace?: string) =>
  invokeOrPreview<string>("plugin_add", { name, marketplace });

/** Remove a plugin via the codex CLI. */
export const pluginRemove = (key: string) =>
  invokeOrPreview<string>("plugin_remove", { key });

/** Master switch status for `[features]plugins`. */
export const featuresPluginsStatus = () =>
  invokeOrPreview<boolean>("features_plugins_status");

/** Toggle `[features]plugins`. */
export const featuresPluginsToggle = (enabled: boolean) =>
  invokeOrPreview<void>("features_plugins_toggle", { enabled });

export const marketplaceList = () =>
  invokeOrPreview<MarketplaceEntry[]>("marketplace_list");

export const marketplaceAdd = (source: string) =>
  invokeOrPreview<string>("marketplace_add", { source });

export const marketplaceUpgrade = (name?: string) =>
  invokeOrPreview<string>("marketplace_upgrade", { name });

export const marketplaceRemove = (name: string) =>
  invokeOrPreview<string>("marketplace_remove", { name });

export const mcpList = () =>
  invokeOrPreview<McpServerEntry[]>("mcp_list");

export const mcpToggle = (name: string, enabled: boolean) =>
  invokeOrPreview<void>("mcp_toggle", { name, enabled });

export const mcpRemove = (name: string) =>
  invokeOrPreview<void>("mcp_remove", { name });

export const mcpAdd = (entry: McpAdd) =>
  invokeOrPreview<void>("mcp_add", { entry });

export const configGet = () =>
  invokeOrPreview<AppConfig>("config_get");

/** Update only the user-facing settings fields (packs/projects are preserved). */
export const configUpdateSettings = (settings: SettingsUpdate) =>
  invokeOrPreview<void>("config_update_settings", { settings });

/** Single source of truth for the app version (reads Cargo.toml version). */
export const appVersion = () =>
  invokeOrPreview<string>("app_version");
