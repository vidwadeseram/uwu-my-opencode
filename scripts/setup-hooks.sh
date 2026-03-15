#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "$0")/.." && pwd)"

git -C "$ROOT_DIR" config core.hooksPath .githooks
chmod +x "$ROOT_DIR/.githooks/pre-commit"

echo "Git hooks configured: core.hooksPath=.githooks"
echo "pre-commit hook enabled"
