#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

CMD="${1:-dev}"

case "$CMD" in
  dev)
    npm run tauri:dev
    ;;
  build)
    npm run build
    npm run tauri:build
    ;;
  test)
    npx tsc --noEmit
    npm run build
    (cd src-tauri && cargo test)
    ;;
  check-win)
    (cd src-tauri && cargo check --target x86_64-pc-windows-gnu)
    ;;
  *)
    echo "Usage: ./scripts/dev.sh [dev|build|test|check-win]"
    exit 1
    ;;
esac
