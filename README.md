# SkillPack

SkillPack is a Tauri desktop app and CLI for installing, updating, linking, and managing AI coding skill packs across local projects.

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

## Release Artifacts

Curated executable artifacts are stored in `release/` when generated for handoff.

## Verification

Before shipping changes, run:

```powershell
npm run build
cd src-tauri
cargo check
cargo test
```
