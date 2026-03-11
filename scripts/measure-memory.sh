#!/usr/bin/env bash
set -euo pipefail

echo "╔══════════════════════════════════════════╗"
echo "║   Telescope Memory Measurement           ║"
echo "╚══════════════════════════════════════════╝"
echo ""
echo "Prerequisites:"
echo "  - k3d cluster running (./scripts/k3d-setup.sh)"
echo "  - Desktop app built (pnpm -C apps/desktop bundle)"
echo ""

BINARY="target/release/telescope-desktop"

if [ ! -f "$BINARY" ] && [ ! -f "${BINARY}.exe" ]; then
  echo "Building release binary..."
  pnpm -C apps/desktop bundle
fi

# Detect platform
if [[ "$OSTYPE" == "msys" ]] || [[ "$OSTYPE" == "cygwin" ]] || [[ -n "${WINDIR:-}" ]]; then
  BINARY="${BINARY}.exe"
  echo "Platform: Windows"
  echo ""
  echo "To measure memory on Windows:"
  echo "  1. Start the app: $BINARY"
  echo "  2. Open Task Manager → Details tab"
  echo "  3. Find 'telescope-desktop.exe'"
  echo "  4. Note 'Memory (Private Working Set)'"
  echo ""
  echo "Expected targets:"
  echo "  - Idle (no cluster):    < 80 MB"
  echo "  - Connected (50 pods):  < 150 MB"
  echo "  - Connected (500 pods): < 250 MB"
  echo "  - Target ceiling:       < 350 MB"
  echo ""
  echo "Compare with Lens:"
  echo "  - Lens idle:            ~400-600 MB"
  echo "  - Lens (50 pods):       ~600-800 MB"
elif [[ "$OSTYPE" == "darwin"* ]]; then
  echo "Platform: macOS"
  echo ""
  echo "To measure memory:"
  echo "  1. Start the app"
  echo "  2. Run: ps aux | grep telescope-desktop | grep -v grep"
  echo "  3. RSS column shows resident memory (KB)"
  echo "  4. Or use Activity Monitor → Memory tab"
else
  echo "Platform: Linux"
  echo ""
  echo "To measure memory:"
  echo "  1. Start the app"
  echo "  2. Run: ps -o pid,rss,vsz,comm -p \$(pgrep telescope-desktop)"
  echo "  3. RSS column shows resident memory (KB)"
fi

echo ""
echo "Memory targets (from PRD):"
echo "  ┌─────────────────────────┬───────────┐"
echo "  │ Scenario                │ Target    │"
echo "  ├─────────────────────────┼───────────┤"
echo "  │ Desktop idle (no conn)  │ < 80 MB   │"
echo "  │ 1 cluster, 50 pods     │ < 150 MB  │"
echo "  │ 1 cluster, 500 pods    │ < 250 MB  │"
echo "  │ PRD ceiling             │ < 350 MB  │"
echo "  └─────────────────────────┴───────────┘"
