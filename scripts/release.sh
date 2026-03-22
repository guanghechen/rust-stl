#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "${ROOT_DIR}"

if ! command -v cargo-release >/dev/null 2>&1; then
  echo "error: cargo-release is not installed. run: cargo install cargo-release --locked" >&2
  exit 1
fi

if [[ "${1:-}" == "-h" || "${1:-}" == "--help" ]]; then
  cat <<'EOF'
Usage:
  scripts/release.sh [--execute] [extra cargo-release options]

Examples:
  scripts/release.sh
  scripts/release.sh --execute
  scripts/release.sh --execute --package rstl-collection

Notes:
  - Default mode is dry-run (no --execute).
  - This script is a thin wrapper over:
      cargo release publish --workspace --no-confirm
EOF
  exit 0
fi

execute_flag=""
if [[ "${1:-}" == "--execute" ]]; then
  execute_flag="-x"
  shift
fi

echo "[release] cargo release publish --workspace --no-confirm ${execute_flag} $*"
cargo release publish --workspace --no-confirm ${execute_flag} "$@"
