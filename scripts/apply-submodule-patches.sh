#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

apply_set() {
  local name="$1"
  local repo="$ROOT/$name"
  local dir="$ROOT/patches/$name"

  if [[ ! -d "$dir" ]]; then
    return 0
  fi

  if [[ ! -d "$repo/.git" ]]; then
    echo "[uwu] skip $name patches (submodule not initialized)"
    return 0
  fi

  echo "[uwu] applying patches for $name"
  shopt -s nullglob
  local patches=("$dir"/*.patch)
  shopt -u nullglob

  if [[ ${#patches[@]} -eq 0 ]]; then
    echo "[uwu] no patch files for $name"
    return 0
  fi

  for patch in "${patches[@]}"; do
    if git -C "$repo" apply --check --reverse "$patch" >/dev/null 2>&1; then
      echo "[uwu] already applied: $(basename "$patch")"
      continue
    fi

    git -C "$repo" apply --check "$patch"
    git -C "$repo" apply "$patch"
    echo "[uwu] applied: $(basename "$patch")"
  done
}

apply_set "opencode"
apply_set "oh-my-opencode"
apply_set "tmux"
apply_set "openagentscontrol"
