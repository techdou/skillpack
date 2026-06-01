import { invoke } from "@tauri-apps/api/core";

// Types
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

export interface SkillLinkInfo {
  skill_name: string;
  pack: string;
  target: string;
  target_dir: string;
}

export interface PluginEntry {
  key: string;
  name: string;
  source: string;
  enabled: boolean;
}

export interface AppConfig {
  version: number;
  packs_dir: string;
  default_targets: string[];
  projects: Record<string, ProjectConfig>;
  packs: Record<string, PackInfo>;
  codex_config_path: string | null;
}

export interface ProjectConfig {
  targets: Record<string, string>;
  links: Record<string, { pack: string; target: string }>;
}

const isTauriRuntime = () =>
  typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

const defaultConfig = (): AppConfig => ({
  version: 1,
  packs_dir: "~/.skillpack/packs",
  default_targets: ["codex", "agents"],
  projects: {},
  packs: {},
  codex_config_path: "~/.codex/config.toml",
});

const invokeOrPreview = async <T,>(command: string, args?: Record<string, unknown>): Promise<T> => {
  if (isTauriRuntime()) {
    return invoke<T>(command, args);
  }

  // Browser preview has no Tauri bridge. Keep read-only pages renderable for UI review.
  switch (command) {
    case "pack_list":
      return [] as T;
    case "project_list":
      return [] as T;
    case "plugin_list":
      return [] as T;
    case "skill_status":
      return [] as T;
    case "config_get":
      return defaultConfig() as T;
    default:
      throw new Error("This action requires the Tauri desktop runtime.");
  }
};

// Pack commands
export const packInstall = (source: string, name: string, skillRoot?: string) =>
  invokeOrPreview<PackInfo>("pack_install", { source, name, skillRoot });

export const packList = () =>
  invokeOrPreview<[string, PackInfo][]>("pack_list");

export const packRemove = (name: string) =>
  invokeOrPreview<void>("pack_remove", { name });

export const packUpdate = (name?: string) =>
  invokeOrPreview<string[]>("pack_update", { name });

// Link commands
export const skillLink = (project: string, skillName: string, pack: string, target: string) =>
  invokeOrPreview<void>("skill_link", { project, skillName, pack, target });

export const skillUnlink = (project: string, skillName: string) =>
  invokeOrPreview<void>("skill_unlink", { project, skillName });

export const skillStatus = (project: string) =>
  invokeOrPreview<SkillLinkInfo[]>("skill_status", { project });

// Project commands
export const projectAdd = (path: string) =>
  invokeOrPreview<ProjectInfo>("project_add", { path });

export const projectRemove = (path: string) =>
  invokeOrPreview<void>("project_remove", { path });

export const projectList = () =>
  invokeOrPreview<ProjectInfo[]>("project_list");

// Plugin commands
export const pluginList = () =>
  invokeOrPreview<PluginEntry[]>("plugin_list");

export const pluginToggle = (key: string, enabled: boolean) =>
  invokeOrPreview<void>("plugin_toggle", { key, enabled });

// Config commands
export const configGet = () =>
  invokeOrPreview<AppConfig>("config_get");

export const configSet = (config: AppConfig) =>
  invokeOrPreview<void>("config_set", { config });
