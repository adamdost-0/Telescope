# Retro: M0 – Foundations

- Link: https://github.com/adamdost-0/Telescope/milestone/1
- Closed: 2026-03-10

## What shipped
- Private repo + baseline docs (PRD/Architecture/Roadmap/Security/UX)
- CI (rust/web/e2e/desktop builds) + tag-based release workflow
- pnpm workspace migration + pnpm lockfile
- Container-first local dev loop (`scripts/dev-test.sh` + `tools/devtest/Dockerfile`)
- Desktop build fixes for Tauri v2 across Windows/macOS (frontendDist isolation, config location, icons)

## What went well
- Fast iteration on CI failures with clear root-cause analysis.
- Early separation of concerns: rust core checks on Linux; desktop builds on Win/Mac.
- Container dev loop reduced “works on my machine” risk.

## What went wrong / surprised us
- Tauri v2 is strict about `frontendDist` containing `node_modules`/`src-tauri`.
- Placeholder icon initially malformed caused a proc-macro panic during build.
- GitHub Actions `setup-node cache: pnpm` requires pnpm present during setup; ordering matters.

## Lessons / decisions
- Keep desktop assets isolated (`dist/`) and generated via a deterministic prebuild step.
- Prefer pnpm for override control + workspace consistency.
- Ship CI + E2E alongside feature changes; no “fix pipeline later”.

## Follow-ups
- M1 issues: #9 #10 #11 #12
