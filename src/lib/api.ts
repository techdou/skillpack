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
  key: string;
  name: string;
  source: string;
  enabled: boolean;
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
      return [] as T;
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

export const pluginToggle = (key: string, enabled: boolean) =>
  invokeOrPreview<void>("plugin_toggle", { key, enabled });

export const configGet = () =>
  invokeOrPreview<AppConfig>("config_get");

/** Update only the user-facing settings fields (packs/projects are preserved). */
export const configUpdateSettings = (settings: SettingsUpdate) =>
  invokeOrPreview<void>("config_update_settings", { settings });

/** Single source of truth for the app version (reads Cargo.toml version). */
export const appVersion = () =>
  invokeOrPreview<string>("app_version");
