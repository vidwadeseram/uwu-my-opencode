#!/usr/bin/env bash
set -euo pipefail

if [[ "$(uname -s)" != "Linux" ]]; then
  echo "[uwu] Linux only." >&2; exit 1
fi

if [[ $EUID -eq 0 ]]; then
  echo "[uwu] Run as a normal user with sudo, not root." >&2; exit 1
fi

HOME_DIR="${HOME:-/root}"

if ! command -v cargo &>/dev/null; then
  echo "[uwu] installing rust..."
  curl https://sh.rustup.rs -sSf | sh -s -- -y
  export PATH="$HOME_DIR/.cargo/bin:$PATH"
fi

echo "[uwu] installing uwu-daemon..."
cargo install --git https://github.com/vidwadeseram/uwu-my-opencode --path daemon uwu-daemon

echo "[uwu] running installer..."
uwu-daemon install "$@"
