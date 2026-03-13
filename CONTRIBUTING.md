# Contributing

Thanks for contributing to Telescope.

## Getting Started

### Prerequisites
- Rust stable toolchain
- Node.js 22+
- pnpm 9.15+
- Git
- Tauri platform dependencies:
  - macOS: Xcode Command Line Tools
  - Windows: Microsoft Edge WebView2
  - Linux: WebKitGTK and other Tauri system packages for your distro

### Clone and install
```bash
git clone https://github.com/adamdost-0/Telescope.git
cd Telescope
pnpm install
```

## Development Workflow

### Rust workspace
```bash
cargo fmt --all -- --check
cargo clippy --workspace --exclude telescope-desktop --all-targets --all-features -- -D warnings
cargo test --workspace --exclude telescope-desktop --all-features
```

### Web app
```bash
pnpm -C apps/web dev
pnpm -C apps/web test
pnpm -C apps/web e2e
```

### Desktop app
```bash
pnpm -C apps/desktop build
pnpm -C apps/desktop bundle
```

### Practical notes
- Desktop and web share the `apps/web` frontend. Make UI changes there unless the work is truly Tauri-specific.
- `apps/desktop` packages the built `apps/web` output for the native app.
- Prefer repo-defined commands and existing CI workflows over ad hoc scripts.

## Project Structure

### Cargo workspace
- `crates/core` - shared domain, state, and storage types
- `crates/engine` - Kubernetes engine code: clients, watchers, logs, exec, port-forward, Helm, metrics, CRDs
- `crates/api` - thin facade over `engine` and `core`

### pnpm workspace
- `apps/web` - shared SvelteKit frontend used by desktop and browser mode
- `apps/desktop` - Tauri desktop shell that packages the web frontend
- `apps/hub` - Axum HTTP/WebSocket service for browser/web mode

## Pull Request Process
- Open PRs with the repository PR template in `.github/pull_request_template.md`.
- Describe the change clearly, including scope, user-visible behavior, and any risks.
- Include a short test plan and note whether unit, E2E, or CI changes were needed.
- Ensure the relevant CI checks pass before requesting review.
- Keep PRs focused; split unrelated changes into separate submissions.

## Code Style
- Rust: run `cargo fmt` and fix all `clippy` warnings required by CI.
- TypeScript/Svelte: follow existing patterns in the app, prefer Svelte 5 runes, and keep code straightforward.
- Avoid excessive comments; add them only when they clarify non-obvious behavior.
- When documentation and code differ, treat the current code and CI behavior as the source of truth.
