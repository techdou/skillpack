# SkillPack

SkillPack is a Tauri desktop app and CLI for installing, updating, linking, and
managing AI coding skill packs across local projects.

A *skill pack* is a Git repo (or local directory) of `SKILL.md` skill folders.
SkillPack installs packs into `~/.skillpack/packs` and *links* individual skills
into a project's toolchain skill directory so Codex / Claude / Gemini can use
them.

## Features

- **Discover** curated **Featured Packs** on the Packs page — one-click
  install with no URL needed. The catalog is fetched from a registry Git repo
  and cached at `~/.skillpack/registry-cache/`; if the network or registry is
  unreachable an embedded fallback list still keeps the grid populated, so the
  first run is never blank. Force a refresh with the *Refresh* button
- **Install** packs from a Git URL, a local directory, or the Featured grid
- **Update** packs (fast-forward `git pull`) — one unreachable remote no longer
  aborts the rest; results come back as `{ updated, failed }`
- **Link** skills into a project per toolchain; links are materialized as
  symlinks when possible, falling back to a directory **copy** on Windows
  without developer mode. Copy-type links are refreshed on pack update
- **Manage Codex plugins** by toggling `enabled` in `~/.codex/config.toml`
- **Browse** skills installed locally or inside any project
- Full GUI plus a `spack` CLI that reuses the same core library

## Supported toolchains

| Target  | Skill directory           |
|---------|---------------------------|
| codex   | `.agents/skills`          |
| agents  | `.agents/skills`          |
| gemini  | `.gemini/skills`          |
| claude  | `.claude/skills`          |
| cursor  | `.cursor/skills`          |

> Each toolchain maps to its own directory. Earlier versions mapped Gemini to
> `.agents/skills` (colliding with Codex); it now uses `.gemini/skills`.

## Link types

- **Symlink** — live: follows the pack automatically. The UI shows a green
  *Symlink* badge.
- **Copy** — snapshot: refreshed when the pack is updated. The UI shows an amber
  *Copy* badge. Typical on Windows without developer mode / admin.

## Configuration

SkillPack keeps all state in `~/.skillpack/config.json` (override with the
`SKILLPACK_CONFIG_PATH` env var). The Settings page edits only the user-facing
fields (`packs_dir`, `codex_config_path`, `default_targets`) via
`config_update_settings` — packs and projects entries are never overwritten,
which prevents a stale frontend from clobbering the registry.

The Featured Packs catalog is cloned into `~/.skillpack/registry-cache/`. Point
SkillPack at a different registry repo (mirror, air-gapped, or local) with the
`SKILLPACK_REGISTRY_URL` env var.

## CLI

```
spack install <url> --name <name> [--skill-root <path>]
spack list [--pack <name>]
spack remove <name>
spack update [--name <name>]
spack link <skill> --project <path> --pack <name> [--target codex|agents|claude|gemini|cursor]
spack unlink <skill> --project <path>
spack project add|remove|list ...
spack plugin list|toggle ...
```

## Development

```powershell
npm install
npm run dev
```

## Build

```powershell
npm run build
cd src-tauri
cargo test
cargo build --release
```

For a full Tauri desktop bundle:

```powershell
npm run tauri build
```

> `uuid` may still appear in `Cargo.lock` as a transitive dependency pulled in by
> Tauri itself; it is no longer a direct dependency of SkillPack and cannot be
> removed without dropping Tauri.

## Release Artifacts

Curated executable artifacts are stored in `release/` when generated for handoff.

## Verification

Before shipping changes, run:

```powershell
npm run build
cd src-tauri
cargo check
cargo test
cargo clippy --all-targets -- -D warnings
```
