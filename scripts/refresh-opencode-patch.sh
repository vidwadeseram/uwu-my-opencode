#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
REPO="$ROOT/opencode"
OUT="$ROOT/patches/opencode/0001-report-workspace-ui-and-html-routing.patch"

if [[ ! -d "$REPO/.git" ]]; then
  echo "[uwu] opencode submodule is missing. Run: git submodule update --init --recursive"
  exit 1
fi

mkdir -p "$ROOT/patches/opencode"

paths=(
  ".github/workflows/test.yml"
  "README.md"
  "packages/app/e2e/app/home.spec.ts"
  "packages/app/package.json"
  "packages/app/playwright.config.ts"
  "packages/app/script/e2e-local.ts"
  "packages/app/src/pages/home.tsx"
  "packages/app/src/pages/layout.tsx"
  "packages/console/app/src/routes/workspace/[id].tsx"
  "packages/console/app/src/routes/workspace/[id]/reports"
  "packages/opencode/src/cli/cmd/report.ts"
  "packages/opencode/src/index.ts"
  "packages/opencode/src/report"
  "packages/opencode/src/server/routes/report.ts"
  "packages/opencode/src/server/server.ts"
  "packages/opencode/test/report"
)

git -C "$REPO" add -N -- "${paths[@]}"
git -C "$REPO" diff --binary HEAD -- "${paths[@]}" > "$OUT"

echo "[uwu] refreshed patch: $OUT"
