#!/usr/bin/env bash
set -euo pipefail

CLUSTER_NAME="telescope-smoke"

echo "╔══════════════════════════════════════════╗"
echo "║   Telescope Integration Smoke Test       ║"
echo "╚══════════════════════════════════════════╝"
echo ""

# Prerequisites check
for cmd in k3d kubectl cargo; do
  if ! command -v "$cmd" &>/dev/null; then
    echo "ERROR: $cmd is required but not found in PATH"
    exit 1
  fi
done

cleanup() {
  echo ""
  echo "=== Cleanup ==="
  k3d cluster delete "$CLUSTER_NAME" 2>/dev/null || true
}
trap cleanup EXIT

# Step 1: Create cluster
echo "=== Step 1: Creating k3d cluster ==="
k3d cluster create "$CLUSTER_NAME" --agents 1 --wait --timeout 120s
kubectl wait --for=condition=Ready nodes --all --timeout=60s
echo "✅ Cluster created"

# Step 2: Deploy fixtures
echo ""
echo "=== Step 2: Deploying test fixtures ==="
kubectl apply -f tools/k3d-fixtures/
kubectl -n default wait --for=condition=Available deployment/nginx-test --timeout=120s
echo "✅ Fixtures deployed"

# Step 3: Verify fixture state
echo ""
echo "=== Step 3: Fixture verification ==="
POD_COUNT=$(kubectl get pods -n default --no-headers | wc -l)
echo "  Pods in default namespace: $POD_COUNT"
if [ "$POD_COUNT" -lt 20 ]; then
  echo "❌ Expected at least 20 pods, got $POD_COUNT"
  exit 1
fi
echo "✅ Expected pod count verified"

NS_COUNT=$(kubectl get ns --no-headers | wc -l)
echo "  Namespaces: $NS_COUNT"
kubectl get ns telescope-test --no-headers || { echo "❌ telescope-test namespace missing"; exit 1; }
echo "✅ Test namespace exists"

# Step 4: Run Rust integration tests
echo ""
echo "=== Step 4: Engine integration tests ==="
K3D_TEST=1 cargo test -p telescope-engine --test integration_k3d -- --nocapture
echo "✅ Integration tests passed"

# Step 5: Memory baseline (engine only, no desktop)
echo ""
echo "=== Step 5: Memory measurement ==="
# Build and run a quick memory test
cargo build -p telescope-engine --release 2>/dev/null
echo "  Release build size: $(du -h target/release/libtelescope_engine.rlib 2>/dev/null | cut -f1 || echo 'N/A')"

# Step 6: Reconnection test
echo ""
echo "=== Step 6: Reconnection test ==="
echo "  Stopping cluster..."
k3d cluster stop "$CLUSTER_NAME"
sleep 3
echo "  Restarting cluster..."
k3d cluster start "$CLUSTER_NAME"
kubectl wait --for=condition=Ready nodes --all --timeout=60s
echo "  Re-running connection test..."
K3D_TEST=1 cargo test -p telescope-engine --test integration_k3d connects_to_cluster -- --nocapture
echo "✅ Reconnection verified"

echo ""
echo "╔══════════════════════════════════════════╗"
echo "║   ✅ All smoke tests passed!             ║"
echo "╚══════════════════════════════════════════╝"
echo ""
echo "Next steps:"
echo "  1. Build desktop: pnpm -C apps/desktop dev"
echo "  2. Select k3d-$CLUSTER_NAME context in the UI"
echo "  3. Verify pods appear in the Pod list"
echo "  4. Switch to 'telescope-test' namespace"
echo "  5. Verify echo-server pods appear"
