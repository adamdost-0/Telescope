#!/usr/bin/env bash
set -euo pipefail

CLUSTER_NAME="${1:-telescope-dev}"

echo "=== Creating k3d cluster: $CLUSTER_NAME ==="
k3d cluster create "$CLUSTER_NAME" \
  --agents 1 \
  --wait \
  --timeout 120s

echo "=== Waiting for cluster to be ready ==="
kubectl wait --for=condition=Ready nodes --all --timeout=60s

echo "=== Deploying test fixtures ==="
kubectl apply -f tools/k3d-fixtures/

echo "=== Waiting for deployments ==="
kubectl -n default wait --for=condition=Available deployment/nginx-test --timeout=120s

echo "=== Cluster ready ==="
echo "Context: k3d-$CLUSTER_NAME"
echo ""
kubectl get pods -A
echo ""
echo "To tear down: ./scripts/k3d-teardown.sh $CLUSTER_NAME"
