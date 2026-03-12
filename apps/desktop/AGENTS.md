# Agent Guidance — Tauri Desktop App

## Overview

`apps/desktop` is a Tauri 2 desktop application that packages the SvelteKit web frontend (`apps/web`) as a native app for Windows, macOS, and Linux.

**Key principle:** The desktop app does NOT maintain its own UI. It consumes the built output from `apps/web`.

## Architecture

```
apps/desktop/
├── src-tauri/           # Rust Tauri backend
│   ├── Cargo.toml       # Desktop crate manifest
│   ├── tauri.conf.json  # Tauri v2 configuration
│   └── src/
│       └── main.rs      # Tauri entry point
├── scripts/
│   └── prepare-frontend.mjs  # Builds web and copies to dist/
└── dist/                # Built web frontend (git-ignored)
```

## Frontend Build Flow

The desktop app has NO direct frontend source files. Instead:

1. `scripts/prepare-frontend.mjs` runs `pnpm run build` in `../web`
2. Copies `apps/web/build/` to `apps/desktop/dist/`
3. Tauri config points `frontendDist` to `./dist`
4. `pnpm run build` (or `bundle`) packages the desktop app with that frontend

**To change the desktop UI:** Edit `apps/web`, not files in this directory.

## Build and Test Commands

```bash
# Debug build (requires Rust + platform SDK)
pnpm -C apps/desktop build

# Release bundle (installer/dmg/deb)
pnpm -C apps/desktop bundle

# Dev mode with hot reload
pnpm -C apps/desktop tauri dev
```

**Note:** Build requires:
- Rust toolchain (stable)
- Platform-specific dependencies:
  - **macOS:** Xcode command-line tools
  - **Linux:** GTK 3, WebKit2GTK, libssl-dev, etc.
  - **Windows:** Windows SDK

## CI Integration

Desktop builds run in a separate CI job with a matrix strategy:

```yaml
strategy:
  matrix:
    os: [windows-latest, macos-latest]
runs-on: ${{ matrix.os }}
```

**Important:** Desktop is excluded from Linux CI runs because of GTK/WebKit system dependencies. Linux builds are possible but require manual dependency installation.

`Cargo.toml` workspace configuration:

```toml
members = [
  # ...
  "apps/desktop/src-tauri",
]
# Desktop NOT in default-members (excluded on Linux CI)
```

## Configuration

### Tauri Config (`src-tauri/tauri.conf.json`)

- Tauri **v2** config format (not v1)
- `frontendDist` points to `../dist` (prepared by `prepare-frontend.mjs`)
- `devUrl` for dev mode (hot reload)
- App identifier, window settings, bundle config

### Cargo Manifest (`src-tauri/Cargo.toml`)

```toml
[package]
name = "telescope-desktop"
version = "0.0.1"
edition.workspace = true

[dependencies]
tauri = { version = "2", features = ["..."] }
serde = { version = "1", features = ["derive"] }
# Add other desktop-specific Rust deps here
```

## Current State (v0.0.1)

**What works:**
- Tauri 2 shell with basic window management
- Packaging the SvelteKit web build as native app
- CI builds on Windows and macOS

**What's NOT implemented yet:**
- Native menu integration
- System tray support
- Desktop-specific IPC commands (Tauri commands)
- Auto-update mechanism
- Deep OS integration (notifications, file system access, etc.)

## Testing

Currently, the desktop app has **no dedicated tests**:

- `pnpm test` in this directory is a no-op
- Integration testing happens at the `apps/web` level (Playwright E2E)
- Manual smoke testing required for native packaging

**Future:** Add Tauri-specific integration tests using WebDriver or similar.

## Development Workflow

1. **Work in `apps/web` first:** Make UI changes in the SvelteKit app and test with `pnpm -C apps/web dev`
2. **Test in desktop context:** Run `pnpm -C apps/desktop tauri dev` to see changes in the native wrapper
3. **Verify packaging:** Periodically run `pnpm -C apps/desktop bundle` to catch platform-specific issues

## Platform-Specific Notes

### macOS
- Requires code signing for distribution (developer ID)
- DMG or PKG output
- Apple Silicon (arm64) and Intel (x86_64) builds

### Windows
- MSI installer output
- Windows Defender may flag unsigned builds
- Requires Windows SDK

### Linux
- AppImage, .deb, or .rpm packaging
- Requires GTK 3, WebKit2GTK, and other system libs
- Not currently built in CI (manual/Docker builds only)

## Code Conventions

- Minimal Rust code in `src-tauri/src/main.rs` — mostly Tauri boilerplate
- Add Tauri commands only when native capabilities are needed (file system, system APIs)
- Keep UI logic in `apps/web` — the desktop app is just a packaging layer

## What's Missing

- Real test coverage (no unit or integration tests)
- System tray and native menus
- Desktop-specific Tauri commands
- Auto-update mechanism
- Linux CI builds
- Deep native integrations (OS notifications, file dialogs, etc.)
