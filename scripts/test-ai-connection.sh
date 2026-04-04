#!/usr/bin/env bash
# Test Azure OpenAI connection using credentials from .env.local
# Usage: ./scripts/test-ai-connection.sh
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
ENV_FILE="$ROOT_DIR/.env.local"

if [[ ! -f "$ENV_FILE" ]]; then
  echo "ERROR: $ENV_FILE not found."
  echo "Copy .env.example to .env.local and fill in your credentials."
  exit 1
fi

# Source the env file (export vars so curl can see them)
set -a
# shellcheck source=/dev/null
source "$ENV_FILE"
set +a

# Validate required variables
missing=()
[[ -z "${AZURE_OPENAI_ENDPOINT:-}" ]] && missing+=("AZURE_OPENAI_ENDPOINT")
[[ -z "${AZURE_OPENAI_API_KEY:-}" ]] && missing+=("AZURE_OPENAI_API_KEY")
[[ -z "${AZURE_OPENAI_DEPLOYMENT:-}" ]] && missing+=("AZURE_OPENAI_DEPLOYMENT")

if [[ ${#missing[@]} -gt 0 ]]; then
  echo "ERROR: Missing required environment variables:"
  for var in "${missing[@]}"; do
    echo "  - $var"
  done
  exit 1
fi

# Strip trailing slash from endpoint
ENDPOINT="${AZURE_OPENAI_ENDPOINT%/}"
API_VERSION="2024-10-21"
URL="${ENDPOINT}/openai/deployments/${AZURE_OPENAI_DEPLOYMENT}/chat/completions?api-version=${API_VERSION}"

echo "Testing Azure OpenAI connection..."
echo "  Endpoint: ${ENDPOINT}"
echo "  Deployment: ${AZURE_OPENAI_DEPLOYMENT}"
echo "  API version: ${API_VERSION}"
echo ""

HTTP_STATUS=$(curl -s -o /dev/stderr -w "%{http_code}" \
  -X POST "$URL" \
  -H "Content-Type: application/json" \
  -H "api-key: ${AZURE_OPENAI_API_KEY}" \
  -d '{
    "messages": [{"role": "user", "content": "Reply with the single word OK."}],
    "max_tokens": 5,
    "temperature": 0
  }' 2>&1)

# Extract the HTTP status code (last 3 chars of output)
BODY="${HTTP_STATUS%???}"
STATUS="${HTTP_STATUS: -3}"

echo ""
echo "HTTP Status: $STATUS"

if [[ "$STATUS" == "200" ]]; then
  echo "SUCCESS: Azure OpenAI connection is working."
  echo "Response: $BODY"
  exit 0
else
  echo "FAILURE: Received HTTP $STATUS from Azure OpenAI."
  echo "Response: $BODY"
  exit 1
fi
